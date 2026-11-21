use std::ptr::null_mut;

use embree4_sys::*;

use crate::{
    bvh::{BvhTrait, QueryResult},
    math::{
        ray::Ray,
        util::Interval,
        vector::{Float, Vec3f},
    },
    tracer::{
        hittable::{HitRecord, Hittable, HittableTrait, Sphere, Triangle},
        material::MaterialID,
    },
};

/// Which native Embree geometry a slot represents.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GeomKind {
    Triangles,
    Spheres,
}

/// One native Embree geometry together with its global primitive base.
struct GeometrySlot {
    handle: RTCGeometry,
    /// The ID Embree assigned in `rtcAttachGeometry`; not assumed to be 0/1.
    geom_id: u32,
    /// Global index of this geometry's first primitive.
    base: usize,
    /// Cached pointer to the Embree-owned vertex buffer filled in `try_new`.
    ///
    /// `rtcSetNewGeometryBuffer` returns memory owned by the geometry, which
    /// stays valid until `rtcReleaseGeometry`. Caching it means a hit can read
    /// positions or sphere parameters without an `rtcGetGeometryBufferData`
    /// call per ray.
    vertex_data: *const f32,
}

struct NativeHit {
    kind: GeomKind,
    prim_id: usize,
    t: Float,
}

pub struct BvhEmbree {
    device_handle: RTCDevice,
    scene_handle: RTCScene,
    /// Fixed native geometry slots; absent if that primitive kind is unused.
    triangles: Option<GeometrySlot>,
    spheres: Option<GeometrySlot>,

    /// Material id per primitive, in triangle-first, sphere-second order.
    /// `RTCHit` carries no material, so this has to be kept on the Rust side.
    materials: Vec<MaterialID>,

    /// Three vertex normals per triangle, indexed directly by `primID` because
    /// the triangle slot has `base == 0`.
    ///
    /// `RTCHit` only exposes the geometric (face) normal `Ng`, and Embree never
    /// interpolates vertex normals, so the smooth normals must live here.
    tri_normals: Vec<[Vec3f; 3]>,
}

// After `rtcCommitScene` the handles and buffers are immutable and only read,
// so sharing the BVH between threads is fine.
unsafe impl Sync for BvhEmbree {}

/// Read three consecutive `f32` from an Embree-owned buffer.
unsafe fn read_vec3(ptr: *const f32, offset: usize) -> Vec3f {
    unsafe { Vec3f::new(*ptr.add(offset), *ptr.add(offset + 1), *ptr.add(offset + 2)) }
}

/// Release everything that was built before a failed `try_new`.
unsafe fn release_all(
    triangles: Option<&GeometrySlot>,
    spheres: Option<&GeometrySlot>,
    scene: RTCScene,
    device: RTCDevice,
) {
    unsafe {
        if let Some(slot) = triangles {
            rtcReleaseGeometry(slot.handle);
        }
        if let Some(slot) = spheres {
            rtcReleaseGeometry(slot.handle);
        }
        rtcReleaseScene(scene);
        rtcReleaseDevice(device);
    }
}

