//! Champions on the wire (CR1d, D73, D83): the official ten are numbers 0–9 everywhere; a
//! match's custom champions are 10 and up, in the order of its **roster**: the server's own
//! champion files first, then the ones its players brought (D83). A process's registry ids
//! for customs differ from another's, so messages carry these wire numbers and each side maps
//! them to its own registry here.
//!
//! A client sets the process-wide table when it has loaded a match's champion files. A server
//! runs many matches in one process, each with its own roster, so it sets the table for the
//! duration of a call with [`scope`] (per thread). A client and a server in one process
//! (tests, tools) load the same files, which the registry gives the same ids.

use std::cell::RefCell;
use std::sync::{Arc, RwLock};

use mftr_sim::ChampionId;

use crate::content::ContentId;

/// Customs one match can name: six bits carry 0–62, and 63 means "none".
pub const MAX_CUSTOMS: usize = 53;
/// Champion files a server may be started with (`--champion-file`), leaving room for players'.
pub const HOST_CUSTOMS: usize = 21;
/// Champions a match's players may bring (one each: D83).
pub const MAX_BROUGHT: usize = 10;
/// Files one `Roster` names: a match's whole custom roster, which fits one packet.
pub const MAX_ROSTER: usize = HOST_CUSTOMS + MAX_BROUGHT;
/// The wire number for "no champion".
pub const NONE: u8 = 63;
const OFFICIAL: u8 = ChampionId::ALL.len() as u8;

static CUSTOMS: RwLock<Vec<ChampionId>> = RwLock::new(Vec::new());

thread_local! {
    /// The roster set by [`scope`] on this thread, over the process-wide one.
    static SCOPED: RefCell<Option<Arc<[ChampionId]>>> = const { RefCell::new(None) };
}

/// Set the custom roster, in wire order (at most [`MAX_CUSTOMS`]; more are cut).
pub fn set_customs(list: &[ChampionId]) {
    let mut t = CUSTOMS.write().unwrap_or_else(|e| e.into_inner());
    *t = list.iter().copied().filter(|c| !c.is_official()).take(MAX_CUSTOMS).collect();
}

/// The custom roster, in wire order.
pub fn customs() -> Vec<ChampionId> {
    if let Some(t) = SCOPED.with(|s| s.borrow().clone()) {
        return t.to_vec();
    }
    CUSTOMS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// While the returned guard lives, this thread maps wire numbers with `list` (a match's
/// roster) instead of the process-wide table. Guards nest; dropping one restores the last.
pub fn scope(list: Arc<[ChampionId]>) -> Scope {
    Scope(SCOPED.with(|s| s.replace(Some(list))))
}

/// See [`scope`].
#[must_use = "the roster applies only while the guard lives"]
pub struct Scope(Option<Arc<[ChampionId]>>);

impl Drop for Scope {
    fn drop(&mut self) {
        let prev = self.0.take();
        SCOPED.with(|s| *s.borrow_mut() = prev);
    }
}

fn with_table<T>(f: impl FnOnce(&[ChampionId]) -> T) -> T {
    if let Some(t) = SCOPED.with(|s| s.borrow().clone()) {
        return f(&t);
    }
    f(&CUSTOMS.read().unwrap_or_else(|e| e.into_inner()))
}

/// A champion's wire number, if it has one here.
pub fn to_wire(c: ChampionId) -> Option<u8> {
    if c.is_official() {
        return Some(c.index() as u8);
    }
    with_table(|t| t.iter().take(MAX_CUSTOMS).position(|&x| x == c).map(|i| OFFICIAL + i as u8))
}

/// The champion a wire number names here.
pub fn from_wire(v: u8) -> Option<ChampionId> {
    if v < OFFICIAL {
        return ChampionId::from_u8(v);
    }
    with_table(|t| t.get((v - OFFICIAL) as usize).copied())
}

/// A roster's hash (from the files' content ids, in order), so a client can say which one it
/// has loaded (`Hello::roster`); 0 is the empty roster.
pub fn roster_hash(files: &[ContentId]) -> u64 {
    if files.is_empty() {
        return 0;
    }
    let text: String = files.iter().map(|id| id.to_string()).collect();
    ContentId::of(&text).short().max(1)
}
