use crate::math::vector::Vec3f;
use crate::tracer::hittable::Triangle;
use std::path::Path;

pub fn load_obj(path: &Path) -> Vec<Triangle> {
    const LOAD_OPTIONS: tobj::LoadOptions = tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ignore_lines: true,
        ignore_points: true,
    };

    let (models, _) = tobj::load_obj(path, &LOAD_OPTIONS).expect("Failed to load OBJ file");
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mesh = &models[0].mesh;
    // Have to reorder: We want all triangles to be in order, so use the indices
    // to achieve that
    // Copy triangle positions
    for index in mesh.indices.iter() {
        let vert_idx = (*index as usize) * 3;
        let vert = Vec3f::new(
            mesh.positions[vert_idx],
            mesh.positions[vert_idx + 1],
            mesh.positions[vert_idx + 2],
        );
        positions.push(vert);
    }
    // Do the same for normals
    if !mesh.normals.is_empty() {
        eprintln!("No normals given in obj file {:?}. Generating...", path);
        for index in mesh.indices.iter() {
            let vert_idx = (*index as usize) * 3;
            let normal = Vec3f::new(
                mesh.normals[vert_idx],
                mesh.normals[vert_idx + 1],
                mesh.normals[vert_idx + 2],
            );
            normals.push(normal);
        }
    } else {
        // If there are no normals specified in the file, compute (flat) normals
        // from vertex positions.
        for tri in positions.chunks(3) {
            let edge1 = tri[1] - tri[0];
            let edge2 = tri[2] - tri[0];
            let obj_normal = edge1.cross(edge2).normalize();
            normals.push(obj_normal);
            normals.push(obj_normal);
            normals.push(obj_normal);
        }
    }

    // Group positions and normals
    let mut triangles = Vec::new();
    for i in (0..positions.len()).step_by(3) {
        triangles.push(Triangle {
            positions: [positions[i + 0], positions[i + 1], positions[i + 2]],
            normals: [normals[i + 0], normals[i + 1], normals[i + 2]],
        });
    }

    triangles
}
