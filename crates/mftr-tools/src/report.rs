//! Shared statistics and report formatting for the Netcode Lab and the UDP bot.

use mftr_client::ClientSession;

/// Scripted "player": clicks around the arena like an impatient human (3–8 clicks/s bursts).
pub struct ClickBot {
    rng: mftr_sim::rng::Pcg32,
    next_click: f64,
    pub arena_min: f32,
    pub arena_max: f32,
}

impl ClickBot {
    pub fn new(seed: u64) -> Self {
        Self { rng: mftr_sim::rng::Pcg32::new(seed, 0x626f74), next_click: 0.5, arena_min: 500.0, arena_max: 3500.0 }
    }

    /// Issue commands that are due. Returns true if any command was issued.
    pub fn act(&mut self, session: &mut ClientSession, now: f64, elapsed: f64) -> bool {
        if elapsed < self.next_click {
            return false;
        }
        let r = self.rng.next_f32();
        let issued = if r < 0.04 {
            session.stop(now).is_some()
        } else {
            let (lo, hi) = (self.arena_min, self.arena_max);
            let target = mftr_sim::Vec2::new(self.rng.range_f32(lo, hi), self.rng.range_f32(lo, hi));
            session.move_to(target, now).is_some()
        };
        // Mostly rapid re-clicks, sometimes a pause.
        let gap = if self.rng.next_f32() < 0.8 { self.rng.range_f32(0.12, 0.35) } else { self.rng.range_f32(0.5, 1.5) };
        self.next_click = elapsed + gap as f64;
        issued
    }
}

/// Dodge-rig player (03 §14): wanders like `ClickBot`, but reacts to enemy missiles exactly as
/// its own client displays them. When a missile shown on the input timeline is predicted to
/// hit, it steps sideways after a human-like reaction delay. Ghost hits are then counted by
/// the client: shown as a dodge, but hit on the server.
pub struct DodgeBot {
    wander: ClickBot,
    reaction: f64,
    dodged: std::collections::BTreeSet<u32>,
    quiet_until: f64,
}

impl DodgeBot {
    pub fn new(seed: u64, reaction: f64) -> Self {
        Self { wander: ClickBot::new(seed), reaction, dodged: Default::default(), quiet_until: 0.0 }
    }

    pub fn act(&mut self, session: &mut ClientSession, now: f64, elapsed: f64) -> bool {
        for th in session.threats(now) {
            if th.predicted_hit.is_none() || th.visible_for < self.reaction || !self.dodged.insert(th.id) {
                continue;
            }
            let Some(own) = session.own_render_position(now) else { continue };
            let perp = mftr_sim::Vec2::new(-th.dir.y, th.dir.x);
            let side = if (own - th.pos).dot(perp) >= 0.0 { 1.0 } else { -1.0 };
            let mut target = own + perp * (300.0 * side);
            target.x = target.x.clamp(300.0, 3700.0);
            target.y = target.y.clamp(300.0, 3700.0);
            self.quiet_until = elapsed + 0.6;
            return session.move_to(target, now).is_some();
        }
        if elapsed < self.quiet_until {
            return false;
        }
        self.wander.act(session, now, elapsed)
    }
}

/// Tracks visible jumps in a rendered position: movement beyond what speed allows in one frame.
/// Also integrates the visible correction offset over time (03 §1 "mean visible position
/// correction during normal play").
#[derive(Default)]
pub struct JumpMeter {
    last: Option<mftr_sim::Vec2>,
    pub jumps: Vec<f32>,
    pub offset_sum: f64,
    pub frames: u64,
}

impl JumpMeter {
    /// Jumps are frame-to-frame steps more than 5 u beyond what move speed allows: a visible
    /// pop. Smoothed blends of large corrections can still register for a frame or two.
    pub fn observe(&mut self, pos: mftr_sim::Vec2, visible_correction: f32, frame_dt: f64, max_speed: f32) {
        if let Some(prev) = self.last {
            let excess = prev.distance(pos) - max_speed * frame_dt as f32;
            if excess > 5.0 {
                self.jumps.push(excess);
            }
        }
        self.last = Some(pos);
        self.offset_sum += visible_correction as f64;
        self.frames += 1;
    }

    /// A frame where fast movement is expected (a dash): the next frame starts fresh.
    pub fn skip(&mut self) {
        self.last = None;
    }
}

