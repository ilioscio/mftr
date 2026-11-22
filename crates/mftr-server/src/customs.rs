//! A server's custom champions (CR1d, 20 §13): champion files it was started with, offered
//! beside or instead of the official ten (`--champions official|customs|both`). Every file is
//! read and checked like a pack (the strict reader and `mftr_sim::limits`), registered, and given
//! a wire number (10 and up, `mftr_net::champions`). Clients fetch the files before they join:
//! a Hello whose roster isn't this one is answered with `ServerMessage::Roster`.

use std::sync::Arc;

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
    /// Read, check and register champion files (at most `mftr_net::champions::MAX_CUSTOMS`),
    /// and give them wire numbers in this process.
    pub fn load(texts: Vec<String>) -> Result<Customs, String> {
        if texts.len() > mftr_net::champions::MAX_CUSTOMS {
            return Err(format!("at most {} champion files", mftr_net::champions::MAX_CUSTOMS));
        }
        let mut ids = Vec::new();
        for text in &texts {
            let id = mftr_sim::limits::load_champion_file(text)?;
            if id.is_official() || ids.contains(&id) {
                return Err(format!("`{}` is an official champion or given twice", id.def().name));
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
