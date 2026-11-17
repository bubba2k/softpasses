use crate::math::vector::{Color, Float, Vec3f};
use crate::tracer::material::Material;
use crate::tracer::hittable::{Hittable, HittableTrait};
use crate::tracer::texture::Texture;

type BvhImpl = crate::bvh::custom_impl::BvhCustom<Hittable>;

pub struct SceneBuilder {
    pub hittables: Vec<Hittable>,
    pub materials: Vec<Material>,
    pub background: Background,
}

impl SceneBuilder {
    pub fn new() -> Self {
        SceneBuilder { hittables: vec![], materials: vec![], background: Background::Solid(glam::vec3(1.0, 1.0, 1.0)) }
    }

    pub fn add_hittable(self, hittable: Hittable) -> Self {
        let mut new_hittables = self.hittables;
        new_hittables.push(hittable);

        SceneBuilder { 
            hittables: new_hittables, 
            ..self
        }
    }

    pub fn add_hittables(self, hittables: Vec<Hittable>) -> Self {
        let mut hittables_mut = hittables;
        let mut new_hittables = self.hittables;
        new_hittables.append(&mut hittables_mut);

        SceneBuilder {
            hittables: new_hittables, 
            ..self
        }
    }

    pub fn set_background(self, background: Background) -> Self {
        SceneBuilder { background: background, ..self }
    }

    pub fn set_materials(self, materials: Vec<Material>) -> Self {
        SceneBuilder { materials: materials, ..self }
    }

    pub fn build(self) -> Scene {
        Scene {
            objects_bvh: BvhImpl::new(self.hittables),
            materials: self.materials,
            background: self.background,
        }
    }
}

pub struct Scene {
    pub objects_bvh: BvhImpl,
    pub materials: Vec<Material>,
    pub background: Background,
}

#[derive(Clone)]
pub enum Background {
    Solid(Color),
    Custom(fn(Vec3f) -> Color),
    EnvironmentMap(Texture, Float),
}

#[allow(dead_code)]
impl Background {
    pub fn from_function(f: fn(Vec3f) -> Color) -> Self {
        Background::Custom(f)
    }

    pub fn from_solid_color(color: Color) -> Self {
        Background::Solid(color)
    }

    pub fn from_environment_texture(texture: Texture, y_rotation: Float, exposure: Float) -> Self {
        let attenuated_texture = Texture {
            height: texture.height,
            width: texture.width,
            data: texture.data.iter().map(|color| color * exposure).collect(),
        };
        Background::EnvironmentMap(attenuated_texture, y_rotation)
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
