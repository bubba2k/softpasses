use std::array;

use glam::DVec3;
use itertools::Itertools;
use rayon::prelude::*;

use super::material::MaterialTrait;
use crate::math::ray::Ray;
use crate::math::util::{self, ImageRegion};
use crate::math::vector::{Color, Float, vec3_from_dvec3};
use crate::tracer::camera::Camera;
use crate::tracer::hittable::{Hittable, HittableTrait, RayInfo};
use crate::tracer::texture::Texture;
use crate::tracer::world::World;
use std::collections::HashMap;

/// A group of named and ordered renderpasses, e.g. Color, Albedo and Normal passes combined into one.
/// Template parameter @N is the number of passes.
/// Grouping of renderpasses allows them to be computed "simultaneously" by smart definition of
/// `trace_sample`, instead of having to traverse the scene over and over again for each pass.
/// This is especially relevant for recursive - i.e. those involving numerous bounces - passes, since
/// a *significant* portion of the runt ime goes towards computation of ray traversal.
pub trait RenderPipeline<const N: usize> {
    /// Compute a sample, where
    /// @ray is the initial ray shot into the scene (usually from a camera)
    /// @settings is the settings to render with.
    /// @world is the scene
    /// @ray_info is extended information about the intial ray
    fn trace_sample(
        ray: &Ray,
        settings: &RenderSettings,
        world: &World,
        ray_info: &RayInfo,
    ) -> [Color; N];

    /// Return the average of the accumulated samples.
    fn yield_estimate(&self) -> [Color; N];

    /// Save a previously computed (by `trace_sample`, usually) sample.
    fn accumulate_sample(
        &mut self,
        ray: &Ray,
        settings: &RenderSettings,
        world: &World,
        ray_info: &RayInfo,
    );

    /// Returns the name of the passes in order.
    fn pass_names(i: usize) -> String;

    /// Erase accumulated samples
    fn clear(&mut self);
}

/// The default pipeline that computes color, albedo and normal passes.
pub struct DefaultPipeline {
    // Accumulate in double precision
    sums: [glam::DVec3; 3],
    num_accumulated_samples: u32,
}

impl Default for DefaultPipeline {
    fn default() -> Self {
        let sums = array::from_fn(|_| glam::dvec3(0.0, 0.0, 0.0));
        DefaultPipeline {
            sums,
            num_accumulated_samples: 0,
        }
    }
}

impl RenderPipeline<3> for DefaultPipeline {
    fn pass_names(i: usize) -> String {
        String::from(["color", "albedo", "normal"][i])
    }

    fn accumulate_sample(
        &mut self,
        ray: &Ray,
        settings: &RenderSettings,
        world: &World,
        ray_info: &RayInfo,
    ) {
        let sample = Self::trace_sample(ray, settings, world, ray_info);
        for i in 0..3 {
            self.sums[i] += DVec3::from(sample[i]);
        }
        self.num_accumulated_samples += 1;
    }

    fn clear(&mut self) {
        for sum in self.sums.iter_mut() {
            *sum = glam::dvec3(0.0, 0.0, 0.0)
        }
        self.num_accumulated_samples = 0;
    }

    fn yield_estimate(&self) -> [Color; 3] {
        let samples_inv = 1.0 / self.num_accumulated_samples as f64;
        let mut estimate: [Color; 3] = [Color::default(); 3];

        for i in 0..3 {
            estimate[i] = vec3_from_dvec3(self.sums[i] * samples_inv);
        }

        estimate
    }

    fn trace_sample(
        ray: &Ray,
        settings: &RenderSettings,
        world: &World,
        ray_info: &RayInfo,
    ) -> [Color; 3] {
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
                return [COLOR_BLACK, ray_albedo, ray_normal];
            }
            // Fire the ray. See if it hits anything.
            if let Some(hit) =
                world
                    .objects_bvh
                    .try_hit(&current_ray, &settings.ray_limits, ray_info)
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
                        return [ray_color * color_att, ray_albedo, ray_normal];
                    }
                    (None, None) => {
                        // The ray was absorbed and no attenuation color was given.
                        // This should not happen, but we have to handle the case. Assume a black hole.
                        return [COLOR_BLACK, ray_albedo, ray_normal];
                    }
                }
            } else {
                // If the ray did not hit objects, we assume it hit the background / sky.
                return [
                    ray_color * world.background.sample(current_ray.dir),
                    ray_albedo,
                    ray_normal,
                ];
            }
            bounce_counter = bounce_counter + 1;
        }
    }
}