impl BvhEmbree {
    pub fn try_new(hittables: Vec<Hittable>) -> Option<Self> {
        // Native Embree geometries only exist for triangles and (point) spheres.
        // Plane is dropped: in debug this is loud, in release it is a no-op that
        // matches today's behaviour (nothing constructs a Plane any more).
        let mut triangles: Vec<Triangle> = Vec::new();
        let mut spheres: Vec<Sphere> = Vec::new();
        let mut tri_normals: Vec<[Vec3f; 3]> = Vec::new();

        for hittable in hittables {
            match hittable {
                Hittable::Triangle(triangle) => {
                    // Triangles are uploaded first, so this index equals primID.
                    tri_normals.push(triangle.normals);
                    triangles.push(triangle);
                }
                Hittable::Sphere(sphere) => spheres.push(sphere),
                Hittable::Plane(_) => unimplemented!(
                    "BvhEmbree cannot represent Plane: Embree has no native geometry for it"
                ),
            }
        }

        let tri_count = triangles.len();
        let sphere_count = spheres.len();

        // Must follow the global primitive order (triangles first, then spheres),
        // which is not necessarily the order the hittables arrived in.
        let materials: Vec<MaterialID> = triangles
            .iter()
            .map(|triangle| triangle.material_id)
            .chain(spheres.iter().map(|sphere| sphere.material_id))
            .collect();

        unsafe {
            let device = rtcNewDevice(c"-verbose".as_ptr() as *const _ as _);
            if device.is_null() {
                return None;
            }

            // The bindings embed the vendored header ABI (which decides, for
            // example, whether `RTCHit` contains `instPrimID`). A runtime with a
            // different version would silently write past our structs again, so
            // fail loudly rather than risk ABI corruption.
            if rtcGetDeviceProperty(device, RTCDeviceProperty::VERSION) != RTC_VERSION as isize {
                rtcReleaseDevice(device);
                unimplemented!(
                    "Embree headers and runtime disagree; rebuild the bindings from matching headers"
                );
            }

            let scene = rtcNewScene(device);
            if scene.is_null() {
                rtcReleaseDevice(device);
                return None;
            }
            // An interesting option to explore might be `RTCSceneFlags::ROBUST` (more accurate scene traversal)
            rtcSetSceneFlags(scene, RTCSceneFlags::NONE);

            let mut triangles_slot = None;
            let mut spheres_slot = None;

            if tri_count > 0 {
                match Self::build_triangle_slot(device, scene, &triangles, 0) {
                    Some(slot) => triangles_slot = Some(slot),
                    None => {
                        release_all(triangles_slot.as_ref(), None, scene, device);
                        return None;
                    }
                }
            }

            if sphere_count > 0 {
                match Self::build_sphere_slot(device, scene, &spheres, tri_count) {
                    Some(slot) => spheres_slot = Some(slot),
                    None => {
                        release_all(triangles_slot.as_ref(), spheres_slot.as_ref(), scene, device);
                        return None;
                    }
                }
            }

            rtcCommitScene(scene);

            Some(Self {
                device_handle: device,
                scene_handle: scene,
                triangles: triangles_slot,
                spheres: spheres_slot,
                materials,
                tri_normals,
            })
        }
    }

    /// Build and attach an `RTC_GEOMETRY_TYPE_TRIANGLE` geometry.
    ///
    /// Mesh data arrives with one index per vertex (`single_index`), so each
    /// triangle gets its own three vertices and an identity index buffer.
    unsafe fn build_triangle_slot(
        device: RTCDevice,
        scene: RTCScene,
        triangles: &[Triangle],
        base: usize,
    ) -> Option<GeometrySlot> {
        unsafe {
            let count = triangles.len();
            let handle = rtcNewGeometry(device, RTCGeometryType::TRIANGLE);
            if handle.is_null() {
                return None;
            }

            let vertices = rtcSetNewGeometryBuffer(
                handle,
                RTCBufferType::VERTEX,
                0,
                RTCFormat::FLOAT3,
                3 * std::mem::size_of::<Float>(),
                count * 3,
            ) as *mut f32;
            let indices = rtcSetNewGeometryBuffer(
                handle,
                RTCBufferType::INDEX,
                0,
                RTCFormat::UINT3,
                3 * std::mem::size_of::<u32>(),
                count,
            ) as *mut u32;

            if vertices.is_null() || indices.is_null() {
                rtcReleaseGeometry(handle);
                return None;
            }

            for (i, triangle) in triangles.iter().enumerate() {
                for k in 0..3 {
                    let position = triangle.positions[k];
                    let offset = (i * 3 + k) * 3;
                    vertices.add(offset).write(position.x);
                    vertices.add(offset + 1).write(position.y);
                    vertices.add(offset + 2).write(position.z);
                }
                indices.add(i * 3).write((i * 3) as u32);
                indices.add(i * 3 + 1).write((i * 3 + 1) as u32);
                indices.add(i * 3 + 2).write((i * 3 + 2) as u32);
            }

            rtcCommitGeometry(handle);

            let geom_id = rtcAttachGeometry(scene, handle);
            if geom_id == RTC_INVALID_GEOMETRY_ID {
                rtcReleaseGeometry(handle);
                return None;
            }

            Some(GeometrySlot {
                handle,
                geom_id,
                base,
                vertex_data: vertices,
            })
        }
    }

