use super::hittable::{HittableList, HittableTrait};
use crate::math::ray::Ray;
use crate::math::vector::{Color, Pixel, Vec3f};
use crate::math::util::{self, ImageRegion, Interval};
use core::f32;
use std::fmt::Display;
use std::{thread, vec};
use crate::tracer::trace;

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
    pub focal_length: f32,
    pub sensor_width: f32,
    pub afov_rad: f32,
    pub aspect_ratio: f32,

    // Distance from the camera to the focus plane
    pub focal_distance: f32,
    // Amount of depth of field 
    pub dof: f32, 
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
    pub position: Vec3f,
    pub direction: Vec3f,
    pub up: Vec3f,
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

    pub fn render(&self, settings: RenderSettings, world: &HittableList) -> RenderResult {
        // Should probably have a more user friendly way to set the number of threads at some point.
        let num_threads = 3;

        // Attempt to get a somewhat accurate estimate of the total render time here.
        // Render the entire image once at 1 spp, then extrapolate the entire render time from that.
        let estimate_settings = RenderSettings{
            samples_per_pixel: 1,
            ..settings
        };
        let estimate_start = std::time::Instant::now();

        let region: ImageRegion = ImageRegion::whole_image(settings.image_width, settings.image_height);
        trace::render_region(self.clone(), estimate_settings, world.clone(), region.clone());

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
            let thread_settings = RenderSettings{
                samples_per_pixel: spp_per_thread,
                ..settings
            };
            let region_copy = region.clone();
            thread_handles.push(
                thread::spawn( || {
                    trace::render_region(cam_copy, thread_settings, world_copy, region_copy)
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