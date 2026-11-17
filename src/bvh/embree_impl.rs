use std::{ffi::c_void, ptr::{null, null_mut}};

use embree4_sys::*;

use crate::{bvh::BvhTrait, tracer::hittable::{Hittable, HittableTrait}};

pub struct BvhEmbree {
    device: RTCDevice,
    scene: RTCScene,
}

unsafe extern "C" fn hittable_bounds_function(args: *const RTCBoundsFunctionArguments) {
    let args = *args;

    let prim_id = args.primID;
    let hittables_ptr = args.geometryUserPtr as *const Hittable;
    let bounds_out = args.bounds_o;

    let aabb = (*hittables_ptr.offset(prim_id as isize)).get_aabb();

    (*bounds_out).lower_x = aabb.min.x;
    (*bounds_out).lower_y = aabb.min.y;
    (*bounds_out).lower_z = aabb.min.z;

    (*bounds_out).lower_x = aabb.min.x;
    (*bounds_out).lower_y = aabb.min.y;
    (*bounds_out).lower_z = aabb.min.z;
}

unsafe extern "C" fn hittable_intersect_function(args: *const RTCIntersectFunctionNArguments) {

}

impl BvhEmbree {
    pub fn try_new(hittables: Vec<Hittable>) -> Option<Self> {
        unsafe{
            let device = rtcNewDevice("-verbose".as_bytes() as *const _ as _);
            if device.is_null() {
                return None;   
            }

            let scene = rtcNewScene(device);
            if scene.is_null() {
                return None;
            }
            // An interesting option to explore might be `RTCSceneFlags::ROBUST`` (more accurate scene traversal)
            rtcSetSceneFlags(scene, RTCSceneFlags::NONE);

            // Setup the user geometry
            let geometry = rtcNewGeometry(device, RTCGeometryType::USER);
            rtcSetGeometryUserPrimitiveCount(geometry, hittables.len() as u32);
            rtcSetGeometryUserData(geometry, hittables.as_ptr() as *mut c_void);
            rtcSetGeometryBoundsFunction(geometry, Some(hittable_bounds_function), null_mut());
            rtcSetGeometryIntersectFunction(geometry, Some(hittable_intersect_function));

            rtcSetGeometryOccludedFunction(geometry, None);


            for hittable in hittables {
                // rtcAttachGeometry(scene, geometry);
            }

            rtcCommitScene(scene);

            Some(Self {
                device: device,
                scene: scene,
            })
        }
    }
}

impl Drop for BvhEmbree {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseDevice(self.device);
            rtcReleaseScene(self.scene);
        }
    }
}

impl BvhTrait for BvhEmbree {
    fn num_primitives(&self) -> usize {
        todo!()
    }

    fn try_hit(&self, ray: &crate::math::ray::Ray, t_interval: &crate::math::util::Interval) -> Option<crate::tracer::hittable::HitRecord> {
        todo!()
    }

    fn try_intersect(&self, ray: &crate::math::ray::Ray, t_interval: &crate::math::util::Interval) -> super::QueryResult {
        todo!()
    }
}