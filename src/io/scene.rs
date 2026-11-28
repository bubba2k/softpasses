use std::path::Path;

use glam::{Affine3A, Mat4, Vec3};
use gltf::mesh::Mode;

use crate::tracer;
use crate::tracer::camera::{Camera, Lens, Pose};
use crate::tracer::hittable::{Hittable, Triangle};
use crate::tracer::material::{
    GltfNormalTexture, GltfOcclusionTexture, GltfTextureInfo, MatGltfPbr, Material,
};
use crate::tracer::scene::{Background, SceneBuilder};

fn gltf_affine(transform: gltf::scene::Transform) -> Affine3A {
    let matrix = transform.matrix();
    let mut column_major = [0.0f32; 16];
    for column in 0..4 {
        for row in 0..4 {
            column_major[column * 4 + row] = matrix[column][row];
        }
    }
    Affine3A::from_mat4(Mat4::from_cols_array(&column_major))
}

fn texture_info(info: gltf::texture::Info) -> GltfTextureInfo {
    GltfTextureInfo {
        texture_index: info.texture().index(),
        tex_coord: info.tex_coord(),
    }
}

fn convert_materials(document: &gltf::Document) -> Vec<Material> {
    document
        .materials()
        .map(|material| {
            let pbr = material.pbr_metallic_roughness();
            let base_color = pbr.base_color_factor();

            MatGltfPbr::new(
                [base_color[0], base_color[1], base_color[2], base_color[3]],
                pbr.base_color_texture().map(texture_info),
                pbr.metallic_factor().clamp(0.0, 1.0),
                pbr.roughness_factor().clamp(0.0, 1.0),
                pbr.metallic_roughness_texture().map(texture_info),
                material.normal_texture().map(|normal| GltfNormalTexture {
                    texture_index: normal.texture().index(),
                    tex_coord: normal.tex_coord(),
                    scale: normal.scale(),
                }),
                material.occlusion_texture().map(|occlusion| GltfOcclusionTexture {
                    texture_index: occlusion.texture().index(),
                    tex_coord: occlusion.tex_coord(),
                    strength: occlusion.strength(),
                }),
                {
                    let factor = material.emissive_factor();
                    [factor[0], factor[1], factor[2]]
                },
                material.emissive_texture().map(texture_info),
                None,
                format!("{:?}", material.alpha_mode()),
                material.alpha_cutoff(),
                material.double_sided(),
            )
        })
        .collect()
}

fn collect_triangles(
    primitive: gltf::mesh::Primitive,
    buffers: &[gltf::buffer::Data],
    material_id: u32,
    world: Affine3A,
) -> Result<Vec<Hittable>, String> {
    if primitive.mode() != Mode::Triangles {
        return Err(format!(
            "unsupported glTF primitive mode {:?}; only triangles are supported",
            primitive.mode()
        ));
    }

    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0[..]));
    let positions: Vec<[f32; 3]> = reader
        .read_positions()
        .ok_or_else(|| "glTF triangle primitive has no POSITION accessor".to_string())?
        .collect();

    let normals: Option<Vec<[f32; 3]>> = reader.read_normals().map(|iterator| iterator.collect());

    let mut triangle_indices = Vec::<usize>::new();
    if let Some(indices) = reader.read_indices() {
        use gltf::mesh::util::ReadIndices;
        match indices {
            ReadIndices::U8(iter) => triangle_indices.extend(iter.map(|index| index as usize)),
            ReadIndices::U16(iter) => triangle_indices.extend(iter.map(|index| index as usize)),
            ReadIndices::U32(iter) => triangle_indices.extend(iter.map(|index| index as usize)),
        }
    } else {
        triangle_indices.extend(0..positions.len());
    }

    if triangle_indices.len() % 3 != 0 {
        return Err("glTF triangle index sequence has an incomplete triangle".to_string());
    }

    let normal_matrix = world.matrix3.inverse().transpose();
    let mut triangles = Vec::new();

    for chunk in triangle_indices.chunks_exact(3) {
        let mut positions_world = [Vec3::ZERO; 3];
        let mut normals_world = [Vec3::ZERO; 3];

        for vertex in 0..3 {
            let vertex_index = chunk[vertex];
            let position = positions
                .get(vertex_index)
                .ok_or_else(|| format!("glTF index {vertex_index} is outside the position accessor"))?;
            positions_world[vertex] = world.transform_point3(Vec3::from(*position));

            if let Some(normals) = &normals {
                let normal = normals
                    .get(vertex_index)
                    .ok_or_else(|| format!("glTF index {vertex_index} is outside the normal accessor"))?;
                let transformed_normal = normal_matrix * Vec3::from(*normal);
                normals_world[vertex] = transformed_normal.normalize_or_zero();
            }
        }

        if normals.is_none() {
            let edge1 = positions_world[1] - positions_world[0];
            let edge2 = positions_world[2] - positions_world[0];
            let normal = edge1.cross(edge2).normalize_or_zero();
            normals_world = [normal, normal, normal];
        }

        triangles.push(Hittable::Triangle(Triangle {
            positions: positions_world,
            normals: normals_world,
            material_id,
        }));
    }

    Ok(triangles)
}

