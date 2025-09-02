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
    aabb: AABoundingBox,
}

fn eval_sah<T: HittableTrait>(node: &BVHNode, primitives: &Vec<T>, split_pos: Float, axis: usize) -> Float {
    let (mut aabb_left, mut aabb_right) = (AABoundingBox::default(), AABoundingBox::default());
    let (mut left_count, mut right_count) = (0, 0);

    for i in 0..node.num_prims {
        let primitive = &primitives[(node.first_prim + i) as usize];

        if primitive.centroid()[axis] < split_pos {
            aabb_left.expand_aabb(&primitive.get_aabb());
            left_count += 1;
        } else {
            aabb_right.expand_aabb(&primitive.get_aabb());
            right_count += 1;
        }
    }

    let aabb_area = |aabb: &AABoundingBox| { 
        let extent = aabb.max - aabb.min;
        let area = 2.0 * extent[0] * extent[1] + 2.0 * extent[0] * extent[2] + 2.0 * extent[1] * extent[2];
        area
    };

    (left_count as f32) * aabb_area(&aabb_left) + (right_count as f32) * aabb_area(&aabb_right)
}

// Compute lowest cost axis and position along it to split
fn best_split<T: HittableTrait>(node: &BVHNode, primitives: &Vec<T>) -> (u32, Float) {
    // Check a certain selection of candidate split positions here
    let num_positions = 100;
    let node_extent = node.aabb.max - node.aabb.min;
    let split_candidates: Vec<(u32, Float)> = (0..3).map(|axis: u32| {
        let axis_extent = node_extent[axis as usize];
        let axis_min = node.aabb.min[axis as usize];
        (0..=(num_positions - 1)).map(|i| {
            (axis, axis_min + (axis_extent as f32) * (i as f32) / (num_positions as f32))
        }).collect::<Vec<(u32, Float)>>()
    }).flatten().collect();

    let lowest_cost_split = split_candidates.iter().min_by(|a, b| {
        let sah_a = eval_sah(node, primitives, a.1, a.0 as usize);
        let sah_b = eval_sah(node, primitives, b.1, b.0 as usize);

        sah_a.total_cmp(&sah_b)
    }).expect("Attempted to find best split on empty node");

    // Find the primitive centroid that is closest to the lowest split previously computed.
    // Otherwise, a split might not actually split at all!
    let prim_indices = (node.first_prim as usize)..((node.first_prim + node.num_prims) as usize);
    let actual_pos = primitives[prim_indices].iter()
    .map(|prim| prim.centroid()[lowest_cost_split.0 as usize])
    .min_by(|a, b| {
        let diff_a = (lowest_cost_split.1 - a).abs();
        let diff_b = (lowest_cost_split.1 - b).abs();
        
        diff_a.total_cmp(&diff_b)
    }).expect("Attempted to find best split on empty node");

    (lowest_cost_split.0, actual_pos)
}

fn subdivide<T: HittableTrait>(bvh_nodes: &mut Vec<BVHNode>, primitives: &mut Vec<T>, bvh_node_index: u32) {
    // Always split along longest axis for now
    let node = &mut bvh_nodes[bvh_node_index as usize];
    if node.num_prims == 1 {
        // For leaf nodes, we do not initalize the aabb or anything like that
        return;
    }
    let begin = node.first_prim as usize;
    let end = (node.first_prim + node.num_prims) as usize;
    for tri in primitives[begin..end].iter() {
        node.aabb.expand_aabb(&tri.get_aabb());
    }

    let (axis, split_value) = best_split(node, primitives);

    // Sort to the left and right of split value
    let mut i = node.first_prim;
    let mut j = i + node.num_prims - 1;
    while i < j + 1 {
        // For now, we use the first corner of each triangle as the centroid
        if primitives[i as usize].centroid()[axis as usize] < split_value {
            i += 1;
        } else {
            primitives.swap(i as usize, j as usize);
            j -= 1;
        }
    }
    // Initialize the two children nodes and go on to subidivide them
    let left_idx = bvh_node_index * 2 + 1;
    let right_idx = bvh_node_index * 2 + 2;
    let left_num = i - bvh_nodes[bvh_node_index as usize].first_prim;
    let right_num = bvh_nodes[bvh_node_index as usize].num_prims - left_num;

    // Handle degenerate splits
    if left_num == 0 || right_num == 0 {
        // We do not want empty leaves! The straightforward approach is to simply abort subdivision here.
        return;

    } else {
        bvh_nodes[left_idx as usize].first_prim = bvh_nodes[bvh_node_index as usize].first_prim;
        bvh_nodes[left_idx as usize].num_prims = left_num;
        bvh_nodes[right_idx as usize].first_prim = i;
        bvh_nodes[right_idx as usize].num_prims = right_num;
        bvh_nodes[bvh_node_index as usize].left_child = left_idx;
        subdivide(bvh_nodes, primitives, left_idx);
        subdivide(bvh_nodes, primitives, right_idx);
    }
}

fn build_bvh<T: HittableTrait> (mut primitives: Vec<T>) -> (Vec<T>, Vec<BVHNode>) {
    // The recursive func to build the BVH search tree
    eprintln!("Building BVH.");
    let start = std::time::Instant::now();
    let num_prims = primitives.len();

    // Assume one triangle per leaf
    let mut bvh_nodes: Vec<BVHNode> = vec![BVHNode::default(); 1000 * num_prims - 1];
    // Set the root node
    bvh_nodes[0].first_prim = 0;
    bvh_nodes[0].num_prims = num_prims as u32;

    subdivide(&mut bvh_nodes, &mut primitives, 0);

    eprintln!("Built BVH in {:.3} s", start.elapsed().as_secs_f32());
    bvh_info(&bvh_nodes);
    (primitives, bvh_nodes)
}

fn bvh_count_leaves(bvh_nodes: &Vec<BVHNode>, index: usize) -> u32 {
    if bvh_nodes[index].left_child == 0 {
        1
    } else {
        // Traverse left and right children and sum
        bvh_count_leaves(bvh_nodes, index * 2 + 1) +
        bvh_count_leaves(bvh_nodes, index * 2 + 2)
    }
}

fn bvh_info(bvh_nodes: &Vec<BVHNode>) {
    let num_primitives = bvh_nodes[0].num_prims;
    let num_leaves = bvh_count_leaves(bvh_nodes, 0);

    eprintln!("Num prims: {}\nNum leafs: {}\nAvg prims per leaf: {:.4}\n", 
        num_primitives, num_leaves, (num_primitives as f32) / (num_leaves as f32));
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
