//! `mftr-gdext`: the Godot side of the MFTR client.
//!
//! Godot never decides gameplay outcomes (04 §1). This extension owns the UDP socket and the
//! [`ClientSession`] (clock sync, prediction, interpolation) and exposes render state plus
//! command entry points to GDScript.

use godot::prelude::*;
use mftr_client::{ClientSession, Notice, Phase, Side};
use mftr_sim::ability::{DamageKind, SLOTS};
use mftr_sim::{ChampionId, Team, UnitId, UnitKind, Vec2};
use std::net::UdpSocket;
use std::time::Instant;

struct MftrExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MftrExtension {}

/// Connection to an `mftr-server`. Add it to the scene, call `connect_to_server`, then read
/// positions every frame. All positions are in game units (1 u = 0.01 m in the client scene).
#[derive(GodotClass)]
#[class(base = Node)]
pub struct MatchClient {
    base: Base<Node>,
    session: ClientSession,
    socket: Option<UdpSocket>,
    clock: Instant,
    next_hello: f64,
    last_error: GString,
    champion_request: Option<ChampionId>,
}

fn side_name(side: Side) -> &'static str {
    match side {
        Side::Own => "own",
        Side::Ally => "ally",
        Side::Enemy => "enemy",
    }
}

#[godot_api]
impl INode for MatchClient {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            session: ClientSession::new(),
            socket: None,
            clock: Instant::now(),
            next_hello: 0.0,
            last_error: GString::new(),
            champion_request: None,
        }
    }

    fn process(&mut self, _delta: f64) {
        self.pump();
    }

    fn exit_tree(&mut self) {
        self.disconnect_from_server();
    }
}

#[godot_api]
impl MatchClient {
    /// Open a UDP socket to `address` ("host:port") and start the handshake.
    #[func]
    fn connect_to_server(&mut self, address: GString) -> bool {
        self.disconnect_from_server();
        let result = UdpSocket::bind("0.0.0.0:0").and_then(|s| {
            s.connect(address.to_string())?;
            s.set_nonblocking(true)?;
            Ok(s)
        });
        match result {
            Ok(s) => {
                self.socket = Some(s);
                self.session = ClientSession::new();
                self.session.set_champion_request(self.champion_request);
                self.next_hello = 0.0;
                self.last_error = GString::new();
                true
            }
            Err(e) => {
                self.last_error = GString::from(&e.to_string());
                false
            }
        }
    }

    #[func]
    fn disconnect_from_server(&mut self) {
        if let Some(s) = self.socket.take() {
            let _ = s.send(&self.session.bye_packet());
        }
    }

    /// "connecting", "joining" or "playing" (empty when disconnected).
    #[func]
    fn phase(&self) -> GString {
        if self.socket.is_none() {
            return GString::new();
        }
        GString::from(match self.session.phase() {
            Phase::Connecting => "connecting",
            Phase::Joining => "joining",
            Phase::Playing => "playing",
        })
    }

    #[func]
    fn last_error(&self) -> GString {
        self.last_error.clone()
    }

    /// Right-click move to a ground point, in game units. Predicted immediately.
    #[func]
    fn move_to(&mut self, target: Vector2) {
        let now = self.now();
        if self.session.move_to(Vec2::new(target.x, target.y), now).is_some() {
            self.send_input(now);
        }
    }

    /// Champion to ask for when connecting ("ember", "vesper"; empty = the server picks).
    /// Returns false for an unknown name.
    #[func]
    fn set_champion(&mut self, name: GString) -> bool {
        let name = name.to_string();
        self.champion_request = ChampionId::by_name(&name);
        name.is_empty() || self.champion_request.is_some()
    }

    /// Cast the ability in `slot` (0–5 = Q W E R D F) toward a ground point (game units).
    /// Windups, missiles, areas, dashes, blinks and shields are predicted at once.
    #[func]
    fn cast(&mut self, slot: i64, target: Vector2) {
        let now = self.now();
        if (0..SLOTS as i64).contains(&slot)
            && self.session.cast(slot as u8, Vec2::new(target.x, target.y), now).is_some()
        {
            self.send_input(now);
        }
    }

    /// Basic-attack a unit by id (chases it into range).
    #[func]
    fn attack_unit(&mut self, id: i64) {
        let now = self.now();
        if self.session.attack(UnitId(id as u32), now).is_some() {
            self.send_input(now);
        }
    }

    /// Attack-move toward a ground point.
    #[func]
    fn attack_move(&mut self, target: Vector2) {
        let now = self.now();
        if self.session.attack_move(Vec2::new(target.x, target.y), now).is_some() {
            self.send_input(now);
        }
    }

    /// The enemy whose drawn hitbox contains a ground point (plus `slack` u), or -1.
    #[func]
    fn pick_enemy(&self, at: Vector2, slack: f32) -> i64 {
        self.session.pick_enemy(Vec2::new(at.x, at.y), slack, self.now()).map_or(-1, |id| id.0 as i64)
    }