    /// Build and attach an `RTC_GEOMETRY_TYPE_SPHERE_POINT` geometry.
    ///
    /// Each vertex is a single-precision `(x, y, z, radius)` tuple.
    unsafe fn build_sphere_slot(
        device: RTCDevice,
        scene: RTCScene,
        spheres: &[Sphere],
        base: usize,
    ) -> Option<GeometrySlot> {
        unsafe {
            let count = spheres.len();
            let handle = rtcNewGeometry(device, RTCGeometryType::SPHERE_POINT);
            if handle.is_null() {
                return None;
            }

            let vertices = rtcSetNewGeometryBuffer(
                handle,
                RTCBufferType::VERTEX,
                0,
                RTCFormat::FLOAT4,
                4 * std::mem::size_of::<Float>(),
                count,
            ) as *mut f32;

            if vertices.is_null() {
                rtcReleaseGeometry(handle);
                return None;
            }

            for (i, sphere) in spheres.iter().enumerate() {
                vertices.add(i * 4).write(sphere.center.x);
                vertices.add(i * 4 + 1).write(sphere.center.y);
                vertices.add(i * 4 + 2).write(sphere.center.z);
                vertices.add(i * 4 + 3).write(sphere.radius);
            }

            rtcCommitGeometry(handle);

            let geom_id = rtcAttachGeometry(scene, handle);
            if geom_id == RTC_INVALID_GEOMETRY_ID {
                rtcReleaseGeometry(handle);
                return None;
            }

            Some(GeometrySlot {
                handle,
                geom_id,
                base,
                vertex_data: vertices,
            })
        }
    }

    /// Rebuild a primitive from Embree's vertex buffer plus the side tables and
    /// hand it to the shared `Hittable::compute_hit`.
    ///
    /// This keeps exactly one implementation of normal interpolation and
    /// front-face handling; the small per-hit copy is negligible next to shading.
    fn reconstruct_primitive(
        &self,
        slot: &GeometrySlot,
        prim_id: usize,
        geom_kind: GeomKind,
        material_id: MaterialID,
    ) -> Hittable {
        unsafe {
            match geom_kind {
                GeomKind::Triangles => Hittable::Triangle(Triangle {
                    positions: [
                        read_vec3(slot.vertex_data, prim_id * 9),
                        read_vec3(slot.vertex_data, prim_id * 9 + 3),
                        read_vec3(slot.vertex_data, prim_id * 9 + 6),
                    ],
                    normals: self.tri_normals[prim_id],
                    material_id,
                }),
                GeomKind::Spheres => {
                    let center = read_vec3(slot.vertex_data, prim_id * 4);
                    let radius = *slot.vertex_data.add(prim_id * 4 + 3);
                    Hittable::Sphere(Sphere {
                        center,
                        radius,
                        material_id,
                    })
                }
            }
        }
    }

