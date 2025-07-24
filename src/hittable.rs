use crate::material::{Material};
use crate::ray::{Ray};
use crate::vec3::{self, Vec3f, CoordinatePlane};
use crate::util::Interval;

use core::f32;
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

pub struct Parallelogram {
    // The plane the rectangle lies on in HNF
    normal: Vec3f,
    d: f32,    
    
    // The rectangles 4 corners (on the projection plane) point in CCW order
    projected_bounds: [Vec3f;4],

    // The coordinate plane to project intersection point onto for bounds checking
    projection_plane: vec3::CoordinatePlane,

    material: Rc<dyn Material>,
}

impl Parallelogram {
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

        // Find the coordinate plane "most parallel" to the rect plane, but most importantly, also avoid any orthogonal ones.
        let projection_plane = [CoordinatePlane::XY, CoordinatePlane::XZ, CoordinatePlane::YZ]
            .map(|p| (p, p.normal())).iter()
            .max_by(|a, b| a.1.dot(&normal).abs().total_cmp(&b.1.dot(&normal).abs()))
            .map(|x| x.0).unwrap();

        let projected_bounds = 
        match projection_plane {
            CoordinatePlane::XY => {
                let projected_botleft = bottom_left;
                let projected_botright = bottom_right;
                let projected_topleft = top_left;
                let projected_topright = bottom_right + up;

                if vec3::PLANE_XY.dot(&normal) > 0.0 {
                    [projected_botleft, projected_botright, projected_topright, projected_topleft]

                } else {
                    [projected_topleft, projected_topright, projected_botright, projected_botleft]
                }
            },
            CoordinatePlane::XZ => {
                // Project onto the XZ plane. So, drop the Y and replace it with the Z coord, since
                // that is how the bounds checking function expects it later on.
                let projected_botleft = Vec3f::new(bottom_left.x(), bottom_left.z(), 0.0);
                let projected_botright = Vec3f::new(bottom_right.x(), bottom_right.z(), 0.0);
                let projected_topleft = Vec3f::new(top_left.x(), top_left.z(), 0.0);
                let top_right = bottom_right + up;
                let projected_topright = Vec3f::new(top_right.x(), top_right.z(), 0.0);

                if vec3::PLANE_XZ.dot(&normal) > 0.0 {
                    [projected_topleft, projected_topright, projected_botright, projected_botleft]
                } else {
                    [projected_botleft, projected_botright, projected_topright, projected_topleft]
                }
            },
            CoordinatePlane::YZ => {
                // Project on XY plane. Drop X coordinate.
                let projected_botleft = Vec3f::new(bottom_left.y(), bottom_left.z(), 0.0);
                let projected_botright = Vec3f::new(bottom_right.y(), bottom_right.z(), 0.0);
                let projected_topleft = Vec3f::new(top_left.y(), top_left.z(), 0.0);
                let top_right = bottom_right + up;
                let projected_topright = Vec3f::new(top_right.y(), top_right.z(), 0.0);

                if vec3::PLANE_YZ.dot(&normal) > 0.0 {
                    [projected_botleft, projected_botright, projected_topright, projected_topleft]

                } else {
                    [projected_topleft, projected_topright, projected_botright, projected_botleft]
                }
            }
        };
        // Find the coordinate plane to project the rectangle (and later the hit point) onto.
        // We choose either the XY or XZ, which ever is the "least perpendicular" -> has the greatest absolute angle
        // to the rectangles normal. Strictly speaking we only need to find one that is simply not
        // orthogonal, but this might help with precision a little bit.

