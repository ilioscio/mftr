//! The pose runtime (10 §6) on the committed shared library.
//! (Rotations compare within 0.1°: `acos` of an f32 dot product near 1 reads ~0.04° for equal
//! quaternions.)

use std::path::PathBuf;

use mftr_pack::animator::{Action, ActionKind, AnimEvent, Animator, Drive, Phase};
use mftr_pack::glb::quat_angle_deg;
use mftr_pack::pose::{Library, Pose, add};

fn library() -> Library {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../art/library/biped/export/biped_library.glb");
    mftr_pack::load_file(&path).expect("the committed library loads").library
}

fn rot_diff(a: &Pose, b: &Pose, bone: usize) -> f32 {
    quat_angle_deg(&a[bone].r, &b[bone].r)
}

fn same(a: &Pose, b: &Pose) -> bool {
    (0..a.len()).all(|i| rot_diff(a, b, i) < 0.1 && (0..3).all(|k| (a[i].t[k] - b[i].t[k]).abs() < 1e-4))
}

#[test]
fn the_rig_is_ordered_parents_first() {
    let lib = library();
    assert_eq!(lib.rig.names.len(), 34);
    for (i, p) in lib.rig.parents.iter().enumerate() {
        assert!(p.is_none_or(|p| p < i), "{} before its parent", lib.rig.names[i]);
    }
    assert_eq!(lib.rig.parents.iter().filter(|p| p.is_none()).count(), 1, "one root");
    let upper = lib.rig.subtree("spine_01");
    assert!(upper[lib.rig.bone("hand_r").unwrap()] && !upper[lib.rig.bone("thigh_l").unwrap()]);
}

#[test]
fn sampling_hits_the_authored_keys_and_loops() {
    let lib = library();
    let walk = lib.clip("walk").unwrap();
    let len = lib.clips[walk].length;
    assert!((len - 32.0 / 30.0).abs() < 1e-6);
    let thigh = lib.rig.bone("thigh_l").unwrap();
    // Left contact (frame 0) and right contact (frame 16) swing the left thigh opposite ways.
    let (a, b) = (lib.sample(walk, 0.0), lib.sample(walk, 16.0 / 30.0));
    assert!(rot_diff(&a, &b, thigh) > 30.0);
    assert!(same(&lib.sample(walk, 0.0), &lib.sample(walk, len)), "a loop's end is its start");
    assert!(same(&lib.sample(walk, 0.1), &lib.sample(walk, 0.1 + len)), "and it wraps");
}

#[test]
fn the_release_lands_on_fire_whatever_the_windup() {
    let lib = library();
    let clip = Animator::action_clip(&lib, ActionKind::Attack { variant: 1 }).unwrap();
    assert_eq!(lib.clips[clip].name, "attack_melee_alt", "the fallback for a champion without attacks");
    let fire = lib.clips[clip].fire();
    for steps in [3, 7, 20] {
        let mut anim = Animator::new(&lib);
        for i in 0..=steps {
            let action = Action {
                kind: ActionKind::Attack { variant: 1 },
                phase: Phase::Windup,
                progress: Some(i as f32 / steps as f32),
            };
            anim.update(&lib, &Drive { dt: 1.0 / 60.0, action: Some(action), ..Default::default() });
        }
        let (c, t) = anim.action_time().unwrap();
        assert_eq!(c, clip);
        assert!((t - fire).abs() < 1e-6, "{steps} frames of windup end on fire: {t} vs {fire}");
    }
    // The follow-through spans fire → end.
    let mut anim = Animator::new(&lib);
    let follow = |p| Action { kind: ActionKind::Attack { variant: 1 }, phase: Phase::FollowThrough, progress: Some(p) };
    anim.update(&lib, &Drive { dt: 0.016, action: Some(follow(0.5)), ..Default::default() });
    let mid = (fire + lib.clips[clip].end()) / 2.0;
    assert!((anim.action_time().unwrap().1 - mid).abs() < 1e-6);
}

