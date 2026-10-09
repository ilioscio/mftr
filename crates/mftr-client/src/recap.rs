//! The death recap (01 §13): exactly what killed you. The server sends every damage event on
//! the own champion, with its source and origin (which attack, ability, item or augment), and
//! the own state says when crowd control landed, so the recap is built from the real hits, not
//! estimated: nothing missing, nothing that wasn't there.

use mftr_sim::ability::DamageKind;
use mftr_sim::world::DamageOrigin;
use mftr_sim::{ChampionId, SimTime, UnitId, UnitKind};
use std::collections::VecDeque;

/// Hits further apart than this end a fight: the recap covers the last continuous one.
pub const FIGHT_GAP_S: f32 = 8.0;
/// The recap never reaches further back than this.
pub const MAX_WINDOW_S: f32 = 30.0;

/// One hit on the own champion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub at: SimTime,
    pub source: UnitId,
    /// What the source was when it hit (it may be gone by the time of the recap).
    pub source_kind: UnitKind,
    pub source_champion: Option<ChampionId>,
    pub origin: DamageOrigin,
    pub kind: DamageKind,
    /// To health, and to shields.
    pub amount: f32,
    pub absorbed: f32,
}

/// Crowd control that landed on the own champion: what and for how long.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CcHit {
    pub at: SimTime,
    pub kind: CcKind,
    pub seconds: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CcKind {
    Stun,
    Root,
    Slow,
}

/// One line of the recap: a source's damage by one origin and kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecapLine {
    pub origin: DamageOrigin,
    pub kind: DamageKind,
    pub total: f32,
    pub hits: u32,
}

/// Everything one source did.
#[derive(Clone, Debug, PartialEq)]
pub struct RecapSource {
    pub source: UnitId,
    pub kind: UnitKind,
    pub champion: Option<ChampionId>,
    pub total: f32,
    /// Biggest first.
    pub lines: Vec<RecapLine>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeathRecap {
    pub killer: UnitId,
    /// Damage taken in the fight (shields included), and how long the fight lasted.
    pub total: f32,
    pub seconds: f32,
    /// Of `total`: physical, magic, true.
    pub physical: f32,
    pub magic: f32,
    pub true_damage: f32,
    /// Taken by shields.
    pub absorbed: f32,
    /// Biggest first.
    pub sources: Vec<RecapSource>,
    /// Seconds of each kind of crowd control in the fight.
    pub stunned: f32,
    pub rooted: f32,
    pub slowed: f32,
}

/// The recent hits and crowd control on the own champion.
#[derive(Clone, Debug, Default)]
pub struct DamageLog {
    hits: VecDeque<Hit>,
    cc: VecDeque<CcHit>,
}

impl DamageLog {
    pub fn hit(&mut self, h: Hit) {
        self.hits.push_back(h);
        self.forget(h.at);
    }

    pub fn crowd_control(&mut self, c: CcHit) {
        self.cc.push_back(c);
        self.forget(c.at);
    }

    fn forget(&mut self, now: SimTime) {
        let old = |t: SimTime| now.secs_since(t) > MAX_WINDOW_S + FIGHT_GAP_S;
        while self.hits.front().is_some_and(|h| old(h.at)) {
            self.hits.pop_front();
        }
        while self.cc.front().is_some_and(|c| old(c.at)) {
            self.cc.pop_front();
        }
    }