    fn native_intersect(&self, ray: &Ray, t_interval: &Interval) -> Option<NativeHit> {
        unsafe {
            let ray_init = RTCRay {
                org_x: ray.orig.x,
                org_y: ray.orig.y,
                org_z: ray.orig.z,
                tnear: *t_interval.start(),
                dir_x: ray.dir.x,
                dir_y: ray.dir.y,
                dir_z: ray.dir.z,
                time: 0.0,
                tfar: *t_interval.end(),
                mask: 0xFFFFFFFF,
                id: 0,
                flags: 0,
            };
            let hit_init = RTCHit {
                Ng_x: 0.0,
                Ng_y: 0.0,
                Ng_z: 0.0,
                u: 0.0,
                v: 0.0,
                primID: 0,
                geomID: RTC_INVALID_GEOMETRY_ID,
                instID: [RTC_INVALID_GEOMETRY_ID; RTC_MAX_INSTANCE_LEVEL_COUNT as usize],
                instPrimID: [RTC_INVALID_GEOMETRY_ID; RTC_MAX_INSTANCE_LEVEL_COUNT as usize],
            };
            let mut ray_hit = RTCRayHit {
                ray: ray_init,
                hit: hit_init,
            };
            let mut intersect_args = RTCIntersectArguments {
                flags: RTCRayQueryFlags::INCOHERENT,
                feature_mask: RTCFeatureFlags::RTC_FEATURE_FLAG_TRIANGLE
                    | RTCFeatureFlags::RTC_FEATURE_FLAG_SPHERE_POINT,
                context: null_mut(),
                filter: None,
                intersect: None,
            };

            rtcIntersect1(
                self.scene_handle,
                std::ptr::from_mut(&mut ray_hit),
                &mut intersect_args,
            );

            if ray_hit.hit.geomID == RTC_INVALID_GEOMETRY_ID {
                return None;
            }

            let kind = if self
                .triangles
                .as_ref()
                .is_some_and(|slot| slot.geom_id == ray_hit.hit.geomID)
            {
                GeomKind::Triangles
            } else if self
                .spheres
                .as_ref()
                .is_some_and(|slot| slot.geom_id == ray_hit.hit.geomID)
            {
                GeomKind::Spheres
            } else {
                return None;
            };

            Some(NativeHit {
                kind,
                prim_id: ray_hit.hit.primID as usize,
                t: ray_hit.ray.tfar,
            })
        }
    }
}

impl Drop for BvhEmbree {
    fn drop(&mut self) {
        unsafe {
            // The scene keeps its own references, but these are the ones we own.
            if let Some(slot) = &self.triangles {
                rtcReleaseGeometry(slot.handle);
            }
            if let Some(slot) = &self.spheres {
                rtcReleaseGeometry(slot.handle);
            }
            rtcReleaseScene(self.scene_handle);
            rtcReleaseDevice(self.device_handle);
        }
    }
}

impl BvhTrait for BvhEmbree {
    fn num_primitives(&self) -> usize {
        self.materials.len()
    }

    fn try_hit(&self, ray: &Ray, t_interval: &Interval) -> Option<HitRecord> {
        let native_hit = self.native_intersect(ray, t_interval)?;
        let slot = match native_hit.kind {
            GeomKind::Triangles => self.triangles.as_ref()?,
            GeomKind::Spheres => self.spheres.as_ref()?,
        };
        let index = slot.base + native_hit.prim_id;
        let material_id = self.materials[index];

        Some(
            self.reconstruct_primitive(slot, native_hit.prim_id,native_hit.kind, material_id)
                .compute_hit(ray, native_hit.t),
        )
    }

