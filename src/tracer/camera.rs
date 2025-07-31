use super::hittable::{HittableList, HittableTrait};
use super::material::MaterialTrait;
use crate::math::ray::Ray;
use crate::math::vector::{Color, Pixel, Vec3f};
use crate::math::util::{self, Interval};
use core::f32;
use std::fmt::Display;
use std::{thread, vec};

#[derive(Clone)]
// A viewport describes the focus plane of a camera.
pub struct Viewport {
    pub viewdown: Vec3f,
    pub viewright: Vec3f,
    pub topleft: Vec3f,
    pub width: f32,
    pub height: f32,
}

impl Viewport {
    pub fn new(center: Vec3f, dir: Vec3f, up: Vec3f, height: f32, aspect_ratio: f32) -> Self {
        let width = height * aspect_ratio;

        let down_dir_norm = -up.normalize();
        let right_dir_norm = dir.cross(&up.normalize()).normalize();

        let top_left = center + (-down_dir_norm * height * 0.5) + (-right_dir_norm * width * 0.5);

        Viewport {  viewdown: down_dir_norm,
                    viewright: right_dir_norm,
                    topleft: top_left,
                    width: height * aspect_ratio,
                    height: height,
        }
    }

    // Shoot a ray at the given uv of the viewport from a point origin (which is presumably the camera position).
    pub fn ray_at_uv(&self, u: f32, v: f32, origin: Vec3f) -> Ray {
        let target = self.topleft + self.viewright * u * self.width 
                   + self.viewdown * v * self.height;
        let dir = (target - origin).normalize();

        Ray { orig: origin, dir: dir }
    }
}


#[derive(Clone)]
pub struct Lens {
    focal_length: f32,
    sensor_width: f32,
    afov_rad: f32,
    aspect_ratio: f32,

    // Distance from the camera to the focus plane
    focal_distance: f32,
    // Amount of depth of field 
    dof: f32, 
}

impl Lens {
    pub fn new(sensor_width_mm: u32, focal_length_mm: u32, aspect_ratio: f32, focal_distance: f32, dof: f32) -> Self {
        let sensor_width = sensor_width_mm as f32 / 1000.0;
        let focal_length = focal_length_mm as f32 / 1000.0;
        let sensor_height = sensor_width / aspect_ratio;
        let afov_rad = 2.0 * f32::atan(sensor_height / (2.0 * focal_length));

        Lens {
            focal_length: focal_length,
            sensor_width: sensor_width,
            afov_rad: afov_rad,
            aspect_ratio: aspect_ratio,

            focal_distance: focal_distance,
            dof: dof,
        }
    }
}

// Position and orientation of a camera
#[derive(Clone)]
pub struct Pose {
    position: Vec3f,
    direction: Vec3f,
    up: Vec3f,
}

impl Pose {
    const UP_DEFAULT: Vec3f = Vec3f::new(0.0, 1.0, 0.0);

    pub const fn new(position: Vec3f, direction: Vec3f) -> Self {
        Pose {
            position: position,
            direction: direction,
            up: Pose::UP_DEFAULT,
        }
    }

    pub fn look_at(position: Vec3f, target: Vec3f) -> Self {
        Pose {
            position: position,
            direction: (target - position).normalize(),
            up: Pose::UP_DEFAULT,
        }
    }
}

#[derive(Clone)]
pub struct RenderSettings {
    pub image_width: u32,
    pub image_height: u32,
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub ray_limits: Interval,
}

fn background_color(dir: Vec3f) -> Color {
    // Compute the background color in the given direction. Basically we think of the environment
    // as a unitsphere, with the camera at the center. That way we can determine the backgrounds
    // color simply by what direction we are looking in.
    // For now, it is a simple gradient along the y axis.
    const BRIGHTNESS: f32 = 1.0;
    const COLOR_A: Color = Color::new(0.5, 0.7, 1.0);
    const COLOR_B: Color = Color::new(1.0, 1.0, 1.0);
    let a = (dir.normalize().y() + 1.0) * 0.5;
    let lerped_color = COLOR_B * (1.0 - a) + COLOR_A * a;
    lerped_color * BRIGHTNESS
}

