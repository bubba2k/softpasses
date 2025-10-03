use super::hittable::{AABoundingBox, HitRecord, Hittable, HittableTrait, RayInfo, Triangle};
use crate::Material;
use crate::math::ray::Ray;
use crate::math::util::Interval;
use crate::{Float, Vec3f};
use crate::{Transform, Transformable};
use std::path::Path;

#[derive(Default, Clone)]
struct BVHNode {
    // First contains the index of the first primitive, if num_prims > 0,
    //  else it contains the index of the left child.
    first: u32,
    num_prims: u32,
    aabb: AABoundingBox,
}

fn eval_sah<T: HittableTrait>(
    node: &BVHNode,
    primitives: &Vec<T>,
    split_pos: Float,
    axis: usize,
) -> Float {
    let (mut aabb_left, mut aabb_right) = (AABoundingBox::default(), AABoundingBox::default());
    let (mut left_count, mut right_count) = (0, 0);

    for i in 0..node.num_prims {
        let primitive = &primitives[(node.first + i) as usize];

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
        let area =
            2.0 * extent[0] * extent[1] + 2.0 * extent[0] * extent[2] + 2.0 * extent[1] * extent[2];
        area
    };

    (left_count as f32) * aabb_area(&aabb_left) + (right_count as f32) * aabb_area(&aabb_right)
}

// Compute lowest cost axis and position along it to split
fn best_split<T: HittableTrait>(node: &BVHNode, primitives: &Vec<T>) -> (u32, Float) {
    // Check a certain selection of candidate split positions here
    let num_positions = 100;
    let node_extent = node.aabb.max - node.aabb.min;
    let split_candidates: Vec<(u32, Float)> = (0..3)
        .map(|axis: u32| {
            let axis_extent = node_extent[axis as usize];
            let axis_min = node.aabb.min[axis as usize];
            (0..=(num_positions - 1))
                .map(|i| {
                    (
                        axis,
                        axis_min + (axis_extent as f32) * (i as f32) / (num_positions as f32),
                    )
                })
                .collect::<Vec<(u32, Float)>>()
        })
        .flatten()
        .collect();

    let lowest_cost_split = split_candidates
        .iter()
        .min_by(|a, b| {
            let sah_a = eval_sah(node, primitives, a.1, a.0 as usize);
            let sah_b = eval_sah(node, primitives, b.1, b.0 as usize);

            sah_a.total_cmp(&sah_b)
        })
        .expect("Attempted to find best split on empty node");

    // Find the primitive centroid that is closest to the lowest split previously computed.
    // Otherwise, a split might not actually split at all!
    let prim_indices = (node.first as usize)..((node.first + node.num_prims) as usize);
    let actual_pos = primitives[prim_indices]
        .iter()
        .map(|prim| prim.centroid()[lowest_cost_split.0 as usize])
        .min_by(|a, b| {
            let diff_a = (lowest_cost_split.1 - a).abs();
            let diff_b = (lowest_cost_split.1 - b).abs();

            diff_a.total_cmp(&diff_b)
        })
        .expect("Attempted to find best split on empty node");

    (lowest_cost_split.0, actual_pos)
}

fn subdivide<T: HittableTrait>(
    bvh_nodes: &mut Vec<BVHNode>,
    primitives: &mut Vec<T>,
    bvh_node_index: u32,
    next_free_index: &mut u32,
) {
    // Always split along longest axis for now
    let node = &mut bvh_nodes[bvh_node_index as usize];

    // At the begin of a node split, the `first` member always points to the first primitive contained by the node.
    // -> The node is currently still treated as a leaf.
    let begin = node.first as usize;
    let end = (node.first + node.num_prims) as usize;
    for tri in primitives[begin..end].iter() {
        node.aabb.expand_aabb(&tri.get_aabb());
    }

    let (axis, split_value) = best_split(node, primitives);

    // Sort to the left and right of split value
    let mut i = node.first;
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
    let left_idx = *next_free_index;
    *next_free_index += 1;
    let right_idx = *next_free_index;
    *next_free_index += 1;
    let left_num = i - bvh_nodes[bvh_node_index as usize].first;
    let right_num = bvh_nodes[bvh_node_index as usize].num_prims - left_num;

    // Handle degenerate splits
    if left_num == 0 || right_num == 0 {
        // We do not want empty leaves! The straightforward approach is to simply abort subdivision here.
        return;
    } else {
        bvh_nodes[left_idx as usize].first = bvh_nodes[bvh_node_index as usize].first;
        bvh_nodes[left_idx as usize].num_prims = left_num;
        bvh_nodes[right_idx as usize].first = i;
        bvh_nodes[right_idx as usize].num_prims = right_num;
        subdivide(bvh_nodes, primitives, left_idx, next_free_index);
        subdivide(bvh_nodes, primitives, right_idx, next_free_index);

        // Since we have split this node, it is not a leaf, and thus does not contain any primitives.
        // Its `first` member must be the index of the left child.
        bvh_nodes[bvh_node_index as usize].first = left_idx;
        bvh_nodes[bvh_node_index as usize].num_prims = 0;
    }
}

