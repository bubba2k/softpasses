use crate::math::ray::Ray;
use crate::math::vector::{Float, Vec3f, project_onto_plane_normalized};

#[derive(Clone)]
// A viewport describes the focus plane of a camera.
pub struct Viewport {
    pub viewdown: Vec3f,
    pub viewright: Vec3f,
    pub topleft: Vec3f,
    pub width: Float,
    pub height: Float,
}

impl Viewport {
    pub fn new(center: Vec3f, dir: Vec3f, up: Vec3f, height: Float, aspect_ratio: Float) -> Self {
        let width = height * aspect_ratio;

        let down_dir_norm = -up.normalize();
        let right_dir_norm = dir.cross(up.normalize()).normalize();

        let top_left = center + (-down_dir_norm * height * 0.5) + (-right_dir_norm * width * 0.5);

        Viewport {
            viewdown: down_dir_norm,
            viewright: right_dir_norm,
            topleft: top_left,
            width: height * aspect_ratio,
            height: height,
        }
    }

    // Shoot a ray at the given uv of the viewport from a point origin (which is presumably the camera position).
    pub fn ray_at_uv(&self, u: Float, v: Float, origin: Vec3f) -> Ray {
        let target =
            self.topleft + self.viewright * u * self.width + self.viewdown * v * self.height;
        let dir = (target - origin).normalize();

        Ray::new(&origin, &dir)
    }
}

#[derive(Clone)]
pub struct Lens {
    pub afov_rad: Float,
    pub aspect_ratio: Float,

    // Distance from the camera to the focus plane
    pub focal_distance: Float,
    // Amount of depth of field
    pub dof: Float,
}

impl Lens {
    pub fn new(
        sensor_width_mm: u32,
        focal_length_mm: u32,
        aspect_ratio: Float,
        focal_distance: Float,
        dof: Float,
    ) -> Self {
        let sensor_width = sensor_width_mm as Float / 1000.0;
        let focal_length = focal_length_mm as Float / 1000.0;
        let sensor_height = sensor_width / aspect_ratio;
        let afov_rad = 2.0 * Float::atan(sensor_height / (2.0 * focal_length));

        Lens {
            afov_rad: afov_rad,
            aspect_ratio: aspect_ratio,

            focal_distance: focal_distance,
            dof: dof,
        }
    }
}

// Position and orientation of a camera
#[derive(Clone)]
pub struct Pose {
    pub position: Vec3f,
    pub direction: Vec3f,
    pub up: Vec3f,
}

#[allow(dead_code)]
impl Pose {
    const UP_DEFAULT: Vec3f = Vec3f::new(0.0, 1.0, 0.0);

    pub const fn new(position: Vec3f, direction: Vec3f) -> Self {
        Pose {
            position: position,
            direction: direction,
            up: Pose::UP_DEFAULT,
        }
    }

    pub fn look_at(position: Vec3f, target: Vec3f) -> Self {
        Pose {
            position: position,
            direction: (target - position).normalize(),
            up: Pose::UP_DEFAULT,
        }
    }
}

#[derive(Clone)]
pub struct Camera {
    pub lens: Lens,
    pub pose: Pose,
    pub viewport: Viewport,
}

impl Camera {
    pub fn new(pose: Pose, lens: Lens) -> Self {
        // Compute the viewport (the focus plane) of the camera
        let viewport_center = pose.position + pose.direction * lens.focal_distance;
        let dir = pose.direction;

        let height = 2.0 * lens.focal_distance * Float::tan(lens.afov_rad * 0.5);

        let aspect_ratio = lens.aspect_ratio;

        // Calculate the actual "up" vector for the viewport.
        let view_up = pose.up;

        // Project the view_up onto the viewport plane to get the actual viewport up
        let viewport_up = project_onto_plane_normalized(view_up, dir).normalize();

        let viewport = Viewport::new(viewport_center, dir, viewport_up, height, aspect_ratio);

        Camera {
            viewport: viewport,
            lens: lens,
            pose: pose,
        }
    }
}
