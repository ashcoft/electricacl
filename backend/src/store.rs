use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::calc::{build_state, MotorTable, StateView};
use crate::models::{CalcSettings, LoadItem, MotorRef, ProjectMeta};

const SEED: &str = include_str!("../data/seed.json");
const STATE_FILE: &str = "load_list_state.json";

#[derive(Debug, Deserialize)]
struct SeedFile {
    #[serde(default)]
    project: ProjectMeta,
    #[serde(default)]
    motor_table: Vec<MotorRef>,
    #[serde(default)]
    items: Vec<LoadItem>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Persisted {
    #[serde(default)]
    project: ProjectMeta,
    #[serde(default)]
    settings: CalcSettings,
    #[serde(default)]
    items: Vec<LoadItem>,
}

pub struct Store {
    project: ProjectMeta,
    settings: CalcSettings,
    items: Vec<LoadItem>,
    motor_table: MotorTable,
    path: PathBuf,
}

fn seed() -> Result<SeedFile> {
    serde_json::from_str(SEED).context("embedded seed data is not valid JSON")
}

impl Store {
    pub fn load(data_dir: &Path) -> Result<Self> {
        let seed = seed()?;
        let path = data_dir.join(STATE_FILE);
        let mut store = Self {
            project: seed.project,
            settings: CalcSettings::default(),
            items: seed.items,
            motor_table: MotorTable::new(seed.motor_table),
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
                    store.settings = persisted.settings;
                    store.items = persisted.items;
                    for item in &mut store.items {
                        item.sanitize();
                    }
                    tracing::info!(items = store.items.len(), "restored load list state");
                }
                Err(error) => {
                    tracing::warn!(%error, "state file is corrupt, falling back to seed data");
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
            settings: self.settings.clone(),
            items: self.items.clone(),
        };
        let json = serde_json::to_vec_pretty(&payload)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("replacing {}", self.path.display()))?;
        Ok(())
    }

    pub fn items(&self) -> &[LoadItem] {
        &self.items
    }

    pub fn motor_table(&self) -> &MotorTable {
        &self.motor_table
    }

    pub fn state(&self) -> StateView {
        build_state(
            &self.project,
            &self.settings,
            &self.items,
            &self.motor_table,
        )
    }

    pub fn settings(&self) -> &CalcSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, mut settings: CalcSettings) -> Result<CalcSettings> {
        settings.sanitize();
        self.settings = settings.clone();
        self.persist()?;
        Ok(settings)
    }

    pub fn create(&mut self, mut item: LoadItem) -> Result<LoadItem> {
        item.id = LoadItem::new_id();
        item.sanitize();
        self.items.push(item.clone());
        self.persist()?;
        Ok(item)
    }

    pub fn update(&mut self, id: &str, mut item: LoadItem) -> Result<Option<LoadItem>> {
        let Some(index) = self.items.iter().position(|existing| existing.id == id) else {
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
        self.items.retain(|item| item.id != id);
        let removed = self.items.len() != before;
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    pub fn duplicate(&mut self, id: &str) -> Result<Option<LoadItem>> {
        let Some(source) = self.items.iter().find(|item| item.id == id).cloned() else {
            return Ok(None);
        };
        let mut copy = source;
        copy.id = LoadItem::new_id();
        copy.tag = copy.tag.map(|tag| format!("{tag}-COPY"));
        copy.line_no = None;
        self.items.push(copy.clone());
        self.persist()?;
        Ok(Some(copy))
    }

    pub fn replace_all(&mut self, mut items: Vec<LoadItem>) -> Result<usize> {
        for item in &mut items {
            item.sanitize();
        }
        let count = items.len();
        self.items = items;
        self.persist()?;
        Ok(count)
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
        let store = Store::load(Path::new("/tmp/electricacl-test-nonexistent")).unwrap();
        assert!(store.items().len() > 600);
        assert!(store.items().iter().all(|item| !item.id.is_empty()));
        assert!(!store.motor_table().rows().is_empty());
    }
}
