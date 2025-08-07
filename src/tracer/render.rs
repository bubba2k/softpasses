use itertools::Itertools;
use rayon::prelude::*;
use std::time::Instant;

use super::material::MaterialTrait;
use crate::math::ray::Ray;
use crate::math::util::{self, ImageRegion};
use crate::math::vector::{Color, Float, Pixel};
use crate::tracer::camera::Camera;
use crate::tracer::hittable::HittableTrait;
use crate::tracer::world::World;

fn trace_ray(ray: &Ray, settings: &RenderSettings, world: &World, bounce: u32) -> Color {
    static COLOR_BLACK: Color = Color::new(0.0, 0.0, 0.0);
    // Abort if max bounce is reached.
    if bounce == settings.max_bounces {
        return COLOR_BLACK;
    }
    // Fire the ray. See if it hits anything.
    if let Some(hit) = world.objects_bvh.try_hit(ray, settings.ray_limits, bounce) {
        match hit.material.scatter(ray, &hit) {
            (Some(scatter_ray), Some(color_att)) => {
                // Fire the reflected/scattered ray we got from the material and surface information.
                // Attenuate with the color attenuation applied by the material.
                trace_ray(&scatter_ray, settings, world, bounce + 1) * color_att
            }
            (Some(scatter_ray), None) => {
                // The ray was reflected, but the color not attenuated.
                // Must be a perfect mirror or a portal or sum
                trace_ray(&scatter_ray, settings, world, bounce + 1)
            }
            (None, Some(color_att)) => {
                // Ray absorbed, just return the attenuation color.
                color_att
            }
            (None, None) => {
                // The ray was absorbed and no color is given. Must have been a black hole.
                COLOR_BLACK
            }
        }
    } else {
        // The ray did not hit anything. Return the background color.
        world.background.sample(ray.dir)
    }
}

fn trace_ray_it(ray: &Ray, settings: &RenderSettings, world: &World, _bounce: u32) -> Color {
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
        if let Some(hit) =
            world
                .objects_bvh
                .try_hit(&current_ray, settings.ray_limits, bounce_counter)
        {
            match hit.material.scatter(&current_ray, &hit) {
                (Some(scatter_ray), Some(color_att)) => {
                    // Fire the reflected/scattered ray we got from the material and surface information.
                    // Attenuate with the color attenuation applied by the material.
                    current_ray = scatter_ray;
                    ray_color = ray_color * color_att;
                }
                (Some(scatter_ray), None) => {
                    // The ray was reflected, but the color not attenuated.
                    // Simply shoot the new, attenuated ray.
                    current_ray = scatter_ray;
                }
                (None, Some(color_att)) => {
                    // Ray absorbed. Do one last attenuation and return.
                    return ray_color * color_att;
                }
                (None, None) => {
                    // The ray was absorbed and no attenuation color was given.
                    // This should not happen, but we have to handle the case. Assume a black hole.
                    return COLOR_BLACK;
                }
            }
        } else {
            // If the ray did not hit objects, we assume it hit the background / sky.
            return ray_color * world.background.sample(current_ray.dir);
        }
        bounce_counter = bounce_counter + 1;
    }
}

// Render a specific region of the image.
pub fn render_region(
    cam: &Camera,
    settings: &RenderSettings,
    world: &World,
    region: util::ImageRegion,
) -> Vec<Color> {
    let mut colors: Vec<Color> = Vec::new();
    let offset_range = 1.0 / settings.image_height as Float;

    for y in region.y.0..region.y.1 {
        for x in region.x.0..region.x.1 {
            let u = x as Float / settings.image_width as Float;
            let v = y as Float / settings.image_height as Float;
            let mut color: Color = Color::default();
            // Perform multisampling here.
            for _ in 0..settings.samples_per_pixel {
                // The random offset into the pixel square we are considering atm (for multisampling)
                // TODO: Make this discy instead
                let rnd_offset_x = util::rand_range_f(0.0, offset_range) - 0.5 * offset_range;
                let rnd_offset_y = util::rand_range_f(0.0, offset_range) - 0.5 * offset_range;
                // Random ray origin offset (for DOF simulation)
                // TODO: Make it so the DOF parameter describes the *actual* depth of field
                let blur_offset =
                    util::rand_vec_on_unit_disc() * cam.lens.dof / cam.lens.focal_distance;
                let ray_origin = cam.pose.position
                    + cam.viewport.viewdown * blur_offset.y
                    + cam.viewport.viewright * blur_offset.x;
                let ray = cam
                    .viewport
                    .ray_at_uv(u + rnd_offset_x, v + rnd_offset_y, ray_origin);
                color += trace_ray(&ray, &settings, &world, 0)
                    * (1.0 / settings.samples_per_pixel as Float);
            }
            colors.push(color);
        }
    }
    colors
}