fn build_bvh<T: HittableTrait>(mut primitives: Vec<T>) -> (Vec<T>, Vec<BVHNode>) {
    // The recursive func to build the BVH search tree
    eprintln!("Building BVH.");
    let start = std::time::Instant::now();
    let num_prims = primitives.len();

    // Assuming one primitve per leaf, this should make sure we never run out of nodes,
    // though we also might end up wasting some space. Mathematically, it would be 2 * num_prims - 1,
    // but we leave index 1 open (see first call to subdivide below).
    // It might be worth instead trying to allocate nodes during BVH construction at some point.
    let mut bvh_nodes: Vec<BVHNode> = vec![BVHNode::default(); 2 * num_prims];
    // Set the root node
    bvh_nodes[0].first = 0;
    bvh_nodes[0].num_prims = num_prims as u32;

    // Use 2 as the next free index, leaving index 1 open. That way neighbouring BVH nodes should sit snugly
    // inside the same cache line, assuming cache line size of 64 bytes. Do not quote me on this.
    let mut next_free_index = 2;
    subdivide(&mut bvh_nodes, &mut primitives, 0, &mut next_free_index);

    eprintln!("Built BVH in {:.3} s", start.elapsed().as_secs_f32());
    bvh_info(&bvh_nodes);
    (primitives, bvh_nodes)
}

fn bvh_count_leaves(bvh_nodes: &Vec<BVHNode>, index: usize) -> u32 {
    if bvh_nodes[index].num_prims != 0 {
        1
    } else {
        // Traverse left and right children and sum
        bvh_count_leaves(bvh_nodes, bvh_nodes[index].first as usize)
            + bvh_count_leaves(bvh_nodes, (bvh_nodes[index].first + 1) as usize)
    }
}

fn bvh_count_prims(bvh_nodes: &Vec<BVHNode>, index: usize) -> u32 {
    if bvh_nodes[index].num_prims != 0 {
        bvh_nodes[index].num_prims
    } else {
        // Traverse left and right children and sum
        bvh_count_prims(bvh_nodes, bvh_nodes[index].first as usize)
            + bvh_count_prims(bvh_nodes, (bvh_nodes[index].first + 1) as usize)
    }
}

fn bvh_prims_min(bvh_nodes: &Vec<BVHNode>, index: usize) -> u32 {
    if bvh_nodes[index].num_prims != 0 {
        bvh_nodes[index].num_prims
    } else {
        // Traverse left and right children and sum
        bvh_prims_min(bvh_nodes, bvh_nodes[index].first as usize).min(bvh_prims_min(
            bvh_nodes,
            (bvh_nodes[index].first + 1) as usize,
        ))
    }
}

fn bvh_prims_max(bvh_nodes: &Vec<BVHNode>, index: usize) -> u32 {
    if bvh_nodes[index].num_prims != 0 {
        bvh_nodes[index].num_prims
    } else {
        // Traverse left and right children and sum
        bvh_prims_max(bvh_nodes, bvh_nodes[index].first as usize).max(bvh_prims_max(
            bvh_nodes,
            (bvh_nodes[index].first + 1) as usize,
        ))
    }
}

fn bvh_depth(bvh_nodes: &Vec<BVHNode>, index: usize, depth: u32) -> u32 {
    if bvh_nodes[index].num_prims != 0 {
        depth
    } else {
        // Traverse left and right children and sum
        bvh_depth(bvh_nodes, bvh_nodes[index].first as usize, depth + 1).max(bvh_depth(
            bvh_nodes,
            (bvh_nodes[index].first + 1) as usize,
            depth + 1,
        ))
    }
}

fn bvh_shallowness(bvh_nodes: &Vec<BVHNode>, index: usize, depth: u32) -> u32 {
    if bvh_nodes[index].num_prims != 0 {
        depth
    } else {
        // Traverse left and right children and sum
        bvh_shallowness(bvh_nodes, bvh_nodes[index].first as usize, depth + 1).min(bvh_shallowness(
            bvh_nodes,
            (bvh_nodes[index].first + 1) as usize,
            depth + 1,
        ))
    }
}