fn percentile(sorted: &[f32], p: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i]
}

#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub label: String,
    pub true_rtt_ms: f64,
    pub est_rtt_ms: f64,
    pub margin_ms: f64,
    pub commands: u64,
    pub late_pct: f64,
    pub lead_p01_ms: f64,
    pub reconciliations: u64,
    pub mismatch_pct: f64,
    pub corr_mean: f32,
    pub corr_p95: f32,
    pub corr_max: f32,
    pub corr_per_min_over_15: f64,
    /// Time-averaged size of the on-screen correction offset (03 §1 metric).
    pub visible_mean: f64,
    pub jumps_per_min: f64,
    pub jump_max: f32,
    pub up_kbps: f64,
    pub down_kbps: f64,
    pub hard_resets: u64,
    pub dodge: mftr_client::DodgeStats,
    /// Per client (duel): confirmed kills, deaths and damage dealt; level and gold at the end.
    pub levels: Vec<u8>,
    pub gold: Vec<f32>,
    /// Items held at the end (per client).
    pub items: Vec<usize>,
    pub kills: Vec<u64>,
    pub deaths: Vec<u64>,
    pub damage_dealt: Vec<f64>,
    /// Lane maps: matches won and lost, summed over clients.
    pub matches_won: u64,
    pub matches_lost: u64,
}

impl Summary {
    pub fn from_sessions(label: &str, true_rtt: f64, seconds: f64, sessions: &[(&ClientSession, &JumpMeter)]) -> Self {
        let n = sessions.len().max(1) as f64;
        let mut corr: Vec<f32> = sessions.iter().flat_map(|(s, _)| s.stats.corrections.iter().copied()).collect();
        corr.sort_by(f32::total_cmp);
        let mut leads: Vec<f32> =
            sessions.iter().flat_map(|(s, _)| s.stats.leads_us.iter().map(|&l| l as f32 / 1000.0)).collect();
        leads.sort_by(f32::total_cmp);
        let jumps: Vec<f32> = sessions.iter().flat_map(|(_, j)| j.jumps.iter().copied()).collect();
        let commands: u64 = sessions.iter().map(|(s, _)| s.stats.commands_issued).sum();
        let late: u64 = sessions.iter().map(|(s, _)| s.stats.commands_late).sum();
        let recon: u64 = sessions.iter().map(|(s, _)| s.stats.reconciliations).sum();
        let mism: u64 = sessions.iter().map(|(s, _)| s.stats.mismatches).sum();
        let minutes = seconds / 60.0 * n;
        Summary {
            label: label.to_string(),
            true_rtt_ms: true_rtt * 1e3,
            est_rtt_ms: sessions.iter().map(|(s, _)| s.rtt()).sum::<f64>() / n * 1e3,
            margin_ms: sessions.iter().map(|(s, _)| s.margin()).sum::<f64>() / n * 1e3,
            commands,
            late_pct: if commands > 0 { late as f64 * 100.0 / commands as f64 } else { 0.0 },
            lead_p01_ms: percentile(&leads, 0.01) as f64,
            reconciliations: recon,
            mismatch_pct: if recon > 0 { mism as f64 * 100.0 / recon as f64 } else { 0.0 },
            corr_mean: if corr.is_empty() { 0.0 } else { corr.iter().sum::<f32>() / corr.len() as f32 },
            corr_p95: percentile(&corr, 0.95),
            corr_max: corr.last().copied().unwrap_or(0.0),
            corr_per_min_over_15: corr.iter().filter(|&&c| c > 15.0).count() as f64 / minutes.max(1e-9),
            visible_mean: sessions.iter().map(|(_, j)| j.offset_sum).sum::<f64>()
                / sessions.iter().map(|(_, j)| j.frames).sum::<u64>().max(1) as f64,
            jumps_per_min: jumps.len() as f64 / minutes.max(1e-9),
            jump_max: jumps.iter().copied().fold(0.0, f32::max),
            up_kbps: sessions.iter().map(|(s, _)| s.stats.bytes_up).sum::<u64>() as f64 / 1024.0 / seconds / n,
            down_kbps: sessions.iter().map(|(s, _)| s.stats.bytes_down).sum::<u64>() as f64 / 1024.0 / seconds / n,
            hard_resets: sessions.iter().map(|(s, _)| s.stats.hard_resets).sum(),
            dodge: sessions.iter().fold(mftr_client::DodgeStats::default(), |mut a, (s, _)| {
                let d = s.dodge_stats();
                a.enemy_missiles += d.enemy_missiles;
                a.near_misses += d.near_misses;
                a.server_hits += d.server_hits;
                a.shown_hits += d.shown_hits;
                a.ghost_hits += d.ghost_hits;
                a.phantom_hits += d.phantom_hits;
                a.uncertain += d.uncertain;
                a.died_first += d.died_first;
                a
            }),
            levels: sessions.iter().map(|(s, _)| s.own_state_now().map_or(0, |st| st.progress.level)).collect(),
            gold: sessions.iter().map(|(s, _)| s.own_state_now().map_or(0.0, |st| st.progress.gold)).collect(),
            items: sessions
                .iter()
                .map(|(s, _)| s.own_state_now().map_or(0, |st| st.progress.items.iter().filter(|i| **i != 0).count()))
                .collect(),
            kills: sessions.iter().map(|(s, _)| s.stats.kills).collect(),
            deaths: sessions.iter().map(|(s, _)| s.stats.deaths).collect(),
            damage_dealt: sessions.iter().map(|(s, _)| s.stats.damage_dealt).collect(),
            matches_won: sessions.iter().map(|(s, _)| s.stats.matches_won).sum(),
            matches_lost: sessions.iter().map(|(s, _)| s.stats.matches_lost).sum(),
        }
    }

