//! `mftr-gdext`: the Godot side of the MFTR client.
//!
//! Godot never decides gameplay outcomes (04 §1). This extension owns the UDP socket and the
//! [`ClientSession`] (clock sync, prediction, interpolation) and exposes render state plus
//! command entry points to GDScript.

use godot::prelude::*;
use mftr_client::blind::{BlindPlan, BlindRating, BlindRecord, RoundStats, TSV_HEADER};
use mftr_client::{ClientSession, Notice, OwnMissileDisplay, Phase, Side};
use mftr_net::conditioner::{LinkProfile, SimLink};
use mftr_sim::ability::{DamageKind, SLOTS};
use mftr_sim::items::{self, INVENTORY};
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
    /// Added latency on top of the real network (blind playtests): (up, down) links.
    links: Option<(SimLink, SimLink)>,
    blind: Option<BlindSession>,
}

/// The blind playtest in progress (03 §14): the plan, the current round, and the stats at
/// the round's start (each round records only its own deltas).
struct BlindSession {
    plan: BlindPlan,
    round: usize,
    started_at: f64,
    base: RoundStats,
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
            links: None,
            blind: None,
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

    // ---- blind playtests (03 §14) --------------------------------------------------------

    /// Start a blind session of `rounds` hidden conditions from `seed` and apply round 1.
    #[func]
    fn blind_begin(&mut self, seed: i64, rounds: i64) {
        let plan = BlindPlan::new(seed as u64, rounds.clamp(1, 100) as usize);
        self.blind = Some(BlindSession { plan, round: 0, started_at: 0.0, base: RoundStats::default() });
        self.blind_apply();
    }

    /// Current round (0-based), or -1 when no blind session is running or it is finished.
    #[func]
    fn blind_round(&self) -> i64 {
        self.blind.as_ref().filter(|b| b.round < b.plan.conditions.len()).map_or(-1, |b| b.round as i64)
    }

    #[func]
    fn blind_rounds(&self) -> i64 {
        self.blind.as_ref().map_or(0, |b| b.plan.conditions.len() as i64)
    }

    /// Restart the current round's clock and stats (call when the tester starts playing it).
    #[func]
    fn blind_start_round(&mut self) {
        let (now, base) = (self.now(), self.round_stats());
        if let Some(b) = self.blind.as_mut() {
            b.started_at = now;
            b.base = base;
        }
    }

    /// Seconds into the current round.
    #[func]
    fn blind_elapsed(&self) -> f64 {
        self.blind.as_ref().map_or(0.0, |b| self.now() - b.started_at)
    }

