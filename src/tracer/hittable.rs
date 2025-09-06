use super::material::Material;
use crate::math::ray::Ray;
use crate::math::transform::{Transform, Transformable};
use crate::math::util::Interval;
use crate::math::vector::{self, CoordinatePlane, Float, Vec3f, project_onto_plane_normalized};
use crate::tracer;
use std::path::Path;

pub struct HitRecord {
    pub point: Vec3f,
    pub normal: Vec3f,
    pub num_bounces: u32, // How many times the ray has bounced so far
    pub material: Material,
    pub t: Float,
    pub front_face: bool, // True if ray hit the front of a face/surface. False if ray is on inside
}

impl<'a> HitRecord {
    pub fn new(
        ray: &Ray,
        t_hit: Float,
        point_hit: Vec3f,
        num_bounces: u32,
        obj_mat: &'a Material,
        obj_normal: Vec3f,
    ) -> Self {
        // Check whether we hit the inside or outside
        if ray.dir.dot(obj_normal) > 0.0 {
            // Hit the "inside" of object. Flip the normal!
            HitRecord {
                point: point_hit,
                normal: -obj_normal,
                material: obj_mat.clone(),
                num_bounces,
                t: t_hit,
                front_face: false,
            }
        } else {
            // Ray hit the face
            HitRecord {
                point: point_hit,
                normal: obj_normal,
                material: obj_mat.clone(),
                num_bounces: num_bounces,
                t: t_hit,
                front_face: true,
            }
        }
    }
}

pub trait HittableTrait {
    // The meat and bones. Detect hits from rays.
    // num_bounces: How many time this ray has bounced already.
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord>;

    fn get_aabb(&self) -> AABoundingBox;

    // Return how many objects the hittable is made of. Just for bookkeepin'
    fn num_primitives(&self) -> u32;

    fn centroid(&self) -> Vec3f;
}

// Helper: Get the t parameter at which a sphere is struck.
fn hit_sphere(ray: &Ray, center: &Vec3f, radius: Float) -> Option<Float> {
    let oc = *center - ray.orig;
    let a = ray.dir.length_squared(); // A vector dotted with itself == its length squared
    let b = -2.0 * ray.dir.dot(oc);
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
            (false, false) => None,
        }
    } else {
        // No hit, nothing
        None
    }
}

#[derive(Clone)]
pub enum Hittable {
    Sphere(Sphere),
    Parallelogram(Parallelogram),
    Parallelepiped(Parallelepiped),
    Plane(Plane),
    Mesh(Mesh),
    BVHMesh(tracer::bvh::BVHMesh),
}

impl Transformable for Hittable {
    fn apply_transform(self, transform: &Transform) -> Self {
        match self {
            Hittable::Sphere(sphere) => Hittable::Sphere(sphere.apply_transform(transform)),
            Hittable::Parallelepiped(parallelepiped) => {
                Hittable::Parallelepiped(parallelepiped.apply_transform(transform))
            }
            Hittable::Parallelogram(parallelogram) => {
                Hittable::Parallelogram(parallelogram.apply_transform(transform))
            }
            Hittable::Plane(plane) => Hittable::Plane(plane.apply_transform(transform)),
            Hittable::Mesh(mesh) => Hittable::Mesh(mesh.apply_transform(transform)),
            Hittable::BVHMesh(bvh_mesh) => Hittable::BVHMesh(bvh_mesh.apply_transform(transform)),
        }
    }
}

impl HittableTrait for Hittable {
    fn centroid(&self) -> Vec3f {
        match self {
            Hittable::Sphere(sphere) => sphere.centroid(),
            Hittable::Parallelepiped(parallelepiped) => parallelepiped.centroid(),
            Hittable::Parallelogram(parallelogram) => parallelogram.centroid(),
            Hittable::Plane(plane) => plane.centroid(),
            Hittable::Mesh(mesh) => mesh.centroid(),
            Hittable::BVHMesh(bvh_mesh) => bvh_mesh.centroid(),
        }
    }