#[test]
fn champion_speed_runs_at_the_authored_rate_and_standing_still_idles() {
    let lib = library();
    let mut anim = Animator::new(&lib);
    let run = lib.clip("run").unwrap();
    let len = lib.clips[run].length;
    // At 330 u/s (the run's stride speed) one cycle takes exactly the clip's length.
    let mut pose = Pose::new();
    for _ in 0..600 {
        pose = anim.update(&lib, &Drive { dt: len / 600.0, speed: 330.0, ..Default::default() });
    }
    assert!((anim.moving_weight() - 1.0).abs() < 1e-6);
    assert!(same(&pose, &lib.sample(run, 0.0)) || same(&pose, &lib.sample(run, len)), "a full cycle later");
    // Stopping blends to idle within 80 ms.
    for _ in 0..6 {
        anim.update(&lib, &Drive { dt: 0.016, ..Default::default() });
    }
    assert_eq!(anim.moving_weight(), 0.0);
}

#[test]
fn upper_body_casts_leave_the_legs_to_locomotion() {
    let lib = library();
    let mut walking = Animator::new(&lib);
    let mut casting = Animator::new(&lib);
    let cast = Action { kind: ActionKind::Cast { slot: 0 }, phase: Phase::Windup, progress: Some(0.9) };
    let (mut a, mut b) = (Pose::new(), Pose::new());
    for _ in 0..30 {
        a = walking.update(&lib, &Drive { dt: 0.016, speed: 330.0, ..Default::default() });
        b = casting.update(&lib, &Drive { dt: 0.016, speed: 330.0, action: Some(cast), ..Default::default() });
    }
    let (thigh, arm) = (lib.rig.bone("thigh_r").unwrap(), lib.rig.bone("upperarm_r").unwrap());
    assert!(rot_diff(&a, &b, thigh) < 0.1, "the legs keep running");
    assert!(rot_diff(&a, &b, arm) > 10.0, "the arm casts (cast_utility is an upper-body clip)");
}

#[test]
fn a_cut_action_fades_out_in_50_ms_and_death_holds() {
    let lib = library();
    let mut anim = Animator::new(&lib);
    let cast = Action { kind: ActionKind::Cast { slot: 4 }, phase: Phase::FollowThrough, progress: None };
    anim.update(&lib, &Drive { dt: 0.016, action: Some(cast), ..Default::default() });
    assert!(anim.action_time().is_some());
    for _ in 0..4 {
        anim.update(&lib, &Drive { dt: 0.016, ..Default::default() });
    }
    assert!(anim.action_time().is_none(), "gone after 64 ms");

    let death = lib.clip("death").unwrap();
    let mut pose = Pose::new();
    for _ in 0..300 {
        pose = anim.update(&lib, &Drive { dt: 0.016, dead: true, ..Default::default() });
    }
    assert!(same(&pose, &lib.sample(death, lib.clips[death].length)), "lies where it fell");
}

#[test]
fn an_additive_layer_at_rest_changes_nothing() {
    let lib = library();
    let walk = lib.clip("walk").unwrap();
    let mut pose = lib.sample(walk, 0.3);
    let before = pose.clone();
    add(&mut pose, &lib.rig.rest.clone(), &lib.rig.rest, 1.0);
    assert!(same(&pose, &before));
}

#[test]
fn continue_keeps_the_action_showing_and_is_nothing_alone() {
    let lib = library();
    let mut anim = Animator::new(&lib);
    let cont = Action { kind: ActionKind::Continue, phase: Phase::FollowThrough, progress: Some(0.5) };
    anim.update(&lib, &Drive { dt: 0.016, action: Some(cont), ..Default::default() });
    assert!(anim.action_time().is_none(), "nothing to continue");
    let cast = Action { kind: ActionKind::Cast { slot: 0 }, phase: Phase::Windup, progress: Some(1.0) };
    anim.update(&lib, &Drive { dt: 0.016, action: Some(cast), ..Default::default() });
    anim.update(&lib, &Drive { dt: 0.016, action: Some(cont), ..Default::default() });
    let (clip, t) = anim.action_time().unwrap();
    let c = &lib.clips[clip];
    assert!((t - (c.fire() + 0.5 * (c.end() - c.fire()))).abs() < 1e-6, "the cast's follow-through, halfway");
}

fn vesper() -> Library {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../art/champions/vesper/export/vesper.glb");
    mftr_pack::load_file(&path).expect("Vesper's pack loads").library
}

