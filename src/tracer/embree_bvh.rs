use crate::tracer::hittable::{Hittable, HittableTrait};

struct BVH {
    
}

impl BVH {
    fn new(hittables: Vec<Hittable>) {

    }
}

impl HittableTrait for BVH {
    fn centroid(&self) -> crate::math::vector::Vec3f {
        todo!()
    }

    fn get_aabb(&self) -> super::hittable::AABoundingBox {
        todo!()
    }

    fn num_primitives(&self) -> u32 {
        todo!()
    }

    fn try_intersect(&self, ray: &crate::math::ray::Ray, t_interval: &crate::math::util::Interval) -> Option<crate::math::vector::Float> {
        todo!()
    }

    fn try_hit(&self, ray: &crate::math::ray::Ray, t_interval: &crate::math::util::Interval) -> Option<super::hittable::HitRecord> {
        todo!()
    }

    fn compute_hit(&self, ray: &crate::math::ray::Ray, t: crate::math::vector::Float) -> super::hittable::HitRecord {
        todo!()
    }
}