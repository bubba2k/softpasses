use rand::Rng;
use rand::distr::Distribution;

use super::hittable::HitRecord;
use crate::math::ray::Ray;
use crate::math::util;
use crate::math::vector::{Color, Float, Vec3f, vec3};

pub trait MaterialTrait {
    // Returns None if the ray was absorbed.
    // Else, returns a new (scattered) ray and color attenuation
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>);

    // Return an instance of this material with randomized parameters.
    fn random_instance() -> Self;
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

    fn random_instance() -> Self {
        rand::random()
    }
}

// To generate random Material enums
impl Distribution<Material> for rand::distr::StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Material {
        match rng.random_range(0..=3) {
            0 => Material::MatEmission(MatEmission::random_instance()),
            1 => Material::MatGlass(MatGlass::random_instance()),
            2 => Material::MatLambertDiffuse(MatLambertDiffuse::random_instance()),
            _ => Material::MatPrincipled(MatPrincipled::random_instance()),
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

        (None, Some(color))
    }

    fn random_instance() -> Self {
        MatNormalDebug {}
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
        (None, Some(color))
    }

    fn random_instance() -> Self {
        MatFaceDebug {}
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
        Material::MatBounceDebug(MatBounceDebug { limit: limit })
    }
}

impl MaterialTrait for MatBounceDebug {
    fn scatter(&self, _ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        static COLOR_RED: Color = Vec3f::new(1.0, 0.0, 0.0);
        static COLOR_LOW: Color = Vec3f::new(0.0, 0.0, 0.0);
        static COLOR_HIGH: Color = Vec3f::new(1.0, 1.0, 1.0);
        let color = if hit.num_bounces >= self.limit {
            COLOR_RED
        } else {
            let t = hit.num_bounces as Float / (self.limit - 1) as Float;
            COLOR_LOW.lerp(COLOR_HIGH, t)
        };

        (None, Some(color))
    }

    fn random_instance() -> Self {
        // Does not really make sense to do something actually random here
        MatBounceDebug { limit: 32 }
    }
}

#[derive(Default, Clone)]
pub struct MatLambertDiffuse {
    albedo: Color,
}

impl MatLambertDiffuse {
    // Note: We use brdf_lam = albedo, instead of the technically more correct
    // brdf_lam = albedo / pi. This is more convenient for users and means that
    // albedo is implicitely multiplied by pi.
    // Further reading: https://seblagarde.wordpress.com/2012/01/08/pi-or-not-to-pi-in-game-lighting-equation/
    pub fn new(c: Color) -> Material {
        Material::MatLambertDiffuse(MatLambertDiffuse { albedo: c })
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
        let new_ray = Ray::new(&hit.point, &new_dir);
        (Some(new_ray), Some(self.albedo))
    }

    fn random_instance() -> Self {
        let albedo = Vec3f::new(
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
        );
        MatLambertDiffuse { albedo: albedo }
    }
}

fn vec_reflect(v_norm: &Vec3f, n_norm: &Vec3f) -> Vec3f {
    *v_norm - (*n_norm * 2.0 * v_norm.dot(*n_norm))
}

fn vec_refract(v: &Vec3f, n: &Vec3f, etai_over_etat: Float) -> Vec3f {
    let cos_theta = Float::min(-v.dot(*n), 1.0);
    let r_out_perp = (*v + (*n * cos_theta)) * etai_over_etat;
    let r_out_parallel = *n * (-(1.0 - r_out_perp.length_squared()).abs().sqrt());
    r_out_perp + r_out_parallel
}

fn schlick_approx(cos_theta: Float, ior: Float) -> Float {
    let r0_root = (1.0 - ior) / (1.0 + ior);
    let r0 = r0_root * r0_root;
    let fac = 1.0 - Float::cos(cos_theta);
    let fac5 = fac * fac * fac * fac * fac;

    r0 + (1.0 - r0) * fac5
}

#[derive(Default, Clone)]
pub struct MatPrincipled {
    albedo: Color,
    reflectiveness: Float,
    gloss_fuzz: Float,
}

