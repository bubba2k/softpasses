pub mod custom_impl;
pub mod embree_impl;

use crate::math::{ray::Ray, util::Interval, vector::Float};
use crate::tracer::hittable::HitRecord;

pub struct QueryResult {
    /// The hit contains a tuple of the index of the hit primitve,
    /// and the distance `t` from the ray origin the hit occured at.
    pub hit: Option<(usize, Float)>,
    pub num_aabb_checks: u32,
    pub num_aabb_hits: u32,
    pub num_primitive_checks: u32,
}

pub trait BvhTrait {
    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> QueryResult;
    fn try_hit(&self, ray: &Ray, t_interval: &Interval) -> Option<HitRecord>;
    fn num_primitives(&self) -> usize;
}