use crate::math::vector::Pixel;

pub fn ppm_header(width: u32, height: u32) -> String {
    format!("P3\n{} {}\n255\n", width, height)
}

pub fn ppm_pixel(p: &Pixel) -> String {
    format!("{} {} {}\n", p.x, p.y, p.z)
}

pub fn ppm_image(width: u32, height: u32, pixels: &[Pixel], comment: String) -> String {
    let mut image = ppm_header(width, height);
    image.push_str(&make_comment(comment));
    image.push_str("\n");
    for pixel in pixels {
        image.push_str(&ppm_pixel(pixel));
    }
    image
}

// Make sure the given string is a proper ppm comment. (Prefix all line beginnings with `#`)
fn make_comment(str: String) -> String {
    String::from("# ") + &str.replace("\n", "\n# ")
}