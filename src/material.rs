use std::rc::Rc;

use crate::vec3::Vec3f;
use crate::util;
use crate::{ray::Ray, vec3::Color};
use crate::hittable::HitRecord;

pub trait MaterialTrait {
    // Returns None if the ray was absorbed.
    // Else, returns a new (scattered) ray and color attenuation
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>);
}

#[derive(Clone)]
pub enum Material {
    MatNormalDebug(MatNormalDebug),
    MatFaceDebug(MatFaceDebug),
    MatLambertDiffuse(MatLambertDiffuse),
    MatPrincipled(MatPrincipled),
    MatGlass(MatGlass),
    MatEmission(MatEmission),
    MatBounceDebug(MatBounceDebug),
}

impl MaterialTrait for Material {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        match self {
            Material::MatEmission(mat) => mat.scatter(ray_in, hit),
            Material::MatNormalDebug(mat) => mat.scatter(ray_in, hit),
            Material::MatFaceDebug(mat) => mat.scatter(ray_in, hit),
            Material::MatLambertDiffuse(mat) => mat.scatter(ray_in, hit),
            Material::MatPrincipled(mat) => mat.scatter(ray_in, hit),
            Material::MatGlass(mat) => mat.scatter(ray_in, hit),
            Material::MatBounceDebug(mat) => mat.scatter(ray_in, hit),
        }
    }
}

#[derive(Default, Clone)]
pub struct MatNormalDebug {}

impl MaterialTrait for MatNormalDebug {
    #[allow(unused_variables)]
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        let color = if hit.front_face {
            (hit.normal + 1.0) * 0.5
        } else {
            (-hit.normal + 1.0) * 0.5
        };

        (None, 
         Some(color))
    }
}

impl MatNormalDebug {
    pub fn new() -> Material {
        Material::MatNormalDebug(MatNormalDebug::default())
    }
}

#[derive(Default, Clone)]
pub struct MatFaceDebug {}

impl MaterialTrait for MatFaceDebug {
    #[allow(unused_variables)]
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        let color = if hit.front_face {
            Color::new(0.0, 1.0, 0.0)
        } else {
            Color::new(1.0, 0.0, 0.0)
        };
        (None, 
         Some(color))
    }
}

impl MatFaceDebug {
    pub fn new() -> Material {
        Material::MatFaceDebug(MatFaceDebug::default())
    }
}

#[derive(Clone)]
pub struct MatBounceDebug {
    limit: u32,
}

impl MatBounceDebug {
    pub fn new(limit: u32) -> Material {
        Material::MatBounceDebug(MatBounceDebug{ limit: limit})
    }
}

impl MaterialTrait for MatBounceDebug {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        static COLOR_RED:  Color = Vec3f::new(1.0, 0.0, 0.0);
        static COLOR_LOW:  Color = Vec3f::new(0.4, 0.4, 0.4);
        static COLOR_HIGH: Color = Vec3f::new(1.0, 1.0, 1.0);
        let color = if hit.num_bounces >= self.limit {
            COLOR_RED
        } else {   
            let t = hit.num_bounces as f32 / self.limit as f32;
            COLOR_LOW.lerp(&COLOR_HIGH, t)
        };

        (None, Some(color))
    }
}

#[derive(Default, Clone)]
pub struct MatLambertDiffuse {
    albedo: Color,
    reflectance: f32,
}

impl MatLambertDiffuse {
    pub fn new(c: Color, refl: f32) -> Material {
        Material::MatLambertDiffuse(MatLambertDiffuse { albedo: c, reflectance: refl })
    }
}

impl MaterialTrait for MatLambertDiffuse {
    #[allow(unused_variables)]
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        // Find the new scatter dir :)
        let new_dir = (util::rand_unit_vec() + hit.normal).normalize(); 
        // Mr. Shirley said to catch the vectors that are "near zero".
        // Those can occur if the generated random vector is parallel but opposite direction
        // of the hit normal.
        // We do not do that here though, because it caused weird bugs, somehow.
        let light_attenuation = hit.normal.dot(&new_dir);
        let new_ray = Ray::new(&hit.point, &new_dir);
        (Some(new_ray), Some(self.albedo.clone() * (self.reflectance * light_attenuation)))
    }
}

fn vec_reflect(v: &Vec3f, n: &Vec3f) -> Vec3f {
    (*v - (*n * 2.0 * v.dot(n))).normalize()
}