    /// Ghost hits per near-miss (03 §1 target: < 0.5% at 60 ms, < 2% at 120 ms).
    pub fn ghost_rate(&self) -> f64 {
        self.dodge.ghost_hits as f64 / self.dodge.near_misses.max(1) as f64
    }

    pub fn dodge_row(&self) -> String {
        let d = &self.dodge;
        format!(
            "{:<10} enemy missiles {:>5}  near-misses {:>5}  server hits {:>5}  shown hits {:>5}  ghost hits {:>4} ({:.2}% of near-misses)  phantom hits {:>4}  unconfirmed {:>4}  died first {:>4}",
            self.label,
            d.enemy_missiles,
            d.near_misses,
            d.server_hits,
            d.shown_hits,
            d.ghost_hits,
            self.ghost_rate() * 100.0,
            d.phantom_hits,
            d.uncertain,
            d.died_first,
        )
    }

    pub fn duel_row(&self) -> String {
        format!(
            "{:<10} levels {:?}  gold {:?}  items {:?}  kills {:?}  deaths {:?}  damage dealt {:?}  matches won/lost by clients {}/{}",
            self.label,
            self.levels,
            self.gold.iter().map(|g| g.round() as i64).collect::<Vec<_>>(),
            self.items,
            self.kills,
            self.deaths,
            self.damage_dealt.iter().map(|d| d.round() as i64).collect::<Vec<_>>(),
            self.matches_won,
            self.matches_lost
        )
    }

    pub fn header() -> String {
        format!(
            "{:<10} {:>7} {:>7} {:>6} {:>6} {:>6} {:>8} {:>7} {:>7} {:>7} {:>7} {:>8} {:>7} {:>7} {:>7} {:>6} {:>6} {:>5}",
            "profile",
            "rtt",
            "rtt min",
            "margin",
            "cmds",
            "late%",
            "lead p1",
            "mism%",
            "corr~",
            "c p95",
            "c max",
            ">15u/min",
            "visible",
            "jump/m",
            "jmp max",
            "up KB",
            "dn KB",
            "reset"
        )
    }

    pub fn row(&self) -> String {
        format!(
            "{:<10} {:>6.0}ms {:>6.1}ms {:>4.1}ms {:>6} {:>6.2} {:>6.1}ms {:>7.2} {:>6.1}u {:>6.1}u {:>6.1}u {:>8.2} {:>6.2}u {:>7.2} {:>6.1}u {:>6.2} {:>6.2} {:>5}",
            self.label,
            self.true_rtt_ms,
            self.est_rtt_ms,
            self.margin_ms,
            self.commands,
            self.late_pct,
            self.lead_p01_ms,
            self.mismatch_pct,
            self.corr_mean,
            self.corr_p95,
            self.corr_max,
            self.corr_per_min_over_15,
            self.visible_mean,
            self.jumps_per_min,
            self.jump_max,
            self.up_kbps,
            self.down_kbps,
            self.hard_resets,
        )
    }
}

