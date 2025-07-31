use crate::vector::Vec3f;

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
    // Generate vectors until we find one that has the properties we want
    // Stictly speaking, NOT evenly distributed on unit sphere!
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
    // Stictly speaking, NOT evenly distributed on disc!
    loop {
        let vec = Vec3f::new(rand_range_f(-1.0, 1.0),
                                          rand_range_f(-1.0, 1.0), 0.0);

        if vec.length_squared() < 1.0 {
            return vec;
        }
    }
}

pub fn rand_unit_vec_on_hemisphere(normal: &Vec3f) -> Vec3f {
    // Strictly speaking, NOT evenly distrubuted on hemisphere.
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