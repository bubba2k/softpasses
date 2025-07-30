// Allow dead code for now
#![allow(dead_code)]

mod ppm;
mod vec3;
mod camera;
mod ray;
mod hittable;
mod util;
mod material;

use std::f32::consts::PI;

use vec3::{Vec3f, Color};
use camera::{Camera};
use hittable::{HittableList, Sphere, Plane, Parallelogram};
use material::{MatLambertDiffuse, MatFaceDebug, MatGlass, MatNormalDebug, MatPrincipled, MaterialTrait};
use util::Interval;

use crate::{hittable::{Hittable, Parallelepiped}, material::MatEmission, vec3::CoordinatePlane};

// Scatter spheres on a plane
fn scatter_spheres(world: &mut HittableList, count: u32, height: f32, scatter_radius: f32, sphere_radius: (f32, f32)) {
    for i in 0..count {
        // Make it so the spheres "sit" on the given plane height
        let radius = util::rand_range_f(sphere_radius.0, sphere_radius.1);
        let height = height + radius;

        let angle = util::rand_range_f(0.0, 2.0 * PI);
        let distance = util::rand_range_f(0.0, 1.0).sqrt() * scatter_radius;
        let (x, y) = (f32::cos(angle) * distance, f32::sin(angle) * distance); 
        let center = Vec3f::new(x, height, y);
        let material = material::Material::random_instance();
        let sphere = Sphere::new(center, radius, material);

        world.push(sphere);
    }
}

fn main() {
    let width: u32  = 1280 / 6;
    let height: u32 = 1024 / 6;
    let aspect_ratio: f32 = width as f32 / height as f32;

    let pose = camera::Pose::look_at(Vec3f::new(0.0, 15.0, 0.1), 
                                            Vec3f::new(0.0, 0.0, -0.1));
    let lens = camera::Lens::new(
        15,
        35, 
        aspect_ratio,
        3.44,
    0.0);
    let settings = camera::RenderSettings {
        samples_per_pixel: 20,
        max_bounces: 8,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 10000.0)
    };
    let camera = Camera::new(pose, lens, settings);

    // Scene setup
    let mat_floor = MatLambertDiffuse::new(Vec3f::new(0.2, 0.9, 0.2), 1.0);
    let floor = Plane::new(Vec3f::new(0.0, 1.0, 0.0), 0.0, mat_floor);
    let mut world: HittableList = HittableList::default();
    world.push(floor);
    scatter_spheres(&mut world, 200, 0.0, 2.0, (0.05, 0.1));


    let render_result = camera.render(&world);

    let comment_string = render_result.to_string();
    eprintln!("{}", comment_string);

    let ppm_string = ppm::ppm_image(width, height, &render_result.pixels, comment_string);
    print!("{}", ppm_string);
}
