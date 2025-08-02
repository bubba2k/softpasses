use crate::{math::vector::{Color, Vec3f}, tracer::hittable::HittableList};

#[derive(Clone)]
pub struct World {
    pub objects: HittableList,
    pub background: Background,
}

impl World {
    pub fn new(objects: HittableList, background: Background) -> Self {
        World {
            objects,
            background,
        }
    }
}

#[derive(Clone)]
pub enum Background {
    Solid(Color),
    Custom(fn(Vec3f) -> Color),
}

impl Background {
    pub fn from_function(f: fn(Vec3f) -> Color) -> Background {
        Background::Custom(f)
    }

    pub fn from_solid_color(color: Color) -> Background {
        Background::Solid(color)
    }

    pub fn sample(&self, dir: Vec3f) -> Color {
        match self  {
            Background::Solid(color) => color.clone(),
            Background::Custom(f) => f(dir),
        }
    }
}