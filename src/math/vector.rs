// Set some type aliases here...
// We do this so we can easily switch between Double and Float,
// and glams handy Vec_A types aligned for SSE2
// Note that Float and Vec3f components must have same precision.
pub type Float = f32;
pub type Vec3f = glam::Vec3;

pub type Color = Vec3f;
pub type Pixel = glam::U8Vec3;

pub const PLANE_XY: Vec3f = Vec3f::new(0.0, 0.0, 1.0);
pub const PLANE_XZ: Vec3f = Vec3f::new(0.0, 1.0, 0.0);
pub const PLANE_YZ: Vec3f = Vec3f::new(1.0, 0.0, 0.0);

pub fn dvec3_to_vec3(v: glam::DVec3) -> Vec3f {
    Vec3f::new(v.x as Float, v.y as Float, v.z as Float)
}

pub fn vec3(x: Float, y: Float, z: Float) -> Vec3f {
    Vec3f::new(x, y, z)
}

pub fn project_onto_plane_normalized(v: Vec3f, plane_normal: Vec3f) -> Vec3f {
    v - v.project_onto_normalized(plane_normal)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CoordinatePlane {
    XY,
    XZ,
    YZ,
}

impl CoordinatePlane {
    pub const fn normal(&self) -> Vec3f {
        match self {
            Self::XY => PLANE_XY,
            Self::XZ => PLANE_XZ,
            Self::YZ => PLANE_YZ,
        }
    }
}

pub fn point_left_of_edge(point: &Vec3f, q: &Vec3f, r: &Vec3f) -> bool {
    let p = point;

    let det = p.x * q.y + p.y * r.x + q.x * r.y - r.x * q.y - r.y * p.x - q.x * p.y;

    det > 0.0
}
