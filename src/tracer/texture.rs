use std::path::Path;

use glam::Vec3;

use crate::math::vector::{Color, Float};

// Does not support transparency
#[derive(Clone)]
pub struct Texture {
    pub data: Vec<Color>,
    pub height: usize,
    pub width: usize,
}

impl Texture {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        crate::io::image::load_path(path).map_err(|e| e.to_string())
    }

    pub fn from_raw(width: usize, height: usize, data: Vec<Vec3>) -> Self {
        Texture {
            width,
            height,
            data,
        }
    }

    pub fn write_32f(&self, path: &Path) -> Result<(), String> {
        let mut img = image::Rgb32FImage::new(self.width as u32, self.height as u32);

        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.data[y * self.width + x];
                img.put_pixel(x as u32, y as u32, image::Rgb([color.x, color.y, color.z]));
            }
        }

        img.save(path).map_err(|e| e.to_string())
    }

    pub fn write_u8(&self, path: &Path) -> Result<(), String> {
        let mut img = image::RgbImage::new(self.width as u32, self.height as u32);

        for y in 0..self.height {
            for x in 0..self.width {
                let color = self.data[y * self.width + x];
                let r = (color.x.clamp(0.0, 1.0) * 255.0).round() as u8;
                let g = (color.y.clamp(0.0, 1.0) * 255.0).round() as u8;
                let b = (color.z.clamp(0.0, 1.0) * 255.0).round() as u8;
                img.put_pixel(x as u32, y as u32, image::Rgb([r, g, b]));
            }
        }

        img.save(path).map_err(|e| e.to_string())
    }

    // Get the pixel at uv, simply truncating to the top-left-most pixel.
    pub fn query_uv_trunc(&self, u: Float, v: Float) -> Color {
        let u = u.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);

        let y = (v * (self.height - 1) as Float) as usize;
        let x = (u * (self.width - 1) as Float) as usize;

        self.data[y * self.width + x]
    }

    pub fn query_uv_bilinear(&self, u: Float, v: Float) -> Color {
        let u = u.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);

        // x_ceil and y_ceil could theoretically lead to *very* rare
        // out of bound errors due to floating point inprecision.
        // We choose not to clamp them here, because those errors would be
        // very very rare, and we rather have some more performance.
        let y_float = v * (self.height - 1) as Float;
        let x_float = u * (self.width - 1) as Float;
        let y_floor = y_float.floor() as usize;
        let y_ceil = y_float.ceil() as usize;
        let x_floor = x_float.floor() as usize;
        let x_ceil = x_float.ceil() as usize;

        let y_t = y_float - y_floor as Float;
        let x_t = x_float - x_floor as Float;

        // Interpolate on y axis first, then on x axis.
        let color_floor_left = self.data[y_floor * self.width + x_floor];
        let color_ceil_left = self.data[y_ceil * self.width + x_floor];
        let color_floor_right = self.data[y_floor * self.width + x_ceil];
        let color_ceil_right = self.data[y_ceil * self.width + x_ceil];

        let color_left = color_floor_left.lerp(color_ceil_left, y_t);
        let color_right = color_floor_right.lerp(color_ceil_right, y_t);

        color_left.lerp(color_right, x_t)
    }
}
