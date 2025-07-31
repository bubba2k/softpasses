use super::hittable::{HittableList, HittableTrait};
use super::material::{MaterialTrait};
use crate::camera::{Camera, RenderSettings};
use crate::math::{ray::Ray, vector::{Vec3f, Color}, util};

// Render a specific region of the image.
pub fn render_region(cam: Camera, settings: RenderSettings, world: HittableList, region: util::ImageRegion) -> Vec<Color> {
    let mut colors: Vec<Color> = Vec::new();
    let offset_range = 1.0 / settings.image_height as f32;

    for y in region.y.0..region.y.1 {
        for x in region.x.0..region.x.1 {
            let u = x as  f32 / settings.image_width as f32;
            let v = y as f32 / settings.image_height as f32;
            let mut color: Color = Color::default();
            // Perform multisampling here.
            for _ in 0..settings.samples_per_pixel {
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
                color += trace_ray(&ray, &settings, &world, 0) * (1.0 / settings.samples_per_pixel as f32);
            }
            colors.push(color);
        }
    }
    colors
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