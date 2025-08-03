use std::{ops::Div, path::Path};

use load_image::load_data;

use crate::math::vector::{Color, Float};

// Does not support transparency
#[derive(Clone)]
pub struct Texture {
    data: Vec<Color>,
    height: usize,
    width: usize,
}

impl Texture {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        if let Ok(image) = load_image::load_path(path) {
            // Convert image data to float format
            let image_rgba = image.into_rgba();
            let data: Vec<Color> = 
                image_rgba.0.pixels().map(|pix| {
                    Color::new( pix.r as Float / 255.0,
                                pix.g as Float / 255.0,
                                pix.b as Float / 255.0)
                }).collect();
            
            Ok(Texture { data: data, height: image_rgba.0.height(), width: image_rgba.0.width() })
        } else {
            Err(format!("Could not load texture from '{}'.", path.to_str().unwrap()))
        }
    }

    pub fn from_data(data: &[u8]) -> Result<Self, String> {
        if let Ok(image) = load_data(data) {
            // Convert image data to float format
            let image_rgba = image.into_rgba();
            let data: Vec<Color> = 
                image_rgba.0.pixels().map(|pix| {
                    Color::new( pix.r as Float / 255.0,
                                pix.g as Float / 255.0,
                                pix.b as Float / 255.0)
                }).collect();
            
            Ok(Texture { data: data, height: image_rgba.0.height(), width: image_rgba.0.width() })
        } else {
            Err(String::from("Could not load texture from data."))
        }
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
        let x_float = u * (self.width - 1)  as Float;
        let y_floor = y_float.floor() as usize;
        let y_ceil  = y_float.ceil()  as usize;
        let x_floor = x_float.floor() as usize;
        let x_ceil  = x_float.ceil()  as usize;

        let y_t = y_float - y_floor as Float;
        let x_t = x_float - x_floor as Float;

        // Interpolate on y axis first, then on x axis.
        let color_floor_left = self.data[y_floor * self.width + x_floor];
        let color_ceil_left  = self.data[y_ceil * self.width + x_floor];
        let color_floor_right = self.data[y_floor * self.width + x_ceil];
        let color_ceil_right  = self.data[y_ceil * self.width + x_ceil];

        let color_left = color_floor_left.lerp(color_ceil_left, y_t);
        let color_right = color_floor_right.lerp(color_ceil_right, y_t);

        color_left.lerp(color_right, x_t)

    }
}