/// Duel Sandbox player (M1 slice 4): fights the nearest visible enemy champion with its whole
/// kit, the way a scripted human might. Skillshots and areas aim at the enemy as drawn; it
/// attacks, kites, dashes, blinks, shields when low, and dodges enemy skillshots exactly like
/// [`DodgeBot`] (reacting to what its own client shows).
pub struct DuelBot {
    rng: mftr_sim::rng::Pcg32,
    reaction: f64,
    next_think: f64,
    dodged: std::collections::BTreeSet<u32>,
    /// Next step of the build path, and when shopping may be tried again.
    build: usize,
    next_shop: f64,
}

/// Bot build paths (components first, so recipes discount them).
fn build_path(champion: mftr_sim::ChampionId) -> &'static [u8] {
    use mftr_sim::items::*;
    match champion {
        mftr_sim::ChampionId::Ember => &[
            BOOTS,
            CHARGED_WAND,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            INFERNO_DIADEM,
            SAGE_BOOTS,
            CHARGED_WAND,
            CHARGED_WAND,
            GRAND_GRIMOIRE,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
        ],
        mftr_sim::ChampionId::Vesper => &[
            BOOTS,
            LONG_KNIFE,
            LEECH_FANG,
            HEAVY_PICK,
            CRIMSON_FANG,
            QUICK_DAGGER,
            BATTLE_BOOTS,
            HEAVY_PICK,
            QUICK_DAGGER,
            ARC_BOW,
            GALE_SABER,
            VITAL_CRYSTAL,
            TITAN_BELT,
            HEAVY_PICK,
            LIFELINE_TALISMAN,
        ],
        mftr_sim::ChampionId::Bastion => &[
            BOOTS,
            VITAL_CRYSTAL,
            PADDED_VEST,
            CHAIN_COAT,
            BRAMBLE_PLATE,
            SWIFT_BOOTS,
            VITAL_CRYSTAL,
            TITAN_BELT,
            VITAL_CRYSTAL,
            HEARTSTONE,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
        ],
        mftr_sim::ChampionId::Rook => &[
            BOOTS,
            LONG_KNIFE,
            LEECH_FANG,
            VITAL_CRYSTAL,
            TITAN_BELT,
            HEAVY_PICK,
            LIFELINE_TALISMAN,
            QUICK_DAGGER,
            BATTLE_BOOTS,
            PADDED_VEST,
            CHAIN_COAT,
            VITAL_CRYSTAL,
            BRAMBLE_PLATE,
        ],
        mftr_sim::ChampionId::Lumen => &[
            BOOTS,
            CHARGED_WAND,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            INFERNO_DIADEM,
            SAGE_BOOTS,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
        ],
        mftr_sim::ChampionId::Shade => &[
            BOOTS,
            LONG_KNIFE,
            LEECH_FANG,
            HEAVY_PICK,
            CRIMSON_FANG,
            SWIFT_BOOTS,
            HEAVY_PICK,
            QUICK_DAGGER,
            ARC_BOW,
            GALE_SABER,
        ],
    }
}

impl DuelBot {
    pub fn new(seed: u64, reaction: f64) -> Self {
        Self {
            rng: mftr_sim::rng::Pcg32::new(seed, 0x6475_656c),
            reaction,
            next_think: 0.5,
            dodged: Default::default(),
            build: 0,
            next_shop: 0.0,
        }
    }

    /// Follow the build path while the shop is open (dead or in the fountain).
    fn shop(&mut self, session: &mut ClientSession, p: &mftr_sim::world::Progress, now: f64, elapsed: f64) -> bool {
        let path = build_path(session.champion());
        while self.build < path.len() && p.items.contains(&path[self.build]) {
            self.build += 1;
        }
        let Some(&next) = path.get(self.build) else { return false };
        if elapsed < self.next_shop || !session.can_shop() {
            return false;
        }
        self.next_shop = elapsed + 0.25;
        let affordable = mftr_sim::items::price(next, &p.items).is_some_and(|(cost, _)| cost <= p.gold);
        affordable && session.buy(next, now).is_some()
    }

    /// The kit's dash or blink (Q W E R), used to dodge and kite.
    fn escape_slot(champ: mftr_sim::ChampionId) -> Option<u8> {
        use mftr_sim::ability::Effect;
        (0..4u8).find(|s| matches!(champ.ability(*s).map(|a| a.effect), Some(Effect::Dash(_) | Effect::Blink(_))))
    }