    fn get_aabb(&self) -> AABoundingBox {
        match self {
            Hittable::Sphere(sphere) => sphere.get_aabb(),
            Hittable::Parallelepiped(parallelepiped) => parallelepiped.get_aabb(),
            Hittable::Parallelogram(parallelogram) => parallelogram.get_aabb(),
            Hittable::Plane(plane) => plane.get_aabb(),
            Hittable::Mesh(mesh) => mesh.get_aabb(),
            Hittable::BVHMesh(bvh_mesh) => bvh_mesh.get_aabb(),
        }
    }

    fn num_primitives(&self) -> u32 {
        match self {
            Hittable::Sphere(sphere) => sphere.num_primitives(),
            Hittable::Parallelepiped(parallelepiped) => parallelepiped.num_primitives(),
            Hittable::Parallelogram(parallelogram) => parallelogram.num_primitives(),
            Hittable::Plane(plane) => plane.num_primitives(),
            Hittable::Mesh(mesh) => mesh.num_primitives(),
            Hittable::BVHMesh(bvh_mesh) => bvh_mesh.num_primitives(),
        }
    }

    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        match self {
            Hittable::Sphere(sphere) => sphere.try_hit(ray, t_interval, num_bounces),
            Hittable::Parallelepiped(parallelepiped) => {
                parallelepiped.try_hit(ray, t_interval, num_bounces)
            }
            Hittable::Parallelogram(parallelogram) => {
                parallelogram.try_hit(ray, t_interval, num_bounces)
            }
            Hittable::Plane(plane) => plane.try_hit(ray, t_interval, num_bounces),
            Hittable::Mesh(mesh) => mesh.try_hit(ray, t_interval, num_bounces),
            Hittable::BVHMesh(bvh_mesh) => bvh_mesh.try_hit(ray, t_interval, num_bounces),
        }
    }
}

#[derive(Default, Clone)]
pub struct AABoundingBox {
    pub min: Vec3f,
    pub max: Vec3f,
}

impl AABoundingBox {
    pub fn new(min: Vec3f, max: Vec3f) -> Self {
        AABoundingBox { min, max }
    }

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

        // Completely flat (among one of the axis) AABBs cause issues,
        // mainly the hit impl below always returning false in that case.
        // The simplest hack is to make sure it is never flat (unless empty) here.
        if self.min.x == self.max.x {
            self.max.x += 0.0001
        }
        if self.min.y == self.max.y {
            self.max.y += 0.0001
        }
        if self.min.z == self.max.z {
            self.max.z += 0.0001
        }
    }

    pub fn expand_aabb(&mut self, aabb: &AABoundingBox) {
        self.expand(&aabb.max);
        self.expand(&aabb.min);
    }

    // Ray-box intersection using slabs method
    pub fn hit(&self, ray: &Ray, t_interval: Interval) -> bool {
        let mut tmin = t_interval.min;
        let mut tmax = t_interval.max;

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
}

#[derive(Default, Clone)]
pub struct HittableList {
    list: Vec<Hittable>,
    aabb: AABoundingBox,
}

impl HittableList {
    pub fn clear(&mut self) {
        self.list.clear();
    }

    pub fn push(&mut self, hittable: Hittable) {
        self.aabb.expand(&hittable.get_aabb().max);
        self.aabb.expand(&hittable.get_aabb().min);
        self.list.push(hittable);
    }
}

impl HittableTrait for HittableList {
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        // Abort if the ray does not hit this lists bounding box (TODO: This seems to worsen performance,
        // so it is turned off for now.)
        // if !self.aabb.hit(ray, t_interval)  { return None; }

        // For all objects, try hitting them, discard Nones, then find the one with the
        // smallest t.
        self.list
            .iter()
            .map(|x| x.try_hit(ray, t_interval, num_bounces))
            .flatten()
            .min_by(|x, y| x.t.total_cmp(&y.t))
    }

    fn num_primitives(&self) -> u32 {
        self.list.len() as u32
    }

    fn get_aabb(&self) -> AABoundingBox {
        self.aabb.clone()
    }

    fn centroid(&self) -> Vec3f {
        let sum: Vec3f = self.list.iter().map(|el| el.centroid()).sum();
        sum / self.list.len() as Float
    }
}

