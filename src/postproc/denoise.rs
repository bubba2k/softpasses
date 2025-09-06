use crate::{
    math::vector::{Color, Float},
    tracer::texture::Texture,
};

fn _denoise(
    image: &Vec<Color>,
    albedo: Option<&Vec<Color>>,
    normals: Option<&Vec<Color>>,
    image_width: usize,
    image_height: usize,
) -> Vec<Color> {
    let noisy_image: Vec<Float> = image.iter().map(|c| [c[0], c[1], c[2]]).flatten().collect();
    let mut denoised_image = vec![f32::default(); image_width * image_height * 3];
    let denoise_device = oidn::Device::new();

    match (albedo, normals) {
        (None, _) => {
            oidn::RayTracing::new(&denoise_device)
                .srgb(false)
                .image_dimensions(image_width as usize, image_height as usize)
                .filter(&noisy_image, &mut denoised_image)
                .expect("Denoise filter config error.");
        }
        (Some(albedo), None) => {
            let albedo_flattened: Vec<f32> = albedo
                .iter()
                .map(|c| [c[0], c[1], c[2]])
                .flatten()
                .collect();

            // Prefilter the albedo pass
            let mut albedo_denoised: Vec<f32> =
                vec![f32::default(); (image_width * image_height * 3) as usize];

            // Prefilter the albedo and normal passes
            oidn::RayTracing::new(&denoise_device)
                .srgb(false)
                .image_dimensions(image_width as usize, image_height as usize)
                .filter(&albedo_flattened, &mut albedo_denoised)
                .expect("Denoise filter config error.");

            oidn::RayTracing::new(&denoise_device)
                .srgb(false)
                .image_dimensions(image_width as usize, image_height as usize)
                .albedo(&albedo_denoised)
                .filter(&noisy_image, &mut denoised_image)
                .expect("Denoise filter config error.");
        }
        (Some(albedo), Some(normal)) => {
            let albedo_flattened: Vec<f32> = albedo
                .iter()
                .map(|c| [c[0], c[1], c[2]])
                .flatten()
                .collect();
            let normals_flattened: Vec<f32> = normal
                .iter()
                .map(|c| [c[0], c[1], c[2]])
                .flatten()
                .collect();

            let mut albedo_denoised: Vec<f32> =
                vec![f32::default(); (image_width * image_height * 3) as usize];
            let mut normal_denoised: Vec<f32> =
                vec![f32::default(); (image_width * image_height * 3) as usize];

            // Prefilter the albedo and normal passes
            oidn::RayTracing::new(&denoise_device)
                .srgb(false)
                .hdr(true)
                .image_dimensions(image_width as usize, image_height as usize)
                .filter(&albedo_flattened, &mut albedo_denoised)
                .expect("Denoise filter config error.");

            oidn::RayTracing::new(&denoise_device)
                .srgb(false)
                .hdr(true)
                .image_dimensions(image_width as usize, image_height as usize)
                .filter(&normals_flattened, &mut normal_denoised)
                .expect("Denoise filter config error.");

            oidn::RayTracing::new(&denoise_device)
                .srgb(false)
                .hdr(true)
                // .clean_aux(true) // TODO: Ideally, this should be enabled, but we should do some further testing to evaluate whether
                // our aux passes are clean _enough_ as they currently are
                .image_dimensions(image_width as usize, image_height as usize)
                .albedo_normal(&albedo_denoised, &normal_denoised)
                .filter(&noisy_image, &mut denoised_image)
                .expect("Denoise filter config error.");
        }
    }

    if let Err(e) = denoise_device.get_error() {
        eprintln!("Error denoising image: {}", e.1);
    }

    denoised_image
        .chunks(3)
        .map(|c| Color::new(c[0], c[1], c[2]))
        .collect()
}

pub fn denoise(image: &Vec<Color>, image_width: usize, image_height: usize) -> Vec<Color> {
    _denoise(image, None, None, image_width, image_height)
}

pub fn denoise_with_albedo_normal(
    image: &Texture,
    albedo: &Option<Texture>,
    normal: &Option<Texture>,
) -> Texture {
    let albedo_data = albedo.as_ref().map_or(None, |t: &Texture| Some(&t.data));
    let normal_data = normal.as_ref().map(|t| &t.data);
    let denoised_data = _denoise(
        &image.data,
        albedo_data,
        normal_data,
        image.width,
        image.height,
    );

    Texture::from_raw(image.width, image.height, denoised_data)
}
