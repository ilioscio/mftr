//! The `<id>.anims.ron` marker sidecar written by the Blender exporter (10 §4.5, §8.3).

use nanoserde::DeRon;

/// Hard cap on the sidecar's size, checked before parsing.
pub const MAX_SIDECAR_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, DeRon)]
pub struct Sidecar {
    pub id: String,
    /// "champion", "library" or "rig".
    pub kind: String,
    pub archetype: String,
    pub rig_version: u32,
    pub fps: u32,
    pub clips: Vec<Clip>,
}

#[derive(Clone, Debug, DeRon)]
pub struct Clip {
    pub name: String,
    /// Length in frames; a looping clip's last frame repeats its first.
    pub frames: u32,
    pub looping: bool,
    /// "full", "upper" or "additive".
    pub layer: String,
    /// Locomotion only: the ground speed the cycle was authored for, in u/s.
    pub stride_speed: Option<f32>,
    pub markers: Vec<Marker>,
}

#[derive(Clone, Debug, DeRon)]
pub struct Marker {
    pub name: String,
    pub frame: u32,
}

impl Clip {
    pub fn marker(&self, name: &str) -> Option<u32> {
        self.markers.iter().find(|m| m.name == name).map(|m| m.frame)
    }
}

pub fn parse(text: &str) -> Result<Sidecar, String> {
    if text.len() > MAX_SIDECAR_BYTES {
        return Err(format!("sidecar is {} bytes, over the {} byte cap", text.len(), MAX_SIDECAR_BYTES));
    }
    Sidecar::deserialize_ron(text).map_err(|e| format!("sidecar: {e}"))
}