fn estimate_render_time(
    camera: &Camera,
    world: &World,
    settings: &RenderSettings,
    num_threads: u32,
) {
    // Attempt to get a somewhat accurate estimate of the total render time here.
    // Render the entire image once at 1 spp, then extrapolate the entire render time from that.
    let estimate_settings = RenderSettings {
        samples_per_pixel: 1,
        ..*settings
    };
    let estimate_start = std::time::Instant::now();
    let region: ImageRegion = ImageRegion::whole_image(settings.image_width, settings.image_height);
    render_region(camera, &estimate_settings, world, region);
    // It seems a bit impossible to estimate how much the number of threads actually influences
    // the render time. Assume half for more than 1. Thats it uhhh
    let estimate_duration = estimate_start.elapsed().as_secs_f64() as Float
                               * settings.samples_per_pixel as Float  // Attenuate for actual spp value
                               * (1.0 / num_threads.clamp(1, 2) as Float); // Attenuate for thread count
    let estimate_minutes = estimate_duration as u32 / 60;
    let estimate_seconds = estimate_duration as u32 % 60;
    let now = chrono::Local::now();
    eprintln!(
        "Started at {}\nEst. render time: {:02}:{:02}",
        now.format("%H:%M:%S"),
        estimate_minutes,
        estimate_seconds
    );
}

fn color_to_pixel(c: &Color) -> Pixel {
    let r = util::linear_to_gamma(c[0]);
    let g = util::linear_to_gamma(c[1]);
    let b = util::linear_to_gamma(c[2]);
    crate::math::vector::Pixel::new((r * 255.99) as u8, (g * 255.99) as u8, (b * 255.99) as u8)
}

#[derive(Clone)]
pub struct RenderSettings {
    pub image_width: u32,
    pub image_height: u32,
    pub samples_per_pixel: u32,
    pub max_bounces: u32,
    pub ray_limits: util::Interval,
}

pub struct RenderResult {
    pub pixels: Vec<Pixel>,
    pub time_elapsed: Float,

    pub image_height: u32,
    pub image_width: u32,
    pub num_samples: u32,
    pub max_bounces: u32,
    pub num_objects: u32,
}

impl std::fmt::Display for RenderResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_samples = self.image_height * self.image_width * self.num_samples;
        write!(
            f,
            "RenderTimeSec\t{}\nWidth\t{}\nHeight\t{}\nSamplesPerPx\t{}\nTotalSamples\t{}\nMaxRayBounces\t{}\nObjects\t{}",
            self.time_elapsed,
            self.image_width,
            self.image_height,
            self.num_samples,
            total_samples,
            self.max_bounces,
            self.num_objects
        )
    }
}

pub trait Scheduler {
    fn render(&self, camera: Camera, settings: RenderSettings, world: &World) -> RenderResult;
}

#[derive(Default)]
pub struct NaiveSingleThreadScheduler {}

impl NaiveSingleThreadScheduler {
    pub fn new() -> Self {
        NaiveSingleThreadScheduler {}
    }
}

impl Scheduler for NaiveSingleThreadScheduler {
    fn render(&self, camera: Camera, settings: RenderSettings, world: &World) -> RenderResult {
        estimate_render_time(&camera, world, &settings, 1);

        let start = Instant::now();

        let region = ImageRegion::whole_image(settings.image_width, settings.image_height);
        let image = render_region(&camera, &settings, &world, region);

        let image_pixels = image.iter().map(color_to_pixel).collect();

        RenderResult {
            pixels: image_pixels,
            time_elapsed: start.elapsed().as_secs_f64() as Float,
            image_height: settings.image_height,
            image_width: settings.image_width,
            num_samples: settings.samples_per_pixel,
            max_bounces: settings.max_bounces,
            num_objects: world.objects_bvh.num_primitives(),
        }
    }
}

pub struct NaiveMultiThreadScheduler {
    num_threads: u32,
}

impl NaiveMultiThreadScheduler {
    pub fn new(num_threads: u32) -> Self {
        NaiveMultiThreadScheduler {
            num_threads: num_threads,
        }
    }
}