#[derive(Clone)]
pub struct Sphere {
    pub center: Vec3f,
    pub radius: Float,
    pub material: Material,
}

impl Sphere {
    pub fn new(c: Vec3f, r: Float, material: Material) -> Hittable {
        Hittable::Sphere(Sphere {
            center: c,
            radius: r,
            material: material,
        })
    }
}

impl Transformable for Sphere {
    fn apply_transform(self, _transform: &Transform) -> Self {
        todo!()
    }
}

impl HittableTrait for Sphere {
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        if let Some(t_hit) = hit_sphere(ray, &self.center, self.radius) {
            if t_interval.contains(t_hit) {
                let point_hit = ray.at(t_hit);
                let sphere_normal = (point_hit - self.center) / self.radius;

                Some(HitRecord::new(
                    ray,
                    t_hit,
                    point_hit,
                    num_bounces,
                    &self.material,
                    sphere_normal,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn num_primitives(&self) -> u32 {
        1
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
    material: Material,
}

impl Plane {
    pub fn new(normal: Vec3f, d: Float, mat: Material) -> Hittable {
        Hittable::Plane(Plane {
            normal: normal,
            d: d,
            material: mat,
        })
    }

    pub fn from_points(botleft: Vec3f, topleft: Vec3f, botright: Vec3f, mat: Material) -> Self {
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
            material: mat,
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
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        // Abort if parallel
        if self.normal.dot(ray.dir) == 0.0 {
            return None;
        }

        let t_intersect = (-self.normal.dot(ray.orig) + self.d) / self.normal.dot(ray.dir);

        if !t_interval.contains(t_intersect) {
            return None;
        } else {
            let p_intersect = ray.at(t_intersect);

            Some(HitRecord::new(
                ray,
                t_intersect,
                p_intersect,
                num_bounces,
                &self.material,
                self.normal,
            ))
        }
    }

    fn num_primitives(&self) -> u32 {
        1
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

    material: Material,
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
        mat: Material,
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

            material: mat,
        })
    }
}

impl HittableTrait for Parallelogram {
    // Checking for Ray-Rectangle intersect involves two steps:
    // 1. Finding intersect point between ray and the plane the rect lies on
    // 2. Projecting the intersect point and the rectangles bound onto a coordinate plane,
    //    then perform a 2D Point-Contains-Polygon Check
    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        // Abort if parallel
        if self.normal.dot(ray.dir) == 0.0 {
            return None;
        }

        let t_intersect = (-self.normal.dot(ray.orig) + self.d) / self.normal.dot(ray.dir);

        if !t_interval.contains(t_intersect) {
            return None;
        } else {
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
                Some(HitRecord::new(
                    ray,
                    t_intersect,
                    p_intersect,
                    num_bounces,
                    &self.material,
                    self.normal,
                ))
            } else {
                None
            }
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

    fn num_primitives(&self) -> u32 {
        1
    }
}

#[derive(Clone)]
pub struct Parallelepiped {
    // List of the 6 faces
    list: HittableList,
    material: Material,
}

impl Transformable for Parallelepiped {
    fn apply_transform(self, _transform: &Transform) -> Self {
        todo!()
    }
}

impl HittableTrait for Parallelepiped {
    fn num_primitives(&self) -> u32 {
        6
    }

    fn get_aabb(&self) -> AABoundingBox {
        self.list.get_aabb()
    }

    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        self.list.try_hit(ray, t_interval, num_bounces)
    }

    fn centroid(&self) -> Vec3f {
        self.list.centroid()
    }
}

impl Parallelepiped {
    fn _new(
        back_bottom_left: Vec3f,
        back_bottom_right: Vec3f,
        front_bottom_left: Vec3f,
        back_top_left: Vec3f,
        material: Material,
    ) -> Self {
        let up = back_top_left - back_bottom_left;
        let depth = front_bottom_left - back_bottom_left;

        // Compute all 8 corners
        let back_top_right = back_bottom_right + up;
        let front_top_left = front_bottom_left + up;
        let front_bottom_right = back_bottom_right + depth;
        let front_top_right = front_bottom_right + up;

        let mut list: HittableList = HittableList::default();

        // Front face
        list.push(Parallelogram::from_points(
            front_bottom_left,
            front_top_left,
            front_bottom_right,
            material.clone(),
        ));
        // Back face
        list.push(Parallelogram::from_points(
            back_bottom_left,
            back_top_left,
            back_bottom_right,
            material.clone(),
        ));
        // Top face
        list.push(Parallelogram::from_points(
            front_top_left,
            back_top_left,
            front_top_right,
            material.clone(),
        ));
        // Bottom face
        list.push(Parallelogram::from_points(
            back_bottom_left,
            front_bottom_left,
            back_bottom_right,
            material.clone(),
        ));
        // Left face
        list.push(Parallelogram::from_points(
            back_bottom_left,
            back_top_left,
            front_bottom_left,
            material.clone(),
        ));
        // Right face
        list.push(Parallelogram::from_points(
            back_bottom_right,
            back_top_right,
            front_bottom_right,
            material.clone(),
        ));

        Parallelepiped {
            list: list,
            material: material,
        }
    }

    pub fn new(
        back_bottom_left: Vec3f,
        back_bottom_right: Vec3f,
        front_bottom_left: Vec3f,
        back_top_left: Vec3f,
        material: Material,
    ) -> Hittable {
        Hittable::Parallelepiped(Self::_new(
            back_bottom_left,
            back_bottom_right,
            front_bottom_left,
            back_top_left,
            material,
        ))
    }

    pub fn new_cube(
        front_bottom_left: Vec3f,
        right_dir: Vec3f,
        up_dir: Vec3f,
        size: Float,
        material: Material,
    ) -> Hittable {
        let depth_dir = up_dir.cross(right_dir).normalize();

        let back_bottom_left = front_bottom_left + depth_dir * size;
        let back_bottom_right = back_bottom_left + right_dir.normalize() * size;
        let back_top_left = back_bottom_left + up_dir.normalize() * size;

        let p = Parallelepiped::_new(
            back_bottom_left,
            back_bottom_right,
            front_bottom_left,
            back_top_left,
            material,
        );
        Hittable::Parallelepiped(p)
    }
}

#[derive(Clone)]
pub struct Triangle {
    pub positions: [Vec3f; 3],
    pub normals: [Vec3f; 3],
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

    fn num_primitives(&self) -> u32 {
        1
    }

    fn try_hit(&self, _ray: &Ray, _t_interval: Interval, _num_bounces: u32) -> Option<HitRecord> {
        // Since the triangles are used only inside BVHMesh, this should never be used.
        todo!()
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
                .positions
                .into_iter()
                .map(|p| (normal_mat * p).normalize())
                .collect::<Vec<Vec3f>>()
                .try_into()
                .unwrap(),
        }
    }
}

impl Triangle {
    pub fn ray_intersection(&self, ray: &Ray, interval: &Interval) -> Option<Float> {
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

        if interval.contains(t) { Some(t) } else { None }
    }
}

#[derive(Clone)]
pub struct Mesh {
    aabb: AABoundingBox,
    triangles: Vec<Triangle>,
    material: Material,
}

pub fn load_obj(path: &Path) -> Vec<Triangle> {
    const LOAD_OPTIONS: tobj::LoadOptions = tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ignore_lines: true,
        ignore_points: true,
    };

