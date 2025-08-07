// Allow dead code for now
#![allow(dead_code)]

mod io;
mod math;
mod tracer;

use std::path::Path;

use math::util::Interval;
use math::vector::{Color, Float, Vec3f, vec3};
use tracer::camera::{self, Camera};
use tracer::hittable::{Hittable, HittableList, Plane, Sphere};
use tracer::material::{MatGlass, MatLambertDiffuse, MatPrincipled, Material, MaterialTrait};

use crate::tracer::hittable::{Mesh, Parallelepiped};
use crate::tracer::material::MatFaceDebug;
use crate::tracer::render::{Scheduler, TiledScheduler};
use crate::tracer::texture::Texture;
use crate::tracer::world::{Background, World};

fn main() {
    let width: u32 = 1280 / 4;
    let height: u32 = 1024 / 4;
    let aspect_ratio: Float = width as Float / height as Float;

    let pose = camera::Pose::look_at(Vec3f::new(0.0, 4.3, 18.0), Vec3f::new(0.0, 2.0, 0.0));
    let lens = camera::Lens::new(15, 35, aspect_ratio, 18.1, 0.0);
    let settings = tracer::render::RenderSettings {
        samples_per_pixel: 16,
        max_bounces: 10,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 1000.0),
    };
    let camera = Camera::new(pose, lens);

    // Scene setup
    let mat_floor = MatLambertDiffuse::new(Vec3f::new(1.0, 1.0, 1.0));
    let mat_glass = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.33);
    let mat_inner = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.0 / 1.33);
    let mat_metal = MatPrincipled::new(vec3(0.2, 0.3, 0.9), 1.0, 0.1);
    let mat_rough = MatLambertDiffuse::new(vec3(1.0, 0.1, 0.1));
    let mat_facedbg = MatFaceDebug::new();

    let floor = Parallelepiped::new(
        vec3(-2.5, -1.0, -2.0),
        vec3(2.5, -1.0, -2.0),
        vec3(-2.5, -1.0, 2.0),
        vec3(-2.5, 0.0, -2.0),
        mat_floor,
    );

    let sphere1: Hittable = Sphere::new(vec3(-1.1, 0.501, 0.0), 0.5, mat_glass.clone());
    let sphere4: Hittable = Sphere::new(vec3(-1.1, 0.501, 0.0), 0.45, mat_inner);
    let sphere2: Hittable = Sphere::new(vec3(0.0, 0.5, 0.0), 0.5, mat_rough.clone());
    let sphere3: Hittable = Sphere::new(vec3(1.1, 0.5, 0.0), 0.5, mat_metal.clone());
    let teapot = Mesh::from_obj_file(Path::new("assets/teapot.obj"), mat_metal.clone());

    let mut objects: HittableList = HittableList::default();
    objects.push(floor);
    //    objects.push(sphere1);
    //    objects.push(sphere2);
    //    objects.push(sphere3);
    //    objects.push(sphere4);
    objects.push(teapot);

    let env_texture =
        Texture::from_path(Path::new("assets/Indoor2_HDRI_4K-TONEMAPPED.jpg")).unwrap();

    let background = Background::from_environment_texture(env_texture, -3.1);

    let world = World::new(objects, background);

    let render_result = TiledScheduler::new(64).render(camera, settings, &world);
    // NaiveMultiThread Sched seems to be faster for the simple scene right now. But lets keep using the Tiled one.
    // let render_result = NaiveMultiThreadScheduler::new(3).render(camera, settings, &world);

    let comment_string = render_result.to_string();
    eprintln!("{}", comment_string);

    let ppm_string = io::ppm::ppm_image(width, height, &render_result.pixels, comment_string);
    print!("{}", ppm_string);
}
