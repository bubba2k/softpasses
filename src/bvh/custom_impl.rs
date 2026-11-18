use crate::bvh::{BvhTrait, QueryResult};
use crate::tracer::hittable::{AABoundingBox, HitRecord, HittableTrait};
use crate::math::ray::Ray;
use crate::math::util::Interval;
use crate::{Float};
use std::array;

#[derive(Default, Clone)]
struct BVHNode {
    // First contains the index of the first primitive, if num_prims > 0,
    //  else it contains the index of the left child.
    first: u32,
    num_prims: u32,
    aabb: AABoundingBox,
}

// Simplest way to split: exactly along the middle of the longest axis of the node
#[allow(dead_code)]
fn simple_midpoint_split<T: HittableTrait>(node: &BVHNode, primitives: &Vec<T>) -> (u32, Float) {
    let extent = node.aabb.extent();
    let longest_axis = if extent.x > extent.y && extent.x > extent.z {
        0
    } else if extent.y > extent.z {
        1
    } else {
        2
    };
    let split_pos = (node.aabb.min[longest_axis] + node.aabb.max[longest_axis]) * 0.5;

    // Find the primitive centroid that is closest to the lowest split previously computed.
    // Otherwise, a split might not actually split at all!
    let prim_indices = (node.first as usize)..((node.first + node.num_prims) as usize);
    let actual_pos = primitives[prim_indices]
        .iter()
        .map(|prim| prim.centroid()[longest_axis as usize])
        .min_by(|a, b| {
            let diff_a = (split_pos - a).abs();
            let diff_b = (split_pos - b).abs();

            diff_a.total_cmp(&diff_b)
        })
        .expect("Attempted to find best split on empty node");

    (longest_axis as u32, actual_pos)
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

    // Reject empty areas
    if left_count == 0 || right_count == 0 {
        return Float::INFINITY;
    }

    let aabb_area = |aabb: &AABoundingBox| {
        let extent = aabb.extent();
        let area =
            2.0 * extent[0] * extent[1] + 2.0 * extent[0] * extent[2] + 2.0 * extent[1] * extent[2];
        area
    };

    let sah_value =
        (left_count as f32) * aabb_area(&aabb_left) + (right_count as f32) * aabb_area(&aabb_right);
    sah_value
}

