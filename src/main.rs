// Allow dead code for now
#![allow(dead_code)]

mod math;
mod io;
mod tracer;

use math::vector::{Vec3f, vec3, Float};
use tracer::camera::{self, Camera};
use tracer::hittable::{HittableList, Hittable, Sphere, Plane};
use tracer::material::{MatLambertDiffuse, MaterialTrait, Material, MatGlass, MatPrincipled};
use math::util::Interval;

use crate::tracer::render::{NaiveMultiThreadScheduler, NaiveSingleThreadScheduler, Scheduler, TiledScheduler};

// Scatter spheres on a plane
fn scatter_spheres(world: &mut HittableList, count: u32, height: Float, scatter_radius: Float, sphere_radius: (Float, Float)) {
    for _ in 0..count {
        // Make it so the spheres "sit" on the given plane height
        let radius = math::util::rand_range_f(sphere_radius.0, sphere_radius.1);
        let height = height + radius;

        let angle = math::util::rand_range_f(0.0, 2.0 * std::f64::consts::PI as Float);
        let distance = math::util::rand_range_f(0.0, 1.0).sqrt() * scatter_radius;
        let (x, y) = (Float::cos(angle) * distance, Float::sin(angle) * distance); 
        let center = Vec3f::new(x, height, y);
        let material = Material::random_instance();
        let sphere = Sphere::new(center, radius, material);

        world.push(sphere);
    }
}

fn main() {
    let width: u32  = 1280 / 2;
    let height: u32 = 1024 / 2;
    let aspect_ratio: Float = width as Float / height as Float;

    let pose = camera::Pose::look_at(Vec3f::new(-10.3, 0.6, 8.9), 
                                            Vec3f::new(0.0, 0.6, -0.1));
    let lens = camera::Lens::new(
        15,
        80, 
        aspect_ratio,
        3.44,
    0.0);
    let settings = tracer::render::RenderSettings {
        samples_per_pixel: 100,
        max_bounces: 10,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 10000.0),
    };
    let camera = Camera::new(pose, lens);

    // Scene setup
    let mat_floor = MatLambertDiffuse::new(Vec3f::new(0.2, 0.9, 0.2), 1.0);
    let mat_glass = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.33);
    let mat_inner = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.0 / 1.33);
    let mat_metal = MatPrincipled::new(vec3(0.2, 0.3, 0.9), 1.0, 0.1);
    let mat_rough = MatLambertDiffuse::new(vec3(1.0, 0.1, 0.1), 1.0);

    let floor = Plane::new(Vec3f::new(0.0, 1.0, 0.0), 0.0, mat_floor);

    let sphere1: Hittable = Sphere::new(vec3(-1.1, 0.5, 0.0), 0.5, mat_glass);
    let sphere4: Hittable = Sphere::new(vec3(-1.1, 0.5, 0.0), 0.45, mat_inner);
    let sphere2: Hittable = Sphere::new(vec3( 0.0, 0.5, 0.0), 0.5, mat_rough);
    let sphere3: Hittable = Sphere::new(vec3( 1.1, 0.5, 0.0), 0.5, mat_metal);

    let mut world: HittableList = HittableList::default();
    world.push(floor);
    world.push(sphere1);
    world.push(sphere2);
    world.push(sphere3);
    world.push(sphere4);
   
    let render_result = TiledScheduler::new(64).render(camera, settings, &world);
    // NaiveMultiThread Sched seems to be faster for the simple scene right now. But lets keep using the Tiled one.
    // let render_result = NaiveMultiThreadScheduler::new(3).render(camera, settings, &world);

    let comment_string = render_result.to_string();
    eprintln!("{}", comment_string);

    let ppm_string = io::ppm::ppm_image(width, height, &render_result.pixels, comment_string);
    print!("{}", ppm_string);
}