fn traverse_nodes(
    node: gltf::scene::Node,
    parent_world: Affine3A,
    buffers: &[gltf::buffer::Data],
    default_material_id: u32,
    hittables: &mut Vec<Hittable>,
    camera: &mut Option<Camera>,
) -> Result<(), String> {
    let world = parent_world * gltf_affine(node.transform());

    if let Some(node_camera) = node.camera() {
        if camera.is_none() {
            if let gltf::camera::Projection::Perspective(perspective) = node_camera.projection() {
                let position = world.transform_point3(Vec3::ZERO);
                let direction = world
                    .transform_vector3(Vec3::new(0.0, 0.0, -1.0))
                    .normalize();
                let projected_up = (world.transform_vector3(Vec3::Y)
                    - direction * world.transform_vector3(Vec3::Y).dot(direction))
                    .normalize_or_zero();

                let pose = Pose {
                    position,
                    direction,
                    up: projected_up,
                };
                let lens = Lens {
                    afov_rad: perspective.yfov(),
                    aspect_ratio: perspective.aspect_ratio().unwrap_or(16.0 / 9.0),
                    focal_distance: perspective.znear(),
                    dof: 0.0,
                };

                *camera = Some(Camera::new(pose, lens));
            }
        }
    }

    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let material_id = primitive
                .material()
                .index()
                .map(|index| index as u32)
                .unwrap_or(default_material_id);

            hittables.extend(collect_triangles(primitive, buffers, material_id, world)?);
        }
    }

    for child in node.children() {
        traverse_nodes(
            child,
            world,
            buffers,
            default_material_id,
            hittables,
            camera,
        )?;
    }

    Ok(())
}

pub fn load_gltf(path: &Path) -> Result<(tracer::scene::SceneBuilder, Camera), String> {
    let (document, buffers, _images) = gltf::import(path)
        .map_err(|error| format!("failed to import glTF file {}: {error}", path.display()))?;

    let mut materials = convert_materials(&document);
    let default_material_id = materials.len() as u32;
    materials.push(MatGltfPbr::new(
        [1.0, 1.0, 1.0, 1.0],
        None,
        0.0,
        1.0,
        None,
        None,
        None,
        [0.0, 0.0, 0.0],
        None,
        None,
        "Opaque".to_string(),
        None,
        false,
    ));

    let scene = document
        .default_scene()
        .or_else(|| document.scenes().next())
        .ok_or_else(|| format!("glTF file {} contains no scenes", path.display()))?;

    let mut hittables = Vec::new();
    let mut camera = None;

    for root in scene.nodes() {
        traverse_nodes(
            root,
            Affine3A::IDENTITY,
            &buffers,
            default_material_id,
            &mut hittables,
            &mut camera,
        )?;
    }

    let camera = camera
        .ok_or_else(|| "glTF file contains no perspective camera in the selected scene".to_string())?;

    let scene_builder = SceneBuilder::new()
        .set_materials(materials)
        .set_background(Background::from_solid_color(Vec3::ONE))
        .add_hittables(hittables);

    Ok((scene_builder, camera))
}