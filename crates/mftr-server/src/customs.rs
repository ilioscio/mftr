//! A server's custom champions (CR1d, 20 §13): champion files it was started with, offered
//! beside or instead of the official ten (`--champions official|customs|both`). Every file is
//! read and checked like a pack (the strict reader and `mftr_sim::limits`), registered, and given
//! a wire number (10 and up, `mftr_net::champions`). Clients fetch the files before they join:
//! a Hello whose roster isn't this one is answered with `ServerMessage::Roster`.
//!
//! Players may also bring their own champion files to champion select (D83, 20 §13.1):
//! [`accept`] checks one the way a client will, and a match's [`Brought`] holds those it took.

use std::collections::{BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mftr_net::content::ContentId;
use mftr_sim::ChampionId;

/// Which champions a server's matches draw from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChampionPolicy {
    /// The official ten (the default).
    #[default]
    Official,
    /// The server's champion files only (the official ten if it has none).
    Customs,
    Both,
}

impl ChampionPolicy {
    pub fn by_name(name: &str) -> Option<ChampionPolicy> {
        match name {
            "official" => Some(ChampionPolicy::Official),
            "customs" => Some(ChampionPolicy::Customs),
            "both" => Some(ChampionPolicy::Both),
            _ => None,
        }
    }
}

/// The champion files, in wire order, their content ids (D83) and their registry ids.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Customs {
    pub texts: Arc<[Arc<str>]>,
    pub files: Arc<[ContentId]>,
    pub ids: Arc<[ChampionId]>,
}

impl Customs {
    /// Read, check and register a server's champion files (at most
    /// `mftr_net::champions::HOST_CUSTOMS`), and give them wire numbers in this process.
    pub fn load(texts: Vec<String>) -> Result<Customs, String> {
        Self::load_with(texts, true, mftr_net::champions::HOST_CUSTOMS)
    }

    /// [`Customs::load`] with the price budget or not, and up to `cap` files: a replay holds
    /// the server's files and those its players brought, as they were played.
    pub fn load_with(texts: Vec<String>, budget: bool, cap: usize) -> Result<Customs, String> {
        if texts.len() > cap {
            return Err(format!("at most {cap} champion files"));
        }
        let mut ids = Vec::new();
        for text in &texts {
            let id = mftr_sim::limits::load_champion_file_with(text, budget)?;
            if id.is_official() || official_name(id.def().name) || ids.contains(&id) {
                return Err(format!("`{}` is an official champion's name or given twice", id.def().name));
            }
            ids.push(id);
        }
        mftr_net::champions::set_customs(&ids);
        let files = texts.iter().map(|t| ContentId::of(t)).collect();
        Ok(Customs { texts: texts.into_iter().map(Arc::from).collect(), files, ids: ids.into() })
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// What `ServerMessage::Roster` carries: each file's content id and length.
    pub fn roster(&self) -> Vec<(ContentId, u32)> {
        self.files.iter().zip(self.texts.iter()).map(|(id, t)| (*id, t.len() as u32)).collect()
    }

    /// The roster's hash, as a client that has loaded it says in its Hello.
    pub fn hash(&self) -> u64 {
        mftr_net::champions::roster_hash(&self.files)
    }

    /// A champion file by its content id (`MapRequest`).
    pub fn file(&self, id: ContentId) -> Option<&Arc<str>> {
        self.files.iter().position(|f| *f == id).map(|i| &self.texts[i])
    }

    /// Each file's content id with its champion, in wire order.
    pub fn entries(&self) -> impl Iterator<Item = (ContentId, ChampionId)> + '_ {
        self.files.iter().copied().zip(self.ids.iter().copied())
    }

    /// The champions a match draws from under `policy`.
    pub fn pool(&self, policy: ChampionPolicy) -> Vec<ChampionId> {
        let official = ChampionId::ALL.to_vec();
        match policy {
            ChampionPolicy::Official => official,
            ChampionPolicy::Customs if self.is_empty() => official,
            ChampionPolicy::Customs => self.ids.to_vec(),
            ChampionPolicy::Both => official.into_iter().chain(self.ids.iter().copied()).collect(),
        }
    }
}

/// Whether `name` is an official champion's (any case): a custom one may not take it.
fn official_name(name: &str) -> bool {
    ChampionId::ALL.iter().any(|c| c.def().name.eq_ignore_ascii_case(name))
}

/// The champions players brought to one match's champion select (D83), after the server's
/// own in wire order: each file's text, content id and registry id.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Brought {
    pub texts: Vec<Arc<str>>,
    pub files: Vec<ContentId>,
    pub ids: Vec<ChampionId>,
}

impl Brought {
    /// Add an accepted file (once: the same file brought twice is one champion).
    pub fn add(&mut self, text: Arc<str>, file: ContentId, id: ChampionId) -> bool {
        if self.files.contains(&file) {
            return true;
        }
        if self.files.len() >= mftr_net::champions::MAX_BROUGHT {
            return false;
        }
        self.texts.push(text);
        self.files.push(file);
        self.ids.push(id);
        true
    }

    /// A brought file by its content id (`MapRequest`).
    pub fn file(&self, id: ContentId) -> Option<&Arc<str>> {
        self.files.iter().position(|f| *f == id).map(|i| &self.texts[i])
    }

    /// What `LobbyState::brought` carries.
    pub fn listed(&self) -> Vec<(ContentId, u32)> {
        self.files.iter().zip(self.texts.iter()).map(|(f, t)| (*f, t.len() as u32)).collect()
    }
}

