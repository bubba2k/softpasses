use crate::math::vector::{Color, Float, Vec3f};

// Hable's 2010 Filmic curve (http://filmicworlds.com/blog/filmic-tonemapping-operators/)
// Cheaper and easier to implement than say, ACES and likewise
fn _filmic(input: Float) -> Float {
    let v = Float::max(0.0, input - 0.004);
    (v * (6.2 * v + 0.5)) / (v * (6.2 * v + 1.7) + 0.06)
}

pub fn filmic(input: &Vec3f) -> Vec3f {
    input.map(_filmic)
}

// A simplemost linear heatmap function
pub fn heatmap(input: Float, limit: Float) -> Color {
    const COLOR_MIN: Color = Vec3f::ZERO;
    const COLOR_MAX: Color = Vec3f::ONE;

    let t = input / limit;

    COLOR_MIN.lerp(COLOR_MAX, t)
}

pub fn gamma_correct(input: &Color, gamma: Float) -> Color {
    input.map(|c| Float::powf(c, 1.0 / gamma))
}
