//! `mftr-gdext`: the Godot side of the MFTR client.
//!
//! Godot never decides gameplay outcomes (04 §1). This extension owns the UDP socket, the
//! secure transport (D40) and the [`ClientSession`] (clock sync, prediction, interpolation) and
//! exposes render state plus command entry points to GDScript.
//!
//! The player's identity key lives in `user://identity.key`, and the servers it trusts in
//! `user://known_servers.txt`: a server's key is pinned the first time the client meets it (or
//! up front with `host:port#fingerprint`), and a server whose key changed is refused.

use godot::classes::ProjectSettings;
use godot::prelude::*;
use mftr_client::blind::{BlindPlan, BlindRating, BlindRecord, RoundStats, TSV_HEADER};
use mftr_client::{ClientSession, Notice, OwnMissileDisplay, Phase, Side};
use mftr_net::conditioner::{LinkProfile, SimLink};
use mftr_net::secure::{Identity, KnownServers, SecureClient, SecureError, parse_address};
use mftr_sim::ability::{DamageKind, SLOTS};
use mftr_sim::augments;
use mftr_sim::items::{self, INVENTORY};
use mftr_sim::{ChampionId, Team, UnitId, UnitKind, Vec2};
use std::net::UdpSocket;
use std::time::Instant;

mod model;

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
    /// The encrypted session on that socket.
    secure: Option<SecureClient>,
    identity: Option<Identity>,
    /// The server's address ("host:port"), and whether its key is saved as trusted.
    server: String,
    trusted: bool,
    clock: Instant,
    next_hello: f64,
    last_error: GString,
    champion_request: Option<ChampionId>,
    /// Reconnect token and spectate wish for the next connection.
    resume: u64,
    spectate: bool,
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

impl MatchClient {
    /// Every champion this client knows about, by unit: its own and the remote ones.
    fn champions_by_unit(&self) -> std::collections::BTreeMap<UnitId, ChampionId> {
        let mut out: std::collections::BTreeMap<UnitId, ChampionId> = self
            .session
            .remote_render_units(self.now())
            .into_iter()
            .filter_map(|u| u.champion.map(|c| (u.id, c)))
            .collect();
        if !self.session.is_spectator() {
            out.insert(self.session.unit(), self.session.champion());
        }
        out
    }
}

const ACTIONS: [&str; SLOTS] = ["q", "w", "e", "r", "d", "f"];

/// Which ability of `champion` produced a line of `radius` (A4b: picks its VFX). Matches the
/// kit's widths, Broadside-widened too; `None` if nothing matches (turret shots).
fn line_action(champion: ChampionId, radius: f32) -> Option<&'static str> {
    (0..SLOTS as u8).find_map(|slot| match champion.ability(slot)?.effect {
        mftr_sim::ability::Effect::Line(l)
            if (l.radius - radius).abs() < 0.5 || (l.radius * augments::WIDE_LINE - radius).abs() < 0.5 =>
        {
            Some(ACTIONS[slot as usize])
        }
        _ => None,
    })
}

