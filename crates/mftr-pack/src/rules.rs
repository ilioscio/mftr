//! The rig standard, the clip catalogue and the caps (10 §2, §5, §7, §9), as data.
//! `tools/blender/mftr_blender` mirrors these names; this is the authoritative copy.

use mftr_sim::MinionKind;
use mftr_sim::ability::Effect;
use mftr_sim::champion::ChampionDef;

pub const FPS: u32 = 30;

// Caps (10 §2, §7.2, §9).
pub const TRIANGLES_TARGET: (usize, usize) = (2500, 4000);
/// Lane minions (A5): many on screen at once, so lighter.
pub const MINION_TRIANGLES_TARGET: (usize, usize) = (600, 2600);
pub const TRIANGLES_MAX: usize = 6000;
pub const BONES_TARGET: usize = 64;
pub const BONES_MAX: usize = 80;
pub const INFLUENCES_TARGET: usize = 2;
/// `accent` is the team color; `accent_glow` is the team color, glowing (A5: minions' eyes and
/// orbs). Neither counts toward the four other slots.
pub const MATERIAL_SLOTS: &[&str] = &["skin", "cloth", "metal", "emissive", "accent", "accent_glow"];
pub const MAX_NON_ACCENT_SLOTS: usize = 4;
/// Mesh + animations (200 KB + 600 KB, 10 §9).
pub const MODEL_BYTES_BUDGET: usize = 800 * 1024;
pub const MAX_CLIP_FRAMES: u32 = 12 * FPS;
/// Root motion tolerance (10 §8.4): clips play in place, the sim moves the unit.
pub const ROOT_XZ_TOLERANCE_M: f32 = 0.01;
/// Seamless loops: first and last pose within these.
pub const LOOP_ROT_TOLERANCE_DEG: f32 = 1.0;
pub const LOOP_POS_TOLERANCE_M: f32 = 0.01;
/// Ground contact of the rest pose.
pub const GROUND_TOLERANCE_M: f32 = 0.03;

/// `biped` v1 (10 §7.2): deforming bones, then the rest. Jaw, fingers-as-mittens included.
pub const BIPED_DEFORM: &[&str] = &[
    "pelvis",
    "spine_01",
    "spine_02",
    "chest",
    "neck",
    "head",
    "clavicle_l",
    "upperarm_l",
    "forearm_l",
    "hand_l",
    "fingers_l",
    "thumb_l",
    "clavicle_r",
    "upperarm_r",
    "forearm_r",
    "hand_r",
    "fingers_r",
    "thumb_r",
    "thigh_l",
    "calf_l",
    "foot_l",
    "toe_l",
    "thigh_r",
    "calf_r",
    "foot_r",
    "toe_r",
];
pub const BIPED_OTHER: &[&str] = &["root", "prop_l", "prop_r"];
pub const SOCKETS: &[&str] =
    &["socket_projectile", "socket_weapon_tip", "socket_cast", "socket_chest", "socket_overhead"];
pub const BIPED_OPTIONAL: &[&str] = &["jaw", "smear"];

/// Every name a `biped` skeleton may use: the standard bones, `extra_<chain>_<n>` chains.
pub fn biped_bone_allowed(name: &str) -> bool {
    BIPED_DEFORM.contains(&name)
        || BIPED_OTHER.contains(&name)
        || SOCKETS.contains(&name)
        || BIPED_OPTIONAL.contains(&name)
        || is_extra_bone(name)
}

fn is_extra_bone(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("extra_") else { return false };
    let Some((chain, n)) = rest.rsplit_once('_') else { return false };
    !chain.is_empty()
        && chain.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !n.is_empty()
        && n.bytes().all(|b| b.is_ascii_digit())
}

pub const EXTRA_BONES_MAX: usize = 16;

/// 10 §5: every champion ships these; abilities add `kit_clips`.
pub const REQUIRED_CHAMPION: &[&str] = &[
    "idle",
    "idle_fidget_1",
    "idle_fidget_2",
    "idle_ready",
    "run",
    "run_fast",
    "walk",
    "attack_1",
    "attack_2",
    "cast_utility",
    "recall",
    "death",
    "respawn",
    "select",
    "cc_stunned",
    "cc_rooted",
    "cc_airborne",
    "cc_knockback",
    "cc_suppressed",
    "cc_sleep",
    "cc_forced_move",
    "emote_taunt",
    "emote_joke",
    "emote_laugh",
    "emote_dance",
];
/// 10 §7.3: the shared library.
pub const SHARED_LIBRARY: &[&str] = &[
    "walk",
    "cast_utility",
    "attack_melee_alt",
    "cc_stunned",
    "cc_rooted",
    "cc_airborne",
    "cc_knockback",
    "cc_suppressed",
    "cc_sleep",
    "cc_forced_move",
];
/// A5: every lane minion pack (`art/minions/<kind>`) ships these.
pub const REQUIRED_MINION: &[&str] = &["idle", "run", "attack_1", "death", "flinch"];
pub const MINION_IDS: &[&str] = &["melee", "caster", "siege", "super"];

