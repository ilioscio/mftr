//! A forged model kept on the player's disk (D78): `mftr-forge` builds the official champions
//! from their recipes in seconds each, so the client keeps each one it built and loads it again
//! next launch. Nothing ships in the download (every byte there is a cost); the file lives only
//! in the player's own data folder and is rebuilt whenever its key changes.
//!
//! The format is ours and plain: a magic, a format version, the caller's key, the pack's
//! fields in order, and an FNV-1a 64 checksum of all of it. A file that's truncated, corrupt,
//! from another format or for another key is refused, never trusted: reads are bounds-checked
//! and lengths are capped by the bytes left, so a bad file can't panic or allocate wildly.

use std::sync::Arc;

use crate::Loaded;
use crate::glb::{Animation, Channel, Model, Node, Paint, Primitive};
use crate::pose::{self, Clip, Layer, Library, Rig, Track, Trs};
use crate::sfx::{Sound, SoundSpec};
use crate::vfx::{Effect, VfxFile};

const MAGIC: &[u8; 8] = b"MFTRFRG\0";
/// Bump whenever what's written changes shape: older files are then refused and rebuilt.
pub const FORMAT: u32 = 1;

/// FNV-1a 64 of `bytes` (the checksum, and a cheap key hash for callers).
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// `pack` as a cache file under `key` (what it was built from; `read` must be given the same).
pub fn write(pack: &Loaded, key: &str) -> Vec<u8> {
    let mut w = W(Vec::with_capacity(1 << 20));
    w.0.extend_from_slice(MAGIC);
    w.u32(FORMAT);
    w.str(key);
    w.str(&pack.kind);
    model(&mut w, &pack.model);
    library(&mut w, &pack.library);
    w.opt(pack.vfx.as_ref(), |w, v| w.vec(&v.effects, effect));
    w.vec(&pack.sounds, sound);
    let sum = fnv1a64(&w.0);
    w.u64(sum);
    w.0
}

/// The pack in `bytes`, if they're a whole, undamaged cache file of this format for `key`.
pub fn read(bytes: &[u8], key: &str) -> Result<Loaded, String> {
    if bytes.len() < MAGIC.len() + 4 + 8 || &bytes[..MAGIC.len()] != MAGIC {
        return Err("not a forged-model cache".into());
    }
    let (body, sum) = bytes.split_at(bytes.len() - 8);
    if fnv1a64(body) != u64::from_le_bytes(sum.try_into().expect("8 bytes")) {
        return Err("checksum mismatch".into());
    }
    let mut r = R { b: body, at: MAGIC.len() };
    if r.u32()? != FORMAT {
        return Err("another format".into());
    }
    if r.str()? != key {
        return Err("built from something else".into());
    }
    let kind = r.str()?;
    let model = read_model(&mut r)?;
    let library = read_library(&mut r)?;
    let vfx = r.opt(|r| Ok(VfxFile { effects: r.vec(read_effect)? }))?;
    let sounds = r.vec(read_sound)?;
    if r.at != r.b.len() {
        return Err("trailing bytes".into());
    }
    check(&model, &library)?;
    Ok(Loaded { kind, model, library, vfx, sounds })
}

/// What the client indexes with, in range: the checksum vouches for the bytes, this for the
/// writer (a bug there mustn't become a crash on every later launch).
fn check(m: &Model, lib: &Library) -> Result<(), String> {
    let n = m.nodes.len();
    for node in &m.nodes {
        if node.parent.is_some_and(|p| p >= n)
            || node.mesh.is_some_and(|i| i >= m.meshes.len())
            || node.skin.is_some_and(|i| i >= m.skins.len())
        {
            return Err("a node points outside the model".into());
        }
    }
    if m.skins.iter().flatten().any(|&j| j >= n) || m.animations.iter().flat_map(|a| &a.channels).any(|c| c.node >= n) {
        return Err("a joint or channel points outside the model".into());
    }
    for p in m.meshes.iter().flatten() {
        let v = p.positions.len();
        let per = |len: usize| len == 0 || len == v;
        if p.indices.iter().any(|&i| i as usize >= v)
            || !per(p.colors.len())
            || !per(p.joints.len())
            || !per(p.weights.len())
            || !per(p.uvs.len())
            || p.targets.iter().any(|t| t.len() != v)
            || p.material.is_some_and(|i| i >= m.materials.len())
            || p.paint.as_ref().is_some_and(|i| i.rgba.len() != (i.size as usize).pow(2) * 4)
        {
            return Err("a primitive's arrays disagree".into());
        }
    }
    let bones = lib.rig.names.len();
    if lib.rig.parents.len() != bones
        || lib.rig.rest.len() != bones
        || lib.rig.bind.len() != bones
        || lib.rig.parents.iter().enumerate().any(|(i, p)| p.is_some_and(|p| p >= i))
        || lib.joint_to_bone.iter().any(|&b| b >= bones)
        || lib.clips.iter().flat_map(|c| &c.tracks).any(|t| t.bone >= bones || t.times.len() != t.values.len())
    {
        return Err("the rig or a clip points outside it".into());
    }
    Ok(())
}