fn trace_ray(ray: &Ray, settings: &RenderSettings, world: &HittableList, bounce: u32) -> Color {
    static COLOR_BLACK: Color = Color::new(0.0, 0.0, 0.0);
    // Abort if max bounce is reached.
    if bounce == settings.max_bounces { return COLOR_BLACK; }
    // Fire the ray. See if it hits anything.
    if let Some(hit) = world.try_hit(ray, settings.ray_limits, bounce) {
        match hit.material.scatter(ray, &hit) {
            (Some(scatter_ray), Some(color_att)) => {
                // Fire the reflected/scattered ray we got from the material and surface information.
                // Attenuate with the color attenuation applied by the material.
                trace_ray(&scatter_ray, settings, world, bounce + 1) * color_att
            },
            (Some(scatter_ray), None) => {
                // The ray was reflected, but the color not attenuated.
                // Must be a perfect mirror or a portal or sum
                trace_ray(&scatter_ray, settings, world, bounce + 1)
            },
            (None, Some(color_att)) => {
                // Ray absorbed, just return the attenuation color.
                color_att
            },
            (None, None) => {
                // The ray was absorbed and no color is given. Must have been a black hole.
                COLOR_BLACK
            }
        }
    } else {
       // The ray did not hit anything. Return the background color.
       background_color(ray.dir)
    }
}
fn trace_ray_it(ray: &Ray, settings: &RenderSettings, world: &HittableList, _bounce: u32) -> Color {
    static COLOR_BLACK: Color = Color::new(0.0, 0.0, 0.0);
    let mut ray_color: Color = Color::new(1.0, 1.0, 1.0);
    let mut current_ray: Ray = ray.clone();
    let mut bounce_counter = 0;
    loop {
        if bounce_counter == settings.max_bounces {
            // If max bounces where reached, the ray never hit a light source
            return COLOR_BLACK;
        }        
        // Fire the ray. See if it hits anything.
        if let Some(hit) = world.try_hit(&current_ray, settings.ray_limits, bounce_counter) {
            match hit.material.scatter(&current_ray, &hit) {
                (Some(scatter_ray), Some(color_att)) => {
                    // Fire the reflected/scattered ray we got from the material and surface information.
                    // Attenuate with the color attenuation applied by the material.
                    current_ray = scatter_ray;
                    ray_color = ray_color * color_att;
                },
                (Some(scatter_ray), None) => {
                    // The ray was reflected, but the color not attenuated.
                    // Simply shoot the new, attenuated ray.
                    current_ray = scatter_ray;
                },
                (None, Some(color_att)) => {
                    // Ray absorbed. Do one last attenuation and return.
                    return ray_color * color_att;
                },
                (None, None) => {
                    // The ray was absorbed and no attenuation color was given.
                    // This should not happen, but we have to handle the case. Assume a black hole.
                    return COLOR_BLACK;
                }
            }
        } else {
            // If the ray did not hit objects, we assume it hit the background / sky.
            return ray_color * background_color(current_ray.dir);
        }
        bounce_counter = bounce_counter + 1;
    }
}


#[derive(Clone)]
pub struct Camera {
    pub lens: Lens,
    pub pose: Pose,
    pub viewport: Viewport,
}

impl Camera {
    pub fn new(pose: Pose, lens: Lens) -> Self {
        // Compute the viewport (the focus plane) of the camera
        let viewport_center = pose.position + pose.direction * lens.focal_distance;
        let dir  = pose.direction;

        let height = 2.0 * lens.focal_distance * f32::tan(lens.afov_rad * 0.5);

        let aspect_ratio = lens.aspect_ratio;

        // Calculate the actual "up" vector for the viewport.
        let view_up = pose.up;

        // Project the view_up onto the viewport plane to get the actual viewport up
        let viewport_up = view_up.proj_plane(&dir).normalize();

        let viewport = Viewport::new(viewport_center, dir, viewport_up, height, aspect_ratio);

        Camera {
           viewport: viewport,
           lens: lens,
           pose: pose,
        }
    }

    fn color_to_pixel(c: &Color) -> Pixel {
        let r = util::linear_to_gamma(c.r());
        let g = util::linear_to_gamma(c.g());
        let b = util::linear_to_gamma(c.b());

        Pixel::new((r * 255.99) as u8, (g * 255.99) as u8, (b * 255.99) as u8)
    }

