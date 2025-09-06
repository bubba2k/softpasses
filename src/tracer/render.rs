use itertools::Itertools;
use rayon::prelude::*;

use super::material::MaterialTrait;
use crate::math::ray::Ray;
use crate::math::util::{self, ImageRegion};
use crate::math::vector::{Color, Float, Pixel};
use crate::tracer::camera::Camera;
use crate::tracer::hittable::HittableTrait;
use crate::tracer::texture::Texture;
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

// Return the albedo of the first object/material hit
// TODO: Instead return albedo of the first non-transmission hit
fn trace_ray_albedo(ray: &Ray, settings: &RenderSettings, world: &World, _bounce: u32) -> Color {
    static COLOR_BLACK: Color = Color::new(0.0, 0.0, 0.0);
    // Abort if max bounce is reached.
    // Fire the ray. See if it hits anything.
    if let Some(hit) = world.objects_bvh.try_hit(ray, settings.ray_limits, 0) {
        match hit.material.scatter(ray, &hit) {
            (_, Some(color)) => color,
            (_, None) => COLOR_BLACK,
        }
    } else {
        // The ray did not hit anything. Return black
        world.background.sample(ray.dir)
    }
}

// Return the normal of the first object/material hit
// TODO: Instead return normal of the first non-transmission hit
fn trace_ray_normal(ray: &Ray, settings: &RenderSettings, world: &World, _bounce: u32) -> Color {
    static COLOR_BLACK: Color = Color::new(0.0, 0.0, 0.0);
    // Abort if max bounce is reached.
    // Fire the ray. See if it hits anything.
    if let Some(hit) = world.objects_bvh.try_hit(ray, settings.ray_limits, 0) {
        if hit.front_face {
            hit.normal
        } else {
            -hit.normal
        }
    } else {
        // We pretend the background is a perfect sphere (that the camera is inside of)
        // So we return the normal of that sphere
        -ray.dir
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

fn trace_ray_multi(
    ray: &Ray,
    settings: &RenderSettings,
    world: &World,
    _bounce: u32,
) -> (Color, Color, Color) {
    static COLOR_BLACK: Color = Color::new(0.0, 0.0, 0.0);
    // Initialize ray_color to the multiplicative neutral element.
    let mut ray_color: Color = Color::new(1.0, 1.0, 1.0);

    // Initialize ray_albedo and ray_normal as if the initial ray had immediately hit
    // the background. Which is exactly what happens if the ray hits nothing on the first bounce.
    let mut ray_albedo = world.background.sample(ray.dir);
    let mut ray_normal = -ray.dir;

    let mut current_ray: Ray = ray.clone();
    let mut bounce_counter = 0;
    loop {
        if bounce_counter == settings.max_bounces {
            // If max bounces where reached, the ray never hit a light source
            return (COLOR_BLACK, ray_albedo, ray_normal);
        }
        // Fire the ray. See if it hits anything.
        if let Some(hit) =
            world
                .objects_bvh
                .try_hit(&current_ray, settings.ray_limits, bounce_counter)
        {
            // The albedo and normal are computed exactly ONCE on the very first bounce.
            if bounce_counter == 0 {
                ray_normal = if hit.front_face {
                    hit.normal
                } else {
                    -hit.normal
                };

                ray_albedo = match hit.material.scatter(ray, &hit) {
                    (_, Some(color)) => color,
                    (_, None) => COLOR_BLACK,
                };
            }

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
                    return (ray_color * color_att, ray_albedo, ray_normal);
                }
                (None, None) => {
                    // The ray was absorbed and no attenuation color was given.
                    // This should not happen, but we have to handle the case. Assume a black hole.
                    return (COLOR_BLACK, ray_albedo, ray_normal);
                }
            }
        } else {
            // If the ray did not hit objects, we assume it hit the background / sky.
            return (
                ray_color * world.background.sample(current_ray.dir),
                ray_albedo,
                ray_normal,
            );
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
    trace_func: fn(ray: &Ray, settings: &RenderSettings, world: &World, bounce: u32) -> Color,
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
                color += trace_func(&ray, &settings, &world, 0)
                    * (1.0 / settings.samples_per_pixel as Float);
            }
            colors.push(color);
        }
    }
    colors
}

pub fn color_to_pixel(c: &Color) -> Pixel {
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
    pub denoise: bool,
}

pub struct RenderResult {
    pub color_pass: Option<Texture>,
    pub albedo_pass: Option<Texture>,
    pub normal_pass: Option<Texture>,
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

impl RenderResult {
    pub fn write_images(
        &self,
        base_dir: &std::path::Path,
        file_extension: &str,
    ) -> Result<(), String> {
        // Attempt to create the directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(base_dir) {
            return Err(format!("Failed to create directory {:?}: {}", base_dir, e));
        }

