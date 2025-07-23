use crate::material::{self, MatLambertDiffuse, Material};
use crate::ray::{Ray};
use crate::vec3::Vec3f;
use crate::util::Interval;

use core::f32;
use std::cell::BorrowMutError;
use std::rc::Rc;

pub struct HitRecord {
    pub point: Vec3f,
    pub normal: Vec3f,
    pub num_bounces: u32, // How many times the ray has bounced so far
    pub material: Rc<dyn Material>,
    pub t: f32,
    pub front_face: bool, // True if ray hit the front of a face/surface. False if ray is on inside
}

impl HitRecord {
    pub fn new(ray: &Ray, t_hit: f32, point_hit: Vec3f, num_bounces: u32, obj_mat: Rc<dyn Material>, obj_normal: Vec3f) -> Self {
        // Check whether we hit the inside or outside
        if ray.dir.dot(&obj_normal) > 0.0 {
            // Hit the "inside" of object. Flip the normal!
            HitRecord {
                point: point_hit,
                normal: -obj_normal,
                material: obj_mat,
                num_bounces: num_bounces,
                t: t_hit,
                front_face: false,
            }
        } else {
            // Ray hit the face
            HitRecord {
                point: point_hit,
                normal: obj_normal,
                material: obj_mat,
                num_bounces: num_bounces,
                t: t_hit,
                front_face: true,
            }
        }
    }
}

pub trait Hittable {
    // The meat and bones. Detect hits from rays.
    // num_bounces: How many time this ray has bounced already.
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord>;

    // Return how many objects the hittable is made of. Just for bookkeepin'
    fn num_objects(&self) -> u32;
}

// Helper: Get the t parameter at which a sphere is struck.
fn hit_sphere(ray: &Ray, center: &Vec3f, radius: f32) -> Option<f32> {
    let oc = *center - ray.orig;
    let a = ray.dir.length_squared(); // A vector dotted with itself == its length squared
    let b = -2.0 * ray.dir.dot(&oc);
    let c = oc.length_squared() - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant >= 0.0 {
        // Got a hit, find the two possible ones.
        let t1 = (-b + discriminant.sqrt()) / (2.0 * a);
        let t2 = (-b - discriminant.sqrt()) / (2.0 * a);
        // Find and return the smaller, nonnegative value of both. (If exists)
        match (t1 >= 0.0, t2 >= 0.0) {
        (true, true) => Some(t1.min(t2)),
        (true, false) => Some(t1),
        (false, true) => Some(t2),
        (false, false) => None, }
    } else {
        // No hit, nothing
        None
    }
}

#[derive(Default)]
pub struct HittableList {
    list: Vec<Rc<dyn Hittable>>,
}

impl HittableList {
    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn push(&mut self, hittable: Rc<dyn Hittable>) {
        self.list.push(hittable);
    }
}

impl Hittable for HittableList {
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        // For all objects, try hitting them, discard Nones, then find the one with the
        // smallest t.
        self.list
            .iter().map(|x| x.try_hit(ray, t_interval, num_bounces))
            .flatten().min_by(|x, y| x.t.total_cmp(&y.t))
    }

    fn num_objects(&self) -> u32 {
        self.list.len() as u32
    }
}

pub struct Sphere {
    pub center: Vec3f,
    pub radius: f32,
    pub material: Rc<dyn Material>,
}

impl Sphere {
    pub fn new(c: Vec3f, r: f32, material: Rc<dyn Material>) -> Rc<Self> {
        Rc::new(Sphere {
            center: c,
            radius: r,
            material: material,
        })
    }
}

impl Hittable for Sphere {
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        if let Some(t_hit) = hit_sphere(ray, &self.center, self.radius) {
            if t_interval.contains(t_hit) {
                let point_hit = ray.at(t_hit);
                let sphere_normal = (point_hit - self.center) / self.radius;

                Some(HitRecord::new(ray, t_hit, point_hit, 
                                num_bounces, self.material.clone(), 
                            sphere_normal))       
            } else {
                None
            }
        } else {
            None
        }
    }

    fn num_objects(&self) -> u32 {
        1
    }
}

