// Allow dead code for now
#![allow(dead_code)]

mod ppm;
mod vec3;
mod camera;
mod ray;
mod hittable;
mod util;
mod material;

use vec3::{Vec3f, Color};
use camera::{Camera};
use hittable::{HittableList, Sphere, Plane, Parallelogram};
use material::{MatLambertDiffuse, MatFaceDebug, MatGlass, MatNormalDebug, MatPrincipled};
use util::Interval;

use crate::{hittable::Parallelepiped, material::MatEmission, vec3::CoordinatePlane};

fn main() {
    let width: u32  = 1280 / 4;
    let height: u32 = 1024 / 4;
    let aspect_ratio: f32 = width as f32 / height as f32;

    let pose = camera::Pose::look_at(Vec3f::new(-2.0, 0.9, 1.2), 
                                            Vec3f::new(0.4, 0.6, -1.0));
    let lens = camera::Lens::new(
        35,
        20, 
        aspect_ratio,
        3.44,
    0.35);
    let settings = camera::RenderSettings {
        samples_per_pixel: 100,
        max_bounces: 10,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 10000.0)
    };
    let camera = Camera::new(pose, lens, settings);

    let mat_facedebug = MatFaceDebug::new();
    let mat_lamyellow = MatLambertDiffuse::new(Color::new(0.75, 0.5, 0.1), 1.0);
    let mat_lamred = MatLambertDiffuse::new(Color::new(0.95, 0.1, 0.1), 1.0);
    let matp_floor = MatLambertDiffuse::new(Color::new(0.2, 0.8, 0.3), 0.8);
    let matp_brushedmet = MatPrincipled::new(Color::new(0.8, 0.9, 0.9), 0.9, 0.09);
    let mat_glass = MatGlass::new(Color::new(1.0, 1.0, 1.0), 1.5);
    let mat_glass_inside = MatGlass::new(Color::new(1.0, 1.0, 1.0), 1.0 / 1.5);
    let mat_emissive = MatEmission::new(Color::new(0.2, 0.2, 1.0), 10.0);
    let mat_normals = MatNormalDebug::new();

    let mut world: HittableList = HittableList::default();

    let cube = Parallelepiped::new_cube(
        Vec3f::new(-2.0, -0.5, -1.6),
    Vec3f::new(1.0, 0.0, -0.2), vec3::PLANE_XZ, 8.3, mat_lamyellow.clone());

    world.push(Sphere::new(Vec3f::new(-3.0, 0.9, 2.7), 1.2, mat_emissive.clone()));
    world.push(Sphere::new(Vec3f::new(3.0, 0.7, 0.5), 1.2, mat_lamred.clone()));
    world.push(Sphere::new(Vec3f::new(0.0, 0.0, -1.2), 0.5, mat_lamred));
    world.push(Sphere::new(Vec3f::new(-1.0, 0.0, -1.0), 0.5, mat_glass));
    world.push(Sphere::new(Vec3f::new(-1.0, 0.0, -1.0), 0.4, mat_glass_inside));
    world.push(Sphere::new(Vec3f::new( 1.0, 0.0, -1.0), 0.5, matp_brushedmet));
    world.push(Plane::new(Vec3f::new(0.0, 1.0, 0.0), -0.5, matp_floor));
    world.push(cube);

    let render_result = camera.render(&world);

    let comment_string = render_result.to_string();
    eprintln!("{}", comment_string);

    let ppm_string = ppm::ppm_image(width, height, &render_result.pixels, comment_string);
    print!("{}", ppm_string);
}
