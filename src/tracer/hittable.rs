use super::material::MaterialID;
use crate::math::ray::Ray;
use crate::math::transform::{Transform, Transformable};
use crate::math::util::Interval;
use crate::math::vector::{Float, Vec3f, project_onto_plane_normalized};

/// Contains 
pub struct HitRecord {
    /// The point at which the surface was hit in world coordinates.
    pub point: Vec3f,
    /// Normal of the surface at intersection point in world coordinates.
    pub normal: Vec3f,
    /// The material of the surface that was hit.
    pub material_id: MaterialID,
    /// The distance traveled by the ray to hit this surface.
    pub t: Float,
    /// Indicates whether the front face of a surface was hit. 
    pub front_face: bool,
}

impl<'a> HitRecord {
    pub fn new(
        ray: &Ray,
        t_hit: Float,
        point_hit: Vec3f,
        obj_mat_id: MaterialID,
        obj_normal: Vec3f,
    ) -> Self {
        // Check whether we hit the inside or outside
        if ray.dir.dot(obj_normal) > 0.0 {
            // Hit the "inside" of object. Flip the normal!
            HitRecord {
                point: point_hit,
                normal: -obj_normal,
                material_id: obj_mat_id,
                t: t_hit,
                front_face: false,
            }
        } else {
            // Ray hit the face
            HitRecord {
                point: point_hit,
                normal: obj_normal,
                material_id: obj_mat_id,
                t: t_hit,
                front_face: true,
            }
        }
    }
}

/// Trait given to all objects that can be hit by a ray.
pub trait HittableTrait {
    /// Check if the given ray hits this object. Computes a full hit record including material, normal, etc.
    /// 
    /// # Arguments
    /// `ray` - The potentially intersecting ray.
    /// `t_interval` - The minimum and maximum distance along the ray for the hit to count. Hits outside the range will be discarded.
    fn try_hit(&self, ray: &Ray, t_interval: &Interval) -> Option<HitRecord> {
        self.try_intersect(ray, t_interval).map(|t| self.compute_hit(ray, t))
    }

    /// Check if a given ray intersects with this object. 
    /// 
    /// # Arguments
    /// `ray` - The potentially intersecting ray.
    /// `t_interval` - The minimum and maximum distance along the ray for the hit to count. Hits outside the range will be discarded.
    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<Float>;

    /// Given a successful intersection `t` units along `ray`, compute the full `HitRecord` containing
    /// information such as hit normal and material index.
    fn compute_hit(&self, ray: &Ray, t: Float) -> HitRecord;

    /// Get the axis aligned bounding box this object.
    fn get_aabb(&self) -> AABoundingBox;

    /// Return the centroid of the object.
    fn centroid(&self) -> Vec3f;
}

#[derive(Clone)]
pub enum Hittable {
    Sphere(Sphere),
    Plane(Plane),
    Triangle(Triangle),
}

impl Transformable for Hittable {
    fn apply_transform(self, transform: &Transform) -> Self {
        match self {
            Hittable::Sphere(sphere) => Hittable::Sphere(sphere.apply_transform(transform)),
            Hittable::Plane(plane) => Hittable::Plane(plane.apply_transform(transform)),
            Hittable::Triangle(triangle) => Hittable::Triangle(triangle.apply_transform(transform)),
        }
    }
}

impl HittableTrait for Hittable {
    fn centroid(&self) -> Vec3f {
        match self {
            Hittable::Sphere(sphere) => sphere.centroid(),
            Hittable::Plane(plane) => plane.centroid(),
            Hittable::Triangle(triangle) => triangle.centroid(),
        }
    }

    fn get_aabb(&self) -> AABoundingBox {
        match self {
            Hittable::Sphere(sphere) => sphere.get_aabb(),
            Hittable::Plane(plane) => plane.get_aabb(),
            Hittable::Triangle(triangle) => triangle.get_aabb(),
        }
    }

    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<Float> {
        match self {
            Hittable::Sphere(sphere) => sphere.try_intersect(ray, t_interval),
            Hittable::Plane(plane) => plane.try_intersect(ray, t_interval),
            Hittable::Triangle(triangle) => triangle.try_intersect(ray, t_interval),
        }
    }

    fn compute_hit(&self, ray: &Ray, t: Float) -> HitRecord {
        match self {
            Hittable::Sphere(sphere) => sphere.compute_hit(ray, t),
            Hittable::Plane(plane) => plane.compute_hit(ray, t),
            Hittable::Triangle(triangle) => triangle.compute_hit(ray, t),
        }
    }
}

