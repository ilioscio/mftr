//! The committed exports pass, and the validator catches each class of broken pack.

use std::path::PathBuf;

use mftr_pack::{Level, Report, validate, validate_file};

fn art(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../art").join(rel)
}

fn library() -> (Vec<u8>, String) {
    let glb = std::fs::read(art("library/biped/export/biped_library.glb")).unwrap();
    let side = std::fs::read_to_string(art("library/biped/export/biped_library.anims.ron")).unwrap();
    (glb, side)
}

/// Rewrites the GLB's JSON chunk with `f` and repacks the container.
fn edit_json(glb: &[u8], f: impl Fn(String) -> String) -> Vec<u8> {
    let u32_at = |o: usize| u32::from_le_bytes(glb[o..o + 4].try_into().unwrap()) as usize;
    let jlen = u32_at(12);
    let json = String::from_utf8(glb[20..20 + jlen].to_vec()).unwrap();
    let mut new_json = f(json).into_bytes();
    while !new_json.len().is_multiple_of(4) {
        new_json.push(b' ');
    }
    let bin = &glb[20 + jlen..];
    let mut out = Vec::new();
    out.extend_from_slice(&0x4654_6C67u32.to_le_bytes());
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&((12 + 8 + new_json.len() + bin.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(new_json.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F_534Au32.to_le_bytes());
    out.extend_from_slice(&new_json);
    out.extend_from_slice(bin);
    out
}

/// A named edit of the GLB's JSON text.
type JsonEdit = (&'static str, fn(String) -> String);

fn errors(r: &Report) -> Vec<String> {
    r.findings.iter().filter(|f| f.level == Level::Error).map(|f| f.msg.clone()).collect()
}

fn assert_error(r: &Report, needle: &str) {
    let errs = errors(r);
    assert!(errs.iter().any(|e| e.contains(needle)), "expected an error containing {needle:?}, got {errs:#?}");
}

#[test]
fn committed_exports_are_clean() {
    let mut files: Vec<String> = [
        "rigs/export/biped_v1.glb",
        "library/biped/export/biped_library.glb",
        "champions/vesper/export/vesper.glb",
        "champions/rook/export/rook.glb",
        "champions/ember/export/ember.glb",
        "champions/bastion/export/bastion.glb",
        "champions/lumen/export/lumen.glb",
        "champions/shade/export/shade.glb",
        "champions/quill/export/quill.glb",
        "champions/cairn/export/cairn.glb",
    ]
    .map(String::from)
    .to_vec();
    files.extend(["melee", "caster", "siege", "super"].map(|k| format!("minions/{k}/export/{k}.glb")));
    for f in &files {
        let r = validate_file(&art(f));
        assert!(r.findings.is_empty(), "{f}: {:#?}", r.findings);
    }
}

#[test]
fn library_round_trip_has_every_shared_clip_with_markers() {
    let (_, side) = library();
    let s = mftr_pack::sidecar::parse(&side).unwrap();
    for name in mftr_pack::rules::SHARED_LIBRARY {
        assert!(s.clips.iter().any(|c| c.name == *name), "{name}");
    }
    let walk = s.clips.iter().find(|c| c.name == "walk").unwrap();
    assert!(walk.looping && walk.stride_speed == Some(140.0));
    assert_eq!(walk.marker("foot_r"), Some(16));
}

#[test]
fn forbidden_gltf_content_is_rejected() {
    let (glb, side) = library();
    let cases: [JsonEdit; 4] = [
        ("no images", |j| j.replacen("{\"asset\"", "{\"images\":[{\"uri\":\"x.png\"}],\"asset\"", 1)),
        ("extensions are not allowed", |j| j.replacen("{\"asset\"", "{\"extensionsUsed\":[\"KHR_x\"],\"asset\"", 1)),
        ("external URIs", |j| j.replacen("\"buffers\":[{", "\"buffers\":[{\"uri\":\"http://evil/x.bin\",", 1)),
        ("unknown top-level key", |j| j.replacen("{\"asset\"", "{\"scripts\":[],\"asset\"", 1)),
    ];
    for (needle, f) in cases {
        assert_error(&validate(&edit_json(&glb, f), &side), needle);
    }
}

#[test]
fn malformed_containers_are_rejected() {
    let (glb, side) = library();
    assert_error(&validate(&glb[..glb.len() - 4], &side), "length");
    assert_error(&validate(b"not a glb at all", &side), "glTF 2.0 binary");
    let mut huge = glb.clone();
    huge.resize(5 * 1024 * 1024, 0);
    assert_error(&validate(&huge, &side), "cap");
    // An accessor pointing past its buffer view.
    let bad = edit_json(&glb, |j| j.replacen("\"byteOffset\":0,", "\"byteOffset\":99999999,", 1));
    assert!(validate(&bad, &side).errors() > 0);
}

#[test]
fn rig_standard_is_enforced() {
    let (glb, side) = library();
    let renamed = edit_json(&glb, |j| j.replace("\"name\":\"socket_chest\"", "\"name\":\"socket_chesX\""));
    let r = validate(&renamed, &side);
    assert_error(&r, "bone `socket_chesX` is not part of the biped standard");
    assert_error(&r, "required bone `socket_chest` is missing");
    let extra = edit_json(&glb, |j| j.replace("\"name\":\"socket_chest\"", "\"name\":\"extra_cape_1\""));
    assert!(!errors(&validate(&extra, &side)).iter().any(|e| e.contains("extra_cape_1")));
    let mats = edit_json(&glb, |j| j.replace("\"name\":\"metal\"", "\"name\":\"chrome\""));
    assert_error(&validate(&mats, &side), "material `chrome`");
}

#[test]
fn markers_and_clip_metadata_are_checked() {
    let (glb, side) = library();
    let cases: [(&str, String); 7] = [
        ("needs a `fire` marker", side.replace("(name: \"fire\", frame: 9), ", "")),
        ("unknown marker `boom`", side.replace("(name: \"fire\", frame: 9)", "(name: \"boom\", frame: 9)")),
        ("authored at 30 fps", side.replace("fps: 30", "fps: 24")),
        (
            "`loop_in` must come before `loop_out`",
            side.replace("(name: \"loop_in\", frame: 6)", "(name: \"loop_in\", frame: 30)"),
        ),
        ("past the end", side.replace("(name: \"foot_r\", frame: 16)", "(name: \"foot_r\", frame: 99)")),
        (
            "frames in the sidecar but",
            side.replace("(name: \"cc_sleep\", frames: 60", "(name: \"cc_sleep\", frames: 45"),
        ),
        ("positive stride_speed", side.replace("stride_speed: 140.0", "stride_speed: None")),
    ];
    for (needle, s) in cases {
        assert_ne!(s, side, "{needle}: the edit didn't apply");
        assert_error(&validate(&glb, &s), needle);
    }
    let unlisted = side.replace("        (name: \"walk\"", "        (name: \"walk_old\"");
    let r = validate(&glb, &unlisted);
    assert_error(&r, "clip `walk` has no sidecar entry");
    assert_error(&r, "sidecar lists clip `walk_old`");
}

#[test]
fn champion_packs_need_the_full_set_and_their_kit() {
    let (glb, side) = library();
    let s = side.replace("id: \"biped_library\"", "id: \"vesper\"").replace("kind: \"library\"", "kind: \"champion\"");
    let r = validate(&glb, &s);
    let missing = errors(&r).into_iter().find(|e| e.starts_with("missing clips")).expect("missing clips");
    for clip in
        ["idle_ready", "run_fast", "attack_1", "recall", "emote_dance", "q", "w", "e_start", "e_travel", "e_land", "r"]
    {
        assert!(missing.split(", ").any(|c| c.ends_with(clip)), "{clip} not required: {missing}");
    }
}

#[test]
fn fire_frames_follow_the_sim_data() {
    use mftr_sim::ChampionId;
    let vesper = ChampionId::Vesper.def();
    // 18% of 1/0.8 s = 225 ms = 6.75 frames; Longshot winds up 250 ms = 7.5 frames.
    assert!((mftr_pack::rules::expected_fire_frame(vesper, "attack_1").unwrap() - 6.75).abs() < 1e-3);
    assert!((mftr_pack::rules::expected_fire_frame(vesper, "q").unwrap() - 7.5).abs() < 1e-3);
    assert_eq!(mftr_pack::rules::expected_fire_frame(vesper, "e_start"), None);
    assert_eq!(mftr_pack::rules::kit_clips(ChampionId::Rook.def()), ["q", "w", "e_start", "e_travel", "e_land", "r"]);
}

fn parsed_library() -> (mftr_pack::glb::Model, mftr_pack::sidecar::Sidecar, usize) {
    let (glb, side) = library();
    (mftr_pack::glb::parse(&glb).unwrap(), mftr_pack::sidecar::parse(&side).unwrap(), glb.len())
}

#[test]
fn root_motion_is_rejected() {
    let (mut m, s, len) = parsed_library();
    let root = m.node_by_name("root").unwrap();
    let walk = m.animations.iter_mut().find(|a| a.name == "walk").unwrap();
    // Root walks 1.4 m forward over the cycle instead of playing in place.
    walk.channels.push(mftr_pack::glb::Channel {
        node: root,
        path: "translation".into(),
        step: false,
        times: vec![0.0, 32.0 / 30.0],
        values: vec![vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 1.4]],
    });
    assert_error(&mftr_pack::validate_model(&m, len, &s), "root moves 1.400 m");
}

#[test]
fn loop_seams_are_checked() {
    let (mut m, s, len) = parsed_library();
    let walk = m.animations.iter_mut().find(|a| a.name == "walk").unwrap();
    let ch = walk.channels.iter_mut().find(|c| c.path == "rotation" && c.values.len() > 2).unwrap();
    // A 30° pop on the last frame.
    let half = (15.0f32).to_radians();
    *ch.values.last_mut().unwrap() = vec![half.sin(), 0.0, 0.0, half.cos()];
    assert_error(&mftr_pack::validate_model(&m, len, &s), "differs between its first and last frame");
}

#[test]
fn facing_and_sides_are_checked() {
    let (mut m, s, len) = parsed_library();
    // Turn the whole model around (180° about +Y): it now faces -Z and its sides swap.
    for n in m.nodes.iter_mut().filter(|n| n.parent.is_none()) {
        n.rotation = mftr_pack::glb::qmul([0.0, 1.0, 0.0, 0.0], n.rotation);
    }
    let r = mftr_pack::validate_model(&m, len, &s);
    assert_error(&r, "does not face +Z");
    assert_error(&r, "left and right are swapped");
}

#[test]
fn influences_and_weights_are_checked() {
    let (mut m, s, len) = parsed_library();
    let p = &mut m.meshes[0][0];
    p.weights[0] = [0.4, 0.3, 0.3, 0.0];
    p.weights[1] = [0.5, 0.2, 0.0, 0.0];
    let r = mftr_pack::validate_model(&m, len, &s);
    assert!(
        r.findings.iter().any(|f| f.level == Level::Warning && f.msg.contains("more than 2 bone influences")),
        "{:#?}",
        r.findings
    );
    assert_error(&r, "weights sum to 0.700");
}

#[test]
fn committed_vfx_load_and_cover_the_kit() {
    // The library defaults every particle phase; Vesper styles all five of her actions.
    let lib = validate_file(&art("library/biped/export/biped_library.glb"));
    assert!(lib.summary.iter().any(|s| s.starts_with("vfx: ")), "{:?}", lib.summary);
    let events = |rel: &str| -> Vec<String> {
        let loaded = mftr_pack::load_file(&art(rel)).unwrap();
        loaded.vfx.expect("a vfx file").effects.into_iter().map(|e| e.event).collect()
    };
    let lib = events("library/biped/export/biped_library.glb");
    for phase in ["release", "impact", "detonate", "start", "land"] {
        assert!(lib.contains(&format!("*.{phase}")), "library lacks *.{phase}");
    }
    let vesper = events("champions/vesper/export/vesper.glb");
    for event in ["attack.projectile", "q.projectile", "w.projectile", "w.detonate", "e.start", "r.projectile"] {
        assert!(vesper.iter().any(|e| e == event), "vesper lacks {event}");
    }
    // The rig is not a pack and has no effects.
    assert!(!mftr_pack::vfx_path(&art("rigs/export/biped_v1.glb")).exists());
}

#[test]
fn too_many_effects_on_one_event_are_rejected() {
    let one = r#"(event: "q.impact", kit: "burst", ramp: [(1.0, 1.0, 1.0), (0.0, 0.0, 0.0)])"#;
    let text = format!("(effects: [{}])", [one; 5].join(", "));
    let errs = mftr_pack::vfx::check(&mftr_pack::vfx::parse(&text).unwrap());
    assert!(errs.iter().any(|e| e.contains("q.impact")), "{errs:?}");
}

#[test]
fn committed_sounds_decode_and_cover_the_kit() {
    // A4c: the library has footstep variants and the hard-CC accent; Vesper voices her kit.
    let events = |rel: &str| -> Vec<String> {
        let loaded = mftr_pack::load_file(&art(rel)).unwrap();
        assert!(loaded.sounds.iter().all(|s| s.rate == 22_050 && s.channels == 1 && !s.pcm.is_empty()));
        loaded.sounds.into_iter().flat_map(|s| s.spec.events).collect()
    };
    let lib = events("library/biped/export/biped_library.glb");
    assert!(lib.iter().filter(|e| *e == "unit.foot").count() >= 2, "footstep variants");
    for event in ["cc.hard", "unit.death", "unit.respawn", "*.impact", "*.cast"] {
        assert!(lib.iter().any(|e| e == event), "library lacks {event}");
    }
    let vesper = events("champions/vesper/export/vesper.glb");
    for action in ["attack", "q", "w", "r"] {
        for phase in ["cast", "release"] {
            let event = format!("{action}.{phase}");
            assert!(vesper.contains(&event), "vesper lacks {event}");
        }
    }
    for event in ["attack.impact", "q.impact", "w.detonate", "e.start", "e.land", "r.impact"] {
        assert!(vesper.iter().any(|e| e == event), "vesper lacks {event}");
    }
}

#[test]
fn generated_sounds_are_built_from_the_current_recipe() {
    // Edit a `sounds.ron` and forget `mftr-tools sfx build`, and this fails.
    for (recipe, binding) in [
        ("library/biped/sounds.ron", "library/biped/export/biped_library.sfx.ron"),
        ("champions/vesper/sounds.ron", "champions/vesper/export/vesper.sfx.ron"),
        ("champions/rook/sounds.ron", "champions/rook/export/rook.sfx.ron"),
        ("champions/ember/sounds.ron", "champions/ember/export/ember.sfx.ron"),
        ("champions/bastion/sounds.ron", "champions/bastion/export/bastion.sfx.ron"),
        ("champions/lumen/sounds.ron", "champions/lumen/export/lumen.sfx.ron"),
        ("champions/shade/sounds.ron", "champions/shade/export/shade.sfx.ron"),
        ("champions/quill/sounds.ron", "champions/quill/export/quill.sfx.ron"),
        ("champions/cairn/sounds.ron", "champions/cairn/export/cairn.sfx.ron"),
    ] {
        let text = std::fs::read_to_string(art(recipe)).unwrap();
        let file = mftr_pack::sfx::parse(&std::fs::read_to_string(art(binding)).unwrap()).unwrap();
        assert_eq!(
            file.source.as_deref(),
            Some(mftr_pack::sfx::source_hash(&text).as_str()),
            "{binding} is stale: run `mftr-tools sfx build`"
        );
    }
}

#[test]
fn stray_or_broken_sounds_fail_the_pack() {
    let f = mftr_pack::sfx::parse(r#"(sounds: [(name: "a", events: ["q.cast"], volume: 0.5)])"#).unwrap();
    let (_, _, errs) = mftr_pack::sfx::load(&f, |_| Ok(b"OggS not really".to_vec()), &["a".into(), "b".into()]);
    assert!(errs.iter().any(|e| e.starts_with("sfx/a.ogg")), "{errs:?}");
    assert!(errs.iter().any(|e| e.contains("sfx/b.ogg is not bound")), "{errs:?}");
}

#[test]
fn minion_packs_are_checked_against_their_kind() {
    // A5: the id must be a sim minion kind, and `attack_1` fires on that kind's windup.
    let glb = std::fs::read(art("minions/melee/export/melee.glb")).unwrap();
    let side = std::fs::read_to_string(art("minions/melee/export/melee.anims.ron")).unwrap();
    assert!(errors(&validate(&glb, &side)).is_empty());
    let late = side.replace(
        r#"(name: "fire", frame: 7), (name: "end", frame: 24)"#,
        r#"(name: "fire", frame: 12), (name: "end", frame: 24)"#,
    );
    assert_ne!(late, side);
    assert_error(&validate(&glb, &late), "the minion fires at 7.2");
    let goblin = side.replace(r#"id: "melee""#, r#"id: "goblin""#);
    assert_error(&validate(&glb, &goblin), "minion `goblin` must be one of");
    // The siege cart's extra bones are allowed; its wheels loop seamlessly.
    let siege = validate_file(&art("minions/siege/export/siege.glb"));
    assert!(siege.summary[0].contains("38 bones"), "{:?}", siege.summary);
}
