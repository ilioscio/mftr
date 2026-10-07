//! Skeletons, clips and poses from a validated pack (10 §6): what the client animates with.
//!
//! Engine-independent: the Godot extension copies the poses computed here into a `Skeleton3D`.
//! A pose is one local transform per bone, in rig order (parents before children).

use crate::glb::{Model, qmul};
use crate::sidecar::Sidecar;

/// A local transform: translation, rotation (xyzw), scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trs {
    pub t: [f32; 3],
    pub r: [f32; 4],
    pub s: [f32; 3],
}

impl Trs {
    pub const IDENTITY: Trs = Trs { t: [0.0; 3], r: [0.0, 0.0, 0.0, 1.0], s: [1.0; 3] };
}

pub type Pose = Vec<Trs>;

#[derive(Clone, Debug)]
pub struct Rig {
    pub names: Vec<String>,
    pub parents: Vec<Option<usize>>,
    pub rest: Pose,
}

impl Rig {
    pub fn bone(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// `root` and every bone below it.
    pub fn subtree(&self, root: &str) -> Vec<bool> {
        let mut inside = vec![false; self.names.len()];
        for i in 0..self.names.len() {
            // Parents come first, so a parent's flag is final when a child looks at it.
            inside[i] = self.names[i] == root || self.parents[i].is_some_and(|p| inside[p]);
        }
        inside
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Translation,
    Rotation,
    Scale,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub bone: usize,
    pub channel: Channel,
    pub step: bool,
    pub times: Vec<f32>,
    pub values: Vec<[f32; 4]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Full,
    Upper,
    Additive,
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub name: String,
    /// Length in seconds.
    pub length: f32,
    pub looping: bool,
    pub layer: Layer,
    /// Locomotion: the ground speed the cycle was authored for (u/s).
    pub stride_speed: Option<f32>,
    /// Marker name and time in seconds.
    pub markers: Vec<(String, f32)>,
    pub tracks: Vec<Track>,
}

impl Clip {
    pub fn marker(&self, name: &str) -> Option<f32> {
        self.markers.iter().find(|(n, _)| n == name).map(|(_, t)| *t)
    }

    /// Where the clip fires (attacks and casts); its end when it has no `fire`.
    pub fn fire(&self) -> f32 {
        self.marker("fire").unwrap_or(self.length)
    }

    /// Where a one-shot's follow-through ends.
    pub fn end(&self) -> f32 {
        self.marker("end").unwrap_or(self.length)
    }
}

/// A rig and its clips, ready to animate.
#[derive(Clone, Debug)]
pub struct Library {
    pub rig: Rig,
    pub clips: Vec<Clip>,
    /// glTF joint index → rig bone index (for remapping the mesh's JOINTS_0).
    pub joint_to_bone: Vec<usize>,
}

impl Library {
    /// From a model and sidecar that passed validation.
    pub fn new(model: &Model, side: &Sidecar) -> Result<Library, String> {
        let joints = model.skins.first().ok_or("no skin")?;
        // Parents before children (Godot skeletons want that), in a stable order.
        let mut order: Vec<usize> = Vec::new();
        let depth = |mut j: usize| {
            let mut d = 0;
            while let Some(p) = model.nodes[j].parent {
                j = p;
                d += 1;
            }
            d
        };
        let mut by_depth: Vec<(usize, usize)> = joints.iter().enumerate().map(|(i, &n)| (depth(n), i)).collect();
        by_depth.sort();
        order.extend(by_depth.iter().map(|&(_, i)| i));
        let mut joint_to_bone = vec![0; joints.len()];
        for (bone, &joint) in order.iter().enumerate() {
            joint_to_bone[joint] = bone;
        }
        let node_to_bone = |node: usize| joints.iter().position(|&n| n == node).map(|j| joint_to_bone[j]);
        let mut rig = Rig { names: Vec::new(), parents: Vec::new(), rest: Vec::new() };
        for &j in &order {
            let n = &model.nodes[joints[j]];
            rig.names.push(n.name.clone());
            rig.parents.push(n.parent.and_then(node_to_bone));
            rig.rest.push(Trs { t: n.translation, r: n.rotation, s: n.scale });
        }
        let fps = side.fps.max(1) as f32;
        let mut clips = Vec::new();
        for a in &model.animations {
            let Some(meta) = side.clips.iter().find(|c| c.name == a.name) else { continue };
            let tracks = a
                .channels
                .iter()
                .filter_map(|c| {
                    let bone = node_to_bone(c.node)?;
                    let channel = match c.path.as_str() {
                        "translation" => Channel::Translation,
                        "rotation" => Channel::Rotation,
                        _ => Channel::Scale,
                    };
                    let values =
                        c.values.iter().map(|v| [v[0], v[1], v[2], v.get(3).copied().unwrap_or(0.0)]).collect();
                    Some(Track { bone, channel, step: c.step, times: c.times.clone(), values })
                })
                .collect();
            clips.push(Clip {
                name: a.name.clone(),
                length: meta.frames as f32 / fps,
                looping: meta.looping,
                layer: match meta.layer.as_str() {
                    "upper" => Layer::Upper,
                    "additive" => Layer::Additive,
                    _ => Layer::Full,
                },
                stride_speed: meta.stride_speed,
                markers: meta.markers.iter().map(|m| (m.name.clone(), m.frame as f32 / fps)).collect(),
                tracks,
            });
        }
        Ok(Library { rig, clips, joint_to_bone })
    }

    pub fn clip(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }

    /// The pose of `clip` at `time` seconds: bones without a channel stay at rest.
    pub fn sample(&self, clip: usize, time: f32) -> Pose {
        let mut pose = self.rig.rest.clone();
        let c = &self.clips[clip];
        let t = if c.looping && c.length > 0.0 { time.rem_euclid(c.length) } else { time.clamp(0.0, c.length) };
        for tr in &c.tracks {
            let v = sample_track(tr, t);
            let p = &mut pose[tr.bone];
            match tr.channel {
                Channel::Translation => p.t = [v[0], v[1], v[2]],
                Channel::Rotation => p.r = v,
                Channel::Scale => p.s = [v[0], v[1], v[2]],
            }
        }
        pose
    }
}

fn sample_track(tr: &Track, t: f32) -> [f32; 4] {
    let times = &tr.times;
    let i = times.partition_point(|&x| x <= t);
    if i == 0 {
        return tr.values[0];
    }
    if i >= times.len() {
        return tr.values[times.len() - 1];
    }
    let (a, b) = (tr.values[i - 1], tr.values[i]);
    if tr.step {
        return a;
    }
    let u = (t - times[i - 1]) / (times[i] - times[i - 1]).max(1e-6);
    match tr.channel {
        Channel::Rotation => slerp(a, b, u),
        _ => lerp4(a, b, u),
    }
}

fn lerp4(a: [f32; 4], b: [f32; 4], u: f32) -> [f32; 4] {
    [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u, a[3] + (b[3] - a[3]) * u]
}

fn lerp3(a: [f32; 3], b: [f32; 3], u: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u, a[2] + (b[2] - a[2]) * u]
}

/// Shortest-arc spherical interpolation of unit quaternions.
pub fn slerp(a: [f32; 4], b: [f32; 4], u: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let b = if dot < 0.0 {
        dot = -dot;
        [-b[0], -b[1], -b[2], -b[3]]
    } else {
        b
    };
    if dot > 0.9995 {
        return normalize(lerp4(a, b, u));
    }
    let theta = dot.clamp(-1.0, 1.0).acos();
    let s = theta.sin();
    let (wa, wb) = (((1.0 - u) * theta).sin() / s, (u * theta).sin() / s);
    [a[0] * wa + b[0] * wb, a[1] * wa + b[1] * wb, a[2] * wa + b[2] * wb, a[3] * wa + b[3] * wb]
}

fn normalize(q: [f32; 4]) -> [f32; 4] {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt().max(1e-12);
    [q[0] / l, q[1] / l, q[2] / l, q[3] / l]
}

fn conj(q: [f32; 4]) -> [f32; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

/// `base` toward `top` by `w` on the bones `mask` allows (all when `None`).
pub fn blend(base: &mut Pose, top: &Pose, w: f32, mask: Option<&[bool]>) {
    if w <= 0.0 {
        return;
    }
    for (i, (b, t)) in base.iter_mut().zip(top).enumerate() {
        if mask.is_some_and(|m| !m[i]) {
            continue;
        }
        if w >= 1.0 {
            *b = *t;
        } else {
            b.t = lerp3(b.t, t.t, w);
            b.r = slerp(b.r, t.r, w);
            b.s = lerp3(b.s, t.s, w);
        }
    }
}

/// Adds `add`'s difference from `rest` on top of `base`, scaled by `w` (additive layers).
pub fn add(base: &mut Pose, add: &Pose, rest: &Pose, w: f32) {
    if w <= 0.0 {
        return;
    }
    for ((b, a), r) in base.iter_mut().zip(add).zip(rest) {
        let delta = qmul(conj(r.r), a.r);
        b.r = normalize(qmul(b.r, slerp([0.0, 0.0, 0.0, 1.0], delta, w.min(1.0))));
        for k in 0..3 {
            b.t[k] += (a.t[k] - r.t[k]) * w;
        }
    }
}
