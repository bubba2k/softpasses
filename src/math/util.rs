use crate::math::vector::Vec3f;

#[derive(Clone, Copy)]
pub struct Interval {
    pub min: f32,
    pub max: f32,
}

impl Interval {
    pub const EMPTY:    Self = Interval { min: f32::INFINITY, max: f32::NEG_INFINITY };
    pub const UNIVERSE: Self = Interval { min: f32::NEG_INFINITY, max: f32::INFINITY };

    pub fn new(min: f32, max: f32) -> Self {
        Interval {
            min: min,
            max: max,
        }
    }

    pub fn size(&self) -> f32 {
        self.max - self.min
    }

    pub fn contains(&self, x: f32) -> bool {
        self.min <= x && x <= self.max
    }

    pub fn surrounds(&self, x: f32) -> bool {
        self.min < x && x < self.max
    }

    pub fn clamp(&self, x: f32) -> f32 {
        num::clamp(x, self.min, self.max)
    }
}

#[derive(Clone)]
pub struct ImageRegion {
    pub x: (u32, u32),
    pub y: (u32, u32),
}

impl ImageRegion {
    pub const fn new(x_begin: u32, y_begin: u32, x_size: u32, y_size: u32) -> Self {
        ImageRegion {
            x: (x_begin, x_begin + x_size),
            y: (y_begin, y_begin + y_size),
        }
    }

    pub const fn whole_image(image_width: u32, image_height: u32) -> Self {
        Self::new(0, 0, image_width, image_height)
    }
}

pub fn linear_to_gamma(linear_component: f32) -> f32 {
    num::clamp(linear_component, 0.0, 1.0).sqrt()
}

pub fn rand_range_f(low: f32, high: f32) -> f32 {
    fastrand::f32() * (high - low) + low
}

pub fn rand_vec3_range(min: f32, max: f32) -> Vec3f {
    Vec3f::new( rand_range_f(min, max), 
                rand_range_f(min, max), 
                rand_range_f(min, max),
    )
}

pub fn rand_unit_vec() -> Vec3f {
    loop {
        let vec_rand = rand_vec3_range(-1.0, 1.0);
        let length_squared = vec_rand.length_squared();

        // Reject all vectors that are not inside the unit sphere
        // or so small that they would cause nasty precision/overflow errors
        if length_squared <= 1.0 && length_squared > 1.0e-100 {
            return vec_rand / length_squared.sqrt();
        }
    }
}

pub fn rand_vec_on_unit_disc() -> Vec3f {
    loop {
        let vec = Vec3f::new(rand_range_f(-1.0, 1.0),
                                          rand_range_f(-1.0, 1.0), 0.0);

        if vec.length_squared() < 1.0 {
            return vec;
        }
    }
}

pub fn rand_unit_vec_on_hemisphere(normal: &Vec3f) -> Vec3f {
    let rnd_vec = rand_unit_vec();
    if rnd_vec.dot(normal) > 0.0 {
        rnd_vec
    } else {
        -rnd_vec
    }
}

pub fn rand_bool(p: f32) -> bool {
    fastrand::f32() < p
}