// --- Writing ---------------------------------------------------------------------------------

struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn len(&mut self, v: usize) {
        self.u32(v as u32);
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32s(&mut self, v: &[f32]) {
        for x in v {
            self.f32(*x);
        }
    }
    fn bool(&mut self, v: bool) {
        self.u8(v as u8);
    }
    fn str(&mut self, s: &str) {
        self.len(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
    fn opt<T>(&mut self, v: Option<T>, f: impl FnOnce(&mut W, T)) {
        match v {
            None => self.u8(0),
            Some(x) => {
                self.u8(1);
                f(self, x);
            }
        }
    }
    fn idx(&mut self, v: Option<usize>) {
        self.opt(v, |w, i| w.len(i));
    }
    fn vec<T>(&mut self, v: &[T], f: impl Fn(&mut W, &T)) {
        self.len(v.len());
        for x in v {
            f(self, x);
        }
    }
}

fn model(w: &mut W, m: &Model) {
    w.vec(&m.nodes, |w, n| {
        w.str(&n.name);
        w.idx(n.parent);
        w.f32s(&n.translation);
        w.f32s(&n.rotation);
        w.f32s(&n.scale);
        w.idx(n.mesh);
        w.idx(n.skin);
    });
    w.vec(&m.bind, |w, b| {
        w.opt(b.as_ref(), |w, (t, r, s)| {
            w.f32s(t);
            w.f32s(r);
            w.f32s(s);
        })
    });
    w.vec(&m.skins, |w, s| w.vec(s, |w, j| w.len(*j)));
    w.vec(&m.meshes, |w, prims| w.vec(prims, primitive));
    w.vec(&m.materials, |w, s| w.str(s));
    w.vec(&m.animations, |w, a| {
        w.str(&a.name);
        w.vec(&a.channels, |w, c| {
            w.len(c.node);
            w.str(&c.path);
            w.bool(c.step);
            w.vec(&c.times, |w, t| w.f32(*t));
            w.vec(&c.values, |w, v| w.vec(v, |w, x| w.f32(*x)));
        });
    });
    w.vec(&m.target_names, |w, names| w.vec(names, |w, s| w.str(s)));
}

fn primitive(w: &mut W, p: &Primitive) {
    w.idx(p.material);
    w.len(p.triangles);
    w.bool(p.has_color);
    w.vec(&p.positions, |w, v| w.f32s(v));
    w.vec(&p.colors, |w, v| w.f32s(v));
    w.vec(&p.indices, |w, i| w.u32(*i));
    w.vec(&p.joints, |w, j| j.iter().for_each(|x| w.u32(*x)));
    w.vec(&p.weights, |w, v| w.f32s(v));
    w.vec(&p.targets, |w, t| w.vec(t, |w, v| w.f32s(v)));
    w.vec(&p.uvs, |w, v| w.f32s(v));
    w.opt(p.paint.as_deref(), |w, i| {
        w.u32(i.size);
        w.vec(&i.rgba, |w, b| w.u8(*b));
    });
}

fn trs(w: &mut W, t: &Trs) {
    w.f32s(&t.t);
    w.f32s(&t.r);
    w.f32s(&t.s);
}

fn library(w: &mut W, l: &Library) {
    w.vec(&l.rig.names, |w, s| w.str(s));
    w.vec(&l.rig.parents, |w, p| w.idx(*p));
    w.vec(&l.rig.rest, trs);
    w.vec(&l.rig.bind, trs);
    w.vec(&l.clips, |w, c| {
        w.str(&c.name);
        w.f32(c.length);
        w.bool(c.looping);
        w.u8(match c.layer {
            Layer::Full => 0,
            Layer::Upper => 1,
            Layer::Additive => 2,
        });
        w.opt(c.stride_speed, |w, s| w.f32(s));
        w.vec(&c.markers, |w, (n, t)| {
            w.str(n);
            w.f32(*t);
        });
        w.vec(&c.tracks, |w, t| {
            w.len(t.bone);
            w.u8(match t.channel {
                pose::Channel::Translation => 0,
                pose::Channel::Rotation => 1,
                pose::Channel::Scale => 2,
            });
            w.bool(t.step);
            w.vec(&t.times, |w, x| w.f32(*x));
            w.vec(&t.values, |w, v| w.f32s(v));
        });
    });
    w.vec(&l.joint_to_bone, |w, b| w.len(*b));
}

fn effect(w: &mut W, e: &Effect) {
    w.str(&e.event);
    w.str(&e.kit);
    w.vec(&e.ramp, |w, (r, g, b)| w.f32s(&[*r, *g, *b]));
    w.opt(e.count, |w, v| w.u32(v));
    for v in [e.size, e.speed, e.lifetime, e.arc, e.tilt] {
        w.opt(v, |w, x| w.f32(x));
    }
}

fn sound(w: &mut W, s: &Sound) {
    w.str(&s.spec.name);
    w.vec(&s.spec.events, |w, e| w.str(e));
    w.f32(s.spec.volume);
    w.f32(s.spec.pitch);
    w.u32(s.rate);
    w.u8(s.channels);
    w.vec(&s.pcm, |w, x| w.0.extend_from_slice(&x.to_le_bytes()));
}

// --- Reading ---------------------------------------------------------------------------------

struct R<'a> {
    b: &'a [u8],
    at: usize,
}

type Res<T> = Result<T, String>;

impl R<'_> {
    fn take(&mut self, n: usize) -> Res<&[u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.b.len()).ok_or("truncated")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Res<u8> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Res<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn f32(&mut self) -> Res<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn f32s<const N: usize>(&mut self) -> Res<[f32; N]> {
        let mut out = [0.0; N];
        for x in &mut out {
            *x = self.f32()?;
        }
        Ok(out)
    }
    fn bool(&mut self) -> Res<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("bad flag".into()),
        }
    }
    fn usize(&mut self) -> Res<usize> {
        Ok(self.u32()? as usize)
    }
    /// A count of items at least one byte each: never more than the bytes left.
    fn count(&mut self) -> Res<usize> {
        let n = self.usize()?;
        if n > self.b.len() - self.at {
            return Err("a count past the end".into());
        }
        Ok(n)
    }
    fn str(&mut self) -> Res<String> {
        let n = self.count()?;
        String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "bad text".into())
    }
    fn opt<T>(&mut self, f: impl FnOnce(&mut Self) -> Res<T>) -> Res<Option<T>> {
        if self.bool()? { Ok(Some(f(self)?)) } else { Ok(None) }
    }
    fn idx(&mut self) -> Res<Option<usize>> {
        self.opt(|r| r.usize())
    }
    fn vec<T>(&mut self, f: impl Fn(&mut Self) -> Res<T>) -> Res<Vec<T>> {
        let n = self.count()?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(f(self)?);
        }
        Ok(out)
    }
}