    fn try_intersect(&self, ray: &Ray, t_interval: &Interval) -> QueryResult {
        let hit = self.native_intersect(ray, t_interval).and_then(|native_hit| {
            let slot = match native_hit.kind {
                GeomKind::Triangles => self.triangles.as_ref()?,
                GeomKind::Spheres => self.spheres.as_ref()?,
            };
            Some((slot.base + native_hit.prim_id, native_hit.t))
        });

        QueryResult {
            hit,
            num_aabb_checks: 0,
            num_aabb_hits: 0,
            num_primitive_checks: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{util::Interval, vector::vec3};

    fn test_ray(origin: Vec3f, direction: Vec3f) -> Ray {
        Ray::new(&origin, &direction)
    }

    fn limits() -> Interval {
        Interval::new(0.001, 1000.0)
    }

    /// Unit triangle in the XY plane with per-vertex normals that make smooth
    /// interpolation observable: a hit at (0.25, 0.25) must not yield the face
    /// normal (0, 0, 1).
    fn triangle(material_id: MaterialID) -> Triangle {
        Triangle {
            positions: [
                vec3(0.0, 0.0, 0.0),
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
            ],
            normals: [
                vec3(0.0, 0.0, 1.0),
                vec3(1.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
            ],
            material_id,
        }
    }

    fn sphere(material_id: MaterialID) -> Sphere {
        Sphere {
            center: vec3(0.0, 0.0, 0.0),
            radius: 1.0,
            material_id,
        }
    }

    #[test]
    fn empty_scene_reports_no_primitives_and_misses() {
        let bvh = BvhEmbree::try_new(vec![]).unwrap();
        assert_eq!(bvh.num_primitives(), 0);
        let hit = bvh.try_hit(
            &test_ray(vec3(0.0, 0.0, 1.0), vec3(0.0, 0.0, -1.0)),
            &limits(),
        );
        assert!(hit.is_none());
    }

    #[test]
    fn triangle_hit_keeps_material_and_interpolates_normals() {
        let bvh = BvhEmbree::try_new(vec![Hittable::Triangle(triangle(7))]).unwrap();
        assert_eq!(bvh.num_primitives(), 1);

        let hit = bvh
            .try_hit(
                &test_ray(vec3(0.25, 0.25, 1.0), vec3(0.0, 0.0, -1.0)),
                &limits(),
            )
            .expect("ray should hit the triangle");

        assert_eq!(hit.material_id, 7);
        assert!((hit.t - 1.0).abs() < 1e-5, "unexpected t: {}", hit.t);

        // Barycentrics at (0.25, 0.25) are (0.5, 0.25, 0.25), so the smooth
        // normal is 0.5*n0 + 0.25*n1 + 0.25*n2.
        let expected = vec3(0.25, 0.25, 0.5).normalize();
        assert!(
            hit.normal.dot(expected) > 0.9999,
            "unexpected normal: {:?}",
            hit.normal
        );
    }

    #[test]
    fn sphere_hit_keeps_material_and_normal() {
        let bvh = BvhEmbree::try_new(vec![Hittable::Sphere(sphere(3))]).unwrap();
        assert_eq!(bvh.num_primitives(), 1);

        let hit = bvh
            .try_hit(
                &test_ray(vec3(0.0, 0.0, 5.0), vec3(0.0, 0.0, -1.0)),
                &limits(),
            )
            .expect("ray should hit the sphere");

        assert_eq!(hit.material_id, 3);
        assert!((hit.t - 4.0).abs() < 1e-5, "unexpected t: {}", hit.t);
        assert!(hit.normal.dot(vec3(0.0, 0.0, 1.0)) > 0.9999);
    }

    #[test]
    fn mixed_input_maps_back_to_the_right_material() {
        // Spheres arrive first, but the BVH stores triangles first; this guards
        // the global index -> (geometry, primID) mapping and the material order.
        // The sphere is offset so it cannot shadow the triangle ray.
        let mut far_sphere = sphere(10);
        far_sphere.center = vec3(10.0, 0.0, 0.0);
        let bvh = BvhEmbree::try_new(vec![
            Hittable::Sphere(far_sphere),
            Hittable::Triangle(triangle(20)),
        ])
        .unwrap();
        assert_eq!(bvh.num_primitives(), 2);

        let triangle_hit = bvh
            .try_hit(
                &test_ray(vec3(0.25, 0.25, 1.0), vec3(0.0, 0.0, -1.0)),
                &limits(),
            )
            .expect("ray should hit the triangle");
        assert_eq!(triangle_hit.material_id, 20);

        let sphere_hit = bvh
            .try_hit(
                &test_ray(vec3(10.0, 0.0, 5.0), vec3(0.0, 0.0, -1.0)),
                &limits(),
            )
            .expect("ray should hit the sphere");
        assert_eq!(sphere_hit.material_id, 10);
    }

    #[test]
    fn ray_pointing_away_misses() {
        let bvh = BvhEmbree::try_new(vec![Hittable::Triangle(triangle(7))]).unwrap();
        let hit = bvh.try_hit(
            &test_ray(vec3(0.25, 0.25, 1.0), vec3(0.0, 0.0, 1.0)),
            &limits(),
        );
        assert!(hit.is_none());
    }
}
