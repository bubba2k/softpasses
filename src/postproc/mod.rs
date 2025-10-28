mod denoise;
mod tonemap;

use std::collections::HashMap;

use crate::{
    math::vector::{Color, Float},
    tracer::{render::RenderResult, texture::Texture},
};

pub struct PostProcessSettings {}

pub struct PostProcessResult {
    images: HashMap<String, Texture>,
}

impl PostProcessResult {
    pub fn write_images(&self, output_dir: &std::path::Path) -> Result<(), String> {
        for (name, texture) in self.images.iter() {
            let file_name = String::from(name) + ".png";
            let mut full_path = std::path::PathBuf::from(output_dir);
            full_path.push(file_name);
            texture.write_u8(full_path.as_path())?;
        }

        Ok(())
    }
}

pub fn postprocess(
    render_result: &RenderResult,
    settings: &PostProcessSettings,
) -> PostProcessResult {
    // TODO: Implement tonemapping, denoising, and so on.
    eprintln!("Postprocessing...");

    let mut image_map: HashMap<String, Texture> = HashMap::default();

    // Perform denoising (with aux inputs, if available)
    let denoised_pass = match (
        render_result.passes.get("color"),
        render_result.passes.get("albedo"),
        render_result.passes.get("normal"),
    ) {
        (None, _, _) => None,
        (Some(color_pass), albedo_opt, normal_opt) => {
            Some(denoise::denoise(color_pass, albedo_opt, normal_opt))
        }
    };

    // Perform tonemapping on denoise pass to receive the final image! :)
    if let Some(pass) = denoised_pass {
        let tonemapped_colors: Vec<Color> = pass
            .data
            .iter()
            .map(|color| {
                // TODO: Current tonemapping looks awful! Make adjustments to exposure,
                // choose a different tonemap algo, or similar.
                let mapped_color = tonemap::filmic(&(color));
                tonemap::gamma_correct(&mapped_color, 2.2)
            })
            .collect();

        let beauty_image = Texture::from_raw(pass.width, pass.height, tonemapped_colors);

        image_map.insert(String::from("beauty_tonemapped"), beauty_image);
    }

    // Make a nice heatmap for the BVH debugging stuff
    if let Some(pass) = render_result.passes.get("num_aabb_checks") {
        let max_value = pass
            .data
            .iter()
            .map(|v| v.x)
            .max_by(f32::total_cmp)
            .unwrap();
        let aabb_checks_heatmap = Texture::from_raw(
            pass.width,
            pass.height,
            pass.data
                .iter()
                .map(|color| tonemap::gamma_correct(&tonemap::heatmap(color.x, max_value), 2.2))
                .collect(),
        );

        image_map.insert(String::from("aabb_checks_heatmapped"), aabb_checks_heatmap);
    }

    if let Some(pass) = render_result.passes.get("num_primitve_checks") {
        let max_value = pass
            .data
            .iter()
            .map(|v| v.x)
            .max_by(f32::total_cmp)
            .unwrap();
        let primitive_checks_heatmap = Texture::from_raw(
            pass.width,
            pass.height,
            pass.data
                .iter()
                .map(|color| tonemap::gamma_correct(&tonemap::heatmap(color.x, max_value), 2.2))
                .collect(),
        );

        image_map.insert(
            String::from("primitive_checks_heatmapped"),
            primitive_checks_heatmap,
        );
    }

    PostProcessResult { images: image_map }
}