fn vec_refract(v: &Vec3f, n: &Vec3f, etai_over_etat: f32) -> Vec3f {
    let cos_theta = f32::min(-v.dot(n), 1.0);
    let r_out_perp =  (*v + (*n * cos_theta)) * etai_over_etat;
    let r_out_parallel = *n * (-(1.0 - r_out_perp.length_squared()).abs().sqrt());
    r_out_perp + r_out_parallel
}

fn schlick_approx(cos_theta: f32, ior: f32) -> f32 {
    let r0_root = (1.0 - ior) / (1.0 + ior);
    let r0 = r0_root * r0_root;
    let fac = 1.0 - f32::cos(cos_theta);
    let fac5 = fac * fac * fac * fac * fac;

    r0 + (1.0 - r0) * fac5
}

#[derive(Default, Clone)]
pub struct MatPrincipled {
    albedo: Color,
    reflectiveness: f32,
    gloss_fuzz: f32,
}

impl MaterialTrait for MatPrincipled {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        if util::rand_bool(self.reflectiveness) { 
            // Either we do a (fuzzy) reflection...
            let fuzz_vec = util::rand_unit_vec() * self.gloss_fuzz;
            let reflect_vec = vec_reflect(&ray_in.dir, &hit.normal);
            let new_ray = Ray::new(&hit.point, &(fuzz_vec + reflect_vec));
            (Some(new_ray), Some(self.albedo))
        } else {
            // Or do old school lambertian diffuse
            let new_dir = util::rand_unit_vec() + hit.normal;
            // Make sure to discard those pesky too tiny vectors.
            if !new_dir.near_zero() {
                let new_ray = Ray::new(&hit.point, &new_dir);
                (Some(new_ray), Some(self.albedo.clone()))
            } else {
                let new_ray = Ray::new(&hit.point, &hit.normal);
                (Some(new_ray), Some(self.albedo.clone()))
            }
        }
    }
}

impl MatPrincipled {
    pub fn new(c: Color, refl: f32, fuzz: f32) -> Material {
        Material::MatPrincipled(MatPrincipled {
            albedo: c,
            reflectiveness: num::clamp(refl, 0.0, 1.0),
            gloss_fuzz: num::clamp(fuzz, 0.0, 1.0),
        })
    }
}

#[derive(Default, Clone)]
pub struct MatEmission {
    color: Color,
    strength: f32,
}

impl MaterialTrait for MatEmission {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        // This material simply absorbs the ray and gives back a solid color of,
        // potentially, quite high brightness.
        let att_color = self.color * self.strength;
        (None, Some(att_color))
    }
}

impl MatEmission {
    pub fn new(c: Color, strength: f32) -> Material {
        Material::MatEmission(MatEmission {
            color: c,
            strength,
        })
    }
}

#[derive(Default, Clone)]
pub struct MatGlass {
    color: Color,
    ior: f32,
}

impl MaterialTrait for MatGlass {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        let unit_direction = ray_in.dir.normalize();

        // Compute the relative index of refraction (ior) depending on whether the ray is entering or exiting the material.
        let ior_rel = if hit.front_face { 1.0 / self.ior } else { self.ior };
        // Calculate the cosine of the angle between the incoming ray and the surface normal.
        let cos_theta = f32::min(-unit_direction.dot(&hit.normal), 1.0);
        // Calculate the sine of the angle using the Pythagorean identity.
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        // Determine if total internal reflection occurs (i.e., refraction is not possible).
        let cant_refract = ior_rel * sin_theta > 1.0;

        // Schlickes approximation
        let reflectance = schlick_approx(cos_theta, ior_rel);

        if cant_refract || reflectance > util::rand_range_f(0.0, 1.0) {
            let ray_reflected = Ray::new(&hit.point, &vec_reflect(&unit_direction, &hit.normal)).step(0.0001);
            (Some(ray_reflected), Some(self.color))
        } else {
            let dir_refracted = vec_refract(&unit_direction, &hit.normal, ior_rel);
            let ray_refracted = Ray::new(&hit.point, &dir_refracted).step(0.0001);

            (Some(ray_refracted), Some(self.color))
        }
    }
}

impl MatGlass {
    pub fn new(c: Color, ior: f32) -> Material {
        Material::MatGlass(MatGlass{
            color: c,
            ior: ior,
        })
    }
}