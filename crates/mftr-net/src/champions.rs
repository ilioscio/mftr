//! Champions on the wire (CR1d, D73): the official ten are numbers 0–9 everywhere; a server's
//! custom champions (champion files it was started with) are 10 and up, in the order of its
//! **roster**. A process's registry ids for customs differ from another's, so messages carry
//! these wire numbers and each side maps them to its own registry here.
//!
//! The table is process-wide: a server sets its roster once at start, a client when it has
//! loaded the server's champion files (before it joins). A client and a server in one process
//! (tests, tools) load the same files, which the registry gives the same ids.

use std::sync::RwLock;

use mftr_sim::ChampionId;

/// Customs one server can offer: five bits carry 0–30, and 31 means "none".
pub const MAX_CUSTOMS: usize = 21;
const OFFICIAL: u8 = ChampionId::ALL.len() as u8;

static CUSTOMS: RwLock<Vec<ChampionId>> = RwLock::new(Vec::new());

/// Set the custom roster, in wire order (at most [`MAX_CUSTOMS`]; more are cut).
pub fn set_customs(list: &[ChampionId]) {
    let mut t = CUSTOMS.write().unwrap_or_else(|e| e.into_inner());
    *t = list.iter().copied().filter(|c| !c.is_official()).take(MAX_CUSTOMS).collect();
}

/// The custom roster, in wire order.
pub fn customs() -> Vec<ChampionId> {
    CUSTOMS.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// A champion's wire number, if it has one here.
pub fn to_wire(c: ChampionId) -> Option<u8> {
    if c.is_official() {
        return Some(c.index() as u8);
    }
    let t = CUSTOMS.read().unwrap_or_else(|e| e.into_inner());
    t.iter().position(|&x| x == c).map(|i| OFFICIAL + i as u8)
}

/// The champion a wire number names here.
pub fn from_wire(v: u8) -> Option<ChampionId> {
    if v < OFFICIAL {
        return ChampionId::from_u8(v);
    }
    let t = CUSTOMS.read().unwrap_or_else(|e| e.into_inner());
    t.get((v - OFFICIAL) as usize).copied()
}

/// A roster's hash (the files' hashes in order), so a client can say which one it has loaded
/// (`Hello::roster`); 0 is the empty roster.
pub fn roster_hash(file_hashes: &[u64]) -> u64 {
    if file_hashes.is_empty() {
        return 0;
    }
    let text: String = file_hashes.iter().map(|h| format!("{h:016x}")).collect();
    mftr_sim::mapspec::text_hash(&text).max(1)
}