#[derive(Clone, Debug)]
// TODO: What if an AABB is completely flat along an axis?
pub struct AABoundingBox {
    pub min: Vec3f,
    pub max: Vec3f,
}

impl Default for AABoundingBox {
    fn default() -> Self {
        AABoundingBox {
            min: Vec3f {
                x: Float::INFINITY,
                y: Float::INFINITY,
                z: Float::INFINITY,
            },
            max: Vec3f {
                x: Float::NEG_INFINITY,
                y: Float::NEG_INFINITY,
                z: Float::NEG_INFINITY,
            },
        }
    }
}

impl AABoundingBox {
    // Expand bounding box to a given point, if necessary
    pub fn expand(&mut self, point: &Vec3f) {
        self.min = Vec3f::new(
            self.min.x.min(point.x),
            self.min.y.min(point.y),
            self.min.z.min(point.z),
        );
        self.max = Vec3f::new(
            self.max.x.max(point.x),
            self.max.y.max(point.y),
            self.max.z.max(point.z),
        );
    }

    pub fn expand_aabb(&mut self, aabb: &AABoundingBox) {
        self.expand(&aabb.max);
        self.expand(&aabb.min);
    }

    pub fn extent(&self) -> Vec3f {
        if self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z {
            glam::vec3(0.0, 0.0, 0.0)
        } else {
            self.max - self.min
        }
    }

    /// Ray-box intersection using slabs method
    pub fn hit(&self, ray: &Ray, t_interval: &Interval) -> bool {
        let mut tmin = *t_interval.start();
        let mut tmax = *t_interval.end();

        for i in 0..3 {
            let inv_d = ray.inv_dir[i];
            let mut t0 = (self.min[i] - ray.orig[i]) * inv_d;
            let mut t1 = (self.max[i] - ray.orig[i]) * inv_d;
            if inv_d < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            tmin = tmin.max(t0);
            tmax = tmax.min(t1);
            if tmax < tmin {
                return false;
            }
        }

        true
    }

    /// Compute if an intersection occurs between the ray and AABB (slabs method),
    /// and if yes, return the distance `t` along the ray
    /// 
    /// # Arguments
    /// ray - The ray.
    /// t_interval - The interval the value of t must be inside of to be a valid hit.
    pub fn dist(&self, ray: &Ray, t_interval: &Interval) -> Option<Float> {
        let mut tmin = *t_interval.start();
        let mut tmax = *t_interval.end();

        for i in 0..3 {
            let inv_d = ray.inv_dir[i];
            let mut t0 = (self.min[i] - ray.orig[i]) * inv_d;
            let mut t1 = (self.max[i] - ray.orig[i]) * inv_d;
            if inv_d < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            tmin = tmin.max(t0);
            tmax = tmax.min(t1);
            if tmax < tmin {
                return None;
            }
        }

        Some(tmin)
    }
}

#[derive(Clone)]
pub struct Sphere {
    pub center: Vec3f,
    pub radius: Float,
    pub material_id: MaterialID,
}

impl Sphere {
    pub fn new(center: Vec3f, radius: Float, material_id: MaterialID) -> Hittable {
        Hittable::Sphere(Sphere {
            center,
            radius,
            material_id,
        })
    }
}

impl Transformable for Sphere {
    fn apply_transform(self, _transform: &Transform) -> Self {
        todo!()
    }
}

