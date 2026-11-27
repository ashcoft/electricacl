//! Persistence for the cable sizing calculation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cable::{build_state, CableCircuit, CableReference, CableStateView};
use crate::models::ProjectMeta;

const SEED: &str = include_str!("../data/cable_seed.json");
const STATE_FILE: &str = "cable_state.json";

#[derive(Debug, Deserialize)]
struct SeedFile {
    #[serde(default)]
    project: ProjectMeta,
    #[serde(default)]
    reference: Vec<CableReference>,
    #[serde(default)]
    circuits: Vec<CableCircuit>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Persisted {
    #[serde(default)]
    project: ProjectMeta,
    #[serde(default)]
    reference: Vec<CableReference>,
    #[serde(default)]
    circuits: Vec<CableCircuit>,
}

pub struct CableStore {
    project: ProjectMeta,
    reference: Vec<CableReference>,
    circuits: Vec<CableCircuit>,
    path: PathBuf,
}

fn seed() -> Result<SeedFile> {
    serde_json::from_str(SEED).context("embedded cable seed data is not valid JSON")
}

impl CableStore {
    pub fn load(data_dir: &Path) -> Result<Self> {
        let seed = seed()?;
        let path = data_dir.join(STATE_FILE);
        let mut store = Self {
            project: seed.project,
            reference: seed.reference,
            circuits: seed.circuits,
            path,
        };
        for circuit in &mut store.circuits {
            circuit.sanitize();
        }
        if store.path.exists() {
            let raw = fs::read_to_string(&store.path)
                .with_context(|| format!("reading {}", store.path.display()))?;
            match serde_json::from_str::<Persisted>(&raw) {
                Ok(persisted) => {
                    store.project = persisted.project;
                    store.reference = persisted.reference;
                    store.circuits = persisted.circuits;
                    for circuit in &mut store.circuits {
                        circuit.sanitize();
                    }
                    tracing::info!(
                        circuits = store.circuits.len(),
                        "restored cable sizing state"
                    );
                }
                Err(error) => {
                    tracing::warn!(%error, "cable state file is corrupt, falling back to the seed");
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
            reference: self.reference.clone(),
            circuits: self.circuits.clone(),
        };
        let json = serde_json::to_vec_pretty(&payload)?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("replacing {}", self.path.display()))?;
        Ok(())
    }

    pub fn state(&self) -> CableStateView {
        build_state(&self.project, &self.circuits, &self.reference)
    }

    pub fn reference(&self) -> &[CableReference] {
        &self.reference
    }

    pub fn create(&mut self, mut circuit: CableCircuit) -> Result<CableCircuit> {
        circuit.id = CableCircuit::new_id();
        circuit.sanitize();
        self.circuits.push(circuit.clone());
        self.persist()?;
        Ok(circuit)
    }

    pub fn update(&mut self, id: &str, mut circuit: CableCircuit) -> Result<Option<CableCircuit>> {
        let Some(index) = self.circuits.iter().position(|entry| entry.id == id) else {
            return Ok(None);
        };
        circuit.id = id.to_string();
        circuit.sanitize();
        self.circuits[index] = circuit.clone();
        self.persist()?;
        Ok(Some(circuit))
    }

    pub fn delete(&mut self, id: &str) -> Result<bool> {
        let before = self.circuits.len();
        self.circuits.retain(|entry| entry.id != id);
        let removed = self.circuits.len() != before;
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }

    pub fn duplicate(&mut self, id: &str) -> Result<Option<CableCircuit>> {
        let Some(source) = self.circuits.iter().find(|entry| entry.id == id).cloned() else {
            return Ok(None);
        };
        let mut copy = source;
        copy.id = CableCircuit::new_id();
        copy.tag = copy.tag.map(|tag| format!("{tag}-COPY"));
        copy.si_no = None;
        self.circuits.push(copy.clone());
        self.persist()?;
        Ok(Some(copy))
    }

    pub fn replace_reference(&mut self, mut reference: Vec<CableReference>) -> Result<usize> {
        reference.sort_by(|a, b| {
            a.cores
                .key()
                .cmp(b.cores.key())
                .then(a.size_mm2.total_cmp(&b.size_mm2))
        });
        let count = reference.len();
        self.reference = reference;
        self.persist()?;
        Ok(count)
    }

    pub fn reset(&mut self) -> Result<usize> {
        let seed = seed()?;
        self.project = seed.project;
        self.reference = seed.reference;
        self.circuits = seed.circuits;
        for circuit in &mut self.circuits {
            circuit.sanitize();
        }
        let count = self.circuits.len();
        self.persist()?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_parses_and_every_circuit_has_an_id() {
        let store = CableStore::load(Path::new("/tmp/electricacl-cable-test-nonexistent")).unwrap();
        assert_eq!(store.state().circuits.len(), 10);
        assert!(store
            .state()
            .circuits
            .iter()
            .all(|row| !row.circuit.id.is_empty()));
        assert_eq!(store.reference().len(), 3);
    }
}