    fn clamp(session: &ClientSession, p: mftr_sim::Vec2) -> mftr_sim::Vec2 {
        let size = session.map().size;
        mftr_sim::Vec2::new(p.x.clamp(300.0, size.x - 300.0), p.y.clamp(300.0, size.y - 300.0))
    }

    pub fn act(&mut self, session: &mut ClientSession, now: f64, elapsed: f64) -> bool {
        use mftr_sim::Vec2;
        use mftr_sim::ability::Effect;
        let (Some(st), Some(t), Some(own)) =
            (session.own_state_now(), session.input_sim_time(now), session.own_render_position(now))
        else {
            return false;
        };
        if self.shop(session, &st.progress, now, elapsed) {
            return true;
        }
        if !st.alive() {
            return false;
        }
        // Spend ability points: the ultimate whenever possible, else the lowest-ranked basic.
        let p = st.progress;
        if p.points > 0 {
            use mftr_sim::champion::max_rank;
            let slot = if p.ranks[3] < max_rank(3, p.level) {
                Some(3)
            } else {
                (0..3u8).filter(|s| p.ranks[*s as usize] < max_rank(*s, p.level)).min_by_key(|s| p.ranks[*s as usize])
            };
            if let Some(slot) = slot
                && session.level_up(slot, now).is_some()
            {
                return true;
            }
        }
        for th in session.threats(now) {
            if th.predicted_hit.is_none() || th.visible_for < self.reaction || !self.dodged.insert(th.id) {
                continue;
            }
            let perp = Vec2::new(-th.dir.y, th.dir.x);
            let side = if (own - th.pos).dot(perp) >= 0.0 { 1.0 } else { -1.0 };
            let target = Self::clamp(session, own + perp * (300.0 * side));
            let escape = Self::escape_slot(session.champion()).filter(|s| st.cooldowns[*s as usize] <= t);
            self.next_think = elapsed + 0.5;
            return if let Some(slot) = escape
                && self.rng.next_f32() < 0.3
            {
                session.cast(slot, target, now).is_some() // dash or blink out of the way
            } else {
                session.move_to(target, now).is_some()
            };
        }
        if elapsed < self.next_think {
            return false;
        }
        self.next_think = elapsed + self.rng.range_f32(0.15, 0.45) as f64;
        let team = session.team();
        let enemy = session
            .remote_render_units(now)
            .into_iter()
            .filter(|r| r.team != team && r.kind == mftr_sim::UnitKind::Champion)
            .min_by(|a, b| a.pos.distance(own).total_cmp(&b.pos.distance(own)));
        let Some(e) = enemy else {
            // On a lane map: push down the lane with the minions.
            let lane = &session.map().layout.lanes[session.team() as usize];
            if let Some(&end) = lane.last() {
                return session.attack_move(end, now).is_some();
            }
            // Otherwise go looking for the enemy champion, through the middle of the arena.
            let p = Vec2::new(self.rng.range_f32(1300.0, 2700.0), self.rng.range_f32(1300.0, 2700.0));
            return if self.rng.next_u32() % 4 == 0 {
                session.attack_move(p, now).is_some()
            } else {
                session.move_to(p, now).is_some()
            };
        };
        let champ = session.champion();
        let ready = |slot: u8| st.cooldowns[slot as usize] <= t && st.cast.is_none();
        let d = e.pos.distance(own);
        let to = (e.pos - own).normalize_or_zero();
        let perp = Vec2::new(-to.y, to.x) * if self.rng.next_u32() % 2 == 0 { 1.0 } else { -1.0 };
        let max_hp = mftr_sim::items::champion_stats(champ.def(), st.progress.level, &st.progress.items).0.max_health;
        if st.health < 0.35 * max_hp && ready(5) {
            return session.cast(5, own, now).is_some();
        }
        // Heals and shields: on the most hurt ally champion in range, else on itself when hurt.
        for slot in (0..4u8).filter(|s| ready(*s)) {
            let Some(Effect::Support(sup)) = champ.ability(slot).map(|a| a.effect) else { continue };
            let ally = session
                .remote_render_units(now)
                .into_iter()
                .filter(|a| a.team == team && a.kind == mftr_sim::UnitKind::Champion && a.max_health > 0.0)
                .filter(|a| sup.range > 0.0 && a.pos.distance(own) <= sup.range && a.health < 0.7 * a.max_health)
                .min_by(|a, b| (a.health / a.max_health).total_cmp(&(b.health / b.max_health)));
            if let Some(a) = ally {
                return session.cast(slot, a.pos, now).is_some();
            }
            if st.health < 0.6 * max_hp {
                return session.cast(slot, own, now).is_some();
            }
        }
        let r = self.rng.next_f32();
        // Skillshots, areas, novas and lunges the enemy is in range of.
        let in_range = |slot: u8| match champ.ability(slot).map(|a| a.effect) {
            Some(Effect::Line(s)) => d <= s.range * 0.9,
            Some(Effect::Area(a)) if a.range == 0.0 => d <= a.radius * 0.8,
            Some(Effect::Area(a)) => d <= a.range,
            Some(Effect::Lunge(l)) => d <= l.range,
            _ => false,
        };
        let shots: Vec<u8> = (0..4u8).filter(|s| ready(*s) && in_range(*s)).collect();
        if !shots.is_empty() && r < 0.45 {
            let slot = shots[self.rng.next_u32() as usize % shots.len()];
            return session.cast(slot, e.pos, now).is_some();
        }
        if r < 0.52
            && let Some(slot) = Self::escape_slot(champ).filter(|s| ready(*s))
        {
            return session.cast(slot, Self::clamp(session, own + perp * 300.0 - to * 100.0), now).is_some();
        }
        if r < 0.53 && ready(4) {
            return session.cast(4, Self::clamp(session, own + to * 400.0), now).is_some();
        }
        if r < 0.85 {
            return session.attack(e.id, now).is_some();
        }
        session.move_to(Self::clamp(session, own + perp * 250.0), now).is_some()
    }
}

