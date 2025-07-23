mod ppm;
mod vec3;
mod camera;
mod ray;
mod hittable;
mod util;
mod material;

use vec3::{Vec3f};
use camera::{Camera};
use hittable::{HittableList, Sphere, Plane};
use material::{MatLambertDiffuse};

use crate::{hittable::Rectangle, material::{MatEmission, MatFaceDebug, MatGlass, MatMetal, MatNormalDebug, MatPoorDiffuse, MatPrincipled, Material}, util::Interval, vec3::{Color, Vec3}};

fn main() {
    let width: u32  = 300 * 2;
    let height: u32 = 200 * 2;
    let aspect_ratio: f32 = width as f32 / height as f32;

    let pose = camera::Pose::look_at(Vec3f::new(-2.0, 2.0, 1.0), 
                                            Vec3f::new(0.0, 0.0, -1.0));
    let lens = camera::Lens::new(
        35,
        45, 
        aspect_ratio,
        3.44,
    0.0);
    let settings = camera::RenderSettings {
        samples_per_pixel: 10,
        max_bounces: 10,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 10000.0)
    };
    let camera = Camera::new(pose, lens, settings);

    let mat_facedebug = MatFaceDebug::new();
    let lambertian_red = MatLambertDiffuse::new(Color::new(0.75, 0.5, 0.1), 1.0);
    let matp_floor = MatLambertDiffuse::new(Color::new(0.2, 0.8, 0.3), 0.8);
    let matp_brushedmet = MatPrincipled::new(Color::new(0.8, 0.9, 0.9), 0.9, 0.03);
    let mat_glass = MatGlass::new(Color::new(1.0, 1.0, 1.0), 1.5);
    let mat_glass_inside = MatGlass::new(Color::new(1.0, 1.0, 1.0), 1.0 / 1.5);
    let mat_normals = MatNormalDebug::new();

    let mut world: HittableList = HittableList::default();
    let rect = Rectangle::from_points(Vec3f::new(-1.0, 0.0, -1.5), 
                                                          Vec3f::new(-1.0, 0.3, -1.5),
                                                          Vec3f::new(1.0, 0.0, -1.5),
                                                          mat_normals.clone());

    let plane = Plane::new(Vec3f::new(0.0, 1.0, 0.0), -0.5, matp_floor);
   // world.push(Sphere::new(Vec3f::new(0.0, -100.5, 0.0), 100.0, matp_floor));
    world.push(Sphere::new(Vec3f::new(0.0, 0.0, -1.2), 0.5, lambertian_red));
    world.push(Sphere::new(Vec3f::new(-1.0, 0.0, -1.0), 0.5, mat_glass));
    world.push(Sphere::new(Vec3f::new(-1.0, 0.0, -1.0), 0.4, mat_glass_inside));
    world.push(Sphere::new(Vec3f::new( 1.0, 0.0, -1.0), 0.5, matp_brushedmet));
    world.push(rect);
    world.push(plane);

    let render_result = camera.render(&world);

    let comment_string = render_result.to_string();
    eprintln!("{}", comment_string);

    let ppm_string = ppm::ppm_image(width, height, &render_result.pixels, comment_string);
    print!("{}", ppm_string);
}
