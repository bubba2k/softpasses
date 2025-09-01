use super::hittable::{HittableTrait, AABoundingBox, Triangle, Hittable, HitRecord};
use std::path::Path;
use crate::{Vec3f, Float};
use crate::{Transformable, Transform};
use crate::math::ray::Ray;
use crate::math::util::Interval;
use crate::Material;


#[derive(Default, Clone)]
struct BVHNode {
    first_prim: u32,
    num_prims: u32,
    left_child: u32,
    right_child: u32,
    aabb: AABoundingBox,
}

fn subdivide<T: HittableTrait>(bvh: &mut Vec<BVHNode>, triangles: &mut Vec<T>, bvh_node_index: u32) {
    // Always split along longest axis for now
    let node = &mut bvh[bvh_node_index as usize];
    if node.num_prims == 1 {
        // For leaf nodes, we do not initalize the aabb or anything like that
        return;
    }
    let begin = node.first_prim as usize;
    let end = (node.first_prim + node.num_prims) as usize;
    for tri in triangles[begin..end].iter() {
        node.aabb.expand_aabb(&tri.get_aabb());
    }
    let extent = node.aabb.max - node.aabb.min;
    // Split along the longest axis, determine split value
    let mut axis = 0;
    if extent.y > extent.x {
        axis = 1
    }
    if extent.z > extent[axis] {
        axis = 2
    }
    let split_value = node.aabb.min[axis] + 0.5 * extent[axis];
    // Sort to the left and right of split value
    let mut i = node.first_prim;
    let mut j = i + node.num_prims - 1;
    while i < j + 1 {
        // For now, we use the first corner of each triangle as the centroid
        // TODO: Use the actual centroid.
        if triangles[i as usize].centroid()[axis] < split_value {
            i += 1;
        } else {
            triangles.swap(i as usize, j as usize);
            j -= 1;
        }
    }
    // Initialize the two children nodes and go on to subidivide them
    let left_idx = bvh_node_index * 2 + 1;
    let right_idx = bvh_node_index * 2 + 2;
    let left_num = i - bvh[bvh_node_index as usize].first_prim;
    let right_num = bvh[bvh_node_index as usize].num_prims - left_num;
    eprintln!("left child: {} | right child: {}", left_num, right_num);
    bvh[left_idx as usize].first_prim = bvh[bvh_node_index as usize].first_prim;
    bvh[left_idx as usize].num_prims = left_num;
    bvh[right_idx as usize].first_prim = i;
    bvh[right_idx as usize].num_prims = right_num;
    // If this happens, the current node shall be a leaf.
    if left_num == 0 || right_num == 0 {
        return;
    }
    bvh[bvh_node_index as usize].left_child = left_idx;
    bvh[bvh_node_index as usize].right_child = right_idx;
    subdivide(bvh, triangles, left_idx);
    subdivide(bvh, triangles, right_idx);
}

fn build_bvh<T: HittableTrait> (mut primitives: Vec<T>) -> (Vec<T>, Vec<BVHNode>) {
    // The recursive func to build the BVH search tree

    eprintln!("Building BVH.");
    let num_prims = primitives.len();

    // Assume one triangle per leaf
    let mut bvh_nodes: Vec<BVHNode> = vec![BVHNode::default(); 3 * num_prims - 1];
    // Set the root node
    bvh_nodes[0].first_prim = 0;
    bvh_nodes[0].num_prims = num_prims as u32;

    subdivide(&mut bvh_nodes, &mut primitives, 0);

    eprintln!("Built BVH.");
    (primitives, bvh_nodes)
}

#[derive(Clone)]
pub struct BVH<T: HittableTrait> {
    hittables: Vec<T>,
    nodes: Vec<BVHNode>,
}

impl<T: HittableTrait> HittableTrait for BVH<T> {
    fn centroid(&self) -> Vec3f {
        (self.get_aabb().max - self.get_aabb().min) * 0.5 + self.get_aabb().min
    }

    fn get_aabb(&self) -> AABoundingBox {
        self.nodes[0].aabb.clone()
    }

    fn num_primitives(&self) -> u32 {
        self.hittables
            .iter()
            .map(HittableTrait::num_primitives)
            .sum()
    }

    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        self.try_hit_rec(ray, t_interval, num_bounces, 0)
    }
}

impl<T: HittableTrait> BVH<T> {
    pub fn new(list: Vec<T>) -> BVH<T> {
        eprintln!("Building scene bvh");

        let (hittables, bvh_nodes) = build_bvh(list);

        Self {
            hittables: hittables,
            nodes: bvh_nodes,
        }
    }