impl HittableTrait for Sphere {
    fn compute_hit(&self, ray: &Ray, t: Float) -> HitRecord {
        let point_hit = ray.at(t);
        let sphere_normal = (point_hit - self.center) / self.radius;

        HitRecord::new(
            ray,
            t,
            point_hit,
            self.material_id,
            sphere_normal,
        )
    }

    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<Float> {
        let oc = self.center - ray.orig;
        let a = ray.dir.length_squared(); // A vector dotted with itself == its length squared
        let b = -2.0 * ray.dir.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant >= 0.0 {
            // Got a hit, find the two possible ones.
            let t1 = (-b + discriminant.sqrt()) / (2.0 * a);
            let t2 = (-b - discriminant.sqrt()) / (2.0 * a);
            // Find and return the smaller, nonnegative value of both. (If it exists, and is within the interal limits)
            if let Some(t) = match (t1 >= 0.0, t2 >= 0.0) {
                (true, true) => Some(t1.min(t2)),
                (true, false) => Some(t1),
                (false, true) => Some(t2),
                (false, false) => None,
            } {
                if t_interval.contains(&t)  {
                    Some(t)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            // No hit, nothing
            None
        }
    }

    fn centroid(&self) -> Vec3f {
        self.center
    }

    fn get_aabb(&self) -> AABoundingBox {
        AABoundingBox {
            min: self.center - Vec3f::new(self.radius, self.radius, self.radius),
            max: self.center + Vec3f::new(self.radius, self.radius, self.radius),
        }
    }
}

#[derive(Clone)]
pub struct Plane {
    // The plane in HNF
    normal: Vec3f,
    d: Float, // Distance from origin
    material_id: MaterialID,
}

#[allow(dead_code)]
impl Plane {
    pub fn new(normal: Vec3f, d: Float, material_id: MaterialID) -> Hittable {
        Hittable::Plane(Plane {
            normal: normal,
            d: d,
            material_id,
        })
    }

    pub fn from_points(botleft: Vec3f, topleft: Vec3f, botright: Vec3f, material_id: MaterialID) -> Self {
        let right = botright - botleft;
        let up = topleft - botleft;

        let normal = right.cross(up).normalize();
        let distance = (project_onto_plane_normalized(botleft, normal) - botleft).length();

        // The plane is in HNF, so we want the plane normal to point away from the CS origin.
        // That means we might have to negate distance:
        let signed_distance = if normal.dot(botleft) >= 0.0 {
            distance
        } else {
            -distance
        };

        Plane {
            material_id,
            normal: normal,
            d: signed_distance,
        }
    }
}

impl Transformable for Plane {
    fn apply_transform(self, _transform: &Transform) -> Self {
        todo!()
    }
}

impl HittableTrait for Plane {
   
    fn compute_hit(&self, ray: &Ray, t: Float) -> HitRecord {
        HitRecord::new(
            ray,
            t,
            ray.at(t),
            self.material_id,
            self.normal,
        )
    }

    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<Float> {
        // Abort if parallel
        if self.normal.dot(ray.dir) == 0.0 {
            return None;
        }

        let t_intersect = (-self.normal.dot(ray.orig) + self.d) / self.normal.dot(ray.dir);

        if !t_interval.contains(&t_intersect) {
            None
        } else {
            Some(t_intersect)
        }
    }

    fn centroid(&self) -> Vec3f {
        self.normal * self.d
    }

    fn get_aabb(&self) -> AABoundingBox {
        AABoundingBox {
            min: Vec3f::new(-Float::INFINITY, -Float::INFINITY, -Float::INFINITY),
            max: Vec3f::new(Float::INFINITY, Float::INFINITY, Float::INFINITY),
        }
    }
}

fn _new_parallelepiped(
    back_bottom_left: Vec3f,
    back_bottom_right: Vec3f,
    front_bottom_left: Vec3f,
    back_top_left: Vec3f,
    material_id: MaterialID,
) -> Vec<Hittable> {
    let up = back_top_left - back_bottom_left;
    let depth = front_bottom_left - back_bottom_left;

    // Compute all 8 corners
    let back_top_right = back_bottom_right + up;
    let front_top_left = front_bottom_left + up;
    let front_bottom_right = back_bottom_right + depth;
    let front_top_right = front_bottom_right + up;

    let right = back_bottom_right - back_bottom_left;

    // Each face is a planar quad plus a vector pointing to its outside. The
    // reference vector only has to point to the outward side; the actual face
    // normal is derived from the winding below, so non-orthogonal (sheared)
    // parallelepipeds are handled correctly too.
    let faces: [([Vec3f; 4], Vec3f); 6] = [
        // Front
        (
            [
                front_bottom_left,
                front_bottom_right,
                front_top_right,
                front_top_left,
            ],
            depth,
        ),
        // Back
        (
            [
                back_bottom_left,
                back_top_left,
                back_top_right,
                back_bottom_right,
            ],
            -depth,
        ),
        // Top
        (
            [
                back_top_left,
                front_top_left,
                front_top_right,
                back_top_right,
            ],
            up,
        ),
        // Bottom
        (
            [
                back_bottom_left,
                back_bottom_right,
                front_bottom_right,
                front_bottom_left,
            ],
            -up,
        ),
        // Left
        (
            [
                back_bottom_left,
                front_bottom_left,
                front_top_left,
                back_top_left,
            ],
            -right,
        ),
        // Right
        (
            [
                back_bottom_right,
                back_top_right,
                front_top_right,
                front_bottom_right,
            ],
            right,
        ),
    ];

    let mut hittables = Vec::<Hittable>::with_capacity(2 * faces.len());
    for (mut quad, outward) in faces {
        // Enforce CCW winding as seen from outside. This also handles a
        // left-handed (mirrored) input basis.
        let mut normal = (quad[1] - quad[0]).cross(quad[2] - quad[0]);
        if normal.dot(outward) < 0.0 {
            quad.reverse();
            normal = -normal;
        }
        let normal = normal.normalize();

        // Split the quad along the 0-2 diagonal.
        for positions in [[quad[0], quad[1], quad[2]], [quad[0], quad[2], quad[3]]] {
            hittables.push(Hittable::Triangle(Triangle {
                positions,
                normals: [normal; 3],
                material_id,
            }));
        }
    }

    hittables
}

pub fn make_parallelepiped(
    back_bottom_left: Vec3f,
    back_bottom_right: Vec3f,
    front_bottom_left: Vec3f,
    back_top_left: Vec3f,
    material_id: MaterialID,
) -> Vec<Hittable> {
    _new_parallelepiped(
        back_bottom_left,
        back_bottom_right,
        front_bottom_left,
        back_top_left,
        material_id,
    )
}

#[allow(unused)]
pub fn make_cube(
    front_bottom_left: Vec3f,
    right_dir: Vec3f,
    up_dir: Vec3f,
    size: Float,
    material_id: MaterialID,
) -> Vec<Hittable> {
    let depth_dir = up_dir.cross(right_dir).normalize();

    let back_bottom_left = front_bottom_left + depth_dir * size;
    let back_bottom_right = back_bottom_left + right_dir.normalize() * size;
    let back_top_left = back_bottom_left + up_dir.normalize() * size;

    _new_parallelepiped(
        back_bottom_left,
        back_bottom_right,
        front_bottom_left,
        back_top_left,
        material_id,
    )
}

#[derive(Clone)]
pub struct Triangle {
    pub positions: [Vec3f; 3],
    pub normals: [Vec3f; 3],
    pub material_id: MaterialID,
}

impl HittableTrait for Triangle {
    fn centroid(&self) -> Vec3f {
        (self.positions[0] + self.positions[1] + self.positions[2]) * 0.33333
    }

    fn get_aabb(&self) -> AABoundingBox {
        let mut aabb = AABoundingBox::default();
        for point in self.positions.iter() {
            aabb.expand(point);
        }
        aabb
    }

    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<Float> {
        let (a, b, c) = (&self.positions[0], &self.positions[1], &self.positions[2]);
        // Determine whether the ray is parallel
        let edge1 = b - a;
        let edge2 = c - a;
        let h = ray.dir.cross(edge2);
        let det = edge1.dot(h);
        // If det is close to 0, the ray is parallel to the triangle
        if det.abs() < 1e-8 {
            return None;
        }
        let f = 1.0 / det;
        let s = ray.orig - a;
        let u = f * s.dot(h);
        if u < 0.0 || u > 1.0 {
            return None;
        }
        let q = s.cross(edge1);
        let v = f * ray.dir.dot(q);
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = f * edge2.dot(q);

        if t_interval.contains(&t) {
            Some(t)
        } else {
            None
        }
    }

    fn compute_hit(&self, ray: &Ray, t: Float) -> HitRecord {
        let point_hit = ray.at(t);
            let (a, b, c) = (&self.positions[0], &self.positions[1], &self.positions[2]);

            // Interpolate normal of the triangle. First, we have to find the barycentric
            // coordinates, u, v, w.
            let v0 = b - a;
            let v1 = c - a;
            let v2 = point_hit - a;
            let d00 = v0.dot(v0);
            let d01 = v0.dot(v1);
            let d11 = v1.dot(v1);
            let d20 = v2.dot(v0);
            let d21 = v2.dot(v1);
            let denom = d00 * d11 - d01 * d01;
            let v = (d11 * d20 - d01 * d21) / denom;
            let w = (d00 * d21 - d01 * d20) / denom;
            let u = 1.0 - v - w;
            // Now interpolate between the three corners.
            let obj_normal = (self.normals[0] * u
                + self.normals[1] * v
                + self.normals[2] * w)
                .normalize();

            HitRecord::new(ray, t, ray.at(t), self.material_id, obj_normal)
    }
}

impl Transformable for Triangle {
    fn apply_transform(self, transform: &Transform) -> Self {
        let affine = transform.get_affine();
        let normal_mat = affine.matrix3.inverse().transpose();

        Triangle {
            positions: self
                .positions
                .into_iter()
                .map(|p| affine.transform_point3(p))
                .collect::<Vec<Vec3f>>()
                .try_into()
                .unwrap(),
            normals: self
                .normals
                .into_iter()
                .map(|n| (normal_mat * n).normalize())
                .collect::<Vec<Vec3f>>()
                .try_into()
                .unwrap(),
            ..self
        }
    }
}