fn bvh_count_nodes(bvh_nodes: &Vec<BVHNode>, index: usize) -> u32 {
    if bvh_nodes[index].num_prims != 0 {
        1
    } else {
        // Traverse left and right children and sum
        bvh_count_nodes(bvh_nodes, bvh_nodes[index].first as usize)
            + bvh_count_nodes(bvh_nodes, (bvh_nodes[index].first + 1) as usize)
            + 1
    }
}

// Imbalance: The ratio between the smaller and larger (as in, how much primitives it is parented to) node
// Only applicable to interior nodes
fn bvh_node_imbalance(bvh_nodes: &Vec<BVHNode>, index: usize) -> Option<f64> {
    if bvh_nodes[index].num_prims != 0 {
        None
    } else {
        let (smaller, larger) = {
            let a = bvh_count_prims(bvh_nodes, (bvh_nodes[index].first) as usize);
            let b = bvh_count_prims(bvh_nodes, (bvh_nodes[index].first + 1) as usize);
            if a < b { (a, b) } else { (b, a) }
        };
        Some(larger as f64 / smaller as f64)
    }
}

fn bvh_min_imbalance(bvh_nodes: &Vec<BVHNode>, index: usize) -> f64 {
    if bvh_nodes[index].num_prims != 0 {
        f64::INFINITY
    } else {
        bvh_min_imbalance(bvh_nodes, bvh_nodes[index].first as usize)
            .min(bvh_min_imbalance(
                bvh_nodes,
                (bvh_nodes[index].first + 1) as usize,
            ))
            .min(bvh_node_imbalance(bvh_nodes, index).unwrap())
    }
}

fn bvh_max_imbalance(bvh_nodes: &Vec<BVHNode>, index: usize) -> f64 {
    if bvh_nodes[index].num_prims != 0 {
        f64::NEG_INFINITY
    } else {
        bvh_max_imbalance(bvh_nodes, bvh_nodes[index].first as usize)
            .max(bvh_max_imbalance(
                bvh_nodes,
                (bvh_nodes[index].first + 1) as usize,
            ))
            .max(bvh_node_imbalance(bvh_nodes, index).unwrap())
    }
}

// Computed ONLY for interior nodes
fn bvh_avg_imbalance(bvh_nodes: &Vec<BVHNode>) -> f64 {
    let imbalance_sum = bvh_avg_imbalance_rec(bvh_nodes, 0);
    // Subtract number of leaves here, else they would introduce bias.
    imbalance_sum / (bvh_count_nodes(bvh_nodes, 0) - bvh_count_leaves(bvh_nodes, 0)) as f64
}

fn bvh_avg_imbalance_rec(bvh_nodes: &Vec<BVHNode>, index: usize) -> f64 {
    if bvh_nodes[index].num_prims != 0 {
        // Leaves do not have a balance, return zero.
        0.0
    } else {
        bvh_avg_imbalance_rec(bvh_nodes, bvh_nodes[index].first as usize)
            + (bvh_avg_imbalance_rec(bvh_nodes, (bvh_nodes[index].first + 1) as usize))
            + bvh_node_imbalance(bvh_nodes, index).unwrap()
    }
}

