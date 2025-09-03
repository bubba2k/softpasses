use crate::math::vector::Pixel;
use std::fs::File;
use std::io::Write;

fn ppm_header(width: u32, height: u32) -> String {
    format!("P3\n{} {}\n255\n", width, height)
}

fn ppm_pixel(p: &Pixel) -> String {
    format!("{} {} {}\n", p.x, p.y, p.z)
}

// Make sure the given string is a proper ppm comment. (Prefix all line beginnings with `#`)
fn make_comment(str: String) -> String {
    String::from("# ") + &str.replace("\n", "\n# ")
}

pub fn ppm_image_string(width: u32, height: u32, pixels: &[Pixel], comment: String) -> String {
    let mut image = ppm_header(width, height);
    image.push_str(&make_comment(comment));
    image.push_str("\n");
    for pixel in pixels {
        image.push_str(&ppm_pixel(pixel));
    }
    image
}

pub fn write_ppm_image(path: &std::path::Path, width: u32, height: u32, pixels: &[Pixel], comment: String) -> Result<(), std::io::Error> {
    let ppm_string = ppm_image_string(width, height, pixels, comment);

    let mut file = File::create(path)?;
    file.write_all(ppm_string.as_bytes())?;

    Ok(())
}