    /// The recap of a death at `at` by `killer`: the hits of the last continuous fight.
    pub fn recap(&self, killer: UnitId, at: SimTime) -> DeathRecap {
        let mut fight: Vec<&Hit> = Vec::new();
        let mut later = at;
        for h in self.hits.iter().rev().filter(|h| h.at <= at) {
            if later.secs_since(h.at) > FIGHT_GAP_S || at.secs_since(h.at) > MAX_WINDOW_S {
                break;
            }
            fight.push(h);
            later = h.at;
        }
        let start = fight.last().map_or(at, |h| h.at);
        let mut r = DeathRecap {
            killer,
            total: 0.0,
            seconds: at.secs_since(start),
            physical: 0.0,
            magic: 0.0,
            true_damage: 0.0,
            absorbed: 0.0,
            sources: Vec::new(),
            stunned: 0.0,
            rooted: 0.0,
            slowed: 0.0,
        };
        for h in fight.iter().rev() {
            let dealt = h.amount + h.absorbed;
            r.total += dealt;
            r.absorbed += h.absorbed;
            match h.kind {
                DamageKind::Physical => r.physical += dealt,
                DamageKind::Magic => r.magic += dealt,
                DamageKind::True => r.true_damage += dealt,
            }
            let i = match r.sources.iter().position(|s| s.source == h.source) {
                Some(i) => i,
                None => {
                    r.sources.push(RecapSource {
                        source: h.source,
                        kind: h.source_kind,
                        champion: h.source_champion,
                        total: 0.0,
                        lines: Vec::new(),
                    });
                    r.sources.len() - 1
                }
            };
            let s = &mut r.sources[i];
            s.total += dealt;
            match s.lines.iter_mut().find(|l| l.origin == h.origin && l.kind == h.kind) {
                Some(l) => {
                    l.total += dealt;
                    l.hits += 1;
                }
                None => s.lines.push(RecapLine { origin: h.origin, kind: h.kind, total: dealt, hits: 1 }),
            }
        }
        for s in &mut r.sources {
            s.lines.sort_by(|a, b| b.total.total_cmp(&a.total));
        }
        r.sources.sort_by(|a, b| b.total.total_cmp(&a.total));
        for c in self.cc.iter().filter(|c| c.at >= start && c.at <= at) {
            // Only the part of it that happened before the death.
            let secs = c.seconds.min(at.secs_since(c.at));
            match c.kind {
                CcKind::Stun => r.stunned += secs,
                CcKind::Root => r.rooted += secs,
                CcKind::Slow => r.slowed += secs,
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::time::SUBTICKS_PER_SECOND;

    fn t(s: f32) -> SimTime {
        SimTime((s * SUBTICKS_PER_SECOND as f32) as u64)
    }

    fn hit(at: f32, source: u32, origin: DamageOrigin, kind: DamageKind, amount: f32) -> Hit {
        Hit {
            at: t(at),
            source: UnitId(source),
            source_kind: UnitKind::Champion,
            source_champion: None,
            origin,
            kind,
            amount,
            absorbed: 0.0,
        }
    }

    /// The recap covers the last fight only, adds up every hit exactly, and breaks it down by
    /// source, origin and damage type, biggest first.
    #[test]
    fn the_recap_is_exactly_the_last_fight() {
        let mut log = DamageLog::default();
        // An earlier skirmish, 9 s before the fight: not part of it.
        log.hit(hit(1.0, 7, DamageOrigin::Attack, DamageKind::Physical, 50.0));
        log.hit(hit(10.0, 7, DamageOrigin::Ability(0), DamageKind::Magic, 200.0));
        log.hit(hit(10.5, 8, DamageOrigin::Attack, DamageKind::Physical, 60.0));
        log.hit(Hit { absorbed: 40.0, ..hit(11.0, 8, DamageOrigin::Attack, DamageKind::Physical, 20.0) });
        log.hit(hit(12.0, 7, DamageOrigin::Ability(3), DamageKind::Magic, 300.0));
        log.hit(hit(12.2, 8, DamageOrigin::Item(24), DamageKind::Magic, 15.0));
        log.crowd_control(CcHit { at: t(11.5), kind: CcKind::Stun, seconds: 1.5 });
        let r = log.recap(UnitId(7), t(12.5));
        assert_eq!(r.total, 200.0 + 60.0 + 60.0 + 300.0 + 15.0);
        assert!((r.seconds - 2.5).abs() < 1e-3);
        assert_eq!((r.physical, r.magic, r.true_damage, r.absorbed), (120.0, 515.0, 0.0, 40.0));
        assert_eq!(r.sources[0].source, UnitId(7));
        assert_eq!(r.sources[0].total, 500.0);
        assert_eq!(r.sources[0].lines[0].origin, DamageOrigin::Ability(3));
        assert_eq!(
            r.sources[1].lines[0],
            RecapLine { origin: DamageOrigin::Attack, kind: DamageKind::Physical, total: 120.0, hits: 2 }
        );
        // A 1.5 s stun landed 1 s before the death: 1 s of it counts.
        assert!((r.stunned - 1.0).abs() < 1e-3);
    }
}