    let (models, _) = tobj::load_obj(path, &LOAD_OPTIONS).expect("Failed to load OBJ file");
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mesh = &models[0].mesh;
    // Have to reorder: We want all triangles to be in order, so use the indices
    // to achieve that
    // Copy triangle positions
    for index in mesh.indices.iter() {
        let vert_idx = (*index as usize) * 3;
        let vert = Vec3f::new(
            mesh.positions[vert_idx],
            mesh.positions[vert_idx + 1],
            mesh.positions[vert_idx + 2],
        );
        positions.push(vert);
    }
    // Do the same for normals
    if !mesh.normals.is_empty() {
        eprintln!("No normals given in obj file {:?}. Generating...", path);
        for index in mesh.indices.iter() {
            let vert_idx = (*index as usize) * 3;
            let normal = Vec3f::new(
                mesh.normals[vert_idx],
                mesh.normals[vert_idx + 1],
                mesh.normals[vert_idx + 2],
            );
            normals.push(normal);
        }
    } else {
        // If there are no normals specified in the file, compute (flat) normals
        // from vertex positions.
        for tri in positions.chunks(3) {
            let edge1 = tri[1] - tri[0];
            let edge2 = tri[2] - tri[0];
            let obj_normal = edge1.cross(edge2).normalize();
            normals.push(obj_normal);
            normals.push(obj_normal);
            normals.push(obj_normal);
        }
    }