    fn try_hit_rec(
        &self,
        ray: &Ray,
        t_interval: Interval,
        num_bounces: u32,
        bvh_idx: u32,
    ) -> Option<HitRecord> {
        // Traverse the bvh
        let node = &self.nodes[bvh_idx as usize];

        // A node is a leaf if it dont have no children
        if node.left_child == 0 {
            let range =
                (node.first_prim as usize)..(node.first_prim as usize + node.num_prims as usize);
            return range
                .map(|idx| self.hittables[idx].try_hit(ray, t_interval, num_bounces))
                .flatten()
                .min_by(|a, b| a.t.total_cmp(&b.t));
        }

        if node.aabb.hit(ray, t_interval) {
            let left_idx = bvh_idx * 2 + 1;
            let right_idx = bvh_idx * 2 + 2;

            [left_idx, right_idx]
                .iter()
                .flat_map(|idx| self.try_hit_rec(ray, t_interval, num_bounces, *idx))
                .min_by(|a, b| a.t.total_cmp(&b.t))
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct BVHMesh {
    triangles: Vec<Triangle>,
    nodes: Vec<BVHNode>,
    material: Material,
}

impl Transformable for BVHMesh {
    fn apply_transform(self, transform: &Transform) -> Self {
        // 1. Apply transform to vertex positions and normals
        let transformed_triangles: Vec<Triangle> = self.triangles.into_iter().map(|tri| tri.apply_transform(transform)).collect();

        eprintln!("Transformed {} triangles", transformed_triangles.len());
        // 2. Rebuild BVH (technically only need to this when transform
        //    includes rotation. 
        // TODO
        let (new_triangles, new_bvh_nodes) = build_bvh(transformed_triangles);

        BVHMesh {
            triangles: new_triangles,
            nodes: new_bvh_nodes,
            ..self
        }
    }
}

impl BVHMesh {
    pub fn from_obj_file(path: &Path, material: Material) -> Hittable {
        let triangles = super::hittable::load_obj(path);
        let bvh_mesh = Self::new(triangles, material);
        Hittable::BVHMesh(bvh_mesh)
    }

    fn new(triangles: Vec<Triangle>, material: Material) -> Self {
        let (triangles, bvh_nodes) = build_bvh(triangles);

        Self {
            triangles: triangles,
            nodes: bvh_nodes,
            material: material,
        }
    }

    fn try_hit_rec(
        &self,
        ray: &Ray,
        t_interval: Interval,
        num_bounces: u32,
        bvh_idx: u32,
    ) -> Option<(u32, Float)> {
        // Traverse the bvh
        let node = &self.nodes[bvh_idx as usize];

        // A node is a leaf if it dont have no children
        if node.left_child == 0 {
            // eprintln!("Hit primitve at {}", bvh_idx);
            let range =
                (node.first_prim as usize)..(node.first_prim as usize + node.num_prims as usize);
            return range
                .map(|idx| (idx as u32, &self.triangles[idx]))
                .map(|(idx, tri)| {
                    if let Some(t_hit) = tri.ray_intersection(ray, &t_interval) {
                        Some((idx, t_hit))
                    } else {
                        None
                    }
                })
                .flatten()
                .min_by(|a, b| a.1.total_cmp(&b.1));
        }

        if node.aabb.hit(ray, t_interval) {
            let left_idx = bvh_idx * 2 + 1;
            let right_idx = bvh_idx * 2 + 2;

            match (
                Self::try_hit_rec(&self, ray, t_interval, num_bounces, left_idx),
                Self::try_hit_rec(&self, ray, t_interval, num_bounces, right_idx),
            ) {
                (Some(res1), Some(res2)) => {
                    if res1.1 < res2.1 {
                        Some(res1)
                    } else {
                        Some(res2)
                    }
                }
                (Some(t1), None) => Some(t1),
                (None, Some(t2)) => Some(t2),
                _ => None,
            }
        } else {
            None
        }
    }
}

impl HittableTrait for BVHMesh {
    fn num_primitives(&self) -> u32 {
        self.triangles.len() as u32
    }

    fn get_aabb(&self) -> AABoundingBox {
        self.nodes[0].aabb.clone()
    }

    fn try_hit(&self, ray: &Ray, t_interval: Interval, num_bounces: u32) -> Option<HitRecord> {
        if let Some((tri_idx, t_hit)) = Self::try_hit_rec(&self, ray, t_interval, num_bounces, 0) {
            let point_hit = ray.at(t_hit);

            // Interpolate normal of the triangle. First, we have to find the barycentric
            // coordinates, u, v, w.
            let a = self.triangles[tri_idx as usize].positions[0];
            let b = self.triangles[tri_idx as usize].positions[1];
            let c = self.triangles[tri_idx as usize].positions[2];
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
            let obj_normal = (self.triangles[tri_idx as usize].normals[0] * u
                + self.triangles[tri_idx as usize].normals[1] * v
                + self.triangles[tri_idx as usize].normals[2] * w)
                .normalize();
            Some(HitRecord::new(
                &ray.step(0.01),
                t_hit,
                point_hit,
                num_bounces,
                &self.material,
                obj_normal,
            ))
        } else {
            None
        }
    }

    fn centroid(&self) -> Vec3f {
        (self.nodes[0].aabb.max - self.nodes[0].aabb.min) * 0.5 + self.nodes[0].aabb.min
    }
}