impl Scheduler for NaiveMultiThreadScheduler {
    fn render(&self, camera: Camera, settings: RenderSettings, world: &World) -> RenderResult {
        // Should probably have a more user friendly way to set the number of threads at some point.
        let num_threads = self.num_threads;
        let region: ImageRegion =
            ImageRegion::whole_image(settings.image_width, settings.image_height);
        // Print estimated render time
        estimate_render_time(&camera, world, &settings, num_threads);
        let start = std::time::Instant::now();
        // Let several threads render the entire image with the same settings. For now,
        // we simply copy all relevant data right over. Might change that later on.
        // The SPP are split evenly between the threads. The resulting images from all threads are then averaged.
        let spp_per_thread = settings.samples_per_pixel / num_threads;
        let images: Vec<Vec<Color>> = (0..num_threads)
            .into_par_iter()
            .map(|_| { 
                let thread_settings = RenderSettings { 
                    samples_per_pixel: spp_per_thread,
                    ..settings    
                };
                render_region(&camera, &thread_settings, &world, region.clone())
            })
            .collect();

        // Perform weighted sum of all generated images.
        let weight = 1.0 / num_threads as Float;
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
        let pixels = result.iter().map(color_to_pixel).collect();
        let duration = start.elapsed();
        eprintln!("Done in {:?} with {} threads.", duration, num_threads);
        RenderResult {
            pixels: pixels,
            time_elapsed: duration.as_secs_f64() as Float,
            image_height: settings.image_height,
            image_width: settings.image_width,
            num_samples: settings.samples_per_pixel,
            max_bounces: settings.max_bounces,
            num_objects: world.objects_bvh.num_primitives(),
        }
    }
}

pub struct TiledScheduler {
    tile_size: u32,
}

impl TiledScheduler {
    pub fn new(tile_size: u32) -> Self {
        TiledScheduler {
            tile_size: tile_size,
        }
    }
}

impl Scheduler for TiledScheduler {
    fn render(&self, camera: Camera, settings: RenderSettings, world: &World) -> RenderResult {
        // Rougly estimate render time here
        estimate_render_time(&camera, world, &settings, 2);

        // Clamp tile size to minimum of 1 and maximum of image width.
        // -> This way, at least 1 tile fits entirely into the image.
        let tile_size_clamped = self
            .tile_size
            .clamp(1, u32::min(settings.image_height, settings.image_width));

        let begin = Instant::now();

        // Compute the number of tiles that fit into the image, including tiles that fit only partially.
        let num_tiles_ver = settings.image_height.div_ceil(tile_size_clamped);
        let num_tiles_hor = settings.image_width.div_ceil(tile_size_clamped);

        let mut tiles = Vec::<ImageRegion>::new();
        for (yi, xi) in (0..num_tiles_ver).cartesian_product(0..num_tiles_hor) {
            let (tile_x, tile_y) = (xi * tile_size_clamped, yi * tile_size_clamped);
            // Calculate the actual width and height of the tile. This only matters at the right and bottom
            // edges, where a tile might not fit entirely into the screen, and we have to crop it to fit.
            let tile_width = (settings.image_width - tile_x).clamp(0, tile_size_clamped);
            let tile_height = (settings.image_height - tile_y).clamp(0, tile_size_clamped);

            tiles.push(ImageRegion::new(tile_x, tile_y, tile_width, tile_height));
        }

        // Render out the tiles
        let rendered_tiles: Vec<Vec<Color>> = tiles
            // Rayon does all the thread magic for us here
            .par_iter()
            .map(|tile| render_region(&camera, &settings, &world, tile.clone()))
            .collect();

        // Flatten the rendered tiles to the final image. This is a bit finicky.
        // Helper func to flatten a row of tiles: Read all first pixel rows of all tiles, then all second, etc ...
        let fn_flatten_tilerow = |tile_row: &[Vec<Color>], tile_height: u32| -> Vec<Color> {
            let mut flattened_colors = Vec::default();
            for y in 0..tile_height {
                for tile in tile_row.iter() {
                    let tile_width = tile.len() / tile_height as usize;
                    let begin_idx = tile_width * y as usize;

                    flattened_colors.extend_from_slice(&tile[begin_idx..(begin_idx + tile_width)]);
                }
            }

            flattened_colors
        };

        let pixels = rendered_tiles
            .chunks(num_tiles_hor as usize)
            .map(|tile_row| {
                // The first tile in every row is guaranted to have full width, so we can use
                // the tile size directly here to get the pixel height of the row.
                let tile_height = tile_row[0].len() as u32 / tile_size_clamped;
                fn_flatten_tilerow(tile_row, tile_height)
            })
            // The iterator now contains a vector of lists of colors in correct order. We can use a simple flatten now.
            .flatten()
            .map(|c| color_to_pixel(&c))
            .collect();

        RenderResult {
            pixels: pixels,
            time_elapsed: begin.elapsed().as_secs_f64() as Float,
            image_height: settings.image_height,
            image_width: settings.image_width,
            num_samples: settings.samples_per_pixel,
            max_bounces: settings.max_bounces,
            num_objects: world.objects_bvh.num_primitives(),
        }
    }
}
