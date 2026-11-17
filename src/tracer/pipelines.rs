use std::array;

use glam::DVec3;

use super::material::MaterialTrait;
use crate::bvh::BvhTrait;
use crate::math::ray::Ray;
use crate::math::vector::{Color, vec3_from_dvec3};
use crate::tracer::world::Scene;

use crate::tracer::render::RenderSettings;


/// A group of named and ordered renderpasses (e.g. Color, Albedo and Normal passes) combined into one.
/// Template parameter @N is the number of passes.
/// Grouping of renderpasses allows them to be computed simultaneously by smart definition of
/// `trace_sample`, instead of having to traverse the scene over and over again for each pass.
/// This is especially relevant for recursive - i.e. those involving numerous bounces - passes, since
/// a *significant* portion of the runtime goes towards computation of ray traversal.
pub trait RenderPipeline<const N: usize> {
    /// Compute a single sample, where
    /// 
    /// # Arguments
    /// `ray` - The initial ray shot into the scene (usually from a camera)
    /// `settings` - The settings to render with.
    /// `world` - The scene
    /// `ray_info` - Extended information about the intial ray
    fn trace_sample(
        ray: &Ray,
        settings: &RenderSettings,
        world: &Scene,
    ) -> [Color; N];

    /// Return the average of the accumulated samples.
    fn yield_estimate(&self) -> [Color; N];

    /// Save a previously computed (by `trace_sample`, usually) sample.
    fn accumulate_sample(
        &mut self,
        ray: &Ray,
        settings: &RenderSettings,
        world: &Scene,
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
        world: &Scene,
    ) {
        let sample = Self::trace_sample(ray, settings, world);
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
        world: &Scene,
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
                    .objects_bvh.try_hit(&current_ray, &settings.ray_limits)
            {
                // Get a reference to the actual material
                let hit_material = &world.materials[hit.material_id as usize];

                // The albedo and normal are computed exactly ONCE on the very first bounce.
                if bounce_counter == 0 {
                    ray_normal = if hit.front_face {
                        hit.normal
                    } else {
                        -hit.normal
                    };

                    ray_albedo = match hit_material.scatter(ray, &hit) {
                        (_, Some(color)) => color,
                        (_, None) => COLOR_BLACK,
                    };
                }

                match hit_material.scatter(&current_ray, &hit) {
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
        world: &Scene,
    ) {
        let sample = Self::trace_sample(ray, settings, world);

        for i in 0..3 {
            self.sums[i] += DVec3::from(sample[i]);
        }

        self.num_accumulated_samples += 1;
    }

    fn trace_sample(
        ray: &Ray,
        settings: &RenderSettings,
        scene: &Scene,
    ) -> [Color; 3] {
        let mut passes: [u32; 3] = [0, 0, 0];
        // Remember:
        // 0: "num_aabb_intersects",
        // 1: "num_aabb_checks",
        // 2: "num_primitive_checks",

        let query_result = scene.objects_bvh.try_intersect(ray, &settings.ray_limits);
            passes[0] = query_result.num_aabb_hits;
            passes[1] = query_result.num_aabb_checks;
            passes[2] = query_result.num_primitive_checks;

        let mut res: [Color; 3] = array::from_fn(|_| Color::default());
        for i in 0..3 {
            res[i] = glam::vec3(passes[i] as f32, passes[i] as f32, passes[i] as f32);
        }
        res
    }
}
