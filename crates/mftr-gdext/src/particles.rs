//! The VFX kit's particles (23 §3, D79), simulated here rather than in GDScript: the kit
//! (`client/scripts/vfx.gd`) decides what to emit, where and how (its kits' patterns), and hands
//! each particle over with `emit`; every frame `step` moves them all and `pack` writes the two
//! MultiMesh buffers `vfx_particle.gdshader` draws, the covering layer and the glowing one.
//! A thousand live particles cost about 3 ms a frame in GDScript, a few hundredths here.

use godot::prelude::*;

/// Most live particles, client-wide (05 §5.1).
pub const MAX: usize = 2048;
/// Floats per instance in a MultiMesh buffer: a 3×4 transform, a color, custom data.
pub const STRIDE: usize = 20;

// Shapes (vfx_particle.gdshader).
pub const FLAME: u8 = 2;
pub const SMOKE: u8 = 3;
pub const SHARD: u8 = 4;
pub const RING: u8 = 6;
pub const DISC: u8 = 7;
// Flags.
pub const BOUNCE: u8 = 1;
pub const PUFF: u8 = 2;
pub const SPLAT: u8 = 4;
pub const FLICKER: u8 = 8;

#[derive(Clone, Copy, Debug, Default)]
pub struct Particle {
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub age: f32,
    pub life: f32,
    pub size: f32,
    /// Size added by the end of its life, eased out.
    pub grow: f32,
    /// m/s² down (negative floats up).
    pub grav: f32,
    /// Speed kept per twentieth of a second.
    pub drag: f32,
    pub rot: f32,
    pub spin: f32,
    pub seed: f32,
    pub shape: u8,
    pub flags: u8,
    /// 1 adds light, 0 covers.
    pub add: f32,
    /// Light at birth (over 1 blooms), easing to 1 (or staying, if 1 or under).
    pub heat: f32,
    /// Metres of depth fade against the scene.
    pub soft: f32,
    /// Seconds of travel drawn as a streak.
    pub stretch: f32,
    /// sRGB colors from birth to death.
    pub ramp: [[f32; 4]; 6],
    pub ramp_n: u8,
}

/// The particles and their buffers.
pub struct Sim {
    pub parts: Vec<Particle>,
    /// Both layers: the covering one from 0, the glowing one from `MAX * STRIDE`.
    pub buf: Vec<f32>,
    pub dust: ([[f32; 4]; 6], u8),
    rng: u32,
}