/// The sim's minion kind a minion pack is for.
pub fn minion_kind(id: &str) -> Option<MinionKind> {
    match id {
        "melee" => Some(MinionKind::Melee),
        "caster" => Some(MinionKind::Caster),
        "siege" => Some(MinionKind::Siege),
        "super" => Some(MinionKind::Super),
        _ => None,
    }
}

/// A minion's attack fire time in frames (its windup, 10 §4.3).
pub fn minion_fire_frame(kind: MinionKind) -> f32 {
    mftr_sim::lane::minion_attack(kind).windup().0 as f32 * FPS as f32 / 1920.0
}

/// Fallbacks the library also carries (A3): champions without their own play these.
pub const LIBRARY_FALLBACKS: &[&str] = &["idle", "run", "death"];
pub const LOCOMOTION: &[&str] = &["walk", "run", "run_fast"];
pub const LAYERS: &[&str] = &["full", "upper", "additive"];
const SLOTS: [&str; 4] = ["q", "w", "e", "r"];

/// Clips an ability kit needs (10 §5.3), derived from the sim's effect kinds.
pub fn kit_clips(def: &ChampionDef) -> Vec<String> {
    let mut out = Vec::new();
    for (slot, ability) in SLOTS.iter().zip(def.abilities.iter()) {
        match ability.effect {
            Effect::Dash(_) | Effect::Lunge(_) => {
                for part in ["start", "travel", "land"] {
                    out.push(format!("{slot}_{part}"));
                }
            }
            _ => out.push(slot.to_string()),
        }
    }
    if def.attack.bolt_speed > 0.0 {
        // Ranged champions need a melee strike for Close Quarters (D46).
        out.push("attack_melee_alt".into());
    }
    out
}

/// The sim's fire time for a clip, in frames at the champion's reference timing (10 §4.3).
pub fn expected_fire_frame(def: &ChampionDef, clip: &str) -> Option<f32> {
    let subticks_to_frames = |d: u64| d as f32 * FPS as f32 / 1920.0;
    if clip == "attack_1" || clip == "attack_2" || clip == "attack_crit" {
        return Some(subticks_to_frames(def.attack.windup().0));
    }
    let slot = SLOTS.iter().position(|s| *s == clip)?;
    match def.abilities[slot].effect {
        Effect::Line(l) => Some(subticks_to_frames(l.windup.0)),
        Effect::Area(a) => Some(subticks_to_frames(a.windup.0)),
        _ => None,
    }
}

/// Markers a clip must carry, from its name (10 §4.5).
pub fn required_markers(clip: &str, looping: bool) -> Vec<&'static str> {
    let mut out = Vec::new();
    let is_slot = |n: &str| SLOTS.contains(&n);
    let base = clip.split('_').next().unwrap_or(clip);
    let fires = clip.starts_with("attack")
        || clip == "cast_utility"
        || is_slot(clip)
        || (is_slot(base) && (clip.ends_with("_start") || clip[2..].bytes().all(|b| b.is_ascii_digit())));
    if fires {
        out.push("fire");
    }
    if !looping {
        out.push("end");
    }
    if LOCOMOTION.contains(&clip) {
        out.extend(["foot_l", "foot_r"]);
    }
    if clip == "cc_airborne" || clip == "recall" {
        out.extend(["loop_in", "loop_out"]);
    }
    out
}

/// Whether `name` is a known marker (fx_/sfx_ cosmetic triggers and hit_<n> included).
pub fn marker_allowed(name: &str) -> bool {
    matches!(name, "fire" | "end" | "loop_in" | "loop_out" | "foot_l" | "foot_r")
        || name.strip_prefix("hit_").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        || name.strip_prefix("fx_").is_some_and(|n| !n.is_empty())
        || name.strip_prefix("sfx_").is_some_and(|n| !n.is_empty())
}