fn read_model(r: &mut R) -> Res<Model> {
    let nodes = r.vec(|r| {
        Ok(Node {
            name: r.str()?,
            parent: r.idx()?,
            translation: r.f32s()?,
            rotation: r.f32s()?,
            scale: r.f32s()?,
            mesh: r.idx()?,
            skin: r.idx()?,
        })
    })?;
    let bind = r.vec(|r| r.opt(|r| Ok((r.f32s()?, r.f32s()?, r.f32s()?))))?;
    let skins = r.vec(|r| r.vec(|r| r.usize()))?;
    let meshes = r.vec(|r| r.vec(read_primitive))?;
    let materials = r.vec(|r| r.str())?;
    let animations = r.vec(|r| {
        Ok(Animation {
            name: r.str()?,
            channels: r.vec(|r| {
                Ok(Channel {
                    node: r.usize()?,
                    path: r.str()?,
                    step: r.bool()?,
                    times: r.vec(|r| r.f32())?,
                    values: r.vec(|r| r.vec(|r| r.f32()))?,
                })
            })?,
        })
    })?;
    let target_names = r.vec(|r| r.vec(|r| r.str()))?;
    Ok(Model { nodes, bind, skins, meshes, materials, animations, target_names })
}

fn read_primitive(r: &mut R) -> Res<Primitive> {
    Ok(Primitive {
        material: r.idx()?,
        triangles: r.usize()?,
        has_color: r.bool()?,
        positions: r.vec(|r| r.f32s())?,
        colors: r.vec(|r| r.f32s())?,
        indices: r.vec(|r| r.u32())?,
        joints: r.vec(|r| Ok([r.u32()?, r.u32()?, r.u32()?, r.u32()?]))?,
        weights: r.vec(|r| r.f32s())?,
        targets: r.vec(|r| r.vec(|r| r.f32s()))?,
        uvs: r.vec(|r| r.f32s())?,
        paint: r.opt(|r| Ok(Arc::new(Paint { size: r.u32()?, rgba: r.vec(|r| r.u8())? })))?,
    })
}

