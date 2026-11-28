mod io;
mod math;
mod postproc;
mod tracer;
mod bvh;

use std::path::{Path, PathBuf};

use crate::postproc::{PostProcessSettings, postprocess};
use crate::tracer::hittable::make_parallelepiped;
use crate::tracer::pipelines::{BVHDebugPipeline, ColorPipeline};
use crate::tracer::render::{LiveScheduler, RenderResult, RenderSettings};
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

// The following three scene setups are no longer used directly, but remain here for reference until they have succesfully been ported to a GLTF file.
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

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The width of the output image (in pixels).
    #[arg(short, long)]
    width: u32,

    /// The height of the output image (in pixels).
    #[arg(short, long)]
    height: u32,

    /// How many samples to take per pixel.
    #[arg(long)]
    spp: u32,

    /// The maximum allowed number of bounces before a ray terminates.
    #[arg(short, long, default_value_t = 32)]
    bounces: u32,

    /// The size of the tiles (in pixels) an image is divided into for rendering.
    #[arg(long, default_value_t = 32)]
    tile_size: u32,

    /// The far clipping plane of the camera.
    #[arg(long, default_value_t = 0.001)]
    clip_near: f32,

    /// The near clipping plane of the camera.
    #[arg(long, default_value_t = 1000.0)]
    clip_far: f32,

    /// The GLTF file to render.
    #[arg(short)]
    input_file: String,

    /// The directory to save the render output to.
    #[arg(short)]
    output_directory: String,

    /// Enable postprocessing (denoising with Intel OIDN, basic tonemapping).
    #[arg(long, default_value_t = false)]
    postprocess: bool,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let render_settings = RenderSettings {
        image_width: args.width,
        image_height: args.height,
        samples_per_pixel: args.spp as u32,
        max_bounces: args.bounces,
        ray_limits: Interval::new(0.001, 1000.0),
    };

    let (scene_builder, camera) = io::scene::load_gltf(std::path::Path::new(&args.input_file))?;

    let scene = scene_builder.build();

    let mut scheduler = LiveScheduler::<DefaultPipeline, 3>::run(camera, render_settings, scene, args.tile_size);

    while !scheduler.is_done() {
        
    }

    let render_result = scheduler.try_yield_result().unwrap();

    let output_path = std::path::Path::new(&args.output_directory);
    eprintln!("{}", render_result.to_string());
    render_result.dump_info(output_path)?;
    render_result.write_images(output_path, ".exr")?;

    if args.postprocess {
        let postproc_settings: PostProcessSettings = PostProcessSettings{};
        let postproc_results = postprocess(&render_result, &postproc_settings);
        postproc_results.write_images(output_path)?;
    }

    Ok(())
}