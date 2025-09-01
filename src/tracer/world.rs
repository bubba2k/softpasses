use crate::math::vector::{Color, Float, Vec3f};
use crate::tracer::bvh::BVH;
use crate::tracer::hittable::{Hittable};
use crate::tracer::texture::Texture;

#[derive(Clone)]
pub struct World {
    pub objects_bvh: BVH<Hittable>,
    pub background: Background,
}

impl World {
    pub fn new(objects: Vec<Hittable>, background: Background) -> Self {
        let bvh = BVH::new(objects);

        World {
            objects_bvh: bvh,
            background,
        }
    }
}

#[derive(Clone)]
pub enum Background {
    Solid(Color),
    Custom(fn(Vec3f) -> Color),
    EnvironmentMap(Texture, Float),
}

impl Background {
    pub fn from_function(f: fn(Vec3f) -> Color) -> Self {
        Background::Custom(f)
    }

    pub fn from_solid_color(color: Color) -> Self {
        Background::Solid(color)
    }

    pub fn from_environment_texture(texture: Texture, y_rotation: Float) -> Self {
        Background::EnvironmentMap(texture, y_rotation)
    }

    pub fn sample(&self, dir: Vec3f) -> Color {
        match self {
            Background::Solid(color) => color.clone(),
            Background::Custom(f) => f(dir),
            Background::EnvironmentMap(tex, rot) => {
                // Apply y_rotation (rot) to the direction vector around the Y axis
                let (sin_r, cos_r) = rot.sin_cos();
                let rotated_dir = Vec3f::new(
                    cos_r * dir.x + sin_r * dir.z,
                    dir.y,
                    -sin_r * dir.x + cos_r * dir.z,
                );

                let theta = -rotated_dir.x.atan2(rotated_dir.z); // longitude (azimuth)
                let phi = rotated_dir.y.clamp(-1.0, 1.0).asin(); // latitude (elevation)

                let u = 0.5 + theta / (2.0 * std::f64::consts::PI) as Float;
                let v = 0.5 - phi / std::f64::consts::PI as Float;

                tex.query_uv_bilinear(u, v)
            }
        }
    }
}
