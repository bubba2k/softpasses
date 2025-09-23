mod denoise;

use crate::tracer::{render::RenderResult, texture::Texture};

pub struct PostProcessSettings {
    pub denoise: bool,
}

pub struct PostProcessResult {
    denoised_image: Option<Texture>,
    beauty_image: Option<Texture>,
}

impl PostProcessResult {
    pub fn write_images(&self, output_dir: &std::path::Path) -> Result<(), String> {
        for pass in [
            (&self.denoised_image, "denoised"),
            // TODO: Remove this clone call
            (&self.beauty_image.clone(), "beauty"),
        ] {
            if let (Some(pass_texture), name) = pass {
                let file_name = String::from(name) + ".png";
                let mut full_path = std::path::PathBuf::from(output_dir);
                full_path.push(file_name);
                pass_texture.write_u8(full_path.as_path())?;
            }
        }

        Ok(())
    }
}

pub fn postprocess(
    render_result: &RenderResult,
    settings: &PostProcessSettings,
) -> PostProcessResult {
    eprintln!("Postprocessing...");
    /* let denoised_pass = if settings.denoise {
        Some(denoise::denoise_with_albedo_normal(
            &render_result.color_pass.as_ref().unwrap(),
            &render_result.albedo_pass,
            &render_result.normal_pass,
        ))
    } else {
        None
    };

    let beauty_image = if settings.denoise {
        denoised_pass.as_ref().unwrap().clone()
    } else {
        render_result.color_pass.as_ref().unwrap().clone()
    };
    */
    PostProcessResult {
        denoised_image: None,
        beauty_image: None,
    }
}