pub struct BVHDebugPipeline {
    sums: [glam::DVec3; 3],
    num_accumulated_samples: usize,
}

impl Default for BVHDebugPipeline {
    fn default() -> Self {
        BVHDebugPipeline {
            sums: std::array::from_fn(|_| glam::dvec3(0.0, 0.0, 0.0)),
            num_accumulated_samples: 0,
        }
    }
}

impl RenderPipeline<3> for BVHDebugPipeline {
    fn clear(&mut self) {
        self.num_accumulated_samples = 0;
        self.sums = std::array::from_fn(|_| glam::dvec3(0.0, 0.0, 0.0))
    }

    fn pass_names(i: usize) -> String {
        String::from(
            [
                "num_aabb_intersects",
                "num_aabb_checks",
                "num_primitive_checks",
            ][i],
        )
    }

    fn yield_estimate(&self) -> [Color; 3] {
        let samples_inv = 1.0 / self.num_accumulated_samples as f64;
        let mut estimate: [Color; 3] = [Color::default(); 3];

        for i in 0..3 {
            estimate[i] = vec3_from_dvec3(self.sums[i] * samples_inv);
        }

        estimate
    }

    fn accumulate_sample(
        &mut self,
        ray: &Ray,
        settings: &RenderSettings,
        world: &World,
        ray_info: &RayInfo,
    ) {
        let sample = Self::trace_sample(ray, settings, world, ray_info);

        for i in 0..3 {
            self.sums[i] += DVec3::from(sample[i]);
        }

        self.num_accumulated_samples += 1;
    }

    fn trace_sample(
        ray: &Ray,
        settings: &RenderSettings,
        world: &World,
        _ray_info: &RayInfo,
    ) -> [Color; 3] {
        let mut passes: [u32; 3] = [0, 0, 0];
        // Remember:
        // 0: "num_aabb_intersects",
        // 1: "num_aabb_checks",
        // 2: "num_primitive_checks",

        for mesh in world.objects_bvh.hittables.iter().flat_map(|e| {
            if let Hittable::BVHMesh(mesh) = e {
                Some(mesh)
            } else {
                None
            }
        }) {
            let query_result = mesh.try_hit_ordered(ray, &settings.ray_limits);
            passes[0] = query_result.num_aabb_hits;
            passes[1] = query_result.num_aabb_checks;
            passes[2] = query_result.num_primitve_checks;
        }

        let mut res: [Color; 3] = array::from_fn(|_| Color::default());
        for i in 0..3 {
            res[i] = glam::vec3(passes[i] as f32, passes[i] as f32, passes[i] as f32);
        }
        res
    }
}

// Render a specific region of the image.
pub fn render_region_with_pass<const N: usize, RP: RenderPipeline<N> + Default>(
    cam: &Camera,
    settings: &RenderSettings,
    world: &World,
    region: util::ImageRegion,
) -> [Vec<Color>; N] {
    let mut renderpass: RP = RP::default();
    let pixel_count = ((region.x.1 - region.x.0) * (region.y.1 - region.y.0)) as usize;
    let mut result: [Vec<Color>; N] = std::array::from_fn(|_| Vec::with_capacity(pixel_count));
    let offset_range = 1.0 / settings.image_height as Float;

    for y in region.y.0..region.y.1 {
        for x in region.x.0..region.x.1 {
            let u = x as Float / settings.image_width as Float;
            let v = y as Float / settings.image_height as Float;
            renderpass.clear();
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

                let ray_info = RayInfo { num_bounces: 0 };

                renderpass.accumulate_sample(&ray, &settings, &world, &ray_info);
            }
            renderpass
                .yield_estimate()
                .iter()
                .enumerate()
                .for_each(|pass| result[pass.0].push(*pass.1));
        }
    }
    result
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
    pub passes: HashMap<String, Texture>,
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

        for (name, pass_texture) in self.passes.iter() {
            let file_name = String::from(name) + file_extension;
            let mut full_path = std::path::PathBuf::from(base_dir);
            full_path.push(file_name);
            pass_texture.write_32f(full_path.as_path())?;
        }

        Ok(())
    }
}

pub trait Scheduler {
    fn render_with_pass<RP: RenderPipeline<N> + Default, const N: usize>(
        &self,
        camera: &Camera,
        settings: RenderSettings,
        world: &World,
    ) -> [Vec<Color>; N];