impl Default for Sim {
    fn default() -> Self {
        Sim {
            parts: Vec::with_capacity(MAX),
            buf: vec![0.0; MAX * STRIDE * 2],
            dust: ([[0.6, 0.57, 0.5, 0.5]; 6], 1),
            rng: 0x9e37_79b9,
        }
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c < 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Sim {
    fn rand(&mut self) -> f32 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn emit(&mut self, mut p: Particle) -> bool {
        if self.parts.len() >= MAX {
            return false;
        }
        p.life = p.life.max(0.02);
        p.seed = self.rand();
        self.parts.push(p);
        true
    }

    /// Moves every particle on by `dt` seconds: gravity, drag, spin; those that touch the
    /// ground bounce, die there or raise a puff of dust, as their flags say.
    pub fn step(&mut self, dt: f32) {
        let mut puffs = Vec::new();
        let mut i = 0;
        while i < self.parts.len() {
            let p = &mut self.parts[i];
            p.age += dt;
            if p.age >= p.life {
                self.parts.swap_remove(i);
                continue;
            }
            let mut v = p.vel;
            v[1] -= p.grav * dt;
            let mut n = [p.pos[0] + v[0] * dt, p.pos[1] + v[1] * dt, p.pos[2] + v[2] * dt];
            if p.flags != 0 {
                let floor = 0.03 + if p.shape == SHARD { p.size * 0.4 } else { 0.0 };
                if n[1] < floor && v[1] < 0.0 {
                    if p.flags & PUFF != 0 {
                        p.flags &= !PUFF;
                        puffs.push((n[0], n[2], p.size));
                    }
                    if p.flags & SPLAT != 0 {
                        self.parts.swap_remove(i);
                        continue;
                    }
                    if p.flags & BOUNCE != 0 {
                        n[1] = floor;
                        v = [v[0] * 0.6, -v[1] * 0.35, v[2] * 0.6];
                        p.spin *= 0.5;
                    }
                }
            }
            let keep = p.drag.powf(dt * 20.0);
            p.pos = n;
            p.vel = [v[0] * keep, v[1] * keep, v[2] * keep];
            p.rot += p.spin * dt;
            i += 1;
        }
        for (x, z, size) in puffs {
            for _ in 0..2 {
                let d = [self.rand() * 2.0 - 1.0, self.rand() * 0.4 + 0.2, self.rand() * 2.0 - 1.0];
                let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().max(0.01);
                let life = 0.6 + 0.4 * self.rand();
                let rot = self.rand() * std::f32::consts::TAU;
                let (ramp, ramp_n) = self.dust;
                self.emit(Particle {
                    pos: [x, 0.15, z],
                    vel: [d[0] / l * 0.6, d[1] / l * 0.6, d[2] / l * 0.6],
                    life,
                    size: (size * 2.2).max(0.25),
                    grow: 1.0,
                    grav: -0.2,
                    drag: 0.85,
                    rot,
                    shape: SMOKE,
                    heat: 1.0,
                    soft: 0.6,
                    ramp,
                    ramp_n,
                    ..Default::default()
                });
            }
        }
    }

    /// The ramp's color at `f` (0–1), blended between its colors, in linear light.
    fn color(p: &Particle, f: f32) -> [f32; 4] {
        let n = p.ramp_n.max(1) as usize;
        let c = if n == 1 {
            p.ramp[0]
        } else {
            let x = f.clamp(0.0, 1.0) * (n - 1) as f32;
            let k = (x as usize).min(n - 2);
            let t = x - k as f32;
            let (a, b) = (p.ramp[k], p.ramp[k + 1]);
            [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t]
        };
        [srgb_to_linear(c[0]), srgb_to_linear(c[1]), srgb_to_linear(c[2]), c[3]]
    }

    /// Writes every live particle into its layer's half of `buf`, as `vfx_particle.gdshader`
    /// reads it; returns how many each layer holds (covering, glowing). `t` is the clock, for
    /// twinkling.
    pub fn pack(&mut self, t: f32) -> (usize, usize) {
        let (mut nc, mut ng) = (0usize, 0usize);
        for p in &self.parts {
            let f = p.age / p.life;
            let mut c = Self::color(p, if p.shape == FLAME || p.shape == DISC { 0.35 + 0.65 * f } else { f });
            let mut a = c[3]
                * match p.shape {
                    FLAME => smoothstep(0.0, 0.1, f) * (1.0 - f),
                    SMOKE => smoothstep(0.0, 0.18, f) * (1.0 - f).powf(1.3),
                    SHARD => 1.0 - smoothstep(0.75, 1.0, f),
                    RING => 1.0 - f * f,
                    DISC => (1.0 - f).powf(1.6),
                    _ => (1.0 - f).powf(1.2),
                };
            if p.flags & FLICKER != 0 {
                a *= 0.55 + 0.45 * (t * 19.0 + p.seed * 40.0).sin();
            }
            let glow = if p.heat > 1.0 { 1.0 + (p.heat - 1.0) * (1.0 - f).powf(1.5) } else { p.heat };
            let s = p.size * (1.0 + p.grow * (1.0 - (1.0 - f) * (1.0 - f)));
            if s <= 0.002 || a <= 0.002 {
                continue;
            }
            let o = if p.add < 0.5 {
                nc += 1;
                (nc - 1) * STRIDE
            } else {
                ng += 1;
                MAX * STRIDE + (ng - 1) * STRIDE
            };
            c[3] = a;
            let b = &mut self.buf[o..o + STRIDE];
            // Rows of the 3×4 transform: column 0 the velocity, column 1 (size, turn, stretch),
            // column 2 (shape, seed, soft), column 3 the position.
            b[0] = p.vel[0];
            b[1] = s;
            b[2] = p.shape as f32;
            b[3] = p.pos[0];
            b[4] = p.vel[1];
            b[5] = p.rot;
            b[6] = p.seed;
            b[7] = p.pos[1];
            b[8] = p.vel[2];
            b[9] = p.stretch;
            b[10] = p.soft;
            b[11] = p.pos[2];
            b[12..16].copy_from_slice(&c);
            b[16] = f;
            b[17] = glow;
            b[18] = p.add;
            b[19] = 0.0;
        }
        (nc, ng)
    }
}

fn ramp_of(colors: &PackedColorArray) -> ([[f32; 4]; 6], u8) {
    let mut ramp = [[1.0; 4]; 6];
    let n = colors.len().min(6);
    for (k, c) in colors.as_slice().iter().take(6).enumerate() {
        ramp[k] = [c.r, c.g, c.b, c.a];
    }
    (ramp, n.max(1) as u8)
}

/// The particle simulation for `vfx.gd`.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct MftrParticles {
    base: Base<RefCounted>,
    sim: Sim,
}

#[godot_api]
impl IRefCounted for MftrParticles {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base, sim: Sim::default() }
    }
}

#[godot_api]
impl MftrParticles {
    /// Most live particles.
    #[func]
    fn capacity() -> i64 {
        MAX as i64
    }

    /// The colors of the dust a landing particle raises.
    #[func]
    fn set_dust(&mut self, ramp: PackedColorArray) {
        self.sim.dust = ramp_of(&ramp);
    }