#[test]
fn a_dash_plays_start_travel_and_land() {
    let lib = vesper();
    let (start, travel, land) =
        (lib.clip("e_start").unwrap(), lib.clip("e_travel").unwrap(), lib.clip("e_land").unwrap());
    let mut anim = Animator::new(&lib);
    let mut seen = Vec::new();
    for i in 0..60 {
        let dashing = (5..25).contains(&i);
        anim.update(&lib, &Drive { dt: 1.0 / 60.0, dashing, ..Default::default() });
        if let Some(c) = anim.flourish_clip()
            && seen.last() != Some(&c)
        {
            seen.push(c);
        }
    }
    assert_eq!(seen, [start, travel, land], "{:?}", seen.iter().map(|c| &lib.clips[*c].name).collect::<Vec<_>>());
    assert!(anim.flourish_clip().is_none(), "the landing finished and faded");
}

#[test]
fn haste_runs_fast_and_standing_still_fidgets() {
    let lib = vesper();
    let mut a = Animator::new(&lib);
    let mut b = Animator::new(&lib);
    let (mut pa, mut pb) = (Pose::new(), Pose::new());
    for _ in 0..40 {
        pa = a.update(&lib, &Drive { dt: 0.016, speed: 450.0, ..Default::default() });
        pb = b.update(&lib, &Drive { dt: 0.016, speed: 330.0, ..Default::default() });
    }
    assert!(!same(&pa, &pb), "450 u/s isn't the 330 u/s cycle");

    let mut anim = Animator::new(&lib);
    let fidgets = [lib.clip("idle_fidget_1").unwrap(), lib.clip("idle_fidget_2").unwrap()];
    for _ in 0..(8.2 / 0.02) as usize {
        anim.update(&lib, &Drive { dt: 0.02, ..Default::default() });
    }
    assert!(anim.flourish_clip().is_some_and(|c| fidgets.contains(&c)), "a fidget after 8 s still");
    for _ in 0..5 {
        anim.update(&lib, &Drive { dt: 0.02, speed: 330.0, ..Default::default() });
    }
    assert!(anim.flourish_clip().is_none(), "moving cuts it");
}

#[test]
fn vespers_own_clips_win_over_the_shared_ones() {
    let lib = vesper();
    let c = Animator::action_clip(&lib, ActionKind::Attack { variant: 2 }).unwrap();
    assert_eq!(lib.clips[c].name, "attack_2");
    let c = Animator::action_clip(&lib, ActionKind::Cast { slot: 3 }).unwrap();
    assert_eq!(lib.clips[c].name, "r");
    let c = Animator::action_clip(&lib, ActionKind::Cast { slot: 4 }).unwrap();
    assert_eq!(lib.clips[c].name, "cast_utility", "utility spells use the shared clip");
}

#[test]
fn running_steps_twice_a_cycle_and_actions_announce_their_start() {
    // A4c: footsteps on the run's foot markers, none while dashing; one start per windup.
    let lib = vesper();
    let run = lib.clip("run").unwrap();
    let len = lib.clips[run].length;
    let mut anim = Animator::new(&lib);
    let mut steps = 0;
    for _ in 0..400 {
        anim.update(&lib, &Drive { dt: len / 100.0, speed: 330.0, ..Default::default() });
        steps += anim.events.iter().filter(|e| **e == AnimEvent::Foot).count();
    }
    assert!((7..=8).contains(&steps), "{steps} steps in 4 cycles");
    let mut dashing = 0;
    for _ in 0..200 {
        anim.update(&lib, &Drive { dt: 0.01, dashing: true, speed: 1000.0, ..Default::default() });
        dashing += anim.events.len();
    }
    assert_eq!(dashing, 0);
    let cast = Action { kind: ActionKind::Cast { slot: 0 }, phase: Phase::Windup, progress: Some(0.1) };
    anim.update(&lib, &Drive { dt: 0.016, action: Some(cast), ..Default::default() });
    assert_eq!(anim.events, vec![AnimEvent::Start(ActionKind::Cast { slot: 0 })]);
    anim.update(&lib, &Drive { dt: 0.016, action: Some(Action { progress: Some(0.3), ..cast }), ..Default::default() });
    assert!(anim.events.is_empty());
}