impl MaterialTrait for MatPrincipled {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        if util::rand_bool(self.reflectiveness) {
            // Either we do a (fuzzy) reflection...
            let fuzz_vec = util::rand_unit_vec() * self.gloss_fuzz;
            let reflect_vec = vec_reflect(&ray_in.dir, &hit.normal);
            let new_ray = Ray::new(&hit.point, &(fuzz_vec + reflect_vec).normalize());
            (Some(new_ray), Some(self.albedo))
        } else {
            // Or do old school lambertian diffuse
            let new_dir = (util::rand_unit_vec() + hit.normal).normalize();
            // Make sure to discard those pesky too tiny vectors.
            if !new_dir.abs_diff_eq(vec3(0.0, 0.0, 0.0), 0.0001) {
                let new_ray = Ray::new(&hit.point, &new_dir);
                (Some(new_ray), Some(self.albedo.clone()))
            } else {
                let new_ray = Ray::new(&hit.point, &hit.normal);
                (Some(new_ray), Some(self.albedo.clone()))
            }
        }
    }

    fn random_instance() -> Self {
        let albedo = Vec3f::new(
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
        );
        let refl = util::rand_range_f(0.0, 1.0);
        let gloss_fuzz = util::rand_range_f(0.0, 1.0);

        MatPrincipled {
            albedo: albedo,
            reflectiveness: refl,
            gloss_fuzz: gloss_fuzz,
        }
    }
}

impl MatPrincipled {
    pub fn new(c: Color, refl: Float, fuzz: Float) -> Material {
        Material::MatPrincipled(MatPrincipled {
            albedo: c,
            reflectiveness: refl.clamp(0.0, 1.0),
            gloss_fuzz: fuzz.clamp(0.0, 1.0),
        })
    }
}

#[derive(Default, Clone)]
pub struct MatEmission {
    color: Color,
    strength: Float,
}

impl MaterialTrait for MatEmission {
    fn scatter(&self, _ray_in: &Ray, _hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        // This material simply absorbs the ray and gives back a solid color of,
        // potentially, quite high brightness.
        let att_color = self.color * self.strength;
        (None, Some(att_color))
    }

    fn random_instance() -> Self {
        let albedo = Vec3f::new(
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
        );
        let strength = util::rand_range_f(0.0, 1.0);

        MatEmission {
            color: albedo,
            strength: strength,
        }
    }
}

impl MatEmission {
    pub fn new(c: Color, strength: Float) -> Material {
        Material::MatEmission(MatEmission { color: c, strength })
    }
}

#[derive(Default, Clone)]
pub struct MatGlass {
    color: Color,
    ior: Float,
}

impl MaterialTrait for MatGlass {
    fn scatter(&self, ray_in: &Ray, hit: &HitRecord) -> (Option<Ray>, Option<Color>) {
        let unit_direction = ray_in.dir;

        // Compute the relative index of refraction (ior) depending on whether the ray is entering or exiting the material.
        let ior_rel = if hit.front_face {
            1.0 / self.ior
        } else {
            self.ior
        };
        // Calculate the cosine of the angle between the incoming ray and the surface normal.
        let cos_theta = Float::min(-unit_direction.dot(hit.normal), 1.0);
        // Calculate the sine of the angle using the Pythagorean identity.
        let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
        // Determine if total internal reflection occurs (i.e., refraction is not possible).
        let cant_refract = ior_rel * sin_theta > 1.0;

        // Schlickes approximation
        let reflectance = schlick_approx(cos_theta, ior_rel);

        if cant_refract || reflectance > util::rand_range_f(0.0, 1.0) {
            let ray_reflected =
                Ray::new(&hit.point, &vec_reflect(&unit_direction, &hit.normal)).step(0.0001);
            (Some(ray_reflected), Some(self.color))
        } else {
            let dir_refracted = vec_refract(&unit_direction, &hit.normal, ior_rel);
            let ray_refracted = Ray::new(&hit.point, &dir_refracted).step(0.0001);

            (Some(ray_refracted), Some(self.color))
        }
    }

    fn random_instance() -> Self {
        let albedo = Vec3f::new(
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
            util::rand_range_f(0.0, 1.0),
        );
        let ior = util::rand_range_f(1.3, 1.8);

        Self {
            color: albedo,
            ior: ior,
        }
    }
}

impl MatGlass {
    pub fn new(c: Color, ior: Float) -> Material {
        Material::MatGlass(MatGlass { color: c, ior: ior })
    }
}