        for pass in [
            (&self.albedo_pass, "albedo_pass"),
            (&self.color_pass, "color_pass"),
            (&self.normal_pass, "normal_pass"),
        ] {
            if let (Some(pass_texture), name) = pass {
                let file_name = String::from(name) + file_extension;
                let mut full_path = std::path::PathBuf::from(base_dir);
                full_path.push(file_name);
                pass_texture.write_32f(full_path.as_path())?;
            }
        }

        Ok(())
    }
}

pub trait Scheduler {
    fn render_pass(
        &self,
        camera: &Camera,
        settings: RenderSettings,
        world: &World,
        trace_func: fn(ray: &Ray, settings: &RenderSettings, world: &World, bounce: u32) -> Color,
    ) -> Vec<Color>;

    fn estimate_render_time(&self, camera: &Camera, world: &World, settings: &RenderSettings) {
        // Attempt to get a somewhat accurate estimate of the total render time here.
        // Render the entire image once at 1 spp, then extrapolate the full render time from that.
        let estimate_settings = RenderSettings {
            samples_per_pixel: 1,
            ..*settings
        };
        let estimate_start = std::time::Instant::now();
        self.render_pass(camera, estimate_settings, world, trace_ray);

        // This should give a rough estimation.
        let elapsed = estimate_start.elapsed().as_secs_f64() as f64;
        let estimate_duration = elapsed as f64
                                   * settings.samples_per_pixel as f64 // Attenuate for actual spp value of full render pass
                                   + (settings.denoise as i32 as f64) * elapsed * 16.0; // Add estimated time for albedo/normal passes, if necessary.
        let estimate_hours = estimate_duration as u32 / 3600;
        let estimate_minutes = (estimate_duration as u32 / 60) % 60;
        let estimate_seconds = estimate_duration as u32 % 60;

        let now = chrono::Local::now();
        eprintln!(
            "Started at {}\nEst. render time: {:02}:{:02}:{:02}",
            now.format("%H:%M:%S"),
            estimate_hours,
            estimate_minutes,
            estimate_seconds
        );
    }

    fn render(&self, camera: Camera, settings: RenderSettings, world: &World) -> RenderResult {
        // Rougly estimate render time here
        self.estimate_render_time(&camera, world, &settings);

        let begin = std::time::Instant::now();

        // Compute the passes
        let (color_pass, albedo_pass, normal_pass) = {
            let aux_pass_settings = RenderSettings {
                samples_per_pixel: 8,
                ..settings
            };

            eprintln!("Performing color pass...");
            let color_pass = self.render_pass(&camera, settings.clone(), world, trace_ray_it);
            eprintln!("Performing albedo pass...");
            let albedo_pass =
                self.render_pass(&camera, aux_pass_settings.clone(), world, trace_ray_albedo);
            eprintln!("Performing normal pass...");
            let normal_pass =
                self.render_pass(&camera, aux_pass_settings.clone(), world, trace_ray_normal);

            (
                Some(Texture::from_raw(
                    settings.image_width as usize,
                    settings.image_height as usize,
                    color_pass.clone(),
                )),
                Some(Texture::from_raw(
                    settings.image_width as usize,
                    settings.image_height as usize,
                    albedo_pass.clone(),
                )),
                Some(Texture::from_raw(
                    settings.image_width as usize,
                    settings.image_height as usize,
                    normal_pass.clone(),
                )),
            )
        };

        RenderResult {
            color_pass: color_pass,
            albedo_pass: albedo_pass,
            normal_pass: normal_pass,
            time_elapsed: begin.elapsed().as_secs_f64() as Float,
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

impl Scheduler for TiledScheduler {
    fn render_pass(
        &self,
        camera: &Camera,
        settings: RenderSettings,
        world: &World,
        trace_func: fn(ray: &Ray, settings: &RenderSettings, world: &World, bounce: u32) -> Color,
    ) -> Vec<Color> {
        // Clamp tile size to minimum of 1 and maximum of image width.
        // -> This way, at least 1 tile fits entirely into the image.
        let tile_size_clamped = self
            .tile_size
            .clamp(1, u32::min(settings.image_height, settings.image_width));

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
            .map(|tile| render_region(&camera, &settings, &world, tile.clone(), trace_func))
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

        rendered_tiles
            .chunks(num_tiles_hor as usize)
            .map(|tile_row| {
                // The first tile in every row is guaranted to have full width, so we can use
                // the tile size directly here to get the pixel height of the row.
                let tile_height = tile_row[0].len() as u32 / tile_size_clamped;
                fn_flatten_tilerow(tile_row, tile_height)
            })
            // The iterator now contains a vector of lists of colors in correct order. We can use a simple flatten now.
            .flatten()
            .collect()
    }
}

impl TiledScheduler {
    pub fn new(tile_size: u32) -> Self {
        TiledScheduler {
            tile_size: tile_size,
        }
    }
}