/// Summary of blind playtest rounds (03 §14): ratings per hidden latency profile and per A/B
/// switch, objective numbers alongside, and the M1 exit verdict (dodging rated fair in ≥ 80%
/// of rounds at 80 ms).
pub fn blind_report(records: &[mftr_client::blind::BlindRecord]) -> String {
    use mftr_client::blind::{EXIT_FAIR_SHARE, EXIT_PROFILE, Tally, exit_verdict, summarize};
    use std::fmt::Write;
    let mut out = String::new();
    let testers: std::collections::BTreeSet<u64> = records.iter().map(|r| r.seed).collect();
    let _ = writeln!(out, "{} rated rounds from {} session(s)\n", records.len(), testers.len());
    let (by_profile, by_option, by_bubble) = summarize(records);
    let row = |out: &mut String, t: &Tally, extra: &str| {
        let line = format!(
            "{:<12} {:>6} {:>9.0}% {:>14.2}  {}",
            t.key,
            t.rounds,
            t.fair_share * 100.0,
            t.responsiveness,
            extra
        );
        let _ = writeln!(out, "{}", line.trim_end());
    };
    let _ = writeln!(out, "{:<12} {:>6} {:>10} {:>14}  measured", "condition", "rounds", "fair", "responsive 1-5");
    for t in &by_profile {
        let rs: Vec<_> = records.iter().filter(|r| r.profile == t.key).collect();
        let n = rs.len().max(1) as f64;
        let rtt = rs.iter().map(|r| r.stats.rtt_ms).sum::<f64>() / n;
        let near: u64 = rs.iter().map(|r| r.stats.near_misses).sum();
        let ghost: u64 = rs.iter().map(|r| r.stats.ghost_hits).sum();
        row(&mut out, t, &format!("rtt {rtt:.0} ms, ghost hits {ghost} of {near} near-misses"));
    }
    let _ = writeln!(out);
    for t in by_option.iter().chain(&by_bubble) {
        row(&mut out, t, "");
    }
    let _ = writeln!(out);
    match exit_verdict(records) {
        Some((passed, t)) => {
            let _ = writeln!(
                out,
                "M1 exit (dodging fair in >= {:.0}% of rounds at {EXIT_PROFILE}): {} ({:.0}% of {} rounds)",
                EXIT_FAIR_SHARE * 100.0,
                if passed { "PASS" } else { "FAIL" },
                t.fair_share * 100.0,
                t.rounds
            );
        }
        None => {
            let _ = writeln!(out, "M1 exit: no rounds at {EXIT_PROFILE} yet");
        }
    }
    out
}