    fn render_region(cam: Camera, settings: RenderSettings, world: HittableList, samples: u32) -> Vec<Color> {
        let mut colors: Vec<Color> = Vec::new();

        let offset_range = 1.0 / settings.image_height as f32;

        for y in 0..settings.image_height {
            for x in 0..settings.image_width {
                let u = x as  f32 / settings.image_width as f32;
                let v = y as f32 / settings.image_height as f32;

                let mut color: Color = Color::default();
                // Perform multisampling here.
                for _ in 0..samples {
                    // The random offset into the pixel square we are considering atm (for multisampling)
                    // TODO: Make this discy instead
                    let rnd_offset_x = util::rand_range_f(0.0, offset_range) - 0.5 * offset_range;
                    let rnd_offset_y = util::rand_range_f(0.0, offset_range) - 0.5 * offset_range;

                    // Random ray origin offset (for DOF simulation)
                    // TODO: Make it so the DOF parameter describes the *actual* depth of field
                    let blur_offset = util::rand_vec_on_unit_disc() * cam.lens.dof / cam.lens.focal_distance;
                    let ray_origin = cam.pose.position
                                              + cam.viewport.viewdown * blur_offset.y() 
                                              + cam.viewport.viewright * blur_offset.x();

                    let ray = cam.viewport.ray_at_uv(u + rnd_offset_x, v + rnd_offset_y, ray_origin);

                    color += trace_ray(&ray, &settings, &world, 0) * (1.0 / samples as f32);
                }

                colors.push(color);
            }
        }

        colors
    }

    pub fn render(&self, settings: RenderSettings, world: &HittableList) -> RenderResult {
        // Should probably have a more user friendly way to set the number of threads at some point.
        let num_threads = 3;

        // Attempt to get a somewhat accurate estimate of the total render time here.
        // Render the entire image once at 1 spp, then extrapolate the entire render time from that.
        let estimate_start = std::time::Instant::now();

        Self::render_region(self.clone(), settings.clone(), world.clone(), 1);

        // It seems a bit impossible to estimate how much the number of threads actually influences
        // the render time. Assume half for more than 1. Thats it uhhh
        let estimate_duration = estimate_start.elapsed().as_secs_f32()
                                   * settings.samples_per_pixel as f32  // Attenuate for actual spp value
                                   * (1.0 / num_threads.clamp(1, 2) as f32); // Attenuate for thread count

        let estimate_minutes = estimate_duration as u32 / 60;
        let estimate_seconds = estimate_duration as u32 % 60;
        let now = chrono::Local::now();
        eprintln!("Started at {}\nEst. render time: {:02}:{:02}", now.format("%H:%M:%S"), estimate_minutes, estimate_seconds);

        let start = std::time::Instant::now();

        // Let several threads render the entire image with the same settings. For now,
        // we simply copy all relevant data right over. Might change that later on.
        // The SPP are split evenly between the threads. The resulting images from all threads are then averaged.
        let spp_per_thread = settings.samples_per_pixel / num_threads;
        let mut thread_handles = Vec::new();
        for _ in 0..num_threads {
            let cam_copy = self.clone();
            let world_copy = world.clone();
            let settings_copy = settings.clone();
            let spp_per_thread_copy = spp_per_thread.clone();
            thread_handles.push(
                thread::spawn(move || {
                    Self::render_region(cam_copy, settings_copy, world_copy, spp_per_thread_copy)
                }));
        };

        // Await and collect images from each thread.
        let mut images: Vec<_> = Vec::new();
        for handle in thread_handles {
            let image = handle.join().unwrap();
            images.push(image);
        }

        // Perform weighted sum of all generated images.
        let weight = 1.0 / num_threads as f32;
        let len = images[0].len();

        // Sum all the images up...
        let mut result = vec![Color::new(0.0, 0.0, 0.0); len];
        for image in images {
            for i in 0..result.len() {
                result[i] = result[i] + image[i];
            }
        }
        // ... and normalize the result
        for i in 0..len {
            result[i] = result[i] * weight
        }

        let pixels = result.iter().map(Self::color_to_pixel).collect();

        let duration = start.elapsed();
        eprintln!("Done in {:?} with {} threads.", duration, num_threads);

        RenderResult {
            pixels: pixels,
            time_elapsed: duration.as_secs_f32(),

            image_height: settings.image_height,
            image_width: settings.image_width,
            num_samples: settings.samples_per_pixel,
            max_bounces: settings.max_bounces,
            num_objects: world.num_objects(),
        }
    }
}

pub struct RenderResult {
    pub pixels: Vec<Pixel>,
    pub time_elapsed: f32,

    pub image_height: u32,
    pub image_width: u32,
    pub num_samples: u32,
    pub max_bounces: u32,
    pub num_objects: u32,
}

impl Display for RenderResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_samples = self.image_height * self.image_width * self.num_samples;
        write!(f, "RenderTimeSec\t{}\nWidth\t{}\nHeight\t{}\nSamplesPerPx\t{}\nTotalSamples\t{}\nMaxRayBounces\t{}\nObjects\t{}",
                   self.time_elapsed, self.image_width, self.image_height, self.num_samples, total_samples, self.max_bounces, self.num_objects )
    }
}