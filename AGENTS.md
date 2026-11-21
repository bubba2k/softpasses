# Project description

This project is a physically based software pathtracer written in Rust.

## Frequent actions

- `./run.sh <output_directory>` - Build and run the renderer in release mode, saving all generated images to `output_directory`. Per convention, these should be saved to a new directory inside the (gitignored) `./images/` directory, using a descriptive name, eg `./run.sh ./images/dragonscene_2000spp_old` etc.

- `cargo run --release` - start a dry run (any visual output is dropped and never saved) in release mode.
- `cargo run` - start a dry run (any visual output is dropped and never saved) in debug mode.

## Directory structure

 - `asssets/` - Assets to be used in test scenes.
 - `src/` - Source code
 - `renders` - Example renders
 - `images` - (gitignored) Temporary output for debugging purposes

 ## Modules

 ### bvh
 
 Contains the implementation of the BVH acceleration structure.

 ### io

 Functionalities for loading and storing assets from/to disk.

 ### postproc

 Postprocessing functionalities. Currently implements simple tonemapping and a bridge to Intel Open Image Denoise.

 ### math

 Commonly used utility functions and definions, such as `Ray` and `Vec3f`.

 ### tracer

 The heart of the renderer.

 #### camera.rs

 Implementation of the camera. Uses the thin lens model and supports depth-of-field effect.

 #### hittable.rs

 Hittables are all those objects which are placable in the scene and can be hit by a ray. 
 The file defines the Hittable trait, a corresponding enum and any variants (such as triangles, or sphers) in it.
 A hittable can be thought of as a primitive.

 #### material.rs

 Implements the material trait, a corresponding enum and all its variants. 
 A material defines how light reacts (bounces/refracts/attenuates its color) when hitting a surface.

 #### pipelines.rs

 A pipeline is a collection 1 or more render passes. A render pass is eg the Normal pass, Albedo pass, and color pass.

 #### render.rs

 Defines the main entry point and hot loop for rendering out a pipeline on a scene.

 #### scene.rs

 3D scene description and construction.

 #### texture.rs

 Support for two dimensional textures.