// Compute lowest cost axis and position along it to split
fn best_split<T: HittableTrait>(node: &BVHNode, primitives: &Vec<T>) -> (u32, Float) {
    // Check a certain selection of candidate split positions here
    let num_positions = 50;
    let node_extent = node.aabb.extent();
    let split_candidates: Vec<(u32, Float)> = (0..3)
        .map(|axis: u32| {
            let axis_extent = node_extent[axis as usize];
            let axis_min = node.aabb.min[axis as usize];
            (0..=(num_positions - 1))
                .map(|i| {
                    (
                        axis,
                        axis_min + (axis_extent as f32) * ((i as f32) / (num_positions as f32)),
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

    (lowest_cost_split.0, lowest_cost_split.1)
}

fn subdivide<T: HittableTrait>(
    bvh_nodes: &mut Vec<BVHNode>,
    primitives: &mut Vec<T>,
    bvh_node_index: u32,
    next_free_index: &mut u32,
) {
    let node = &mut bvh_nodes[bvh_node_index as usize];

    // At the begin of a node split, the `first` member always points to the first primitive contained by the node.
    // -> The node is currently still "treated as a leaf". We only split
    let begin = node.first as usize;
    let end = (node.first + node.num_prims) as usize;
    // First things first: build this nodes AABB.
    for tri in primitives[begin..end].iter() {
        node.aabb.expand_aabb(&tri.get_aabb());
    }

    // If this node already has less than N primitives, dont split it.
    if node.num_prims < 3 {
        return;
    }

    // Determine the axis and the point on it to split
    // axis 0 -> x, 1 -> y, 2 -> z
    let (axis, split_value) = best_split(node, primitives);

    // Sort to the left and right of split value
    let mut i: isize = node.first as isize;
    let mut j: isize = i + node.num_prims as isize - 1;
    while i < j + 1 {
        if primitives[i as usize].centroid()[axis as usize] < split_value {
            i += 1;
        } else {
            primitives.swap(i as usize, j as usize);
            j -= 1;
        }
    }

    let left_num = i as u32 - bvh_nodes[bvh_node_index as usize].first;
    let right_num = bvh_nodes[bvh_node_index as usize].num_prims - left_num;

    if left_num == 0 || right_num == 0 {
        // If one of the split nodes would be empty, abort.
        return;
    } else {
        // Initialize the two children nodes and go on to subidivide them
        let left_idx = *next_free_index;
        *next_free_index += 1;
        let right_idx = *next_free_index;
        *next_free_index += 1;

        bvh_nodes[left_idx as usize].first = bvh_nodes[bvh_node_index as usize].first;
        bvh_nodes[left_idx as usize].num_prims = left_num;
        bvh_nodes[right_idx as usize].first = i as u32;
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

// Functional-style fold for the BVH node tree
fn fold_bvh<T>(bvh_nodes: &Vec<BVHNode>, index: usize, func_interior: fn(T, T) -> T, func_leaf: fn(&BVHNode) -> T) -> T {
    if bvh_nodes[index].num_prims != 0 { // Leaf
        func_leaf(&bvh_nodes[index])
    } else {
        func_interior(
            fold_bvh(bvh_nodes, bvh_nodes[index].first as usize, 
                func_interior, func_leaf),
            fold_bvh(bvh_nodes, bvh_nodes[index].first as usize + 1, 
                func_interior, func_leaf)
        )
    }
}

fn bvh_count_leaves(bvh_nodes: &Vec<BVHNode>) -> u32 {
    fold_bvh(bvh_nodes, 0, 
        |a, b| a + b,
        |_leaf| 1)
}

fn bvh_count_prims(bvh_nodes: &Vec<BVHNode>, index: usize) -> u32 {
    fold_bvh(bvh_nodes, index,
        u32::strict_add,
        |leaf| leaf.num_prims
    )
}

fn bvh_prims_min(bvh_nodes: &Vec<BVHNode>) -> u32 {
    fold_bvh(bvh_nodes, 0, 
        u32::min,
        |leaf| leaf.num_prims)
}

fn bvh_prims_max(bvh_nodes: &Vec<BVHNode>) -> u32 {
    fold_bvh(bvh_nodes, 0, 
        u32::max,
        |leaf| leaf.num_prims)
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
    imbalance_sum / (bvh_count_nodes(bvh_nodes, 0) - bvh_count_leaves(bvh_nodes)) as f64
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
    let num_leaves = bvh_count_leaves(bvh_nodes);
    let num_nodes = bvh_count_nodes(bvh_nodes, 0);
    let depth = bvh_depth(bvh_nodes, 0, 0);
    let shallowness = bvh_shallowness(bvh_nodes, 0, 0);
    let avg_imbalance = bvh_avg_imbalance(bvh_nodes);
    let min_imbalance = bvh_min_imbalance(bvh_nodes, 0);
    let max_imbalance = bvh_max_imbalance(bvh_nodes, 0);
    let min_leaf_prims = bvh_prims_min(bvh_nodes);
    let max_leaf_prims = bvh_prims_max(bvh_nodes);

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
pub struct BvhCustom<T: HittableTrait> {
    pub hittables: Vec<T>,
    nodes: Vec<BVHNode>,
}

impl<T: HittableTrait> BvhTrait for BvhCustom<T> {
    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> QueryResult {
        self.try_hit_ordered(ray, t_interval)
    }

    fn try_hit(&self, ray: &Ray, t_interval: &Interval) -> Option<HitRecord> {
        let query_result = self.try_hit_ordered(ray, t_interval);
        
        if let Some(hit) = query_result.hit {
            Some(self.hittables[hit.0].compute_hit(ray, hit.1))
        } else {
            None
        }
    }

    fn num_primitives(&self) -> usize {
        self.hittables.len()
    }
}

impl<T: HittableTrait> BvhCustom<T> {
    pub fn try_new(list: Vec<T>) -> Option<BvhCustom<T>> {
        eprintln!("Building scene bvh");

        let (hittables, bvh_nodes) = build_bvh(list);

        Some(Self {
            hittables: hittables,
            nodes: bvh_nodes,
        })
    }

    fn try_hit_ordered(&self, ray: &Ray, t_interval: &Interval) -> super::QueryResult {
        // Keep track of the nodes to discover here (DFS)
        // Try to preallocate a reasonably sized stack. Inside the stack, we save the near distance of
        // the aabb to the ray, and its index of course.
        // We use a statically sized stack here. This could cause some very mean issues
        // if the number of nodes we yet have to discover exceeds the capacity.
        // 64 cap should serve us well for now.
        let mut candidate_queue = StaticPriorityQueue::<usize, Float, 64>::new(0, 0.0);

        // The closest primitive hit found so far
        let mut best_t = Float::INFINITY;
        let mut best_prim_idx: usize = 0;

        let mut num_aabb_intersects = 0;
        let mut num_aabb_checks = 0;
        let mut num_primitive_checks = 0;

        // Start at the root node. We only push it on the stack if it is actually hit,
        // otherwise the algorithm just terminates right away.
        if let Some(dist) = self.nodes[0].aabb.dist(ray, t_interval) {
            num_aabb_intersects += 1;
            candidate_queue.push(0, dist);
        }                    
        num_aabb_checks += 1;
        num_aabb_intersects += 1;

        while let Some((node_idx, node_dist)) = candidate_queue.pop() {
            // Traverse the bvh

            // If this nodes AABB is hit *behind* the currently best t, we do not need to check it further.
            if node_dist > best_t {
                break;
            }

            let node = &self.nodes[node_idx];

            // A node is a leaf if it contains more than 0 prims
            if node.num_prims > 0 { // LEAF NODE
                // Determine the closest primitve hit inside this leaf node.
                // Since we are doing an ordered traverse (from nodes closest to furthest to camera),
                // we know that we have definitely found the closest hit, if there is one.
                num_primitive_checks += node.num_prims;
                if let Some(t_hit) = 
                    ((node.first as usize)..((node.first + node.num_prims) as usize))
                    .map(|idx| (idx as u32, &self.hittables[idx]))
                    .map(|(idx, prim)| {
                       if let Some (t_hit) = prim.try_intersect(ray, t_interval) {
                            Some((idx, t_hit))
                       } else {
                            None
                       }
                    })
                    .flatten()
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                {
                    if t_hit.1 < best_t {
                        best_t = t_hit.1;
                        best_prim_idx = t_hit.0 as usize;
                    }
                }
            } else { // INTERIOR NODE
                // Node is interior, check whether the ray intersects its children...
                let (a, b) = (node.first as usize, (node.first + 1) as usize);
                num_aabb_checks += 2;
                if let Some(t) = self.nodes[a].aabb.dist(ray, t_interval) {
                    num_aabb_intersects += 1;
                    if t < best_t {
                        candidate_queue.push(a, t);
                    }
                }
                if let Some(t) = self.nodes[b].aabb.dist(ray, t_interval) {
                    num_aabb_intersects += 1;
                    if t < best_t {
                        candidate_queue.push(b, t);
                    }
                }
            }
        }

        let hit = if best_t != Float::INFINITY {
            Some((best_prim_idx, best_t))
        } else {
            None
        };

        return super::QueryResult {
            hit: hit,
            num_aabb_checks: num_aabb_checks,
            num_aabb_hits: num_aabb_intersects,
            num_primitive_checks,
        };
    }
}

// A simple, statically allocated priority queue
struct StaticPriorityQueue<V: Copy, P: Copy + PartialOrd, const N: usize> {
    data: [(V,P); N],
    stack_ptr: usize,
}

impl<V: Copy, P: Copy + PartialOrd, const N: usize> StaticPriorityQueue<V, P, N> {
    pub fn new(fill_value: V, fill_priority: P) -> Self {
        StaticPriorityQueue {
            // TODO: It could have a slight performance advantage to leave
            // the data uninitialized. Would require unsafe code.
            data: array::from_fn(|_| (fill_value, fill_priority)),
            stack_ptr: 0,
        }
    }

    pub fn push(&mut self, value: V, prio: P) {
        self.data[self.stack_ptr] = (value, prio);
        self.stack_ptr += 1;
    }

    pub fn pop(&mut self) -> Option<(V, P)> {
        if self.stack_ptr == 0 {
            None
        } else {
            // Find the best element in the list
            let mut best_idx = 0;
            let mut best_prio = self.data[0].1;
            for idx in 1..self.stack_ptr {
                if self.data[idx].1 < best_prio {
                    best_idx        = idx;
                    best_prio   = self.data[idx].1;
                }
            }
            let best_value = self.data[best_idx];

            // Remove the element
            self.stack_ptr -= 1;
            self.data[best_idx] = self.data[self.stack_ptr];

            Some(best_value)
        }
    }
}

// A statically sized stack that lives in stack memory instead of heap
#[allow(unused)]
struct StaticStack<T: Copy, const N: usize> {
    data: [T; N],
    stack_ptr: usize,
}

#[allow(unused)]
impl<T: Copy, const N: usize> StaticStack<T, N> {
    fn new(fill: T) -> Self {
        StaticStack {
            stack_ptr: 0,
            // TODO: It could have a slight performance advantage to leave
            // the data uninitialized. Would require unsafe code.
            data: array::from_fn(|_| fill),
        }
    }

    fn push(&mut self, value: T) {
        self.data[self.stack_ptr] = value;
        self.stack_ptr += 1;
    }

    fn pop(&mut self) -> Option<T> {
        if self.stack_ptr == 0 {
            None
        } else {
            self.stack_ptr -= 1;
            Some(self.data[self.stack_ptr])
        }
    }
}