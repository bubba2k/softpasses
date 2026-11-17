use super::material::MaterialID;
use crate::math::ray::Ray;
use crate::math::transform::{Transform, Transformable};
use crate::math::util::Interval;
use crate::math::vector::{self, CoordinatePlane, Float, Vec3f, project_onto_plane_normalized};

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
    Parallelogram(Parallelogram),
    Plane(Plane),
    Triangle(Triangle),
}

impl Transformable for Hittable {
    fn apply_transform(self, transform: &Transform) -> Self {
        match self {
            Hittable::Sphere(sphere) => Hittable::Sphere(sphere.apply_transform(transform)),
            Hittable::Parallelogram(parallelogram) => Hittable::Parallelogram(parallelogram.apply_transform(transform)),
            Hittable::Plane(plane) => Hittable::Plane(plane.apply_transform(transform)),
            Hittable::Triangle(triangle) => Hittable::Triangle(triangle.apply_transform(transform)),
        }
    }
}

impl HittableTrait for Hittable {
    fn centroid(&self) -> Vec3f {
        match self {
            Hittable::Sphere(sphere) => sphere.centroid(),
            Hittable::Parallelogram(parallelogram) => parallelogram.centroid(),
            Hittable::Plane(plane) => plane.centroid(),
            Hittable::Triangle(triangle) => triangle.centroid(),
        }
    }

    fn get_aabb(&self) -> AABoundingBox {
        match self {
            Hittable::Sphere(sphere) => sphere.get_aabb(),
            Hittable::Parallelogram(parallelogram) => parallelogram.get_aabb(),
            Hittable::Plane(plane) => plane.get_aabb(),
            Hittable::Triangle(triangle) => triangle.get_aabb(),
        }
    }

    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<Float> {
        match self {
            Hittable::Sphere(sphere) => sphere.try_intersect(ray, t_interval),
            Hittable::Parallelogram(parallelogram) => parallelogram.try_intersect(ray, t_interval),
            Hittable::Plane(plane) => plane.try_intersect(ray, t_interval),
            Hittable::Triangle(triangle) => triangle.try_intersect(ray, t_interval),
        }
    }