fn read_trs(r: &mut R) -> Res<Trs> {
    Ok(Trs { t: r.f32s()?, r: r.f32s()?, s: r.f32s()? })
}

fn read_library(r: &mut R) -> Res<Library> {
    let rig = Rig {
        names: r.vec(|r| r.str())?,
        parents: r.vec(|r| r.idx())?,
        rest: r.vec(read_trs)?,
        bind: r.vec(read_trs)?,
    };
    let clips = r.vec(|r| {
        Ok(Clip {
            name: r.str()?,
            length: r.f32()?,
            looping: r.bool()?,
            layer: match r.u8()? {
                0 => Layer::Full,
                1 => Layer::Upper,
                2 => Layer::Additive,
                _ => return Err("bad layer".into()),
            },
            stride_speed: r.opt(|r| r.f32())?,
            markers: r.vec(|r| Ok((r.str()?, r.f32()?)))?,
            tracks: r.vec(|r| {
                Ok(Track {
                    bone: r.usize()?,
                    channel: match r.u8()? {
                        0 => pose::Channel::Translation,
                        1 => pose::Channel::Rotation,
                        2 => pose::Channel::Scale,
                        _ => return Err("bad channel".into()),
                    },
                    step: r.bool()?,
                    times: r.vec(|r| r.f32())?,
                    values: r.vec(|r| r.f32s())?,
                })
            })?,
        })
    })?;
    let joint_to_bone = r.vec(|r| r.usize())?;
    Ok(Library { rig, clips, joint_to_bone })
}

fn read_effect(r: &mut R) -> Res<Effect> {
    Ok(Effect {
        event: r.str()?,
        kit: r.str()?,
        ramp: r.vec(|r| Ok((r.f32()?, r.f32()?, r.f32()?)))?,
        count: r.opt(|r| r.u32())?,
        size: r.opt(|r| r.f32())?,
        speed: r.opt(|r| r.f32())?,
        lifetime: r.opt(|r| r.f32())?,
        arc: r.opt(|r| r.f32())?,
        tilt: r.opt(|r| r.f32())?,
    })
}

fn read_sound(r: &mut R) -> Res<Sound> {
    Ok(Sound {
        spec: SoundSpec { name: r.str()?, events: r.vec(|r| r.str())?, volume: r.f32()?, pitch: r.f32()? },
        rate: r.u32()?,
        channels: r.u8()?,
        pcm: r.vec(|r| {
            let b = r.take(2)?;
            Ok(i16::from_le_bytes([b[0], b[1]]))
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_comes_back_whole_and_a_damaged_or_foreign_file_is_refused() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let pack = crate::load_file(&root.join("art/champions/bastion/export/bastion.glb")).expect("bastion loads");
        let bytes = write(&pack, "key");
        let back = read(&bytes, "key").expect("reads back");
        assert_eq!(write(&back, "key"), bytes, "what's read writes the same bytes");
        assert_eq!(back.model.triangles(), pack.model.triangles());
        assert_eq!(back.library.clips.len(), pack.library.clips.len());
        assert!(read(&bytes, "another key").is_err());
        assert!(read(&bytes[..bytes.len() / 2], "key").is_err());
        let mut bad = bytes.clone();
        let mid = bad.len() / 2;
        bad[mid] ^= 0x40;
        assert!(read(&bad, "key").is_err());
    }
}
