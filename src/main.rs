mod io;
mod math;
mod postproc;
mod tracer;
mod bvh;

use std::path::{Path, PathBuf};

use crate::postproc::{PostProcessSettings, postprocess};
use crate::tracer::hittable::make_parallelepiped;
use crate::tracer::pipelines::BVHDebugPipeline;
use crate::tracer::render::RenderSettings;
#[allow(unused_imports)]
use crate::tracer::pipelines::{DefaultPipeline};
#[allow(unused_imports)]
use crate::tracer::render::{Scheduler, TiledScheduler};
use crate::tracer::texture::Texture;
use crate::tracer::scene::{Background, Scene, SceneBuilder};
use math::util::Interval;
use math::vector::{Float, Vec3f, vec3};
use tracer::camera::{self, Camera};
#[allow(unused_imports)]
use tracer::hittable::{Hittable, Plane, Sphere};
#[allow(unused_imports)]
use tracer::material::{ MatGlass, MatLambertDiffuse, MatPrincipled,
    Material, MaterialID
};

#[allow(unused)]
fn sponza_scene_setup() -> (Scene, Camera, RenderSettings) {
    let mat_wall = MatLambertDiffuse::new(vec3(0.9, 0.9, 1.0));
    let materials = vec![mat_wall];
    const MAT_WALL: MaterialID = 0;

    let sponza = io::mesh::load_obj_file(Path::new("assets/models/sponza_merged.obj"), MAT_WALL);

    let env_texture = Texture::from_path(Path::new("assets/hdri/brownstudio_4k.hdr")).unwrap();
    // let background = Background::from_environment_texture(env_texture, 2.85, 0.15);
    let background = Background::from_solid_color(vec3(1.0, 0.8, 0.8));

    let scene = SceneBuilder::new().add_hittables(sponza).set_background(background).set_materials(materials).build();

    // Camera setup
    let width: u32 = 1920 / 1;
    let height: u32 = 1080 / 1;
    let aspect_ratio: Float = width as Float / height as Float;

    let render_settings = tracer::render::RenderSettings {
        samples_per_pixel: 2000,
        max_bounces: 100,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 1000.0),
    };

    let pose = camera::Pose::look_at(Vec3f::new(0.0, 1.0, 1.0), Vec3f::new(0.0, 0.0, 1.0));
    let lens = camera::Lens::new(15, 20, aspect_ratio, 0.0, 0.0);

    let camera = Camera::new(pose, lens);

    return (scene, camera, render_settings);
}

#[allow(unused)]
fn glass_bunny_scene_setup() -> (Scene, Camera, RenderSettings) {
    let mat_glass = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.33);
    let mat_mirror = MatPrincipled::new(vec3(0.9, 0.9, 0.9), 1.0, 0.01);
    let mat_table = MatLambertDiffuse::new(vec3(0.18, 0.18, 0.2));
    let mat_wall = MatLambertDiffuse::new(vec3(0.18, 0.25, 0.2));

    let materials = vec![mat_glass, mat_mirror, mat_table, mat_wall];
    const MAT_GLASS: MaterialID     = 0;
    const MAT_MIRROR: MaterialID    = 1;
    const MAT_TABLE: MaterialID     = 2;
    const MAT_WALL: MaterialID      = 3;

    let table = make_parallelepiped(
        vec3(-3.0, -1.0, -1.0),
        vec3(2.0, -1.0, -1.0),
        vec3(-3.0, -1.0, 1.0),
        vec3(-3.0, 0.0, -1.0),
        MAT_TABLE,
    );

    let right_wall = make_parallelepiped(
        vec3(1.0, 0.0, -1.0),
        vec3(2.0, 0.0, -1.0),
        vec3(1.0, 0.0, 1.0),
        vec3(1.0, 3.0, -1.0),
        MAT_WALL,
    );

    let sphere1_outer: Hittable = Sphere::new(vec3(-1.3, 0.501, -0.4), 0.5, MAT_MIRROR);

    let model1 = io::mesh::load_obj_file(Path::new("assets/models/bunny.obj"), MAT_GLASS);

    let env_texture = Texture::from_path(Path::new("assets/hdri/brownstudio_4k.hdr")).unwrap();
    let background = Background::from_environment_texture(env_texture, 2.85, 0.15);

    let scene = SceneBuilder::new()
        .set_background(background)
        .set_materials(materials)
        .add_hittables(model1)
        .add_hittable(sphere1_outer)
        .add_hittables(right_wall)
        .add_hittables(table)
        .build();

    // Camera setup
    let width: u32 = 1920 / 1;
    let height: u32 = 1080 / 1;
    let aspect_ratio: Float = width as Float / height as Float;

    let render_settings = tracer::render::RenderSettings {
        samples_per_pixel: 2000,
        max_bounces: 10,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 1000.0),
    };

    let pose = camera::Pose::look_at(Vec3f::new(-0.25, 1.4, 5.0), Vec3f::new(-0.25, 0.5, 0.0));
    let lens = camera::Lens::new(15, 28, aspect_ratio, 5.2, 0.25);

    let camera = Camera::new(pose, lens);

    return (scene, camera, render_settings);
}