    /// Record the tester's rating of the current round (appending one line to the TSV file at
    /// `path`, header first if it's new), then apply the next round's hidden condition.
    /// Returns false if the file couldn't be written.
    #[func]
    fn blind_rate(&mut self, fair: bool, responsiveness: i64, notes: GString, path: GString) -> bool {
        let now = self.now();
        let stats = self.round_stats();
        let Some(b) = self.blind.as_mut() else { return false };
        if b.round >= b.plan.conditions.len() {
            return false;
        }
        let delta = RoundStats {
            seconds: now - b.started_at,
            rtt_ms: stats.rtt_ms,
            near_misses: stats.near_misses - b.base.near_misses,
            ghost_hits: stats.ghost_hits - b.base.ghost_hits,
            phantom_hits: stats.phantom_hits - b.base.phantom_hits,
            corrections_over_15: stats.corrections_over_15 - b.base.corrections_over_15,
        };
        let rating = BlindRating {
            dodge_fair: fair,
            responsiveness: responsiveness.clamp(1, 5) as u8,
            notes: notes.to_string(),
        };
        let line = BlindRecord::new(&b.plan, b.round, rating, delta).to_tsv();
        b.round += 1;
        let path = path.to_string();
        let ok = append_line(&path, &line).is_ok();
        if !ok {
            self.last_error = GString::from(&format!("could not write {path}"));
        }
        self.blind_apply();
        ok
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

    /// Spend an ability point on slot 0–3 (Q W E R).
    #[func]
    fn level_up(&mut self, slot: i64) {
        let now = self.now();
        if (0..4).contains(&slot) && self.session.level_up(slot as u8, now).is_some() {
            self.send_input(now);
        }
    }

    /// Buy an item by id (see `shop_catalog`).
    #[func]
    fn buy(&mut self, item: i64) {
        let now = self.now();
        if (1..256).contains(&item) && self.session.buy(item as u8, now).is_some() {
            self.send_input(now);
        }
    }

    /// Sell the item in inventory slot 0–5.
    #[func]
    fn sell(&mut self, slot: i64) {
        let now = self.now();
        if (0..INVENTORY as i64).contains(&slot) && self.session.sell(slot as u8, now).is_some() {
            self.send_input(now);
        }
    }

    /// Every item: `{ id, name, cost, price (with owned components), tier, stats (text),
    /// recipe (ids), affordable, owned }`, in catalog order.
    #[func]
    fn shop_catalog(&self) -> VarArray {
        let mut out = VarArray::new();
        let p = self.session.own_state_now().map(|s| s.progress);
        for it in items::CATALOG.iter() {
            let inv = p.map_or([0; INVENTORY], |p| p.items);
            let price = items::price(it.id, &inv).map_or(it.cost, |(c, _)| c);
            let mut d = VarDictionary::new();
            d.set("id", it.id as i64);
            d.set("name", it.name);
            d.set("cost", it.cost as i64);
            d.set("price", price as i64);
            // 0 = component, 1 = upgraded component or boots, 2 = legendary.
            let tier = if it.recipe.is_empty() {
                0
            } else if it.cost >= 2500.0 {
                2
            } else {
                1
            };
            d.set("tier", tier as i64);
            d.set("stats", item_text(it).as_str());
            let mut recipe = VarArray::new();
            for r in it.recipe {
                recipe.push(&(*r as i64).to_variant());
            }
            d.set("recipe", &recipe);
            d.set("affordable", p.is_some_and(|p| p.gold >= price));
            d.set("owned", inv.contains(&it.id));
            out.push(&d.to_variant());
        }
        out
    }

    /// Undo the last buy or sell while the shop is open.
    #[func]
    fn undo_trade(&mut self) {
        let now = self.now();
        if self.session.undo_trade(now).is_some() {
            self.send_input(now);
        }
    }

    /// Whether buying and selling works right now (dead or in the own fountain, ranked).
    #[func]
    fn can_shop(&self) -> bool {
        self.session.can_shop()
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
            d.set("heal", c.heal);
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

    /// Kills, respawns and blinks since the last call: `{ kind: "died"|"respawned"|"blinked",
    /// unit, killer?, from?, to? }`.
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
                Notice::Reward { gold, xp } => {
                    d.set("kind", "reward");
                    d.set("gold", gold);
                    d.set("xp", xp as i64);
                }
                Notice::MatchEnded { winner } => {
                    d.set("kind", "match_ended");
                    d.set("won", winner == self.session.team());
                }
                Notice::Blinked { unit, from, to } => {
                    d.set("kind", "blinked");
                    d.set("unit", unit.0 as i64);
                    d.set("from", Vector2::new(from.x, from.y));
                    d.set("to", Vector2::new(to.x, to.y));
                }
            }
            out.push(&d.to_variant());
        }
        out
    }

    /// Missiles to draw: `{ key, pos, dir, radius, side: "own"|"ally"|"enemy", impact,
    /// unconfirmed, hard_cc, owner }`.
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
            d.set("owner", m.owner.0 as i64);
            out.push(&d.to_variant());
        }
        out
    }

    /// The match map for drawing: `{ walls: [PackedVector2Array], brush: [PackedVector2Array],
    /// size: Vector2, fountains: [{ center, radius, ally }] }` in game units. Valid once the
    /// phase is "playing".
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
        d.set("size", Vector2::new(map.size.x, map.size.y));
        let mut fountains = VarArray::new();
        for (i, f) in map.layout.fountains.iter().enumerate() {
            if let Some((c, r)) = f {
                let mut fd = VarDictionary::new();
                fd.set("center", Vector2::new(c.x, c.y));
                fd.set("radius", *r);
                fd.set("ally", i == self.session.team() as usize);
                fountains.push(&fd.to_variant());
            }
        }
        d.set("fountains", &fountains);
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
            let (stats, attack) = items::champion_stats(champ.def(), s.progress.level, &s.progress.items);
            d.set("max_health", stats.max_health);
            d.set("attack_damage", stats.attack_damage);
            d.set("ability_power", stats.ability_power);
            d.set("armor", stats.armor);
            d.set("magic_resist", stats.magic_resist);
            d.set("attack_speed", attack.attack_speed);
            d.set("move_speed", stats.move_speed);
            d.set("ability_haste", stats.ability_haste);
            let mut inv = VarArray::new();
            for i in s.progress.items {
                inv.push(&(i as i64).to_variant());
            }
            d.set("items", &inv);
            d.set("can_undo", s.progress.undo_len > 0);
            d.set("shield", if s.shield_until > t { s.shield } else { 0.0 });
            d.set("dead", !s.alive());
            d.set("respawn_in", s.respawn_at.map_or(0.0, |r| r.secs_since(t)));
            let p = s.progress;
            d.set("level", p.level as i64);
            d.set("xp", p.xp as i64);
            d.set("xp_next", mftr_sim::world::xp_to_next(p.level) as i64);
            d.set("gold", p.gold.floor() as i64);
            d.set("points", p.points as i64);
            d.set("ranked", self.session.rules().ranked);
            let mut ranks = VarArray::new();
            let mut can_rank = VarArray::new();
            for slot in 0..4u8 {
                let r = p.ranks[slot as usize];
                ranks.push(&(r as i64).to_variant());
                let ok = p.points > 0 && r < mftr_sim::champion::max_rank(slot, p.level);
                can_rank.push(&ok.to_variant());
            }
            d.set("ranks", &ranks);
            d.set("can_rank", &can_rank);
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
            d.set("turret", matches!(u.kind, UnitKind::RigTurret | UnitKind::Turret));
            d.set(
                "kind",
                match u.kind {
                    UnitKind::Champion => "champion",
                    UnitKind::Minion => "minion",
                    UnitKind::RigTurret | UnitKind::Turret => "turret",
                    UnitKind::Gatehouse => "gatehouse",
                    UnitKind::Base => "base",
                    UnitKind::Relic => "relic",
                },
            );
            d.set("protected", u.protected);
            d.set("champion", u.champion.map_or("", |c| c.def().name));
            d.set("health", u.health);
            d.set("max_health", u.max_health);
            d.set("shield", u.shield);
            d.set("level", u.level as i64);
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