    // Group positions and normals
    let mut triangles = Vec::new();
    for i in (0..positions.len()).step_by(3) {
        triangles.push(Triangle {
            positions: [positions[i + 0], positions[i + 1], positions[i + 2]],
            normals: [normals[i + 0], normals[i + 1], normals[i + 2]],
        });
    }

    triangles
}

impl Mesh {
    pub fn from_obj_file(path: &Path, material: Material) -> Hittable {
        let triangles = load_obj(path);

        let mut aabb = AABoundingBox::default();
        for tri in triangles.iter() {
            aabb.expand(&tri.positions[0]);
            aabb.expand(&tri.positions[1]);
            aabb.expand(&tri.positions[2]);
        }

        Hittable::Mesh(Mesh {
            aabb,
            material,
            triangles,
        })
    }
}

impl Transformable for Mesh {
    fn apply_transform(self, transform: &Transform) -> Self {
        // 1. Apply transform to vertex positions and normals
        let transformed_triangles: Vec<Triangle> = self
            .triangles
            .into_iter()
            .map(|tri| tri.apply_transform(transform))
            .collect();
        let mut new_aabb = AABoundingBox::default();

        for tri in transformed_triangles.iter() {
            new_aabb.expand(&tri.positions[0]);
            new_aabb.expand(&tri.positions[1]);
            new_aabb.expand(&tri.positions[2]);
        }

        Mesh {
            triangles: transformed_triangles,
            aabb: new_aabb,
            ..self
        }
    }
}

impl HittableTrait for Mesh {
    fn num_primitives(&self) -> u32 {
        1
    }

    fn get_aabb(&self) -> AABoundingBox {
        self.aabb.clone()
    }

    fn centroid(&self) -> Vec3f {
        (self.aabb.max - self.aabb.min) * 0.5 + self.aabb.min
    }

    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        // Do a simple aabb check
        if !self.aabb.hit(ray, t_interval) {
            return None;
        }

        if let Some((tri_idx, t_hit)) = self
            .triangles
            .iter()
            .enumerate()
            .map(|(idx, tri)| {
                if let Some(t) = tri.ray_intersection(ray, &t_interval) {
                    Some((idx, t))
                } else {
                    None
                }
            })
            .flatten()
            .min_by(|(_, t1), (_, t2)| t1.total_cmp(t2))
        {
            if t_interval.contains(t_hit) {
                let point_hit = ray.at(t_hit);

                // Interpolate normal of the triangle. First, we have to find the barycentric
                // coordinates, u, v, w.
                let a = self.triangles[tri_idx].positions[0];
                let b = self.triangles[tri_idx].positions[1];
                let c = self.triangles[tri_idx].positions[2];
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
                let obj_normal = (self.triangles[tri_idx].normals[0] * u
                    + self.triangles[tri_idx].normals[1] * v
                    + self.triangles[tri_idx].normals[2] * w)
                    .normalize();

                Some(HitRecord::new(
                    ray,
                    t_hit,
                    point_hit,
                    num_bounces,
                    &self.material,
                    obj_normal,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}