pub struct Plane {
    // The plane in HNF
    normal: Vec3f,
    d: f32,    // Distance from origin
    material: Rc<dyn Material>,
}

impl Plane {
    pub fn new(normal: Vec3f, d: f32, mat: Rc::<dyn Material>) -> Rc<Self> {
        Rc::new(Plane {
            normal: normal,
            d: d,
            material: mat, 
       })
    }

    pub fn from_points(botleft: Vec3f, topleft: Vec3f, botright: Vec3f, mat: Rc<dyn Material>) -> Rc<Self> {
        let right = botright - botleft;
        let up = topleft - botleft;
        
        let normal = right.cross(&up).normalize();
        let distance = (botleft.proj_plane(&normal) - botleft).length();

        // The plane is in HNF, so we want the plane normal to point away from the CS origin.
        // That means we might have to negate distance:
        let signed_distance = if normal.dot(&botleft) >= 0.0 {
            distance
        } else {
            -distance
        };

        Rc::new(Plane {
            material: mat,
            normal: normal,
            d: signed_distance,
        })
    }
}

impl Hittable for Plane {
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        // Abort if parallel
        if self.normal.dot(&ray.dir) == 0.0 {
            return None;
        }

        let t_intersect = (-self.normal.dot(&ray.orig) + self.d) / self.normal.dot(&ray.dir);

        if !t_interval.contains(t_intersect) {
            return None;
        } else {
            let p_intersect = ray.at(t_intersect);

            Some(HitRecord::new(ray, t_intersect, p_intersect, num_bounces, self.material.clone(), self.normal))
        }
    }

    fn num_objects(&self) -> u32 {
        1
    }
}

enum CoordinatePlane {
    XY,
    YZ,
    XZ,
}

pub struct Rectangle {
    // The plane the rectangle lies on in HNF
    normal: Vec3f,
    d: f32,    
    
    // The rectangles bottom left (on the projection plane) point
    botleft: Vec3f, 
    width: f32, 
    height: f32,

    // The coordinate plane to project plane and intersection point onto 
    // for bounds checking
    projection_plane: CoordinatePlane,

    material: Rc<dyn Material>,
}

impl Rectangle {
    pub fn from_points(bottom_left: Vec3f, top_left: Vec3f, bottom_right: Vec3f, mat: Rc::<dyn Material>) -> Rc<Self> {
        // Together with `bottom_left`, these three letters form represent the plane in parametric form.
        let up = top_left - bottom_left;
        let right = bottom_right - bottom_left;

        let normal = right.cross(&up).normalize();
        let distance = (bottom_left.proj_plane(&normal) - bottom_left).length();

        // The plane is in HNF, so we want the plane normal to point away from the CS origin.
        // That means we might have to negate distance:
        let signed_distance = if normal.dot(&bottom_left) >= 0.0 {
            distance
        } else {
            -distance
        };

        let width  = right.length();
        let height = up.length();

        // TODO: Project the plane on one of the coordinate planes.
        // Then check bounds later in try_hit func

        Rc::new(Rectangle {
            width: width,
            height: height,
            botleft: bottom_left,

            normal: normal.normalize(),
            d: signed_distance,

            projection_plane: CoordinatePlane::XY,
            
            material: mat,
        })
    }   
}

impl Hittable for Rectangle {
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        // Abort if parallel
        if self.normal.dot(&ray.dir) == 0.0 {
            return None;
        }

        let t_intersect = (-self.normal.dot(&ray.orig) + self.d) / self.normal.dot(&ray.dir);

        if !t_interval.contains(t_intersect) {
            return None;
        } else {
            let p_intersect = ray.at(t_intersect);

            Some(HitRecord::new(ray, t_intersect, p_intersect, num_bounces, self.material.clone(), self.normal))
        }
    }

    fn num_objects(&self) -> u32 {
        1
    }
}