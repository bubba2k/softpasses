use crate::math::vector::{Float, Vec3f};

#[derive(Debug, Clone)]
pub struct Ray {
    pub orig: Vec3f,
    pub dir: Vec3f,
}

impl Ray {
    pub fn new(origin: &Vec3f, dir: &Vec3f) -> Self {
        Ray {
            orig: origin.clone(),
            dir: dir.clone(),
        }
    }

    pub fn at(&self, t: Float) -> Vec3f {
        self.orig + (self.dir * t)
    }

    pub fn step(&self, fac: Float) -> Self {
        Ray {
            orig: self.orig + (self.dir) * fac,
            dir: self.dir,
        }
    }
}