fn append_line(path: &str, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    let new = std::fs::metadata(path).map_or(true, |m| m.len() == 0);
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    if new {
        writeln!(f, "{TSV_HEADER}")?;
    }
    writeln!(f, "{line}")
}

impl MatchClient {
    fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    /// Apply the current blind round's hidden condition (or clear it when the session is over).
    fn blind_apply(&mut self) {
        let condition = self.blind.as_ref().and_then(|b| b.plan.conditions.get(b.round).copied());
        let (profile, own, bubble) = match condition {
            Some(c) => (c.profile, c.own_missiles, c.minion_bubble),
            None => (LinkProfile::PERFECT, OwnMissileDisplay::default(), true),
        };
        self.set_link_profile(profile);
        self.session.set_own_missile_display(own);
        self.session.set_minion_bubble(bubble);
    }

    /// Route packets through simulated links with this profile (none for `perfect`). Packets
    /// in flight on the old links are dropped, like a short loss burst.
    fn set_link_profile(&mut self, profile: LinkProfile) {
        let seed = (self.now() * 1e6) as u64;
        self.links = (profile != LinkProfile::PERFECT)
            .then(|| (SimLink::new(profile, seed * 2 + 1), SimLink::new(profile, seed * 2 + 2)));
    }

    fn round_stats(&self) -> RoundStats {
        let d = self.session.dodge_stats();
        RoundStats {
            seconds: 0.0,
            rtt_ms: self.session.rtt() * 1e3,
            near_misses: d.near_misses,
            ghost_hits: d.ghost_hits,
            phantom_hits: d.phantom_hits,
            corrections_over_15: self.session.stats.corrections.iter().filter(|c| **c > 15.0).count() as u64,
        }
    }

    /// Send a packet now, or into the simulated uplink.
    fn transmit(&mut self, packet: Vec<u8>, now: f64) {
        match (&mut self.links, &self.socket) {
            (Some((up, _)), _) => up.send(packet, now),
            (None, Some(s)) => {
                let _ = s.send(&packet);
            }
            _ => {}
        }
    }

    fn send_input(&mut self, now: f64) {
        let packet = self.session.input_packet(now);
        self.transmit(packet, now);
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
        if let Some((up, down)) = self.links.as_mut() {
            // Added latency (blind playtests): deliver what is due in both directions. Timing
            // granularity is one frame.
            for p in packets.drain(..) {
                down.send(p, now);
            }
            while let Some(p) = down.recv(now) {
                packets.push(p);
            }
            while let Some(p) = up.recv(now) {
                let _ = socket.send(&p);
            }
        }
        for p in packets {
            self.session.handle_packet(&p, now);
        }
        self.session.update(now);
        match self.session.phase() {
            Phase::Connecting if now >= self.next_hello => {
                let hello = self.session.hello_packet(now);
                self.transmit(hello, now);
                self.next_hello = now + 0.25;
            }
            Phase::Playing if self.session.should_send(now) => self.send_input(now),
            _ => {}
        }
    }
}

/// One line describing an item's bonuses and passive.
fn item_text(it: &items::Item) -> String {
    let b = &it.bonus;
    let mut parts: Vec<String> = Vec::new();
    let mut flat = |v: f32, name: &str| {
        if v != 0.0 {
            parts.push(format!("+{v} {name}"));
        }
    };
    flat(b.health, "HP");
    flat(b.health_regen, "HP/s");
    flat(b.armor, "armor");
    flat(b.magic_resist, "MR");
    flat(b.attack_damage, "AD");
    flat(b.ability_power, "AP");
    flat(b.ability_haste, "haste");
    flat(b.move_speed, "MS");
    let mut pct = |v: f32, name: &str| {
        if v != 0.0 {
            parts.push(format!("+{}% {name}", (v * 100.0).round()));
        }
    };
    pct(b.attack_speed, "AS");
    pct(b.life_steal, "life steal");
    pct(b.move_speed_pct, "MS");
    pct(b.ability_power_pct, "AP");
    match it.passive {
        items::Passive::OnHitMagic { base, ap_ratio } => {
            parts.push(format!("attacks deal {base} + {}% AP magic", (ap_ratio * 100.0).round()))
        }
        items::Passive::Lifeline { shield, threshold, cooldown_ms, .. } => parts.push(format!(
            "{shield} shield below {}% HP ({} s cooldown)",
            (threshold * 100.0).round(),
            cooldown_ms / 1000
        )),
        items::Passive::None => {}
    }
    parts.join(", ")
}