/// The same for a delayed area of `radius` that detonates `delay` after it spawns; the delay
/// tells apart areas of the same radius (Marrow's Siphon and Ossuary).
fn area_action(champion: ChampionId, radius: f32, delay: mftr_sim::SimDuration) -> Option<&'static str> {
    let fits = |slot: u8, timed: bool| match champion.ability(slot)?.effect {
        mftr_sim::ability::Effect::Area(a)
            if ((a.radius - radius).abs() < 0.5 || (a.radius * augments::WIDE_AREA - radius).abs() < 0.5)
                && (!timed || a.delay == delay) =>
        {
            Some(ACTIONS[slot as usize])
        }
        _ => None,
    };
    (0..SLOTS as u8).find_map(|slot| fits(slot, true)).or_else(|| (0..SLOTS as u8).find_map(|slot| fits(slot, false)))
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
            secure: None,
            identity: None,
            server: String::new(),
            trusted: false,
            clock: Instant::now(),
            next_hello: 0.0,
            last_error: GString::new(),
            champion_request: None,
            resume: 0,
            spectate: false,
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
    /// Open a UDP socket to `address` ("host:port", or "host:port#fingerprint" to pin the
    /// server's key) and start the handshake.
    #[func]
    fn connect_to_server(&mut self, address: GString) -> bool {
        self.disconnect_from_server();
        let address = address.to_string();
        let (host, pin) = match parse_address(&address) {
            Ok(v) => v,
            Err(e) => {
                self.last_error = GString::from(&e);
                return false;
            }
        };
        let pin = pin.or_else(|| known_servers().get(host));
        let identity = match self.identity() {
            Ok(i) => i,
            Err(e) => {
                self.last_error = GString::from(&format!("could not read or create the identity key: {e}"));
                return false;
            }
        };
        let result = UdpSocket::bind("0.0.0.0:0").and_then(|s| {
            s.connect(host)?;
            s.set_nonblocking(true)?;
            Ok(s)
        });
        match result {
            Ok(s) => {
                self.socket = Some(s);
                self.secure = Some(SecureClient::new(&identity, pin));
                self.server = host.to_string();
                self.trusted = false;
                self.session = ClientSession::new();
                self.session.set_champion_request(self.champion_request);
                self.session.set_resume(self.resume);
                self.session.set_spectate(self.spectate);
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
        let bye = self.session.bye_packet();
        if let (Some(s), Some(secure)) = (self.socket.take(), self.secure.as_mut()) {
            for d in secure.seal(&bye) {
                let _ = s.send(&d);
            }
        }
        self.secure = None;
    }

    /// What the server runs, for the server list: "ARAM", "Duel Sandbox", ... (empty until
    /// champion select or the welcome; champion select only exists in ARAM).
    #[func]
    fn game_type(&self) -> GString {
        if self.socket.is_none() {
            return GString::new();
        }
        let mode = self.session.game_mode();
        GString::from(match self.session.phase() {
            Phase::Connecting => "",
            Phase::Lobby if mode == mftr_net::msg::GameMode::Empty => mftr_net::msg::GameMode::Aram.label(),
            _ => mode.label(),
        })
    }

    /// Every champion's name, for pickers.
    #[func]
    fn champion_names(&self) -> PackedStringArray {
        ChampionId::ALL.iter().map(|c| GString::from(c.def().name)).collect()
    }

    /// The server's key fingerprint (empty until its handshake reply arrives).
    #[func]
    fn server_fingerprint(&self) -> GString {
        let fp = self.secure.as_ref().and_then(|s| s.server_fingerprint());
        fp.map_or_else(GString::new, |f| GString::from(&f.to_string()))
    }

    /// "connecting", "joining" or "playing" (empty when disconnected).
    #[func]
    fn phase(&self) -> GString {
        if self.socket.is_none() {
            return GString::new();
        }
        GString::from(match self.session.phase() {
            Phase::Connecting => "connecting",
            Phase::Lobby => "lobby",
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

    /// Reconnect: the token (hex, from `session_token` of an earlier session) to present when
    /// connecting. Empty = a new player.
    #[func]
    fn set_resume_token(&mut self, token: GString) {
        self.resume = u64::from_str_radix(&token.to_string(), 16).unwrap_or(0);
    }

    /// This session's token (hex; empty before the Welcome): save it to reconnect later.
    #[func]
    fn session_token(&self) -> GString {
        match self.session.token() {
            0 => GString::new(),
            t => GString::from(&format!("{t:016x}")),
        }
    }

    /// Watch instead of play (set before connecting).
    #[func]
    fn set_spectate(&mut self, spectate: bool) {
        self.spectate = spectate;
    }

    #[func]
    fn is_spectator(&self) -> bool {
        self.session.is_spectator()
    }

    /// Champion select: `{ you, starts_in, bench: [names], slots: [{ player, team, ally, you,
    /// champion, rerolls, ready, bot }] }`, or an empty dictionary outside it.
    #[func]
    fn lobby_state(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Some(l) = self.session.lobby() else { return d };
        let my_team = l.slots.iter().find(|s| s.player == l.you).map(|s| s.team);
        d.set("you", l.you.0 as i64);
        d.set("starts_in", l.starts_in_ms as f64 / 1000.0);
        let mut bench = VarArray::new();
        for c in &l.bench {
            bench.push(&c.def().name.to_variant());
        }
        d.set("bench", &bench);
        let mut slots = VarArray::new();
        for s in &l.slots {
            let mut e = VarDictionary::new();
            e.set("player", s.player.0 as i64);
            e.set("team", if s.team == Team::Blue { "blue" } else { "red" });
            e.set("ally", Some(s.team) == my_team);
            e.set("you", s.player == l.you);
            e.set("champion", s.champion.def().name);
            e.set("rerolls", s.rerolls as i64);
            e.set("ready", s.ready);
            e.set("bot", s.bot);
            slots.push(&e.to_variant());
        }
        d.set("slots", &slots);
        d
    }

    #[func]
    fn lobby_reroll(&mut self) {
        self.lobby_action(mftr_net::msg::LobbyAction::Reroll);
    }

    /// Take a champion from the team bench (by name).
    #[func]
    fn lobby_take(&mut self, name: GString) {
        if let Some(c) = ChampionId::by_name(&name.to_string()) {
            self.lobby_action(mftr_net::msg::LobbyAction::Take(c));
        }
    }

    #[func]
    fn lobby_ready(&mut self, ready: bool) {
        self.lobby_action(mftr_net::msg::LobbyAction::Ready(ready));
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

    /// Keep choice 0–2 of the open augment draft (ARAM: Mayhem).
    #[func]
    fn pick_augment(&mut self, choice: i64) {
        let now = self.now();
        if (0..augments::CHOICES as i64).contains(&choice) && self.session.pick_augment(choice as u8, now).is_some() {
            self.send_input(now);
        }
    }

    /// Buy a Stat Anvil (Mayhem, level 9+, while shopping).
    #[func]
    fn buy_anvil(&mut self) {
        let now = self.now();
        if self.session.buy_anvil(now).is_some() {
            self.send_input(now);
        }
    }

    /// Keep choice 0–2 of the open anvil.
    #[func]
    fn pick_anvil(&mut self, choice: i64) {
        let now = self.now();
        if (0..mftr_sim::anvils::CHOICES as i64).contains(&choice)
            && self.session.pick_anvil(choice as u8, now).is_some()
        {
            self.send_input(now);
        }
    }

    /// Use the item in inventory slot 0–5 (keys 1–6): drink a potion.
    #[func]
    fn use_item(&mut self, slot: i64) {
        let now = self.now();
        if (0..INVENTORY as i64).contains(&slot) && self.session.use_item(slot as u8, now).is_some() {
            self.send_input(now);
        }
    }

    /// Reroll choice 0–2 of the open augment draft (each once per draft).
    #[func]
    fn reroll_augment(&mut self, choice: i64) {
        let now = self.now();
        if (0..augments::CHOICES as i64).contains(&choice) && self.session.reroll_augment(choice as u8, now).is_some() {
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
            d.set("consumable", items::consumable(it.id).is_some());
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
        let champions = self.champions_by_unit();
        for a in self.session.areas_render(self.now()) {
            let mut d = VarDictionary::new();
            let champion = champions.get(&a.owner).copied();
            d.set("owner", a.owner.0 as i64);
            d.set("champion", champion.map_or("", |c| c.def().name));
            d.set("action", champion.and_then(|c| area_action(c, a.radius, a.delay)).unwrap_or(""));
            d.set("key", a.key as i64);
            d.set("center", Vector2::new(a.center.x, a.center.y));
            d.set("radius", a.radius);
            d.set("side", side_name(a.side));
            d.set("progress", a.progress);
            d.set("detonated", a.detonated);
            d.set("hard_cc", a.hard_cc);
            out.push(&d.to_variant());
        }
        out
    }

    /// Basic-attack bolts in flight: `{ key, pos, dir, side }`.
    #[func]
    fn bolts(&self) -> VarArray {
        let mut out = VarArray::new();
        let champions = self.champions_by_unit();
        for b in self.session.bolts_render(self.now()) {
            let mut d = VarDictionary::new();
            d.set("owner", b.owner.0 as i64);
            d.set("champion", champions.get(&b.owner).map_or("", |c| c.def().name));
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
        let champions = self.champions_by_unit();
        for m in self.session.missiles_render(self.now()) {
            let mut d = VarDictionary::new();
            let champion = champions.get(&m.owner).copied();
            d.set("champion", champion.map_or("", |c| c.def().name));
            d.set("action", champion.and_then(|c| line_action(c, m.radius)).unwrap_or(""));
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

    /// Everything a tooltip says about our champion's ability in `slot` (0–3 Q W E R, 4–5 D F;
    /// F may be an augment's spell), straight from the sim's data, so tooltips can't drift
    /// from the game: `{ name, kind, rank, max_rank, cooldowns: [s per rank], range, radius,
    /// speed, windup, delay, damage: { kind, base: [per rank], ad, ap, total }, heal: {…},
    /// shield: {…}, cc: { kind, seconds, pct } }`, with absent parts left out. `total`s use the
    /// champion's current attack damage and ability power at its current rank (rank 1 if
    /// unlearned).
    #[func]
    fn ability_info(&self, slot: i64) -> VarDictionary {
        use mftr_sim::ability::{Cc, Effect};
        let mut d = VarDictionary::new();
        let Some(st) = self.session.own_state_now() else { return d };
        let champ = self.session.champion();
        let slot = slot.clamp(0, 5) as u8;
        let spell = if slot == 5 { augments::spell(&st.progress.augments) } else { None };
        let Some(a) = spell.or_else(|| champ.ability(slot)) else { return d };
        let (stats, _) = items::champion_stats(champ.def(), &st.progress.stats_key());
        let ranked = slot < 4 && self.session.rules().ranked;
        let max_rank: u8 = if !ranked {
            1
        } else if slot == 3 {
            3
        } else {
            5
        };
        let rank = if slot < 4 { st.progress.ranks[slot as usize] } else { 1 };
        d.set("name", a.name);
        d.set("rank", rank as i64);
        d.set("max_rank", max_rank as i64);
        d.set("ranked", ranked);
        let per_rank = |f: &dyn Fn(u8) -> f32| -> PackedFloat32Array { (1..=max_rank).map(f).collect() };
        // Ability haste (items, augments, Hyper) shortens Q W E R as the sim does when casting.
        let hyper = if slot < 3 && st.progress.hyper { mftr_sim::world::HYPER_HASTE } else { 0.0 };
        let haste = if slot < 4 { stats.ability_haste + hyper } else { 0.0 };
        d.set("haste", haste);
        d.set(
            "cooldowns",
            &per_rank(&|r| {
                a.cooldown_at(r).0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32 * 100.0 / (100.0 + haste)
            }),
        );
        let secs = |t: mftr_sim::SimDuration| t.0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
        let now_rank = rank.max(1);
        let damage = |dmg: mftr_sim::ability::Damage| -> VarDictionary {
            let mut m = VarDictionary::new();
            m.set(
                "kind",
                match dmg.kind {
                    DamageKind::Physical => "physical",
                    DamageKind::Magic => "magic",
                    DamageKind::True => "true",
                },
            );
            m.set("base", &per_rank(&|r| dmg.base + a.bonus_damage_at(r)));
            m.set("ad", dmg.ad_ratio);
            m.set("ap", dmg.ap_ratio);
            m.set("total", dmg.raw(stats.attack_damage, stats.ability_power) + a.bonus_damage_at(now_rank));
            m
        };
        let cc = |c: Cc| -> Option<VarDictionary> {
            let mut m = VarDictionary::new();
            let (kind, t, pct) = match c {
                Cc::None => return None,
                Cc::Stun(t) => ("stun", secs(t), 0),
                Cc::Root(t) => ("root", secs(t), 0),
                Cc::Knockup(t) => ("knockup", secs(t), 0),
                Cc::Pull(_) => ("pull", 0.0, 0),
                Cc::Slow { pct, duration } => ("slow", secs(duration), pct),
            };
            m.set("kind", kind);
            m.set("seconds", t);
            m.set("pct", pct as i64);
            Some(m)
        };
        let set_cc = |d: &mut VarDictionary, c: Cc| {
            if let Some(m) = cc(c) {
                d.set("cc", &m);
            }
        };
        match a.effect {
            Effect::Line(l) => {
                d.set("kind", "line");
                d.set("range", l.range);
                d.set("radius", l.radius);
                d.set("speed", l.speed);
                d.set("windup", secs(l.windup));
                if l.damage.base > 0.0 || l.damage.ad_ratio > 0.0 || l.damage.ap_ratio > 0.0 {
                    d.set("damage", &damage(l.damage));
                }
                set_cc(&mut d, l.cc);
            }
            Effect::Area(r) => {
                d.set("kind", if r.range == 0.0 { "nova" } else { "area" });
                d.set("range", r.range);
                d.set("radius", r.radius);
                d.set("windup", secs(r.windup));
                d.set("delay", secs(r.delay));
                d.set("damage", &damage(r.damage));
                set_cc(&mut d, r.cc);
            }
            Effect::Lunge(l) => {
                d.set("kind", "lunge");
                d.set("range", l.range);
                d.set("speed", l.speed);
                d.set("damage", &damage(l.damage));
                set_cc(&mut d, l.cc);
            }
            Effect::Dash(x) => {
                d.set("kind", "dash");
                d.set("range", x.range);
                d.set("speed", x.speed);
            }
            Effect::Blink(b) => {
                d.set("kind", "blink");
                d.set("range", b.range);
            }
            Effect::Shield(sh) => {
                d.set("kind", "barrier");
                let mut m = VarDictionary::new();
                m.set("base", &per_rank(&|_| sh.amount));
                m.set("ap", 0.0f32);
                m.set("total", sh.amount);
                m.set("seconds", secs(sh.duration));
                d.set("shield", &m);
            }
            Effect::Support(sup) => {
                d.set("kind", "support");
                d.set("range", sup.range);
                if sup.heal > 0.0 || sup.heal_missing > 0.0 {
                    let extra = |r: u8| if sup.shield > 0.0 { 0.0 } else { a.bonus_damage_at(r) };
                    let mut m = VarDictionary::new();
                    m.set("base", &per_rank(&|r| sup.heal + extra(r)));
                    m.set("ap", sup.heal_ap);
                    m.set("missing_pct", sup.heal_missing * 100.0);
                    m.set("total", sup.heal + sup.heal_ap * stats.ability_power + extra(now_rank));
                    d.set("heal", &m);
                }
                if sup.shield > 0.0 {
                    let mut m = VarDictionary::new();
                    m.set("base", &per_rank(&|r| sup.shield + a.bonus_damage_at(r)));
                    m.set("ap", sup.shield_ap);
                    m.set("total", sup.shield + sup.shield_ap * stats.ability_power + a.bonus_damage_at(now_rank));
                    m.set("seconds", secs(sup.duration));
                    d.set("shield", &m);
                }
            }
        }
        d
    }

    /// What our team sees, for the minimap and the fog drawn over the world: `{ cols, rows,
    /// cell, data }`, one byte per `cell`-unit square of the map (row-major from the map's
    /// origin), 255 seen and 0 not. The sim's rules (03 §10): each allied unit sees its radius,
    /// walls block sight, brush hides from outside it. A spectator sees everything.
    #[func]
    fn fog_grid(&self, cell: f32) -> VarDictionary {
        use mftr_sim::vision::vision_radius;
        let map = self.session.map();
        let cell = cell.max(25.0);
        let (cols, rows) = ((map.size.x / cell).ceil().max(1.0) as usize, (map.size.y / cell).ceil().max(1.0) as usize);
        let mut data = vec![0u8; cols * rows];
        let now = self.now();
        let team = self.session.team();
        let mut sources: Vec<(Vec2, f32)> = self
            .session
            .remote_render_units(now)
            .iter()
            .filter(|u| u.team == team && u.health > 0.0)
            .map(|u| (u.pos, vision_radius(u.kind)))
            .collect();
        if self.session.own_state_now().is_some_and(|s| s.alive())
            && let Some(own) = self.session.own_render_position(now)
        {
            sources.push((own, mftr_sim::vision::VISION_CHAMPION));
        }
        if self.session.is_spectator() {
            data.fill(255);
        } else {
            for (s, r) in sources.into_iter().filter(|(_, r)| *r > 0.0) {
                let brush = map.brush_at(s);
                let (c0, c1) =
                    (((s.x - r) / cell).floor().max(0.0) as usize, (((s.x + r) / cell).ceil() as usize).min(cols));
                let (r0, r1) =
                    (((s.y - r) / cell).floor().max(0.0) as usize, (((s.y + r) / cell).ceil() as usize).min(rows));
                for row in r0..r1 {
                    for col in c0..c1 {
                        let i = row * cols + col;
                        if data[i] == 255 {
                            continue;
                        }
                        // How far inside the vision circle the cell is, over about one cell:
                        // the client's linear filtering then draws a smooth circle edge
                        // instead of the cells' stair steps.
                        let p = Vec2::new((col as f32 + 0.5) * cell, (row as f32 + 0.5) * cell);
                        let inside = ((r - (p - s).length()) / cell + 0.5).clamp(0.0, 1.0);
                        let v = (inside * 255.0).round() as u8;
                        if v > data[i] && map.brush_at(p).is_none_or(|b| Some(b) == brush) && map.line_of_sight(s, p) {
                            data[i] = v;
                        }
                    }
                }
            }
        }
        let mut d = VarDictionary::new();
        d.set("cols", cols as i64);
        d.set("rows", rows as i64);
        d.set("cell", cell);
        d.set("data", &PackedByteArray::from(data.as_slice()));
        d
    }

    /// The action (`q`…`f`) a champion's dash or lunge sits on, or "" (A4b: dash VFX).
    #[func]
    fn action_info(&self, champion: GString, action: GString) -> VarDictionary {
        // A6: where `<action>.fire` effects play: `shape` (`nova`, `line`, `area`, `dash`,
        // `support`, `melee`, `ranged`), `radius` and `reach` in meters from the kit.
        use mftr_sim::ability::Effect;
        let mut d = VarDictionary::new();
        let Some(c) = ChampionId::by_name(&champion.to_string()) else { return d };
        let def = c.def();
        let (shape, radius, reach) = match action.to_string().as_str() {
            "attack" if def.attack.bolt_speed > 0.0 => ("ranged", 0.0, def.attack.range),
            "attack" => ("melee", 0.0, def.attack.range),
            a => match ACTIONS.iter().position(|s| *s == a).and_then(|s| c.ability(s as u8)).map(|ab| ab.effect) {
                Some(Effect::Line(l)) => ("line", l.radius, l.range),
                Some(Effect::Area(r)) if r.range == 0.0 => ("nova", r.radius, 0.0),
                Some(Effect::Area(r)) => ("area", r.radius, r.range),
                Some(Effect::Dash(_) | Effect::Lunge(_)) => ("dash", 0.0, 0.0),
                Some(_) => ("support", 0.0, 0.0),
                None => ("", 0.0, 0.0),
            },
        };
        d.set("shape", shape);
        d.set("radius", radius * 0.01);
        d.set("reach", reach * 0.01);
        d
    }

    /// Casts with no windup since the last call (A6): `[{ unit, slot }]`, own included.
    #[func]
    fn take_instant_casts(&mut self) -> VarArray {
        let mut out = VarArray::new();
        for (unit, slot) in self.session.take_instant_casts() {
            let mut d = VarDictionary::new();
            d.set("unit", unit.0 as i64);
            d.set("slot", slot as i64);
            out.push(&d.to_variant());
        }
        out
    }

    #[func]
    fn dash_action(&self, champion: GString) -> GString {
        let Some(c) = ChampionId::by_name(&champion.to_string()) else { return GString::new() };
        let slot = (0..SLOTS as u8).find(|s| {
            matches!(
                c.ability(*s).map(|a| a.effect),
                Some(mftr_sim::ability::Effect::Dash(_) | mftr_sim::ability::Effect::Lunge(_))
            )
        });
        GString::from(slot.map_or("", |s| ACTIONS[s as usize]))
    }

    /// Own champion on the input timeline: `{ champion, health, max_health, shield, dead,
    /// respawn_in, stunned, rooted, casting, attacking, dashing, facing (radians), attack_variant,
    /// recovering, buffered_slot (-1 = none), cooldowns: [6 × seconds], abilities: [6 × name] }`.
    /// Health and shield are predicted; damage arrives from the server.
    #[func]
    fn own_status(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        let now = self.now();
        let champ = self.session.champion();
        d.set("champion", champ.def().name);
        let mut names = VarArray::new();
        // F may be an augment's spell in its place.
        let spell = self.session.own_state_now().and_then(|s| augments::spell(&s.progress.augments));
        for slot in 0..SLOTS as u8 {
            let a = if slot == 5 { spell.or_else(|| champ.ability(slot)) } else { champ.ability(slot) };
            names.push(&a.map_or("", |a| a.name).to_variant());
        }
        d.set("abilities", &names);
        if let (Some(s), Some(t)) = (self.session.own_state_now(), self.session.input_sim_time(now)) {
            d.set("health", s.health);
            let (stats, attack) = items::champion_stats(champ.def(), &s.progress.stats_key());
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
            let charges: PackedInt32Array = s.progress.charges.iter().map(|c| *c as i32).collect();
            d.set("charges", &charges);
            d.set("potion", if s.potion_until > t { s.potion_until.secs_since(t) } else { 0.0 });
            d.set("can_undo", s.progress.undo_len > 0);
            d.set("hitbox", self.session.own_radius());
            // ARAM: Mayhem: held augments and the open draft.
            let card = |id: u8| {
                let mut c = VarDictionary::new();
                if let Some(a) = augments::augment(id) {
                    c.set("id", id as i64);
                    c.set("name", a.name);
                    c.set("tier", a.tier.name());
                    c.set("text", a.text);
                }
                c
            };
            let mut held = VarArray::new();
            for id in s.progress.augments.iter().filter(|id| **id != 0) {
                let mut c = card(*id);
                // Built-up augments show how far along they are.
                let p = &s.progress;
                match augments::augment(*id).map(|a| a.effect) {
                    Some(augments::Effect::Spellhunger) => {
                        c.set("progress", format!("{}/{}", p.stacks, augments::SPELLHUNGER_CAP));
                    }
                    Some(augments::Effect::ChampionOfChaos) => {
                        let done = p.takedowns >= augments::CHAOS_TAKEDOWNS;
                        let n = p.takedowns.min(augments::CHAOS_TAKEDOWNS);
                        c.set(
                            "progress",
                            if done { "done".to_string() } else { format!("{n}/{}", augments::CHAOS_TAKEDOWNS) },
                        );
                    }
                    _ => {}
                }
                held.push(&c.to_variant());
            }
            d.set("augments", &held);
            let mut offer = VarArray::new();
            for id in s.progress.offer.iter().filter(|id| **id != 0) {
                offer.push(&card(*id).to_variant());
            }
            d.set("offer", &offer);
            // Per choice: can it still be rerolled, and is its reroll golden (one tier up)?
            let open = s.progress.offer[0] != 0;
            let can: Vec<bool> = (0..augments::CHOICES).map(|c| open && s.progress.rerolled & (1 << c) == 0).collect();
            let mut rerolls = VarArray::new();
            for c in can {
                rerolls.push(&c.to_variant());
            }
            d.set("can_reroll", &rerolls);
            // Stat Anvils (Mayhem): the open one's choices, and what's been kept so far.
            let tiers = ["Silver", "Gold", "Prismatic"];
            let mut anvil = VarArray::new();
            for c in s.progress.anvil_offer {
                if let Some((tier, stat)) = mftr_sim::anvils::unpack(c) {
                    let mut m = VarDictionary::new();
                    m.set("tier", tiers[tier.min(2) as usize]);
                    m.set("stat", stat.name());
                    m.set("text", anvil_text(stat, mftr_sim::anvils::TIER_UNITS[tier.min(2) as usize]).as_str());
                    anvil.push(&m.to_variant());
                }
            }
            d.set("anvil_offer", &anvil);
            let mut kept = VarArray::new();
            for (stat, n) in mftr_sim::anvils::STATS.iter().zip(s.progress.anvil) {
                if n > 0 {
                    kept.push(&anvil_text(*stat, n).to_variant());
                }
            }
            d.set("anvils_kept", &kept);
            d.set("anvil_cost", mftr_sim::anvils::COST);
            d.set("anvil_level", mftr_sim::anvils::MIN_LEVEL as i64);
            d.set("mayhem", self.session.rules().augments);
            d.set("golden", s.progress.golden as i64 - 1);
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
            // A2 (10 §3–4): facing (radians), attack animation variant, follow-through, buffer.
            d.set("facing", s.facing.y.atan2(s.facing.x));
            d.set("attack_variant", (s.attacks & 3) as i64);
            d.set("recovering", s.recovering(t));
            d.set("buffered_slot", s.buffered.map_or(-1, |b| b.slot as i64));
            // A3: what the animator shows, retimed to the predicted sim times (10 §4.3).
            let walking = s.can_move(t) && s.heading().is_some();
            let windup_of = |slot: u8| match champ.ability(slot).map(|a| a.effect) {
                Some(mftr_sim::ability::Effect::Line(l)) => l.windup,
                Some(mftr_sim::ability::Effect::Area(a)) => a.windup,
                _ => mftr_sim::SimDuration(0),
            };
            let progress = |end: mftr_sim::SimTime, len: mftr_sim::SimDuration| {
                if len.0 == 0 { 1.0 } else { (1.0 - end.secs_since(t) / (len.0 as f32 / 1920.0)).clamp(0.0, 1.0) }
            };
            let fired = mftr_sim::SimTime(s.attack_ready_at.0.saturating_sub(attack.period().0 - attack.windup().0));
            let (action, phase, prog) = if let Some(c) = s.cast {
                d.set("anim_slot", c.slot as i64);
                ("cast", "windup", progress(c.fire_at, windup_of(c.slot)))
            } else if let Some(w) = s.attack {
                ("attack", "windup", progress(w.fire_at, attack.windup()))
            } else if s.recovering(t) {
                ("continue", "follow", -1.0)
            } else if !walking && t >= fired && t < s.attack_ready_at && s.attacks > 0 {
                // An attack's follow-through runs until the attack timer (cut by walking).
                ("attack", "follow", progress(s.attack_ready_at, mftr_sim::SimDuration(s.attack_ready_at.0 - fired.0)))
            } else {
                ("", "", -1.0)
            };
            d.set("anim_action", action);
            d.set("anim_phase", phase);
            d.set("anim_progress", prog);
            d.set("anim_variant", s.attacks as i64);
            d.set("slowed", s.slow > 0 && s.slowed_until > t);
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
    /// ""), red, ally, radius (collision), gameplay_radius, augments ([{ name, tier }]),
    /// health, max_health, shield, stunned, rooted, attacking, attack_variant, recovering, facing
    /// (radians), dashing, windup?, windup_dir? }`.
    /// Champions are on `T_interp`; minions near us blend toward `T_input` (03a §5).
    #[func]
    fn remote_units(&self) -> VarArray {
        let mut out = VarArray::new();
        for u in self.session.remote_render_units(self.now()) {
            let mut d = VarDictionary::new();
            d.set("id", u.id.0 as i64);
            d.set("pos", Vector2::new(u.pos.x, u.pos.y));
            d.set("minion", u.kind == UnitKind::Minion);
            // A5: which minion model to draw ("" for anything else).
            d.set(
                "minion_kind",
                match u.minion {
                    Some(mftr_sim::MinionKind::Melee) => "melee",
                    Some(mftr_sim::MinionKind::Caster) => "caster",
                    Some(mftr_sim::MinionKind::Siege) => "siege",
                    Some(mftr_sim::MinionKind::Super) => "super",
                    None => "",
                },
            );
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
            let mut held = VarArray::new();
            for a in u.augments.iter().filter_map(|id| augments::augment(*id)) {
                let mut c = VarDictionary::new();
                c.set("name", a.name);
                c.set("tier", a.tier.name());
                held.push(&c.to_variant());
            }
            d.set("augments", &held);
            d.set("attacking", u.attacking);
            d.set("attack_variant", u.attack_variant as i64);
            d.set("recovering", u.recovering);
            // A3: the animator's inputs (10 §6). Remote attacks have no timing on the wire:
            // they play at the clip's own rate.
            let (action, phase, prog) = if let Some((p, _)) = u.windup {
                d.set("anim_slot", u.windup_slot as i64);
                ("cast", "windup", p)
            } else if u.attacking {
                ("attack", "windup", -1.0)
            } else if u.recovering {
                ("continue", "follow", -1.0)
            } else {
                ("", "", -1.0)
            };
            d.set("anim_action", action);
            d.set("anim_phase", phase);
            d.set("anim_progress", prog);
            d.set("anim_variant", u.attack_variant as i64);
            d.set("facing", u.facing);
            d.set("rooted", u.rooted);
            d.set("dashing", u.dashing);
            d.set("slowed", u.slowed);
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

    /// Seconds since the match began (the sim's clock, predicted), for the HUD's game timer.
    #[func]
    fn match_seconds(&self) -> f64 {
        let started = self.session.scoreboard().map_or(mftr_sim::SimTime(0), |b| b.started_at);
        self.session.input_sim_time(self.now()).map_or(0.0, |t| t.secs_since(started).max(0.0) as f64)
    }

    /// Every champion's score (the Tab breakdown): `[{ unit, champion, team, ally, you, bot,
    /// level, kills, deaths, assists, cs, items: [ids], augments: [{ id, name, tier, text }],
    /// respawn }]`, our team first.
    #[func]
    fn scoreboard(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(b) = self.session.scoreboard() else { return out };
        let mine = self.session.team();
        let mut rows: Vec<_> = b.rows.iter().collect();
        rows.sort_by_key(|r| (r.team != mine, r.unit.0));
        for r in rows {
            let mut d = VarDictionary::new();
            d.set("unit", r.unit.0 as i64);
            d.set("champion", r.champion.def().name);
            d.set("team", if r.team == Team::Blue { "blue" } else { "red" });
            d.set("ally", r.team == mine);
            d.set("you", r.unit == self.session.unit());
            d.set("bot", r.bot);
            d.set("level", r.level as i64);
            d.set("kills", r.kills as i64);
            d.set("deaths", r.deaths as i64);
            d.set("assists", r.assists as i64);
            d.set("cs", r.cs as i64);
            let items: PackedInt32Array = r.items.iter().map(|i| *i as i32).collect();
            d.set("items", &items);
            let mut augs = VarArray::new();
            for id in r.augments.iter().filter(|id| **id != 0) {
                if let Some(a) = augments::augment(*id) {
                    let mut c = VarDictionary::new();
                    c.set("id", *id as i64);
                    c.set("name", a.name);
                    c.set("tier", a.tier.name());
                    c.set("text", a.text);
                    augs.push(&c.to_variant());
                }
            }
            d.set("augments", &augs);
            d.set("respawn", r.respawn_ds as f64 / 10.0);
            out.push(&d.to_variant());
        }
        out
    }

    /// The recap of our latest death (01 §13), every hit of the fight accounted for: `{ killer,
    /// total, seconds, physical, magic, true, absorbed, stunned, rooted, slowed, sources: [{
    /// name, champion, kind, total, lines: [{ what, key, kind, total, hits }] }] }`, or empty.
    #[func]
    fn death_recap(&self) -> VarDictionary {
        use mftr_sim::world::DamageOrigin;
        let mut d = VarDictionary::new();
        let Some(r) = self.session.last_recap() else { return d };
        let kind_name = |k: DamageKind| match k {
            DamageKind::Physical => "physical",
            DamageKind::Magic => "magic",
            DamageKind::True => "true",
        };
        let unit_name = |kind: UnitKind, champion: Option<ChampionId>| -> String {
            match (kind, champion) {
                (_, Some(c)) => c.def().name.to_string(),
                (UnitKind::Minion, _) => "Minion".into(),
                (UnitKind::Turret, _) => "Turret".into(),
                (UnitKind::Gatehouse, _) => "Gatehouse".into(),
                (UnitKind::Base, _) => "Base".into(),
                _ => "Unknown".into(),
            }
        };
        let killer = r.sources.iter().find(|s| s.source == r.killer);
        d.set("killer", killer.map_or("the fountain".to_string(), |s| unit_name(s.kind, s.champion)).as_str());
        for (k, v) in [
            ("total", r.total),
            ("seconds", r.seconds),
            ("physical", r.physical),
            ("magic", r.magic),
            ("true", r.true_damage),
            ("absorbed", r.absorbed),
            ("stunned", r.stunned),
            ("rooted", r.rooted),
            ("slowed", r.slowed),
        ] {
            d.set(k, v);
        }
        let mut sources = VarArray::new();
        for s in &r.sources {
            let mut m = VarDictionary::new();
            m.set("name", unit_name(s.kind, s.champion).as_str());
            m.set("champion", s.champion.map_or("", |c| c.def().name));
            m.set("killer", s.source == r.killer);
            m.set("total", s.total);
            let mut lines = VarArray::new();
            for l in &s.lines {
                let mut x = VarDictionary::new();
                let (what, key) = match l.origin {
                    DamageOrigin::Attack => ("Basic attacks".to_string(), ""),
                    DamageOrigin::Ability(slot) => (
                        s.champion.and_then(|c| c.ability(slot)).map_or("Ability".to_string(), |a| a.name.to_string()),
                        ["Q", "W", "E", "R", "D", "F"].get(slot as usize).copied().unwrap_or(""),
                    ),
                    DamageOrigin::Item(id) => (items::item(id).map_or("Item".to_string(), |i| i.name.to_string()), ""),
                    DamageOrigin::Augment(id) => {
                        (augments::augment(id).map_or("Augment".to_string(), |a| a.name.to_string()), "")
                    }
                    DamageOrigin::Fountain => ("Fountain".to_string(), ""),
                };
                x.set("what", what.as_str());
                x.set("key", key);
                x.set("kind", kind_name(l.kind));
                x.set("total", l.total);
                x.set("hits", l.hits as i64);
                lines.push(&x.to_variant());
            }
            m.set("lines", &lines);
            sources.push(&m.to_variant());
        }
        d.set("sources", &sources);
        d
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
    fn lobby_action(&mut self, action: mftr_net::msg::LobbyAction) {
        let now = self.now();
        let p = self.session.lobby_packet(action);
        self.transmit(p, now);
    }

    /// Seal a game packet and send it.
    fn transmit(&mut self, packet: Vec<u8>, now: f64) {
        let datagrams = self.secure.as_mut().map(|s| s.seal(&packet)).unwrap_or_default();
        for d in datagrams {
            self.send_datagram(d, now);
        }
    }

    /// Send a datagram now, or into the simulated uplink.
    fn send_datagram(&mut self, datagram: Vec<u8>, now: f64) {
        match (&mut self.links, &self.socket) {
            (Some((up, _)), _) => up.send(datagram, now),
            (None, Some(s)) => {
                let _ = s.send(&datagram);
            }
            _ => {}
        }
    }

    /// The player's identity key, created on first use.
    fn identity(&mut self) -> std::io::Result<Identity> {
        if self.identity.is_none() {
            let path = user_path(IDENTITY_FILE);
            self.identity = Some(Identity::load_or_create(path.as_ref(), "player identity key")?.0);
        }
        Ok(self.identity.clone().expect("loaded"))
    }

    /// Transport state after receiving: report a refused connection, and trust a server's key
    /// once the handshake with it is done.
    fn check_transport(&mut self) {
        let Some(secure) = &self.secure else { return };
        if let Some(e) = secure.error() {
            let mut text = e.to_string();
            if matches!(e, SecureError::KeyChanged { .. }) {
                text += &format!(", or delete its line in {}", user_path(KNOWN_FILE));
            }
            self.last_error = GString::from(&text);
        } else if !self.trusted
            && secure.is_open()
            && let Some(fp) = secure.server_fingerprint()
        {
            let mut known = known_servers();
            if known.get(&self.server) != Some(fp) {
                known.set(&self.server, fp);
                if std::fs::write(user_path(KNOWN_FILE), known.to_text()).is_err() {
                    self.last_error = GString::from(&format!("could not write {}", user_path(KNOWN_FILE)));
                }
            }
            self.trusted = true;
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
                Ok(n) => {
                    packets.push(buf[..n].to_vec());
                    if self.last_error == NO_ANSWER {
                        self.last_error = GString::new();
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                // ICMP "port unreachable" (a reset on Windows): the server may just not be up
                // yet, so keep trying.
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionRefused
                    ) =>
                {
                    self.last_error = GString::from(NO_ANSWER);
                    break;
                }
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
            let Some(secure) = self.secure.as_mut() else { break };
            let packet = secure.receive(&p);
            let replies = secure.take_outgoing();
            if let Some(packet) = packet {
                self.session.handle_packet(&packet, now);
            }
            for d in replies {
                self.send_datagram(d, now);
            }
        }
        self.check_transport();
        self.session.update(now);
        match self.session.phase() {
            Phase::Connecting | Phase::Lobby if now >= self.next_hello => {
                let hello = self.session.hello_packet(now);
                self.transmit(hello, now);
                self.next_hello = now + 0.25;
            }
            Phase::Playing if self.session.should_send(now) => self.send_input(now),
            _ => {}
        }
    }
}

const IDENTITY_FILE: &str = "identity.key";
const NO_ANSWER: &str = "No server is answering at this address yet (is it running, and is its UDP port open?)";
const KNOWN_FILE: &str = "known_servers.txt";

/// The OS path of a file in Godot's user data folder.
fn user_path(name: &str) -> String {
    ProjectSettings::singleton().globalize_path(&format!("user://{name}")).to_string()
}

fn known_servers() -> KnownServers {
    KnownServers::from_text(&std::fs::read_to_string(user_path(KNOWN_FILE)).unwrap_or_default())
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

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::SimDuration;

    #[test]
    fn areas_of_one_radius_are_told_apart_by_their_delay() {
        // Marrow's Siphon (a nova) and Ossuary are both 300 u wide.
        assert_eq!(area_action(ChampionId::Marrow, 300.0, SimDuration(0)), Some("q"));
        assert_eq!(area_action(ChampionId::Marrow, 300.0, SimDuration::from_millis(1300)), Some("r"));
        // An unknown delay still falls back to the radius alone.
        assert_eq!(area_action(ChampionId::Cairn, 180.0, SimDuration(7)), Some("e"));
    }
}

/// A Stat Anvil's bonus as text: "+10 attack damage", "+6% move speed".
fn anvil_text(stat: mftr_sim::anvils::Stat, units: u16) -> String {
    use mftr_sim::anvils::Stat;
    let b = stat.bonus(units);
    let v = match stat {
        Stat::AttackDamage => b.attack_damage,
        Stat::AbilityPower => b.ability_power,
        Stat::Health => b.health,
        Stat::Armor => b.armor,
        Stat::MagicResist => b.magic_resist,
        Stat::AttackSpeed => b.attack_speed * 100.0,
        Stat::AbilityHaste => b.ability_haste,
        Stat::MoveSpeed => b.move_speed_pct * 100.0,
    };
    let pct = matches!(stat, Stat::AttackSpeed | Stat::MoveSpeed);
    format!("+{}{} {}", (v * 10.0).round() / 10.0, if pct { "%" } else { "" }, stat.name())
}