    fn estimate_render_time<RP: RenderPipeline<N> + Default, const N: usize>(
        &self,
        camera: &Camera,
        world: &World,
        settings: &RenderSettings,
    ) {
        // Attempt to get a somewhat accurate estimate of the total render time here.
        // Render the entire image once at 1 spp, then extrapolate the full render time from that.
        let estimate_settings = RenderSettings {
            samples_per_pixel: 1,
            ray_limits: settings.ray_limits.clone(),
            ..*settings
        };
        let estimate_start = std::time::Instant::now();
        self.render_with_pass::<RP, _>(camera, estimate_settings, world);

        // This should give a rough estimation.
        let elapsed = estimate_start.elapsed().as_secs_f64() as f64;
        let estimate_duration = elapsed as f64 * settings.samples_per_pixel as f64; // Attenuate for actual spp value of full render pass
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

    fn render<RP: RenderPipeline<N> + Default, const N: usize>(
        &self,
        camera: Camera,
        settings: RenderSettings,
        world: &World,
    ) -> RenderResult {
        // Rougly estimate render time here
        self.estimate_render_time::<RP, N>(&camera, world, &settings);

        let begin = std::time::Instant::now();

        // Compute the passes
        let passes: [Vec<Color>; _] =
            self.render_with_pass::<RP, _>(&camera, settings.clone(), world);

        // Make the hashmap
        let mut passes_map: HashMap<String, Texture> = HashMap::default();
        for i in 0..N {
            passes_map.insert(
                RP::pass_names(i),
                Texture::from_raw(
                    settings.image_width as usize,
                    settings.image_height as usize,
                    // TODO: Might be nice to explicitely remove this clone at some point
                    passes[i].clone(),
                ),
            );
        }

        RenderResult {
            passes: passes_map,
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
    fn render_with_pass<RP: RenderPipeline<N> + Default, const N: usize>(
        &self,
        camera: &Camera,
        settings: RenderSettings,
        world: &World,
    ) -> [Vec<Color>; N] {
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
        let rendered_tiles: Vec<[Vec<Color>; N]> = tiles
            // Rayon does all the thread magic for us here
            .par_iter()
            .map(|tile| render_region_with_pass::<N, RP>(&camera, &settings, &world, tile.clone()))
            .collect();

        let mut per_channel_tiles: [Vec<Vec<Color>>; N] =
            std::array::from_fn(|_| Vec::with_capacity(rendered_tiles.len()));

        for tile in rendered_tiles.iter() {
            for i in 0..N {
                per_channel_tiles[i].push(tile[i].clone());
            }
        }

        let assembled_per_channel: [Vec<Color>; N] = std::array::from_fn(|i| {
            let tiles_for_channel = per_channel_tiles[i].clone();
            Self::assemble_tiles(tiles_for_channel, num_tiles_hor as usize, tile_size_clamped)
        });

        assembled_per_channel
    }
}

impl TiledScheduler {
    // Helper func to flatten a row of tiles: Read all first pixel rows of all tiles, then all second, etc ...
    fn flatten_tilerow(tile_row: &[Vec<Color>], tile_height: u32) -> Vec<Color> {
        let mut flattened_colors = Vec::default();
        for y in 0..tile_height {
            for tile in tile_row.iter() {
                let tile_width = tile.len() / tile_height as usize;
                let begin_idx = tile_width * y as usize;
                flattened_colors.extend_from_slice(&tile[begin_idx..(begin_idx + tile_width)]);
            }
        }
        flattened_colors
    }

    // Assemble rendered tiles back into the full image
    fn assemble_tiles(
        rendered_tiles: Vec<Vec<Color>>,
        num_tiles_hor: usize,
        tile_size_clamped: u32,
    ) -> Vec<Color> {
        rendered_tiles
            .chunks(num_tiles_hor as usize)
            .map(|tile_row| {
                // The first tile in every row is guaranted to have full width, so we can use
                // the tile size directly here to get the pixel height of the row.
                let tile_height = tile_row[0].len() as u32 / tile_size_clamped;
                Self::flatten_tilerow(tile_row, tile_height)
            })
            // The iterator now contains a vector of lists of colors in correct order. We can use a simple flatten now.
            .flatten()
            .collect()
    }

    pub fn new(tile_size: u32) -> Self {
        TiledScheduler {
            tile_size: tile_size,
        }
    }
}