    /// A particle at `pos` moving at `vel`, in `ramp`'s colors, of `shape`; `k` is its knobs:
    /// life, size, grow, grav, drag, rot, spin, flags, add, heat, soft, stretch. False when
    /// the kit is full.
    #[func]
    fn emit(&mut self, pos: Vector3, vel: Vector3, ramp: PackedColorArray, shape: i64, k: PackedFloat32Array) -> bool {
        let k = k.as_slice();
        if k.len() < 12 {
            godot_error!("MftrParticles.emit: {} knobs, 12 needed", k.len());
            return false;
        }
        let (ramp, ramp_n) = ramp_of(&ramp);
        self.sim.emit(Particle {
            pos: [pos.x, pos.y, pos.z],
            vel: [vel.x, vel.y, vel.z],
            life: k[0],
            size: k[1],
            grow: k[2],
            grav: k[3],
            drag: k[4],
            rot: k[5],
            spin: k[6],
            flags: k[7] as u8,
            add: k[8],
            heat: k[9],
            soft: k[10],
            stretch: k[11],
            shape: shape.clamp(0, 7) as u8,
            ramp,
            ramp_n,
            ..Default::default()
        })
    }

    #[func]
    fn step(&mut self, dt: f64) {
        self.sim.step(dt as f32);
    }

    /// Packs the buffers for this frame: how many particles each layer draws, covering then
    /// glowing.
    #[func]
    fn pack(&mut self, t: f64) -> PackedInt32Array {
        let (nc, ng) = self.sim.pack(t as f32);
        PackedInt32Array::from(&[nc as i32, ng as i32][..])
    }

    /// The covering layer's MultiMesh buffer (`MAX` instances).
    #[func]
    fn cover_buffer(&self) -> PackedFloat32Array {
        PackedFloat32Array::from(&self.sim.buf[..MAX * STRIDE])
    }

    /// The glowing layer's MultiMesh buffer (`MAX` instances).
    #[func]
    fn glow_buffer(&self) -> PackedFloat32Array {
        PackedFloat32Array::from(&self.sim.buf[MAX * STRIDE..])
    }

    #[func]
    fn count(&self) -> i64 {
        self.sim.parts.len() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spark(y: f32, vy: f32, flags: u8) -> Particle {
        Particle {
            pos: [0.0, y, 0.0],
            vel: [1.0, vy, 0.0],
            life: 1.0,
            size: 0.1,
            drag: 1.0,
            grav: 9.0,
            flags,
            heat: 2.0,
            add: 1.0,
            ramp_n: 2,
            ramp: [[1.0, 1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 1.0], [0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4]],
            ..Default::default()
        }
    }

    #[test]
    fn particles_fall_bounce_and_die_of_age() {
        let mut s = Sim::default();
        s.emit(spark(0.5, -4.0, BOUNCE));
        s.emit(spark(0.5, -4.0, SPLAT));
        s.emit(spark(0.5, -4.0, 0));
        for _ in 0..12 {
            s.step(1.0 / 60.0);
        }
        // The splatting one died on the ground, the bouncing one is above it and rising, the
        // plain one fell through.
        assert_eq!(s.parts.len(), 2);
        assert!(s.parts.iter().any(|p| p.flags == BOUNCE && p.pos[1] >= 0.03 && p.vel[1] > 0.0));
        assert!(s.parts.iter().any(|p| p.flags == 0 && p.pos[1] < 0.0));
        for _ in 0..60 {
            s.step(1.0 / 60.0);
        }
        assert!(s.parts.is_empty());
    }

    #[test]
    fn a_landing_raises_dust_once() {
        let mut s = Sim::default();
        s.emit(spark(0.2, -6.0, BOUNCE | PUFF));
        for _ in 0..30 {
            s.step(1.0 / 60.0);
        }
        assert_eq!(s.parts.iter().filter(|p| p.shape == SMOKE).count(), 2);
    }

    #[test]
    fn the_buffers_split_by_layer_in_linear_light() {
        let mut s = Sim::default();
        s.emit(spark(1.0, 0.0, 0));
        let mut smoke = spark(1.0, 0.0, 0);
        smoke.add = 0.0;
        smoke.shape = SMOKE;
        smoke.ramp[0] = [0.5, 0.5, 0.5, 1.0];
        s.emit(smoke);
        s.step(0.1);
        assert_eq!(s.pack(0.0), (1, 1));
        // The glowing one: at the glow layer's start, its position in column 3, hot and white.
        let g = &s.buf[MAX * STRIDE..MAX * STRIDE + STRIDE];
        assert!((g[3] - 0.1).abs() < 1e-4 && (g[7] - 1.0).abs() < 0.1);
        assert!(g[17] > 1.0, "a hot particle blooms");
        // The smoke's grey is linear: 0.5 sRGB is about 0.21.
        let c = &s.buf[..STRIDE];
        assert!(c[12] < 0.3, "{}", c[12]);
    }

    #[test]
    fn the_kit_fills_to_its_cap() {
        let mut s = Sim::default();
        for _ in 0..MAX {
            assert!(s.emit(spark(1.0, 0.0, 0)));
        }
        assert!(!s.emit(spark(1.0, 0.0, 0)));
    }
}
