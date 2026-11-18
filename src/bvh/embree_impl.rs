use std::{ffi::c_void, ptr::null_mut};

use embree4_sys::*;

use crate::{
    bvh::{BvhTrait, QueryResult},
    math::{ray::Ray, vector::Vec3f},
    tracer::hittable::{HitRecord, Hittable, HittableTrait},
};

pub struct BvhEmbree {
    device_handle: RTCDevice,
    scene_handle: RTCScene,
    geometry_handle: RTCGeometry,

    // These need to be kept in memory here, because RTCGeometry only keeps pointers to it
    hittables: Vec<Hittable>,
}

// RTCDevice and RTCScene are perfectly fine to share between threads.
unsafe impl Sync for BvhEmbree {
    
}

unsafe extern "C" fn hittable_bounds_function(args: *const RTCBoundsFunctionArguments) {
    unsafe{
        let args = *args;

        let prim_id = args.primID;
        let hittables_ptr = args.geometryUserPtr as *const Hittable;
        let bounds_out = args.bounds_o;

        let aabb = (*hittables_ptr.offset(prim_id as isize)).get_aabb();

        (*bounds_out).lower_x = aabb.min.x;
        (*bounds_out).lower_y = aabb.min.y;
        (*bounds_out).lower_z = aabb.min.z;

        (*bounds_out).upper_x = aabb.max.x;
        (*bounds_out).upper_y = aabb.max.y;
        (*bounds_out).upper_z = aabb.max.z;
    }
}

unsafe extern "C" fn hittable_intersect_function(args: *const RTCIntersectFunctionNArguments) {
    unsafe {
        let args = *args;

        // Only rtcIntersect1 is used atm, which always invokes this callback with exactly
        // one, always-active lane. That also makes RTCRayHitN layout-compatible with
        // RTCRayHit, which the cast below relies on. Packet queries (rtcIntersect4/8)
        // store rays and hits in separate structure-of-arrays blocks and would need a
        // per-lane rewrite. Keeping these as asserts documents the assumption and makes
        // a future packet switch fail loudly in debug instead of silently writing into
        // neighbouring lanes.
        debug_assert_eq!(args.N, 1);
        debug_assert!(!args.valid.is_null() && *args.valid == -1);

        let hittables_ptr = args.geometryUserPtr as *const Hittable;
        let hittable = &*hittables_ptr.add(args.primID as usize);

        let ray_hit = &mut *(args.rayhit as *mut RTCRayHit);

        // Embree clips with tnear <= t < tfar, so build the interval from the ray
        // state it handed us. Ray::new also fills inv_dir, which the primitive code
        // expects.
        let ray = Ray::new(
            &Vec3f::new(ray_hit.ray.org_x, ray_hit.ray.org_y, ray_hit.ray.org_z),
            &Vec3f::new(ray_hit.ray.dir_x, ray_hit.ray.dir_y, ray_hit.ray.dir_z),
        );
        // Interval is a RangeInclusive, so t == tfar is admitted here; the check
        // below enforces Embree's strict t < tfar.
        let interval = ray_hit.ray.tnear..=ray_hit.ray.tfar;

        if let Some(t) = hittable.try_intersect(&ray, &interval) {
            if t < ray_hit.ray.tfar {
                ray_hit.ray.tfar = t;

                // geomID is mandatory: Embree never fills it in for user geometry and
                // treats a hit whose geomID is still RTC_INVALID_GEOMETRY_ID as a miss.
                // primID is what BvhEmbree indexes into `hittables` with.
                ray_hit.hit.geomID = args.geomID;
                ray_hit.hit.primID = args.primID;

                // Ng_* (geometric normal) and u/v (barycentric coordinates) are
                // intentionally left untouched because they are recomputed by
                // calling Hittable::compute_hit anyway
            }
        }
    }
}

impl BvhEmbree {
    pub fn try_new(hittables: Vec<Hittable>) -> Option<Self> {
        unsafe{
            let device = rtcNewDevice(c"-verbose".as_ptr() as *const _ as _);
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
            rtcCommitGeometry(geometry);

            rtcAttachGeometry(scene, geometry);

            rtcCommitScene(scene);

            Some(Self {
                device_handle: device,
                scene_handle: scene,
                geometry_handle: geometry,
                hittables: hittables,
            })
        }
    }
}

impl Drop for BvhEmbree {
    fn drop(&mut self) {
        unsafe {
            rtcReleaseGeometry(self.geometry_handle);
            rtcReleaseScene(self.scene_handle);
            rtcReleaseDevice(self.device_handle);
        }
    }
}

impl BvhTrait for BvhEmbree {
    fn num_primitives(&self) -> usize {
        self.hittables.len()
    }

    fn try_hit(&self, ray: &crate::math::ray::Ray, t_interval: &crate::math::util::Interval) -> Option<HitRecord> {
        let queryresult = self.try_intersect(ray, t_interval);

        if let Some((idx, t_hit)) = queryresult.hit {
            Some(self.hittables[idx].compute_hit(ray, t_hit))
        } else {
            None
        }
    }

    fn try_intersect(&self, ray: &crate::math::ray::Ray, t_interval: &crate::math::util::Interval) -> QueryResult {
        unsafe {
            let ray_init: RTCRay = RTCRay { 
                org_x: ray.orig.x, 
                org_y: ray.orig.y, 
                org_z: ray.orig.z, 
                tnear: *t_interval.start(), 
                dir_x: ray.dir.x, 
                dir_y: ray.dir.y, 
                dir_z: ray.dir.z, 
                time: 0.0,  // Time is ignored 
                tfar: *t_interval.end(), 
                mask: 0xFFFFFFFF, 
                id: 0, 
                flags: 0, 
            };
            let hit_init: RTCHit = RTCHit {
                Ng_x: 0.0,
                Ng_y: 0.0,
                Ng_z: 0.0,
                u: 0.0,
                v: 0.0,
                primID: 0,
                geomID: RTC_INVALID_GEOMETRY_ID,
                instID: [0] 
            };
            let mut ray_hit: RTCRayHit = RTCRayHit { ray: ray_init, hit: hit_init };

            // Passing NULL here would mean "RTC_FEATURE_FLAG_ALL"; the scene only holds
            // user geometry, so restricting the feature mask lets Embree skip traversal
            // paths for primitive types that cannot occur.
            let mut intersect_args = RTCIntersectArguments {
                flags: RTCRayQueryFlags::INCOHERENT,
                feature_mask: RTCFeatureFlags::RTC_FEATURE_FLAG_USER_GEOMETRY,
                context: null_mut(),
                filter: None,
                intersect: None,
            };

            rtcIntersect1(self.scene_handle, std::ptr::from_mut(&mut ray_hit), &mut intersect_args);

            // See if something was hit. If yes, embree writes the t of the hit to ray.tfar
            if ray_hit.hit.geomID != RTC_INVALID_GEOMETRY_ID {
                QueryResult { 
                    hit: Some((ray_hit.hit.primID as usize, ray_hit.ray.tfar)),
                    num_aabb_checks: 0,
                    num_aabb_hits: 0,
                    num_primitive_checks: 0
                }
            } else {
                QueryResult { 
                    hit: None,
                    num_aabb_checks: 0,
                    num_aabb_hits: 0,
                    num_primitive_checks: 0
                }
            }
        }
    }
}