/// New champion definitions one server process takes from players in an hour (D83). The
/// registry holds 4,096 for the life of the process and its definitions can't be freed (the
/// sim hands out `&'static` ones), so this keeps a stream of distinct uploads from filling it:
/// at this rate that takes days. Files it has taken before don't count.
pub const NEW_PER_HOUR: usize = 60;

/// When this process took each new file in the last hour, and every file it has taken.
static TAKEN: Mutex<(VecDeque<Instant>, BTreeSet<ContentId>)> = Mutex::new((VecDeque::new(), BTreeSet::new()));

/// Check a champion file a player brings (D83) as every client will, then register it: the
/// strict reader, the limits, the price (unless `budget` is off), its looks, a name no
/// official champion and no other champion in the match (`roster`) has, and the hourly
/// limit on new files. The checks run without interning (`limits::check_champion_file`), so
/// a refused file leaves nothing behind. Returns the champion, or why it was refused.
pub fn accept(text: &str, budget: bool, roster: &[(ContentId, ChampionId)]) -> Result<ChampionId, String> {
    if text.len() > mftr_sim::data::MAX_INPUT {
        return Err(format!("longer than {} KB", mftr_sim::data::MAX_INPUT / 1024));
    }
    let name = mftr_sim::limits::check_champion_file(text, budget)?;
    // The move library is read the first time it's used: not inside the check below, where its
    // names would be placeholders for good.
    let _ = mftr_forge::libmoves::all();
    let looks = mftr_sim::data::without_interning(|| {
        std::panic::catch_unwind(|| mftr_forge::looks::of_champion_file(text).map(|_| ()).map_err(|e| e.to_string()))
            .unwrap_or_else(|_| Err("its looks broke the reader (a bug: please report it)".into()))
    });
    looks.0.map_err(|e| format!("`{name}`'s looks: {}", mftr_sim::data::restore_names(&e, &looks.1)))?;
    if official_name(&name) {
        return Err(format!("`{name}` is an official champion's name"));
    }
    let file = ContentId::of(text);
    if let Some((_, other)) = roster.iter().find(|(f, c)| *f != file && c.def().name.eq_ignore_ascii_case(&name)) {
        return Err(format!("a different champion named `{}` is in this match already", other.def().name));
    }
    let mut taken = TAKEN.lock().unwrap_or_else(|e| e.into_inner());
    if !taken.1.contains(&file) {
        let now = Instant::now();
        while taken.0.front().is_some_and(|t| now.duration_since(*t) > Duration::from_secs(3600)) {
            taken.0.pop_front();
        }
        if taken.0.len() >= NEW_PER_HOUR {
            return Err("this server has taken many new champions this hour: try again later".into());
        }
    }
    let id = mftr_sim::limits::load_champion_file_with(text, budget).map_err(|e| {
        if e.contains("too many") { "this server can't take more new champions until it restarts".into() } else { e }
    })?;
    if taken.1.insert(file) {
        taken.0.push_back(Instant::now());
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRUMBLE: &str = include_str!("../../../art/champions/examples/grumble.champion.ron");

    /// Grumble at full strength: within the limits, over the price budget.
    fn strong() -> String {
        let t = GRUMBLE.replace(
            "max_health: 670, health_regen: 1.8, armor: 34, magic_resist: 32, attack_damage: 66, move_speed: 335",
            "max_health: 900, health_regen: 1.8, armor: 60, magic_resist: 55, attack_damage: 80, move_speed: 360",
        );
        assert_ne!(t, GRUMBLE, "Grumble's stats are in the file");
        t.replace("Old Grumble", "Big Grumble").replace("name: \"Grumble\"", "name: \"Big Grumble\"")
    }

    /// D83: a brought file is checked like a client checks it, with the budget or without.
    #[test]
    fn brought_files_are_checked_like_a_clients_check() {
        let id = accept(GRUMBLE, true, &[]).unwrap();
        // Checking first in a process leaves the tables it builds as they should be.
        assert_eq!(ChampionId::Rook.def().name, "Rook");
        assert!(mftr_forge::libmoves::all().iter().all(|m| !m.ability.name.contains(char::from(1))));
        assert_eq!(id.def().name, "Grumble");
        assert_eq!(accept(GRUMBLE, true, &[(ContentId::of(GRUMBLE), id)]), Ok(id), "the same file again");

        let strong = strong();
        let e = accept(&strong, true, &[]).unwrap_err();
        assert!(e.contains("budget"), "{e}");
        let big = accept(&strong, false, &[]).unwrap();
        assert_eq!(big.def().name, "Big Grumble", "a host lifted the budget");

        // Names: no official champion's, and no other champion's in the match.
        let ember = GRUMBLE.replace("name: \"Grumble\"", "name: \"ember\"");
        assert!(accept(&ember, true, &[]).unwrap_err().contains("official"));
        let other = GRUMBLE.replace("max_health: 670", "max_health: 600");
        let e = accept(&other, true, &[(ContentId::of(GRUMBLE), id)]).unwrap_err();
        assert!(e.contains("different champion named `Grumble`"), "{e}");

        // Broken files and broken looks are refused with a reason.
        assert!(!accept("Champion((", true, &[]).unwrap_err().is_empty());
        let looks = GRUMBLE.replace("build: \"humanoid_slim\"", "build: \"no_such_build\"");
        let e = accept(&looks, true, &[]).unwrap_err();
        assert!(e.contains("looks"), "{e}");
    }
}