    /// Ground areas: `{ key, center, radius, side, progress (0..1 to detonation), detonated }`.
    #[func]
    fn areas(&self) -> VarArray {
        let mut out = VarArray::new();
        for a in self.session.areas_render(self.now()) {
            let mut d = VarDictionary::new();
            d.set("key", a.key as i64);
            d.set("center", Vector2::new(a.center.x, a.center.y));
            d.set("radius", a.radius);
            d.set("side", side_name(a.side));
            d.set("progress", a.progress);
            d.set("detonated", a.detonated);
            out.push(&d.to_variant());
        }
        out
    }

    /// Basic-attack bolts in flight: `{ key, pos, dir, side }`.
    #[func]
    fn bolts(&self) -> VarArray {
        let mut out = VarArray::new();
        for b in self.session.bolts_render(self.now()) {
            let mut d = VarDictionary::new();
            d.set("key", b.key as i64);
            d.set("pos", Vector2::new(b.pos.x, b.pos.y));
            d.set("dir", Vector2::new(b.dir.x, b.dir.y));
            d.set("side", side_name(b.side));
            out.push(&d.to_variant());
        }
        out
    }

    /// Confirmed damage since the last call: `{ target, source, amount, absorbed, kind }`
    /// (`kind`: "physical" | "magic" | "true"). Call once per frame.
    #[func]
    fn take_combat_text(&mut self) -> VarArray {
        let mut out = VarArray::new();
        for c in self.session.take_combat_text() {
            let mut d = VarDictionary::new();
            d.set("target", c.target.0 as i64);
            d.set("source", c.source.0 as i64);
            d.set("amount", c.amount);
            d.set("absorbed", c.absorbed);
            d.set(
                "kind",
                match c.kind {
                    DamageKind::Physical => "physical",
                    DamageKind::Magic => "magic",
                    DamageKind::True => "true",
                },
            );
            out.push(&d.to_variant());
        }
        out
    }

    /// Kills and respawns since the last call: `{ kind: "died"|"respawned", unit, killer }`.
    #[func]
    fn take_notices(&mut self) -> VarArray {
        let mut out = VarArray::new();
        for n in self.session.take_notices() {
            let mut d = VarDictionary::new();
            match n {
                Notice::Died { unit, killer } => {
                    d.set("kind", "died");
                    d.set("unit", unit.0 as i64);
                    d.set("killer", killer.0 as i64);
                }
                Notice::Respawned { unit } => {
                    d.set("kind", "respawned");
                    d.set("unit", unit.0 as i64);
                }
            }
            out.push(&d.to_variant());
        }
        out
    }

    /// Missiles to draw: `{ key, pos, dir, radius, side: "own"|"ally"|"enemy", impact,
    /// unconfirmed, hard_cc }`.
    #[func]
    fn missiles(&self) -> VarArray {
        let mut out = VarArray::new();
        for m in self.session.missiles_render(self.now()) {
            let mut d = VarDictionary::new();
            d.set("key", m.key as i64);
            d.set("pos", Vector2::new(m.pos.x, m.pos.y));
            d.set("dir", Vector2::new(m.dir.x, m.dir.y));
            d.set("radius", m.radius);
            d.set("side", side_name(m.side));
            d.set("impact", m.impact);
            d.set("unconfirmed", m.unconfirmed);
            d.set("hard_cc", m.hard_cc);
            out.push(&d.to_variant());
        }
        out
    }

    /// The match map for drawing: `{ walls: [PackedVector2Array], brush: [PackedVector2Array] }`
    /// in game units. Valid once the phase is "playing".
    #[func]
    fn map_geometry(&self) -> VarDictionary {
        let to_arrays = |polys: &[Vec<Vec2>]| {
            let mut arr = VarArray::new();
            for p in polys {
                let pts: PackedVector2Array = p.iter().map(|v| Vector2::new(v.x, v.y)).collect();
                arr.push(&pts.to_variant());
            }
            arr
        };
        let map = self.session.map();
        let mut d = VarDictionary::new();
        d.set("walls", &to_arrays(&map.walls).to_variant());
        d.set("brush", &to_arrays(&map.brush).to_variant());
        d
    }

    /// Own champion on the input timeline: `{ champion, health, max_health, shield, dead,
    /// respawn_in, stunned, rooted, casting, attacking, dashing, cooldowns: [6 × seconds],
    /// abilities: [6 × name] }`. Health and shield are predicted; damage arrives from the server.
    #[func]
    fn own_status(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let now = self.now();
        let champ = self.session.champion();
        d.set("champion", champ.def().name);
        let mut names = VarArray::new();
        for slot in 0..SLOTS as u8 {
            names.push(&champ.ability(slot).map_or("", |a| a.name).to_variant());
        }
        d.set("abilities", &names);
        if let (Some(s), Some(t)) = (self.session.own_state_now(), self.session.input_sim_time(now)) {
            d.set("health", s.health);
            d.set("max_health", champ.def().stats.max_health);
            d.set("shield", if s.shield_until > t { s.shield } else { 0.0 });
            d.set("dead", !s.alive());
            d.set("respawn_in", s.respawn_at.map_or(0.0, |r| r.secs_since(t)));
            d.set("stunned", s.stunned_until > t);
            d.set("rooted", s.rooted_until > t);
            d.set("casting", s.cast.is_some());
            d.set("attacking", s.attack.is_some());
            d.set("dashing", s.dash.is_some());
            let mut cds = VarArray::new();
            for c in s.cooldowns {
                cds.push(&c.secs_since(t).to_variant());
            }
            d.set("cooldowns", &cds);
        }
        d
    }