#[allow(unused)]
fn stanford_dragon_scene_setup() -> (Scene, Camera, RenderSettings) {
    // World setup
    let mat_table = MatLambertDiffuse::new(Vec3f::new(0.651, 0.541, 0.451));
    let mat_metal = MatPrincipled::new(vec3(0.806, 0.351, 0.959), 0.9, 0.1);

    let materials = vec![mat_table, mat_metal];
    const MAT_TABLE: MaterialID = 0;
    const MAT_METAL: MaterialID = 1;

    let table = make_parallelepiped(
        vec3(-1.5, -1.0, -1.0),
        vec3(1.5, -1.0, -1.0),
        vec3(-1.5, -1.0, 1.0),
        vec3(-1.5, 0.0, -1.0),
        MAT_TABLE,
    );

    let object = io::mesh::load_obj_file(Path::new("assets/models/dragon.obj"), MAT_METAL);

    let env_texture = Texture::from_path(Path::new("assets/hdri/apartmentbalcony_4k.exr")).unwrap();
    let background = Background::from_environment_texture(env_texture, std::f32::consts::PI, 1.0);

    let scene = SceneBuilder::new()
        .set_background(background)
        .set_materials(materials)
        .add_hittables(table)
        .add_hittables(object)
        .build();

    // Camera setup
    let width: u32 = 1920 / 1;
    let height: u32 = 1080 / 1;
    let aspect_ratio: Float = width as Float / height as Float;

    let render_settings = tracer::render::RenderSettings {
        samples_per_pixel: 2000,
        max_bounces: 6,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 1000.0),
    };

    let pose = camera::Pose::look_at(Vec3f::new(-1.0, 1.4, 4.0), Vec3f::new(0.0, 0.4, 0.0));
    let lens = camera::Lens::new(15, 32, aspect_ratio, 4.1, 0.3);

    let camera = Camera::new(pose, lens);

    return (scene, camera, render_settings);
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();

    let output_dir: Option<PathBuf> = if args.len() != 2 {
        eprintln!("No output directory specified. Starting dry run...\n");
        None
    } else {
        Some(PathBuf::from(std::env::args().nth(1).expect(
            "Please provide an output directory as the first argument.",
        )))
    };

    // let (scene, camera, render_settings) = glass_bunny_scene_setup();

    let (scene, camera) = io::scene::load_gltf(std::path::Path::new("./scenes/bmw-m6/gltf/bmw_m6_core.gltf"))?;

    let render_settings = tracer::render::RenderSettings {
        samples_per_pixel: 1000,
        max_bounces: 6,
        image_width: 1920,
        image_height: 1080,
        ray_limits: Interval::new(0.001, 1000.0),
    };

    let postproc_settings = PostProcessSettings {};

    let scheduler = TiledScheduler::new(64);
    let render_result =
        scheduler.render::<DefaultPipeline, 3>(camera.clone(), render_settings.clone(), &scene);
    eprintln!("{}", render_result.to_string());
    let bvh_result = scheduler.render::<BVHDebugPipeline, 3>(camera.clone(), render_settings.clone(), &scene);
    eprintln!("{}", bvh_result.to_string());

    let render_postproc_result = postprocess(&render_result, &postproc_settings);
    let bvh_postproc_result = postprocess(&bvh_result, &postproc_settings);

    if let Some(output_dir) = output_dir {
        render_result.write_images(output_dir.as_path(), ".exr")?;
        render_result.dump_info(&output_dir)?;
        render_postproc_result.write_images(output_dir.as_path())?;

        bvh_result.write_images(output_dir.as_path(), ".exr")?;
        bvh_postproc_result.write_images(output_dir.as_path())?;
    }

    Ok(())
}
