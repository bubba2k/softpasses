use glam::{Vec3, Affine3A, vec3, Quat};

pub trait Transformable {
    fn apply_transform(self, transform: &Transform) -> Self;
}

pub struct Transform {
    affine: Affine3A,
}

impl Transformable for Transform {
    fn apply_transform(self, transform: &Transform) -> Self {
        Transform {
            affine: transform.affine * self.affine
        }
    }
}

impl Transform {
    pub fn new() -> Self {
        Transform {
            affine: Affine3A::IDENTITY,
        }
    }

    pub fn translate(self, offset: Vec3) -> Self {
        Transform {
            affine: Affine3A::from_translation(offset) * self.affine,
        }
    }

    pub fn scale_uniform(self, scale: f32) -> Self {
        Transform {
            affine: Affine3A::from_scale(vec3(scale, scale, scale)) * self.affine,
        }
    }

    pub fn scale(self, scale: Vec3) -> Self {
        Transform {
            affine: Affine3A::from_scale(scale) * self.affine,
        }
    }

    pub fn rotate_x(self, angle: f32) -> Self {
        Transform {
            affine: Affine3A::from_rotation_x(angle) * self.affine,
        }
    }

    pub fn rotate_y(self, angle: f32) -> Self {
        Transform {
            affine: Affine3A::from_rotation_y(angle) * self.affine,
        }
    }

    pub fn rotate_z(self, angle: f32) -> Self {
        Transform {
            affine: Affine3A::from_rotation_z(angle) * self.affine,
        }
    }

    pub fn rotate_axis(self, axis: Vec3, angle: f32) -> Self {
        Transform {
            affine: Affine3A::from_axis_angle(axis, angle) * self.affine,
        }
    }

    pub fn rotate_quat(self, quat: Quat) -> Self {
        Transform {
            affine: Affine3A::from_quat(quat) * self.affine,
        }
    }

    pub fn get_affine(&self) -> &Affine3A {
        &self.affine
    }
}