    #[func]
    fn stop(&mut self) {
        let now = self.now();
        if self.session.stop(now).is_some() {
            self.send_input(now);
        }
    }

    #[func]
    fn own_unit_id(&self) -> i64 {
        self.session.unit().0 as i64
    }

    /// Own champion on the input timeline (03a §3), including correction smoothing.
    #[func]
    fn own_position(&self) -> Vector2 {
        let p = self.session.own_render_position(self.now()).unwrap_or(Vec2::ZERO);
        Vector2::new(p.x, p.y)
    }

    /// Remote units to draw. Each entry: `{ id, pos: Vector2, minion, turret, champion (name or
    /// ""), red, ally, radius (collision), gameplay_radius, health, max_health, shield,
    /// stunned, rooted, attacking, dashing, windup?, windup_dir? }`.
    /// Champions are on `T_interp`; minions near us blend toward `T_input` (03a §5).
    #[func]
    fn remote_units(&self) -> VarArray {
        let mut out = VarArray::new();
        for u in self.session.remote_render_units(self.now()) {
            let mut d = VarDictionary::new();
            d.set("id", u.id.0 as i64);
            d.set("pos", Vector2::new(u.pos.x, u.pos.y));
            d.set("minion", u.kind == UnitKind::Minion);
            d.set("turret", u.kind == UnitKind::Turret);
            d.set("champion", u.champion.map_or("", |c| c.def().name));
            d.set("health", u.health);
            d.set("max_health", u.max_health);
            d.set("shield", u.shield);
            d.set("gameplay_radius", u.gameplay_radius);
            d.set("attacking", u.attacking);
            d.set("rooted", u.rooted);
            d.set("dashing", u.dashing);
            d.set("stunned", u.stunned);
            if let Some((p, dir)) = u.windup {
                d.set("windup", p);
                d.set("windup_dir", Vector2::new(dir.x, dir.y));
            }
            d.set("red", u.team == Team::Red);
            d.set("ally", u.team == self.session.team());
            d.set("radius", u.collision_radius);
            out.push(&d.to_variant());
        }
        out
    }

    /// Turn client collision proxies and the minion bubble on or off (A/B testing, 03a §12).
    #[func]
    fn set_collision_proxies(&mut self, enabled: bool) {
        self.session.set_collision_proxies(enabled);
    }

    /// Net graph data (03 §14).
    #[func]
    fn net_stats(&self) -> VarDictionary {
        let s = &self.session;
        let st = &s.stats;
        let mut d = VarDictionary::new();
        d.set("rtt_ms", s.rtt() * 1e3);
        d.set("margin_ms", s.margin() * 1e3);
        d.set("interp_ms", s.interp_buffer() * 1e3);
        d.set("commands", st.commands_issued as i64);
        d.set("late", st.commands_late as i64);
        d.set("mismatches", st.mismatches as i64);
        d.set("corrections", st.corrections.len() as i64);
        d.set("last_correction", st.corrections.last().copied().unwrap_or(0.0));
        d.set("visible_correction", s.visible_correction());
        d.set("kb_up", st.bytes_up as f64 / 1024.0);
        d.set("kb_down", st.bytes_down as f64 / 1024.0);
        let dodge = s.dodge_stats();
        d.set("enemy_missiles", dodge.enemy_missiles as i64);
        d.set("near_misses", dodge.near_misses as i64);
        d.set("ghost_hits", dodge.ghost_hits as i64);
        d.set("phantom_hits", dodge.phantom_hits as i64);
        d.set("kills", st.kills as i64);
        d.set("deaths", st.deaths as i64);
        d
    }
}

impl MatchClient {
    fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    fn send_input(&mut self, now: f64) {
        let packet = self.session.input_packet(now);
        if let Some(s) = &self.socket {
            let _ = s.send(&packet);
        }
    }

    /// Drain the socket, advance the session, and send whatever is due.
    fn pump(&mut self) {
        let Some(socket) = self.socket.as_ref() else { return };
        let mut buf = [0u8; 2048];
        let mut packets = Vec::new();
        loop {
            match socket.recv(&mut buf) {
                Ok(n) => packets.push(buf[..n].to_vec()),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                // Windows reports ICMP "port unreachable" as a reset; the server may just not be up yet.
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => {
                    self.last_error = GString::from(&e.to_string());
                    break;
                }
            }
        }
        let now = self.now();
        for p in packets {
            self.session.handle_packet(&p, now);
        }
        self.session.update(now);
        match self.session.phase() {
            Phase::Connecting if now >= self.next_hello => {
                let hello = self.session.hello_packet(now);
                if let Some(s) = &self.socket {
                    let _ = s.send(&hello);
                }
                self.next_hello = now + 0.25;
            }
            Phase::Playing if self.session.should_send(now) => self.send_input(now),
            _ => {}
        }
    }
}