        /* 
        if Self::PLANE_XY.dot(&normal).abs() > Self::PLANE_XZ.dot(&normal).abs() {
            // T project on the XY plane, simply drop the Z coordinate.
            // Since the function for the projected bounds check expects XY coordinates, 
            // we do not have to do anything.
            let projected_botleft = bottom_left;
            let projected_botright = bottom_right;
            let projected_topleft = top_left;
            let projected_topright = bottom_right + up;

            (ProjectionPlane::XY, [projected_botleft, projected_botright, projected_topright, projected_topleft])
        } else {
            // Project onto the XZ plane. So, drop the Y and replace it with the Z coord, since
            // that is how the bounds checking function expects it later on.
            let projected_botleft = Vec3f::new(bottom_left.x(), bottom_left.z(), 0.0);
            let projected_botright = Vec3f::new(bottom_right.x(), bottom_right.z(), 0.0);
            let projected_topleft = Vec3f::new(top_left.x(), top_left.z(), 0.0);
            let top_right = bottom_right + up;
            let projected_topright = Vec3f::new(top_right.x(), top_right.z(), 0.0);

            // We have to flip the order here as well. "Z  up" is -Z, but expected "Y up" is positive.
            // Z can stay negative, but wee need to reorder to maintain CCW order
            (ProjectionPlane::XZ, [projected_topleft, projected_topright, projected_botright, projected_botleft] )
        }; */

 
        Rc::new(Parallelogram {
            normal: normal.normalize(),
            d: signed_distance,

            projected_bounds: projected_bounds,
            projection_plane: projection_plane.clone(),

            material: mat,
        })
    }   
}

impl Hittable for Parallelogram {
    // Checking for Ray-Rectangle intersect involves two steps:
    // 1. Finding intersect point between ray and the plane the rect lies on
    // 2. Projecting the intersect point and the rectangles bound onto a coordinate plane,
    //    then perform a 2D Point-Contains-Polygon Check
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

            // Check what coordinate plane we have to project onto.
            let projected_intersect_point = match self.projection_plane {
                CoordinatePlane::XY => p_intersect,
                CoordinatePlane::XZ => Vec3f::new(p_intersect.x(), p_intersect.z(), 0.0),
                CoordinatePlane::YZ => Vec3f::new(p_intersect.y(), p_intersect.z(), 0.0),
            };

            // Check whether the point we found is inside the rectangles boundaries.
            // Now perform the classic ole "left of all edges" check. Lets us know whether the projected point 
            // lies inside the projected bounds.
            let mut is_left_of_all = true; 
            for i in 0..4 {
                let a = &self.projected_bounds[i];
                let b = &self.projected_bounds[(i + 1) % 4];
                if !vec3::point_left_of_edge(&projected_intersect_point, a, b) { is_left_of_all = false; }
            };

            if is_left_of_all {
                Some(HitRecord::new(ray, t_intersect, p_intersect, num_bounces, self.material.clone(), self.normal))
            } else {
                None
            }
        }
    }

    fn num_objects(&self) -> u32 {
        1
    }
}

pub struct Parallelepiped {
    // List of the 6 faces 
    list: HittableList,
    material: Rc<dyn Material>,
}

impl Hittable for Parallelepiped {
    fn num_objects(&self) -> u32 {
        6
    }

    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        self.list.try_hit(ray, t_interval, num_bounces)
    }
}

impl Parallelepiped {
    pub fn new(back_bottom_left: Vec3f, back_bottom_right: Vec3f, front_bottom_left: Vec3f, back_top_left: Vec3f, material: Rc<dyn Material>) -> Rc<Self> {
        let up = back_top_left - back_bottom_left;
        let right = back_bottom_right - back_bottom_left;

        let front_top_left = front_bottom_left + up;
        let front_bottom_right = front_top_left + right;
        let back_top_right = back_bottom_right + up;
        let front_top_left = front_bottom_left + up;
        let front_top_right = front_bottom_right + up;

        let mut list: HittableList = HittableList::default();

        // Front
        list.push(Parallelogram::from_points(front_bottom_left, front_top_left, front_bottom_right, material.clone()));
        // Back
        list.push(Parallelogram::from_points(back_bottom_right, back_top_right, back_bottom_left, material.clone()));
        // Top
        list.push(Parallelogram::from_points(front_top_left, back_top_left, front_top_right, material.clone()));
        // Bottom
        list.push(Parallelogram::from_points(back_bottom_right, front_bottom_left, back_bottom_left, material.clone()));
        // Left
        list.push(Parallelogram::from_points(back_bottom_left, back_top_left, front_bottom_left, material.clone()));
        // Right
        list.push(Parallelogram::from_points(front_bottom_right, front_top_right, back_bottom_right, material.clone()));


        Rc::new(Parallelepiped { list: list, material: material })
    }

    /* 
    pub fn new_cube(front_bottom_left: Vec3f, right_dir: Vec3f, up_dir: Vec3f, size: f32) -> Rc<Self> {
        let depth_dir = 

        Parallelepiped::new(back_bottom_left, back_bottom_right, front_bottom_left, back_top_left, material)
    } */
}