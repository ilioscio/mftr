//! The world's noise texture for the shaders (05 §2, D81): `MftrNoise.world_image()` is
//! `mftr_client::noise`'s texture as a mipmapped Godot image, which the client makes the
//! `world_noise` shader global once at start (`main.gd`).

use godot::classes::Image;
use godot::classes::image::Format;
use godot::prelude::*;
use mftr_client::noise;

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrNoise {
    base: Base<RefCounted>,
}

#[godot_api]
impl MftrNoise {
    /// The noise as an R8 image, `noise::SIZE` square, with mipmaps.
    #[func]
    fn world_image() -> Gd<Image> {
        let bytes: PackedByteArray = noise::texture().into_iter().collect();
        let mut image = Image::create_from_data(noise::SIZE as i32, noise::SIZE as i32, false, Format::R8, &bytes)
            .expect("noise image");
        image.generate_mipmaps();
        image
    }

    /// Lattice cells across the texture before it repeats (the shaders' `W_NOISE_PERIOD`).
    #[func]
    fn period() -> i32 {
        noise::PERIOD as i32
    }
}
