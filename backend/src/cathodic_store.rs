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

fn write_state(path: &Path, project: &ProjectMeta, items: &[CathodicItem]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let payload = Persisted {
        project: project.clone(),
        items: items.to_vec(),
    };
    let json = serde_json::to_vec_pretty(&payload)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
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
                    let stamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|duration| duration.as_secs())
                        .unwrap_or(0);
                    let backup = store.path.with_extension(format!("json.corrupt-{stamp}"));
                    match fs::rename(&store.path, &backup) {
                        Ok(()) => tracing::warn!(
                            %error,
                            backup = %backup.display(),
                            "cathodic state file is corrupt, preserved it and fell back to the seed"
                        ),
                        Err(rename_error) => tracing::warn!(
                            %error,
                            %rename_error,
                            "cathodic state file is corrupt, falling back to the seed"
                        ),
                    }
                }
            }
        }
        Ok(store)
    }

    pub fn persist(&self) -> Result<()> {
        write_state(&self.path, &self.project, &self.items)
    }

    pub fn state(&self) -> CathodicStateView {
        build_state(&self.project, &self.items)
    }

    pub fn create(&mut self, mut item: CathodicItem) -> Result<CathodicItem> {
        item.id = CathodicItem::new_id();
        item.sanitize();
        let mut items = self.items.clone();
        items.push(item.clone());
        write_state(&self.path, &self.project, &items)?;
        self.items = items;
        Ok(item)
    }

    pub fn update(&mut self, id: &str, mut item: CathodicItem) -> Result<Option<CathodicItem>> {
        let Some(index) = self.items.iter().position(|entry| entry.id == id) else {
            return Ok(None);
        };
        item.id = id.to_string();
        item.sanitize();
        let mut items = self.items.clone();
        items[index] = item.clone();
        write_state(&self.path, &self.project, &items)?;
        self.items = items;
        Ok(Some(item))
    }

    pub fn delete(&mut self, id: &str) -> Result<bool> {
        let mut items = self.items.clone();
        items.retain(|entry| entry.id != id);
        let removed = items.len() != self.items.len();
        if removed {
            write_state(&self.path, &self.project, &items)?;
            self.items = items;
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
        let mut items = self.items.clone();
        items.push(copy.clone());
        write_state(&self.path, &self.project, &items)?;
        self.items = items;
        Ok(Some(copy))
    }

    pub fn reset(&mut self) -> Result<usize> {
        let seed = seed()?;
        let mut items = seed.items;
        for item in &mut items {
            item.sanitize();
        }
        write_state(&self.path, &seed.project, &items)?;
        self.project = seed.project;
        self.items = items;
        Ok(self.items.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_state_is_preserved_before_the_seed_is_written() {
        let dir = std::env::temp_dir().join(format!(
            "eacl-cathodic-corrupt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(STATE_FILE);
        fs::write(&path, "{ not json").unwrap();
        let store = CathodicStore::load(&dir).unwrap();
        assert_eq!(store.state().items.len(), 23);
        assert!(!path.exists());
        let backups = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
            .count();
        assert_eq!(backups, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_write_does_not_publish_the_mutation() {
        let dir = std::env::temp_dir().join(format!(
            "eacl-cathodic-write-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let mut store = CathodicStore::load(&dir).unwrap();
        let before = store.state().items.len();
        fs::create_dir(dir.join(STATE_FILE)).unwrap();
        let result = store.create(CathodicItem::default());
        assert!(result.is_err());
        assert_eq!(store.state().items.len(), before);
        let _ = fs::remove_dir_all(&dir);
    }

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