fn bvh_info(bvh_nodes: &Vec<BVHNode>) {
    let num_primitives = bvh_count_prims(bvh_nodes, 0);
    let num_leaves = bvh_count_leaves(bvh_nodes, 0);
    let num_nodes = bvh_count_nodes(bvh_nodes, 0);
    let depth = bvh_depth(bvh_nodes, 0, 0);
    let shallowness = bvh_shallowness(bvh_nodes, 0, 0);
    let avg_imbalance = bvh_avg_imbalance(bvh_nodes);
    let min_imbalance = bvh_min_imbalance(bvh_nodes, 0);
    let max_imbalance = bvh_max_imbalance(bvh_nodes, 0);
    let min_leaf_prims = bvh_prims_min(bvh_nodes, 0);
    let max_leaf_prims = bvh_prims_max(bvh_nodes, 0);

    eprintln!(
        "Num nodes: {}\nNum prims: {}\nNum leafs: {}\nMin prims: {}\nMax prims: {}\n: {:.3}\nDepth: {}\nShallowness {}",
        num_nodes,
        num_primitives,
        num_leaves,
        min_leaf_prims,
        max_leaf_prims,
        (num_primitives as f32) / (num_leaves as f32),
        depth,
        shallowness
    );
    eprintln!(
        "Avg imbalance: {:.3}\nMin imbalance: {:.3}\nMax imbalance: {:.3}\n",
        avg_imbalance, min_imbalance, max_imbalance
    );
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

    fn try_hit(&self, ray: &Ray, t_interval: Interval, ray_info: &RayInfo) -> Option<HitRecord> {
        self.try_hit_rec(ray, t_interval, ray_info, 0)
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
        ray_info: &RayInfo,
        bvh_idx: u32,
    ) -> Option<HitRecord> {
        // Traverse the bvh
        let node = &self.nodes[bvh_idx as usize];

        // A node is a leaf if it has primitives
        if node.num_prims != 0 {
            let range = (node.first as usize)..(node.first as usize + node.num_prims as usize);
            return range
                .map(|idx| self.hittables[idx].try_hit(ray, t_interval, ray_info))
                .flatten()
                .min_by(|a, b| a.t.total_cmp(&b.t));
        }

        if node.aabb.hit(ray, t_interval) {
            let left_idx = node.first;
            let right_idx = node.first + 1;

            [left_idx, right_idx]
                .iter()
                .flat_map(|idx| self.try_hit_rec(ray, t_interval, ray_info, *idx))
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

pub struct BVHQueryResult {
    pub primitive_index: u32,
    pub hit_t: Float,
    pub num_aabb_checks: u32,
    pub num_aabb_hits: u32,
    pub num_primitve_checks: u32,
}

impl Transformable for BVHMesh {
    fn apply_transform(self, transform: &Transform) -> Self {
        // 1. Apply transform to vertex positions and normals
        let transformed_triangles: Vec<Triangle> = self
            .triangles
            .into_iter()
            .map(|tri| tri.apply_transform(transform))
            .collect();

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

    fn try_hit_it_ordered(&self, ray: &Ray, t_interval: Interval) -> Option<BVHQueryResult> {
        // Can abort right away if the root AABB is not hit.
        if !self.nodes[0].aabb.hit(ray, t_interval) {
            return None;
        }

        // Keep track of the nodes to discover here (DFS)
        let mut to_discover = Vec::<usize>::new();

        let mut num_aabb_intersects = 0;
        let mut num_aabb_checks = 0;
        let mut num_primitive_checks = 0;

        // Start at root node
        to_discover.push(0);
        while to_discover.len() != 0 {
            // Traverse the bvh
            let node = &self.nodes[to_discover.pop().unwrap()];

            // A node is a leaf if it contains more than 0 prims
            if node.num_prims > 0 {
                // Determine the closest primitve hit inside this leaf node.
                // Since we are doing an ordered traverse (from nodes closest to furthest to camera),
                // we know that we have definitely found the closest hit, if there is one.
                num_primitive_checks += node.num_prims;
                if let Some(closest_hit) = ((node.first as usize)
                    ..((node.first + node.num_prims) as usize))
                    .map(|idx| (idx as u32, &self.triangles[idx]))
                    .map(|(idx, tri)| {
                        if let Some(t_hit) = tri.ray_intersection(ray, &t_interval) {
                            Some((idx, t_hit))
                        } else {
                            None
                        }
                    })
                    .flatten()
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                {
                    return Some(BVHQueryResult {
                        primitive_index: closest_hit.0,
                        hit_t: closest_hit.1,
                        num_aabb_checks: num_aabb_checks,
                        num_aabb_hits: num_aabb_intersects,
                        num_primitve_checks: num_primitive_checks,
                    });
                }
            } else {
                // Node is interior, check whether we care about its children
                // Remember: We want to go for the _closest_ nodes first, and we ignore all nodes whose
                // AABB is not hit.
                let (a, b) = (node.first as usize, (node.first + 1) as usize);
                num_aabb_checks += 2;
                match (
                    self.nodes[a].aabb.dist(ray, t_interval),
                    self.nodes[b].aabb.dist(ray, t_interval),
                ) {
                    (None, None) => {}
                    (None, Some(_dist_b)) => {
                        to_discover.push(b);
                        num_aabb_intersects += 1;
                    }
                    (Some(_dist_a), None) => {
                        to_discover.push(a);
                        num_aabb_intersects += 1;
                    }
                    (Some(dist_a), Some(dist_b)) => {
                        num_aabb_intersects += 2;

                        if dist_a < dist_b {
                            to_discover.push(b);
                            to_discover.push(a);
                        } else {
                            to_discover.push(a);
                            to_discover.push(b);
                        }
                    }
                };
            }
        }

        None
    }

    fn try_hit_it(&self, ray: &Ray, t_interval: Interval) -> Option<BVHQueryResult> {
        // Query metrics
        let mut num_aabb_intersects = 0;
        let mut num_aabb_checks = 0;
        let mut num_primitive_checks = 0;

        // Keep track of the nodes to discover here (DFS)
        let mut to_discover = Vec::<usize>::new();
        // Save the leaf nodes containing the primitives we have to check later here
        let mut visited_leaf_nodes = Vec::<&BVHNode>::new();
        // TODO: Reserve some space for the two Vecs above.

        // First traverse all the relevant nodes and store the indices of the visited leaf nodes.
        // Once node discovery is done, we check the actual primitives.

        // Start at root node
        to_discover.push(0);
        while to_discover.len() != 0 {
            // Traverse the bvh
            let node = &self.nodes[to_discover.pop().unwrap()];
            num_aabb_checks += 1;

            if !node.aabb.hit(ray, t_interval) {
                continue;
            } else {
                num_aabb_intersects += 1;
                if node.num_prims > 0 {
                    // Node is a leaf, save it!
                    visited_leaf_nodes.push(node);
                } else {
                    // Node is interior, discover its children
                    to_discover.push((node.first + 1) as usize);
                    to_discover.push(node.first as usize);
                }
            }
        }

        // Node discovery done, now check all the primitives and find the closest hit.
        let closest_hit = visited_leaf_nodes
            .iter()
            .map(|node| (node.first as usize)..(node.first as usize + node.num_prims as usize))
            .map(|idcs| idcs.map(|idx| (idx as u32, &self.triangles[idx])))
            .flatten()
            .map(|(idx, tri)| {
                if let Some(t_hit) = tri.ray_intersection(ray, &t_interval) {
                    Some((idx, t_hit))
                } else {
                    None
                }
            })
            .flatten()
            .min_by(|a, b| a.1.total_cmp(&b.1));

        num_primitive_checks = visited_leaf_nodes.iter().map(|node| node.num_prims).sum();

        if let Some((prim_idx, t_hit)) = closest_hit {
            Some(BVHQueryResult {
                primitive_index: prim_idx,
                hit_t: t_hit,
                num_aabb_checks: num_aabb_checks,
                num_aabb_hits: num_aabb_checks,
                num_primitve_checks: num_primitive_checks,
            })
        } else {
            None
        }
    }

    fn try_hit_rec(&self, ray: &Ray, t_interval: Interval, bvh_idx: u32) -> Option<(u32, Float)> {
        // Traverse the bvh
        let node = &self.nodes[bvh_idx as usize];

        // A node is a leaf if it has primitives
        if node.num_prims != 0 {
            // eprintln!("Hit primitve at {}", bvh_idx);
            let range = (node.first as usize)..(node.first as usize + node.num_prims as usize);
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
            let left_idx = node.first;
            let right_idx = node.first + 1;

            match (
                Self::try_hit_rec(&self, ray, t_interval, left_idx),
                Self::try_hit_rec(&self, ray, t_interval, right_idx),
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

    fn try_hit(&self, ray: &Ray, t_interval: Interval, ray_info: &RayInfo) -> Option<HitRecord> {
        if let Some(BVHQueryResult {
            primitive_index,
            hit_t: t_hit,
            num_aabb_checks,
            num_aabb_hits,
            num_primitve_checks,
        }) = Self::try_hit_it(&self, ray, t_interval)
        {
            let point_hit = ray.at(t_hit);

            // Interpolate normal of the triangle. First, we have to find the barycentric
            // coordinates, u, v, w.
            let a = self.triangles[primitive_index as usize].positions[0];
            let b = self.triangles[primitive_index as usize].positions[1];
            let c = self.triangles[primitive_index as usize].positions[2];
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
            let obj_normal = (self.triangles[primitive_index as usize].normals[0] * u
                + self.triangles[primitive_index as usize].normals[1] * v
                + self.triangles[primitive_index as usize].normals[2] * w)
                .normalize();
            Some(HitRecord::new(
                &ray.step(0.01),
                t_hit,
                point_hit,
                ray_info.num_bounces,
                &self.material,
                obj_normal,
                num_aabb_hits,
                num_aabb_checks,
                num_primitve_checks,
            ))
        } else {
            None
        }
    }

    fn centroid(&self) -> Vec3f {
        (self.nodes[0].aabb.max - self.nodes[0].aabb.min) * 0.5 + self.nodes[0].aabb.min
    }
}
