use crate::tracer::texture::Texture;
use glam::vec3;
use image::{ImageError, ImageReader};

pub fn load_path(path: &std::path::Path) -> Result<Texture, ImageError> {
    let img = ImageReader::open(path)?.decode()?.to_rgb32f();

    let chunked = img
        .as_raw()
        .clone()
        .chunks(3)
        .map(|chunk| vec3(chunk[0], chunk[1], chunk[2]))
        .collect();

    Ok(Texture::from_raw(
        img.width() as usize,
        img.height() as usize,
        chunked,
    ))
}
