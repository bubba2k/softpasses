mod denoise;

use std::collections::HashMap;

use crate::tracer::{render::RenderResult, texture::Texture};

pub struct PostProcessSettings {
    pub denoise: bool,
}

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

    PostProcessResult {
        images: HashMap::default(),
    }
}
