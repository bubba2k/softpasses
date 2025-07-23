use std::ops::{Add, AddAssign, Div, Mul, Sub, Neg, Index};

pub type Vec3f = Vec3<f32>;
pub type Color = Vec3<f32>;
pub type Pixel = Vec3<u8>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Vec3<T> {
    v: [T;3]
}

impl<T: Add<Output = T> + Copy> Add for Vec3<T> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Vec3 {
           v: [ self.v[0] + rhs.v[0],
                self.v[1] + rhs.v[1],
                self.v[2] + rhs.v[2] ]
        }
    }
}

impl<T: AddAssign + Copy> AddAssign for Vec3<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.v[0] += rhs.v[0];
        self.v[1] += rhs.v[1];
        self.v[2] += rhs.v[2];
    }
}

impl<T: Copy + Add<Output = T>> Add<T> for Vec3<T> {
    type Output = Vec3<T>;
    fn add(self, rhs: T) -> Self::Output {
        Vec3 {
            v: [ self.v[0] + rhs, 
                 self.v[1] + rhs, 
                 self.v[2] + rhs ]
        }
    }
}

impl<T: Sub<Output = T> + Copy> Sub for Vec3<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Vec3 {
           v: [ self.v[0] - rhs.v[0],
                self.v[1] - rhs.v[1],
                self.v[2] - rhs.v[2] ]
        }
    }
}

impl<T: Copy + Sub<Output = T>> Sub<T> for Vec3<T> {
    type Output = Vec3<T>;
    fn sub(self, rhs: T) -> Self::Output {
        Vec3 {
            v: [ self.v[0] - rhs, 
                 self.v[1] - rhs, 
                 self.v[2] - rhs ]
        }
    }
}

impl<T: Mul<Output = T> + Copy> Mul for Vec3<T> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        Vec3 {
           v: [ self.v[0] * rhs.v[0],
                self.v[1] * rhs.v[1],
                self.v[2] * rhs.v[2] ]
        }
    }
}

impl<T: Copy + Mul<Output = T>> Mul<T> for Vec3<T> {
    type Output = Vec3<T>;
    fn mul(self, rhs: T) -> Self::Output {
        Vec3 {
            v: [ self.v[0] * rhs, 
                 self.v[1] * rhs, 
                 self.v[2] * rhs ]
        }
    }
}

impl<T: Div<Output = T> + Copy> Div for Vec3<T> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Vec3 {
           v: [ self.v[0] / rhs.v[0],
                self.v[1] / rhs.v[1],
                self.v[2] / rhs.v[2] ]
        }
    }
}

impl<T: Copy + Div<Output = T>> Div<T> for Vec3<T> {
    type Output = Vec3<T>;
    fn div(self, rhs: T) -> Self::Output {
        Vec3 {
            v: [ self.v[0] / rhs, 
                 self.v[1] / rhs, 
                 self.v[2] / rhs ]
        }
    }
}

impl<T: Neg<Output = T> + Copy> Neg for Vec3<T> {
    type Output = Vec3<T>;
    fn neg(self) -> Self::Output {
        Vec3 {
            v: [ -self.v[0], -self.v[1], -self.v[2] ]
        }
    }
} 

impl<T: Default> Default for Vec3<T> {
    fn default() -> Self {
        Self {
            v: [ T::default(), T::default(), T::default() ]
        }
    }
}

impl<T: num::Num + Copy + Mul<Output = T> + Add<Output = T> + Sub<Output = T>> Vec3<T> {
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { v: [x, y, z] }
    }

    // Dot product
    pub fn dot(&self, rhs: &Vec3<T>) -> T {
        self.v[0] * rhs.v[0] + self.v[1] * rhs.v[1]+ self.v[2] * rhs.v[2]
    }

    // Cross product
    pub fn cross(&self, rhs: &Vec3<T>) -> Vec3<T> {
        Self { v: [ self.v[1] * rhs.v[2] - self.v[2] * rhs.v[1],
           self.v[2] * rhs.v[0] - self.v[0] * rhs.v[2],
           self.v[0] * rhs.v[1] - self.v[1] * rhs.v[0]] }
    }

    pub fn length_squared(&self) -> T {
        self.v[0] * self.v[0] + self.v[1] * self.v[1] + self.v[2] * self.v[2]
    }

    // Interpolate
    pub fn lerp(&self, other: &Vec3<T>, t: T) -> Vec3<T> {
        *self + (*other - *self) * t
    }



    // Getters
    pub fn x(&self) -> T {
        self.v[0]
    }
    pub fn y(&self) -> T {
        self.v[1]
    }
    pub fn z(&self) -> T {
        self.v[2]
    }

    pub fn r(&self) -> T {
        self.v[0]
    }
    pub fn g(&self) -> T {
        self.v[1]
    }
    pub fn b(&self) -> T {
        self.v[2]
    }
}

impl<T: Copy + Default + Neg<Output = T> + PartialOrd> Vec3<T> {
    fn quick_abs(a: T) -> T {
        if a < T::default() {
            -a
        } else {
            a
        }
    }

    pub fn abs(&self) -> Vec3<T> {
        Vec3 {
            v: [ Self::quick_abs(self.v[0]), 
                 Self::quick_abs(self.v[1]), 
                 Self::quick_abs(self.v[2]) ]
        }
    }
}

impl Vec3<f32> {
    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn normalize(&self) -> Vec3<f32>  {
        *self / self.length()
    }

    pub fn near_zero(&self) -> bool {
        self.x() < 1.0e-8 && self.y() < 1.0e-8 && self.z() < 1.0e-8
    }

    pub fn nlerp(&self, other: &Vec3<f32>, t: f32) -> Vec3<f32> {
        self.lerp(other, t).normalize()
    }

    pub fn slerp(&self, other: &Vec3<f32>, t: f32) -> Vec3<f32> {
        let dot = self.dot(other).clamp(-1.0, 1.0);
        let omega = dot.acos();
        let sin_omega = omega.sin();

        if sin_omega.abs() < 1e-6 {
            // If angle is very small, use lerp and normalize
            self.lerp(other, t).normalize()
        } else {
            *self  * ((1.0 - t) * omega).sin() / sin_omega
          + *other * (t * omega).sin() / sin_omega
        }
    }

    pub fn proj_vec(&self, vec: Vec3f) -> Vec3f {
        // Projects self onto vec
        vec * self.dot(&vec)
    }

    pub fn proj_plane(&self, plane_normal: &Vec3<f32>) -> Vec3f {
        *self - self.proj_vec(*plane_normal)
    }
}

impl Vec3<f64> {
    pub fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    pub fn normalize(&self) -> Vec3<f64>  {
        *self / self.length()
    }


}

impl Vec3<i32> {
    fn length(&self) -> f64 {
        (self.length_squared() as f64).sqrt()
    }
}

impl Vec3<i64> {
    fn length(&self) -> f64 {
        (self.length_squared() as f64).sqrt()
    }
}

impl Vec3<u32> {
    fn length(&self) -> f64 {
        (self.length_squared() as f64).sqrt()
    }
}

impl Vec3<u64> {
    fn length(&self) -> f64 {
        (self.length_squared() as f64).sqrt()
    }
}

impl<T> Index<usize> for Vec3<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.v[index]
    }
}
