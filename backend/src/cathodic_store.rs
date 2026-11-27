//! Persistence for the cathodic protection calculation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cathodic::{build_state, CathodicItem, CathodicStateView};
use crate::models::ProjectMeta;

const SEED: &str = include_str!("../data/cathodic_seed.json");
const STATE_FILE: &str = "cathodic_state.json";

#[derive(Debug, Deserialize)]
struct SeedFile {
    #[serde(default)]
    project: ProjectMeta,
    #[serde(default)]
    items: Vec<CathodicItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Persisted {
    #[serde(default)]
    project: ProjectMeta,
    #[serde(default)]
    items: Vec<CathodicItem>,
}

pub struct CathodicStore {
    project: ProjectMeta,
    items: Vec<CathodicItem>,
    path: PathBuf,
}

fn seed() -> Result<SeedFile> {
    serde_json::from_str(SEED).context("embedded cathodic seed data is not valid JSON")
}

impl CathodicStore {
    pub fn load(data_dir: &Path) -> Result<Self> {
        let seed = seed()?;
        let path = data_dir.join(STATE_FILE);
        let mut store = Self {
            project: seed.project,
            items: seed.items,
            path,
        };
        for item in &mut store.items {
            item.sanitize();
        }
        if store.path.exists() {
            let raw = fs::read_to_string(&store.path)
                .with_context(|| format!("reading {}", store.path.display()))?;
            match serde_json::from_str::<Persisted>(&raw) {
                Ok(persisted) => {
                    store.project = persisted.project;
                    store.items = persisted.items;
                    for item in &mut store.items {
                        item.sanitize();
                    }
                    tracing::info!(
                        items = store.items.len(),
                        "restored cathodic protection state"
                    );
                }
                Err(error) => {
                    tracing::warn!(%error, "cathodic state file is corrupt, falling back to the seed");
                }
            }
        }
        Ok(store)
    }

    pub fn persist(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        let payload = Persisted {
            project: self.project.clone(),
            items: self.items.clone(),
        };
        let json = serde_json::to_vec_pretty(&payload)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("replacing {}", self.path.display()))?;
        Ok(())
    }

    pub fn state(&self) -> CathodicStateView {
        build_state(&self.project, &self.items)
    }

    pub fn create(&mut self, mut item: CathodicItem) -> Result<CathodicItem> {
        item.id = CathodicItem::new_id();
        item.sanitize();
        self.items.push(item.clone());
        self.persist()?;
        Ok(item)
    }

    pub fn update(&mut self, id: &str, mut item: CathodicItem) -> Result<Option<CathodicItem>> {
        let Some(index) = self.items.iter().position(|entry| entry.id == id) else {
            return Ok(None);
        };
        item.id = id.to_string();
        item.sanitize();
        self.items[index] = item.clone();
        self.persist()?;
        Ok(Some(item))
    }

    pub fn delete(&mut self, id: &str) -> Result<bool> {
        let before = self.items.len();
        self.items.retain(|entry| entry.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    pub fn duplicate(&mut self, id: &str) -> Result<Option<CathodicItem>> {
        let Some(source) = self.items.iter().find(|entry| entry.id == id).cloned() else {
            return Ok(None);
        };
        let mut copy = source;
        copy.id = CathodicItem::new_id();
        copy.tag = copy.tag.map(|tag| format!("{tag}-COPY"));
        copy.source_sheet = None;
        self.items.push(copy.clone());
        self.persist()?;
        Ok(Some(copy))
    }

    pub fn reset(&mut self) -> Result<usize> {
        let seed = seed()?;
        self.project = seed.project;
        self.items = seed.items;
        for item in &mut self.items {
            item.sanitize();
        }
        let count = self.items.len();
        self.persist()?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_parses_and_every_item_has_an_id() {
        let store =
            CathodicStore::load(Path::new("/tmp/electricacl-cathodic-test-nonexistent")).unwrap();
        assert_eq!(store.state().items.len(), 23);
        assert!(store
            .state()
            .items
            .iter()
            .all(|row| !row.item.id.is_empty()));
        assert!(store.state().stats.sacp >= 20);
    }
}