#[test]
fn minions_flinch_when_hit_and_finish_their_swings() {
    // A5: a hit plays the additive `flinch`; an attack whose windup ended plays on to its end
    // while the minion stands (with `finish_attacks`), instead of being cut.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../art/minions/melee/export/melee.glb");
    let pack = mftr_pack::load_file(&path).unwrap();
    assert_eq!(pack.kind, "minion");
    let lib = pack.library;
    let mut calm = Animator::new(&lib);
    let mut hit = Animator::new(&lib);
    calm.update(&lib, &Drive { dt: 0.016, ..Default::default() });
    hit.update(&lib, &Drive { dt: 0.016, hit: true, ..Default::default() });
    let (mut a, mut b) = (Pose::new(), Pose::new());
    for _ in 0..4 {
        a = calm.update(&lib, &Drive { dt: 0.016, ..Default::default() });
        b = hit.update(&lib, &Drive { dt: 0.016, ..Default::default() });
    }
    let head = lib.rig.bone("head").unwrap();
    assert!(rot_diff(&a, &b, head) > 2.0, "the flinch tips the head back");

    let clip = lib.clip("attack_1").unwrap();
    let end = lib.clips[clip].end();
    let attack = Action { kind: ActionKind::Attack { variant: 1 }, phase: Phase::Windup, progress: None };
    for finish in [false, true] {
        let mut anim = Animator::new(&lib);
        anim.finish_attacks = finish;
        for _ in 0..20 {
            anim.update(&lib, &Drive { dt: 0.016, action: Some(attack), ..Default::default() });
        }
        let mut t = 0.0;
        while anim.action_time().is_some() && t < 2.0 {
            anim.update(&lib, &Drive { dt: 0.016, ..Default::default() });
            t += 0.016;
        }
        if finish {
            assert!(t > end - lib.clips[clip].fire() - 0.05, "the swing played out ({t:.2} s)");
        } else {
            assert!(t < 0.1, "cut within the 50 ms blend ({t:.2} s)");
        }
    }
}

fn rook() -> Library {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../art/champions/rook/export/rook.glb");
    mftr_pack::load_file(&path).expect("Rook's pack loads").library
}

#[test]
fn actions_announce_their_fire_once_even_when_cut_at_the_windup_or_instant() {
    // A6: `Fire` when an action passes its `fire` marker: the moment a melee blow lands.
    let lib = rook();
    let fires = |anim: &Animator| anim.events.iter().filter(|e| matches!(e, AnimEvent::Fire(_))).count();
    let attack = ActionKind::Attack { variant: 1 };
    let mut anim = Animator::new(&lib);
    let mut n = 0;
    for i in 0..=10 {
        let action = Action { kind: attack, phase: Phase::Windup, progress: Some(i as f32 / 10.0) };
        anim.update(&lib, &Drive { dt: 0.04, action: Some(action), ..Default::default() });
        n += fires(&anim);
    }
    for _ in 0..10 {
        let action = Action { kind: attack, phase: Phase::FollowThrough, progress: Some(0.5) };
        anim.update(&lib, &Drive { dt: 0.04, action: Some(action), ..Default::default() });
        n += fires(&anim);
    }
    assert_eq!(n, 1, "one blow, one fire");

    // A cast whose follow-through is skipped (the caster walks on) still fired at its windup's end.
    let cleave = ActionKind::Cast { slot: 0 };
    let mut anim = Animator::new(&lib);
    for i in 0..10 {
        let action = Action { kind: cleave, phase: Phase::Windup, progress: Some(i as f32 / 10.0 + 0.05) };
        anim.update(&lib, &Drive { dt: 0.02, action: Some(action), ..Default::default() });
    }
    anim.update(&lib, &Drive { dt: 0.02, speed: 340.0, ..Default::default() });
    assert_eq!(anim.events, vec![AnimEvent::Fire(cleave)]);

    // An instant cast (Second Wind) pulses: its clip plays from `fire` and fires in the next update.
    let wind = ActionKind::Cast { slot: 1 };
    let mut anim = Animator::new(&lib);
    anim.pulse(&lib, wind);
    anim.update(&lib, &Drive { dt: 0.016, ..Default::default() });
    assert_eq!(anim.events, vec![AnimEvent::Fire(wind)]);
    let clip = lib.clip("w").unwrap();
    assert!(anim.action_time().is_some_and(|(c, t)| c == clip && t >= lib.clips[clip].fire()));
    for _ in 0..60 {
        anim.update(&lib, &Drive { dt: 0.016, ..Default::default() });
    }
    assert!(anim.action_time().is_none(), "the pulse played out and blended away");
}