    fn compute_hit(&self, ray: &Ray, t: Float) -> HitRecord {
        match self {
            Hittable::Sphere(sphere) => sphere.compute_hit(ray, t),
            Hittable::Parallelogram(parallelogram) => parallelogram.compute_hit(ray, t),
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
            if tmax <= tmin {
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
            if tmax <= tmin {
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

#[derive(Clone)]
pub struct Parallelogram {
    // The plane the rectangle lies on in HNF
    normal: Vec3f,
    d: Float,

    // The rectangles 4 corners (on the projection plane) point in CCW order
    projected_bounds: [Vec3f; 4],

    // The coordinate plane to project intersection point onto for bounds checking
    projection_plane: vector::CoordinatePlane,

    material_id: MaterialID,
}

impl Transformable for Parallelogram {
    fn apply_transform(self, _transform: &Transform) -> Self {
        todo!()
    }
}

impl Parallelogram {
    pub fn from_points(
        bottom_left: Vec3f,
        top_left: Vec3f,
        bottom_right: Vec3f,
        material_id: MaterialID,
    ) -> Hittable {
        // Together with `bottom_left`, these three letters form represent the plane in parametric form.
        let up = top_left - bottom_left;
        let right = bottom_right - bottom_left;

        let normal = right.cross(up).normalize();
        let distance = (project_onto_plane_normalized(bottom_left, normal) - bottom_left).length();

        let signed_distance = if normal.dot(bottom_left) >= 0.0 {
            distance
        } else {
            -distance
        };

        // Find the coordinate plane "most parallel" to the rect plane, but most importantly, also avoid any orthogonal ones.
        let projection_plane = [
            CoordinatePlane::XY,
            CoordinatePlane::XZ,
            CoordinatePlane::YZ,
        ]
        .map(|p| (p, p.normal()))
        .iter()
        .max_by(|a, b| a.1.dot(normal).abs().total_cmp(&b.1.dot(normal).abs()))
        .map(|x| x.0)
        .unwrap();

        let projected_bounds = match projection_plane {
            CoordinatePlane::XY => {
                let projected_botleft = bottom_left;
                let projected_botright = bottom_right;
                let projected_topleft = top_left;
                let projected_topright = bottom_right + up;

                if vector::PLANE_XY.dot(normal) > 0.0 {
                    [
                        projected_botleft,
                        projected_botright,
                        projected_topright,
                        projected_topleft,
                    ]
                } else {
                    [
                        projected_topleft,
                        projected_topright,
                        projected_botright,
                        projected_botleft,
                    ]
                }
            }
            CoordinatePlane::XZ => {
                // Project onto the XZ plane. So, drop the Y and replace it with the Z coord, since
                // that is how the bounds checking function expects it later on.
                let projected_botleft = Vec3f::new(bottom_left.x, bottom_left.z, 0.0);
                let projected_botright = Vec3f::new(bottom_right.x, bottom_right.z, 0.0);
                let projected_topleft = Vec3f::new(top_left.x, top_left.z, 0.0);
                let top_right = bottom_right + up;
                let projected_topright = Vec3f::new(top_right.x, top_right.z, 0.0);

                if vector::PLANE_XZ.dot(normal) > 0.0 {
                    [
                        projected_topleft,
                        projected_topright,
                        projected_botright,
                        projected_botleft,
                    ]
                } else {
                    [
                        projected_botleft,
                        projected_botright,
                        projected_topright,
                        projected_topleft,
                    ]
                }
            }
            CoordinatePlane::YZ => {
                // Project on XY plane. Drop X coordinate.
                let projected_botleft = Vec3f::new(bottom_left.y, bottom_left.z, 0.0);
                let projected_botright = Vec3f::new(bottom_right.y, bottom_right.z, 0.0);
                let projected_topleft = Vec3f::new(top_left.y, top_left.z, 0.0);
                let top_right = bottom_right + up;
                let projected_topright = Vec3f::new(top_right.y, top_right.z, 0.0);

                if vector::PLANE_YZ.dot(normal) > 0.0 {
                    [
                        projected_botleft,
                        projected_botright,
                        projected_topright,
                        projected_topleft,
                    ]
                } else {
                    [
                        projected_topleft,
                        projected_topright,
                        projected_botright,
                        projected_botleft,
                    ]
                }
            }
        };

        Hittable::Parallelogram(Parallelogram {
            normal: normal.normalize(),
            d: signed_distance,

            projected_bounds: projected_bounds,
            projection_plane: projection_plane.clone(),

            material_id
        })
    }
}

impl HittableTrait for Parallelogram {
    fn compute_hit(&self, ray: &Ray, t: Float) -> HitRecord {
        HitRecord::new(
            ray,
            t,
            ray.at(t),
            self.material_id,
            self.normal,
        )
    }

    // Checking for Ray-Parallelogram intersect involves two steps:
    // 1. Finding intersect point between ray and the plane the rect lies on
    // 2. Projecting the intersect point and the rectangles bound onto a coordinate plane,
    //    then perform a 2D Point-Contains-Polygon Check
    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<Float> {
        // Abort if parallel
        if self.normal.dot(ray.dir) == 0.0 {
            return None;
        }

        let t_intersect = (-self.normal.dot(ray.orig) + self.d) / self.normal.dot(ray.dir);

        if !t_interval.contains(&t_intersect) {
            return None;
        }

        let p_intersect = ray.at(t_intersect);

        // Check what coordinate plane we have to project onto.
        let projected_intersect_point = match self.projection_plane {
            CoordinatePlane::XY => p_intersect,
            CoordinatePlane::XZ => Vec3f::new(p_intersect.x, p_intersect.z, 0.0),
            CoordinatePlane::YZ => Vec3f::new(p_intersect.y, p_intersect.z, 0.0),
        };

        // Check whether the point we found is inside the rectangles boundaries.
        // Now perform the classic ole "left of all edges" check. Lets us know whether the projected point
        // lies inside the projected bounds.
        let mut is_left_of_all = true;
        for i in 0..4 {
            let a = &self.projected_bounds[i];
            let b = &self.projected_bounds[(i + 1) % 4];
            if !vector::point_left_of_edge(&projected_intersect_point, a, b) {
                is_left_of_all = false;
            }
        }

        if is_left_of_all {
            Some(t_intersect)
        } else {
            None
        }
    }

    fn centroid(&self) -> Vec3f {
        self.projected_bounds
            .iter()
            .map(|p| project_onto_plane_normalized(*p, self.normal) + self.normal * self.d)
            .sum::<Vec3f>()
            / 4.0
    }

    fn get_aabb(&self) -> AABoundingBox {
        // Project the projected points back onto the parallelogram normal, offset them by the correct
        // distance along the normal and send them to the aabb
        let mut aabb: AABoundingBox = AABoundingBox::default();
        for p in self.projected_bounds.iter() {
            let point = project_onto_plane_normalized(*p, self.normal) + self.normal * self.d;
            aabb.expand(&point);
        }
        aabb
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

    let mut hittables = Vec::<Hittable>::new();

    // Front face
    hittables.push(Parallelogram::from_points(
        front_bottom_left,
        front_top_left,
        front_bottom_right,
        material_id,
    ));
    // Back face
    hittables.push(Parallelogram::from_points(
        back_bottom_left,
        back_top_left,
        back_bottom_right,
        material_id,
    ));
    // Top face
    hittables.push(Parallelogram::from_points(
        front_top_left,
        back_top_left,
        front_top_right,
        material_id,
    ));
    // Bottom face
    hittables.push(Parallelogram::from_points(
        back_bottom_left,
        front_bottom_left,
        back_bottom_right,
        material_id,
    ));
    // Left face
    hittables.push(Parallelogram::from_points(
        back_bottom_left,
        back_top_left,
        front_bottom_left,
        material_id,
    ));
    // Right face
    hittables.push(Parallelogram::from_points(
        back_bottom_right,
        back_top_right,
        front_bottom_right,
        material_id,
    ));

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