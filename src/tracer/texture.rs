use std::{ops::Div, path::Path};

use load_image::load_data;

use crate::math::vector::{Color, Float};

// Does not support transparency
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
    pub fn query_uv(&self, u: Float, v: Float) -> Color {
        let y = (v * self.height as Float).floor() as usize;
        let x = (u * self.width as Float).floor() as usize;

        self.data[y * self.height + x]
    }
}