// Allow dead code for now
#![allow(dead_code)]

mod io;
mod math;
mod postproc;
mod tracer;
use std::path::{Path, PathBuf};

use crate::postproc::{PostProcessSettings, postprocess};
#[allow(unused_imports)]
use crate::tracer::render::{Scheduler, TiledScheduler};
use crate::tracer::texture::Texture;
use crate::tracer::world::{Background, World};
use math::transform::{Transform, Transformable};
use math::util::Interval;
use math::vector::{Float, Vec3f, vec3};
use tracer::bvh::BVHMesh;
use tracer::camera::{self, Camera};
#[allow(unused_imports)]
use tracer::hittable::{Hittable, Mesh, Parallelepiped, Plane, Sphere};
#[allow(unused_imports)]
use tracer::material::{
    MatBounceDebug, MatFaceDebug, MatGlass, MatLambertDiffuse, MatNormalDebug, MatPrincipled,
    Material, MaterialTrait,
};

fn main() -> Result<(), String> {
    let output_dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("Please provide an output directory as the first argument."),
    );

    let width: u32 = 1280 / 4;
    let height: u32 = 1024 / 4;
    let aspect_ratio: Float = width as Float / height as Float;

    let pose = camera::Pose::look_at(Vec3f::new(0.0, 1.4, 4.0), Vec3f::new(0.0, 0.4, 0.0));
    let lens = camera::Lens::new(15, 35, aspect_ratio, 18.1, 0.0);
    let render_settings = tracer::render::RenderSettings {
        samples_per_pixel: 8,
        max_bounces: 10,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 1000.0),
        denoise: true,
    };

    let postproc_settings = PostProcessSettings { denoise: true };

    let camera = Camera::new(pose, lens);

    // Scene setup
    let mat_floor = MatLambertDiffuse::new(Vec3f::new(1.0, 1.0, 1.0));
    let mat_glass = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.33);
    let mat_inner = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.0 / 1.33);
    let mat_metal = MatPrincipled::new(vec3(0.2, 0.3, 0.9), 1.0, 0.1);
    let mat_rough = MatLambertDiffuse::new(vec3(0.9, 0.9, 0.9));
    let _mat_normal_dbg = MatNormalDebug::new();
    let _mat_face_dbg = MatFaceDebug::new();

    let floor = Parallelepiped::new(
        vec3(-1.5, -1.0, -1.0),
        vec3(1.5, -1.0, -1.0),
        vec3(-1.5, -1.0, 1.0),
        vec3(-1.5, 0.0, -1.0),
        mat_floor,
    );

    let _sphere1: Hittable = Sphere::new(vec3(-1.1, 0.501, 0.0), 0.5, mat_glass.clone());
    let _sphere4: Hittable = Sphere::new(vec3(-1.1, 0.501, 0.0), 0.45, mat_inner);
    let _sphere2: Hittable = Sphere::new(vec3(0.0, 0.5, 0.0), 0.5, mat_rough.clone());
    let _sphere3: Hittable = Sphere::new(vec3(1.1, 0.5, 0.0), 0.5, mat_metal.clone());
    let sponza = BVHMesh::from_obj_file(Path::new("assets/models/bunny.obj"), mat_rough.clone());

    let objects = vec![sponza, floor];

    let env_texture = Texture::from_path(Path::new("assets/hdri/overcastpines_4k.hdr")).unwrap();

    let background = Background::from_environment_texture(env_texture, 0.0);

    let world = World::new(objects, background);

    let render_result = TiledScheduler::new(64).render(camera, render_settings.clone(), &world);
    eprintln!("{}", render_result.to_string());
    let postproc_result = postprocess(&render_result, &postproc_settings);

    render_result.write_images(output_dir.as_path(), ".exr")?;
    postproc_result.write_images(output_dir.as_path())?;

    Ok(())
}
