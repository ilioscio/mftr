//! The world's noise texture (05 §2, D81): the value noise the ground's and cliffs' shaders build
//! their materials from (`world.gdshaderinc`), baked once at load instead of hashed per pixel.
//!
//! Hashing four lattice corners for every octave of every noise in every pixel cost the ground
//! over 60 ms a frame on integrated graphics (integer multiplies run at a quarter of the float
//! rate there); one filtered fetch of this texture gives the same value. And a texture can be
//! mipmapped, so octaves finer than a pixel blur to their mean instead of shimmering as the
//! camera pans.
//!
//! The noise is the shader's own: the same PCG hash of the same lattice ([`lattice`] is
//! `w_hash` of a lattice point), smoothly interpolated the same way, sampled at the centers of
//! [`TEXELS_PER_CELL`]² texels a cell, so bilinear filtering between them reproduces the curve to
//! well under a 1/255 step. Only the period is new: it repeats every [`PERIOD`] cells (the shader
//! scales its coordinates by 1/`PERIOD`), and with each octave of the shader's fBm turned against
//! the last, the sum never visibly repeats.

/// Lattice cells across the texture before the noise repeats.
pub const PERIOD: usize = 256;
/// Texels per cell along each axis.
pub const TEXELS_PER_CELL: usize = 8;
/// The texture's width and height in texels.
pub const SIZE: usize = PERIOD * TEXELS_PER_CELL;

/// The shader's 2D PCG hash (`w_pcg2d`): u32 arithmetic wrapping as the GPU's does.
pub fn pcg2d(x: u32, y: u32) -> (u32, u32) {
    let mut x = x.wrapping_mul(1664525).wrapping_add(1013904223);
    let mut y = y.wrapping_mul(1664525).wrapping_add(1013904223);
    x = x.wrapping_add(y.wrapping_mul(1664525));
    y = y.wrapping_add(x.wrapping_mul(1664525));
    x ^= x >> 16;
    y ^= y >> 16;
    x = x.wrapping_add(y.wrapping_mul(1664525));
    y = y.wrapping_add(x.wrapping_mul(1664525));
    x ^= x >> 16;
    y ^= y >> 16;
    (x, y)
}

/// The value at lattice point (i, j), 0..=1: the shader's `w_hash(vec2(i, j))`, which hashes the
/// point's 1/16 grid cell offset by 2^20.
pub fn lattice(i: i32, j: i32) -> f32 {
    let u = (i * 16 + (1 << 20)) as u32;
    let v = (j * 16 + (1 << 20)) as u32;
    (pcg2d(u, v).0 >> 8) as f32 / 16_777_215.0
}

/// The texture: R8, [`SIZE`]² texels, row-major, texel (x, y) holding the noise at the cell
/// coordinates of its center, ((x + 0.5, y + 0.5) / [`TEXELS_PER_CELL`]).
pub fn texture() -> Vec<u8> {
    let corners: Vec<f32> = (0..PERIOD * PERIOD).map(|k| lattice((k % PERIOD) as i32, (k / PERIOD) as i32)).collect();
    let at = |i: usize, j: usize| corners[(j % PERIOD) * PERIOD + i % PERIOD];
    // The smoothstep weight of each texel's place in its cell (the same for every cell).
    let weights: Vec<f32> = (0..TEXELS_PER_CELL)
        .map(|t| {
            let f = (t as f32 + 0.5) / TEXELS_PER_CELL as f32;
            f * f * (3.0 - 2.0 * f)
        })
        .collect();
    let mut out = vec![0u8; SIZE * SIZE];
    for y in 0..SIZE {
        let (j, v) = (y / TEXELS_PER_CELL, weights[y % TEXELS_PER_CELL]);
        for x in 0..SIZE {
            let (i, u) = (x / TEXELS_PER_CELL, weights[x % TEXELS_PER_CELL]);
            let a = at(i, j) + (at(i + 1, j) - at(i, j)) * u;
            let b = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * u;
            out[y * SIZE + x] = ((a + (b - a) * v) * 255.0).round() as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shader's smooth value noise at `(x, y)` (cell units), hashed as `w_noise` did.
    fn reference(x: f32, y: f32) -> f32 {
        let (i, j) = (x.floor(), y.floor());
        let (fx, fy) = (x - i, y - j);
        let (u, v) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
        let (i, j) = (i as i32, j as i32);
        let a = lattice(i, j) + (lattice(i + 1, j) - lattice(i, j)) * u;
        let b = lattice(i, j + 1) + (lattice(i + 1, j + 1) - lattice(i, j + 1)) * u;
        a + (b - a) * v
    }

    /// Bilinear filtering of the texture at `(x, y)` (cell units), wrapping, as the GPU samples it.
    fn sample(tex: &[u8], x: f32, y: f32) -> f32 {
        let tx = x * TEXELS_PER_CELL as f32 - 0.5;
        let ty = y * TEXELS_PER_CELL as f32 - 0.5;
        let (x0, y0) = (tx.floor(), ty.floor());
        let (fx, fy) = (tx - x0, ty - y0);
        let t = |x: f32, y: f32| {
            let x = (x as i64).rem_euclid(SIZE as i64) as usize;
            let y = (y as i64).rem_euclid(SIZE as i64) as usize;
            tex[y * SIZE + x] as f32 / 255.0
        };
        let a = t(x0, y0) + (t(x0 + 1.0, y0) - t(x0, y0)) * fx;
        let b = t(x0, y0 + 1.0) + (t(x0 + 1.0, y0 + 1.0) - t(x0, y0 + 1.0)) * fx;
        a + (b - a) * fy
    }

    #[test]
    fn lattice_values_are_uniform() {
        let vals: Vec<f32> = (0..64).flat_map(|j| (0..64).map(move |i| lattice(i, j))).collect();
        assert!(vals.iter().all(|v| (0.0..=1.0).contains(v)));
        let mean = vals.iter().sum::<f32>() / vals.len() as f32;
        assert!((mean - 0.5).abs() < 0.03, "mean {mean}");
    }

    #[test]
    fn filtering_the_texture_reproduces_the_shaders_noise() {
        let tex = texture();
        assert_eq!(tex.len(), SIZE * SIZE);
        let mut worst = 0.0f32;
        for k in 0..20_000 {
            // Spread over the period, off the texel grid (short of its last cell, where the reference
            // reads lattice point PERIOD and the texture wraps to 0).
            let x = (k as f32 * 0.618_034 * 37.0) % (PERIOD - 1) as f32;
            let y = (k as f32 * 0.414_214 * 53.0) % (PERIOD - 1) as f32;
            worst = worst.max((sample(&tex, x, y) - reference(x, y)).abs());
        }
        // Quantizing to 8 bits (half a step, 0.002) plus the linear between texels of the
        // smoothstep curve (its bend over an eighth of a cell).
        assert!(worst < 0.02, "worst {worst}");
    }

    #[test]
    fn it_wraps_seamlessly() {
        let tex = texture();
        for k in 0..PERIOD * 4 {
            let y = k as f32 * 0.25 + 0.1;
            let left = sample(&tex, 0.01, y);
            let right = sample(&tex, PERIOD as f32 + 0.01, y);
            assert!((left - right).abs() < 1e-6);
            // Across the seam it's as smooth as anywhere: the lattice wraps with it.
            let across = (sample(&tex, PERIOD as f32 - 0.02, y) - sample(&tex, 0.02, y)).abs();
            assert!(across < 0.05, "seam step {across}");
        }
    }
}
