// Allow dead code for now
#![allow(dead_code)]

mod math;
mod io;
mod tracer;

use math::vector::{Vec3f, vec3, Float, Color};
use tracer::camera::{self, Camera};
use tracer::hittable::{HittableList, Hittable, Sphere, Plane};
use tracer::material::{MatLambertDiffuse, MaterialTrait, Material, MatGlass, MatPrincipled};
use math::util::Interval;

use crate::tracer::hittable::Parallelepiped;
use crate::tracer::render::{Scheduler, TiledScheduler};
use crate::tracer::texture::Texture;
use crate::tracer::world::{Background, World};

// A nice custom background gradient
fn background_color(dir: Vec3f) -> Color {
    // Compute the background color in the given direction. Basically we think of the environment
    // as a unitsphere, with the camera at the center. That way we can determine the backgrounds
    // color simply by what direction we are looking in.
    // For now, it is a simple gradient along the y axis.
    const BRIGHTNESS: Float = 1.0;
    const COLOR_A: Color = Color::new(0.5, 0.7, 1.0);
    const COLOR_B: Color = Color::new(1.0, 1.0, 1.0);
    let a = (dir.normalize().y + 1.0) * 0.5;
    let lerped_color = COLOR_B * (1.0 - a) + COLOR_A * a;
    lerped_color * BRIGHTNESS
}

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

    let pose = camera::Pose::look_at(Vec3f::new(0.0, 1.5, 4.0), 
                                            Vec3f::new(0.0, 0.5, 0.0));
    let lens = camera::Lens::new(
        15,
        20, 
        aspect_ratio,
        13.44,
    0.0);
    let settings = tracer::render::RenderSettings {
        samples_per_pixel: 200,
        max_bounces: 10,
        image_width: width,
        image_height: height,
        ray_limits: Interval::new(0.001, 10000.0),
    };
    let camera = Camera::new(pose, lens);

    // Scene setup
    let mat_floor = MatLambertDiffuse::new(Vec3f::new(0.8, 0.8, 0.8), 1.0);
    let mat_glass = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.33);
    let mat_inner = MatGlass::new(vec3(1.0, 1.0, 1.0), 1.0 / 1.33);
    let mat_metal = MatPrincipled::new(vec3(0.2, 0.3, 0.9), 1.0, 0.1);
    let mat_rough = MatLambertDiffuse::new(vec3(1.0, 0.1, 0.1), 1.0);

    let floor = Parallelepiped::new(
        vec3(-1.5, -1.0, -1.0),
        vec3(1.5, -1.0, -1.0),
            vec3(-1.5, -1.0, 1.0),
            vec3(-1.5, 0.0, -1.0),
            mat_floor);

    let sphere1: Hittable = Sphere::new(vec3(-1.1, 0.501, 0.0), 0.5, mat_glass);
    let sphere4: Hittable = Sphere::new(vec3(-1.1, 0.501, 0.0), 0.45, mat_inner);
    let sphere2: Hittable = Sphere::new(vec3( 0.0, 0.5, 0.0), 0.5, mat_rough);
    let sphere3: Hittable = Sphere::new(vec3( 1.1, 0.5, 0.0), 0.5, mat_metal);

    let mut objects: HittableList = HittableList::default();
    objects.push(floor);
    objects.push(sphere1);
    objects.push(sphere2);
    objects.push(sphere3);
    objects.push(sphere4);

    let texture_bytes = include_bytes!("../assets/Indoor1_HDRI_2K-TONEMAPPED.jpg");
    let env_texture = Texture::from_data(texture_bytes).unwrap();

    let background = Background::from_environment_texture(env_texture, -4.0);

    let world = World::new(objects, background);
   
    let render_result = TiledScheduler::new(64).render(camera, settings, &world);
    // NaiveMultiThread Sched seems to be faster for the simple scene right now. But lets keep using the Tiled one.
    // let render_result = NaiveMultiThreadScheduler::new(3).render(camera, settings, &world);

    let comment_string = render_result.to_string();
    eprintln!("{}", comment_string);

    let ppm_string = io::ppm::ppm_image(width, height, &render_result.pixels, comment_string);
    print!("{}", ppm_string);
}
