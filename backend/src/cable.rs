//! Cable sizing calculation (calculation 02).
//!
//! Reproduces `0000-EF-D-41335_0 Power Cable Sizing Calc.XLSX`:
//!
//! ```text
//! IFLC  = kW x 1000 / (k x V x cos phi)          k = 1.732 (3-ph) or 1 (1-ph)
//! IZ    = It x Cg x Ca                            It from AS/NZS 3008 Table 14
//! runs  = IFLC / IZ
//! VD    = k x IFLC x (L/1000) / runs x (R cos phi + X sin phi)
//! %VD   = VD / V x 100
//! %VDlast = sum of %VD along the chain from the panel to the last fixture
//! Lmax  = (allowable % x V / 100) / (k x IFLC / runs x 1/1000 x (R cos phi + X sin phi))
//! ```
//!
//! The workbook keeps the cumulative drop and the chain length in a single
//! "head" row per chain, which is what `group` models here.

use serde::{Deserialize, Serialize};

/// A data-quality finding for a circuit.
#[derive(Debug, Clone, Serialize)]
pub struct CableWarning {
    pub code: &'static str,
    pub message: String,
}

pub const SQRT3: f64 = 1.732;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phases {
    #[default]
    Single,
    Three,
}

impl Phases {
    pub fn key(self) -> &'static str {
        match self {
            Phases::Single => "single",
            Phases::Three => "three",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Phases::Single => "Single phase",
            Phases::Three => "Three phase",
        }
    }

    /// Constant used for the load current: 1 for single phase, 1.732 otherwise.
    pub fn current_factor(self) -> f64 {
        match self {
            Phases::Single => 1.0,
            Phases::Three => SQRT3,
        }
    }

    /// Constant used for the voltage drop: 2 for single phase, 1.732 otherwise.
    pub fn drop_factor(self) -> f64 {
        match self {
            Phases::Single => 2.0,
            Phases::Three => SQRT3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CableCores {
    #[default]
    TwoCoreEarth,
    ThreeCoreEarth,
    FourCoreEarth,
}

impl CableCores {
    pub const ALL: [CableCores; 3] = [
        CableCores::TwoCoreEarth,
        CableCores::ThreeCoreEarth,
        CableCores::FourCoreEarth,
    ];

    pub fn key(self) -> &'static str {
        match self {
            CableCores::TwoCoreEarth => "2C+E",
            CableCores::ThreeCoreEarth => "3C+E",
            CableCores::FourCoreEarth => "4C+E",
        }
    }
}

/// One row of the AS/NZS 3008 reference tables (Attach-3/4/5 of the workbook).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CableReference {
    pub cores: CableCores,
    pub size_mm2: f64,
    /// `It`, current carrying capacity in air (Table 14).
    pub ampacity_a: f64,
    /// `R`, a.c. resistance at 90 °C (Table 35).
    pub resistance_ohm_km: f64,
    /// `X`, reactance at 50 Hz, XLPE (Table 30).
    pub reactance_ohm_km: f64,
    #[serde(default)]
    pub source: Option<String>,
}

impl CableReference {
    pub fn label(&self) -> String {
        format!("{} x {} mm2", self.cores.key(), self.size_mm2)
    }
}

fn default_runs() -> u32 {
    1
}
fn default_factor() -> f64 {
    1.0
}
fn default_grouping() -> f64 {
    0.66
}
fn default_pf() -> f64 {
    0.9
}
fn default_voltage() -> f64 {
    230.0
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CableCircuit {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub si_no: Option<i64>,
    #[serde(default)]
    pub tag: Option<String>,
    /// Chain identifier: circuits sharing a group are in series, so their drops
    /// add up. The first circuit of a group reports the cumulative drop.
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub voltage_grade: Option<String>,
    #[serde(default)]
    pub cores: CableCores,
    #[serde(default)]
    pub size_mm2: f64,
    #[serde(default)]
    pub cable_type: Option<String>,
    #[serde(default)]
    pub length_m: f64,
    #[serde(default)]
    pub from_description: Option<String>,
    #[serde(default)]
    pub from_tag: Option<String>,
    #[serde(default)]
    pub from_location: Option<String>,
    #[serde(default)]
    pub to_description: Option<String>,
    #[serde(default)]
    pub to_tag: Option<String>,
    #[serde(default)]
    pub to_location: Option<String>,
    #[serde(default)]
    pub building: Option<String>,
    #[serde(default)]
    pub feeder_load: Option<String>,
    #[serde(default)]
    pub equipment_kw: Option<f64>,
    /// `R` column: kW loading of the circuit (per run basis, as in the sheet).
    #[serde(default)]
    pub load_kw: f64,
    #[serde(default = "default_voltage")]
    pub voltage_v: f64,
    #[serde(default)]
    pub phases: Phases,
    #[serde(default = "default_pf")]
    pub power_factor: f64,
    /// `IPR`: protection device trip setting.
    #[serde(default)]
    pub protection_rating_a: f64,
    #[serde(default)]
    pub breaker_rating_a: f64,
    #[serde(default)]
    pub breaker_type: Option<String>,
    #[serde(default = "default_runs")]
    pub runs_selected: u32,
    /// `Cg`, grouping factor (Table 24).
    #[serde(default = "default_grouping")]
    pub grouping_factor: f64,
    /// `Ca`, ambient temperature factor (Table 27).
    #[serde(default = "default_factor")]
    pub temperature_factor: f64,
    /// Allowable voltage drop from the panel to the last fixture, in %.
    #[serde(default)]
    pub allowable_vd_pct: Option<f64>,
    #[serde(default)]
    pub remark: Option<String>,
}

impl CableCircuit {
    pub fn new_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    pub fn sanitize(&mut self) {
        if self.id.trim().is_empty() {
            self.id = Self::new_id();
        }
        if self.voltage_v <= 0.0 {
            self.voltage_v = default_voltage();
        }
        if self.size_mm2 < 0.0 {
            self.size_mm2 = 0.0;
        }
        self.length_m = self.length_m.max(0.0);
        self.load_kw = self.load_kw.max(0.0);
        self.power_factor = self.power_factor.clamp(0.01, 1.0);
        self.grouping_factor = self.grouping_factor.clamp(0.01, 1.5);
        self.temperature_factor = self.temperature_factor.clamp(0.01, 1.5);
        self.runs_selected = self.runs_selected.max(1);
        self.protection_rating_a = self.protection_rating_a.max(0.0);
        self.breaker_rating_a = self.breaker_rating_a.max(0.0);
        for field in [
            &mut self.tag,
            &mut self.group,
            &mut self.voltage_grade,
            &mut self.cable_type,
            &mut self.from_description,
            &mut self.from_tag,
            &mut self.from_location,
            &mut self.to_description,
            &mut self.to_tag,
            &mut self.to_location,
            &mut self.building,
            &mut self.feeder_load,
            &mut self.breaker_type,
            &mut self.remark,
        ] {
            if let Some(value) = field.as_mut() {
                let trimmed = value.trim().to_string();
                *field = if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                };
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CableChecks {
    /// IZ >= IFLC
    pub ampacity: bool,
    /// IZ > IPR
    pub protection: bool,
    /// %VD last <= allowable %
    pub voltage_drop: Option<bool>,
    /// maximum length >= total circuit length
    pub max_length: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CableDerived {
    pub iflc_a: f64,
    /// `It` from the reference table.
    pub ampacity_a: f64,
    /// `CO` = Cg x Ca
    pub derating: f64,
    pub iz_a: f64,
    pub runs_required: f64,
    pub vd_v: f64,
    pub vd_pct: f64,
    /// Cumulative drop from the panel to the last fixture (chain head only).
    pub vd_last_pct: Option<f64>,
    pub chain_length_m: Option<f64>,
    pub max_length_m: Option<f64>,
    pub voltage_at_end_v: f64,
    pub checks: CableChecks,
    pub acceptable: bool,
    pub warnings: Vec<CableWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CableRow {
    #[serde(flatten)]
    pub circuit: CableCircuit,
    pub derived: CableDerived,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CableStats {
    pub circuits: usize,
    pub acceptable: usize,
    pub failed: usize,
    pub groups: usize,
    pub total_length_m: f64,
    pub reference_rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CableStateView {
    pub project: crate::models::ProjectMeta,
    pub reference: Vec<CableReference>,
    pub circuits: Vec<CableRow>,
    pub stats: CableStats,
}

fn find_reference(
    reference: &[CableReference],
    cores: CableCores,
    size_mm2: f64,
) -> Option<&CableReference> {
    reference
        .iter()
        .find(|row| row.cores == cores && (row.size_mm2 - size_mm2).abs() < 1e-9)
}

/// Evaluate every circuit, including the cumulative drop per chain.
pub fn evaluate(circuits: &[CableCircuit], reference: &[CableReference]) -> Vec<CableRow> {
    // First pass: per-circuit values (no chain aggregation yet).
    struct Partial {
        derived: CableDerived,
    }

    let mut partials: Vec<Partial> = Vec::with_capacity(circuits.len());
    for circuit in circuits {
        let mut warnings = Vec::new();
        let pf = circuit.power_factor.clamp(0.01, 1.0);
        let sin_phi = (1.0 - pf * pf).max(0.0).sqrt();
        let k_current = circuit.phases.current_factor();
        let k_drop = circuit.phases.drop_factor();
        let iflc = if circuit.voltage_v > 0.0 {
            circuit.load_kw * 1000.0 / (k_current * circuit.voltage_v * pf)
        } else {
            0.0
        };

        let reference_row = find_reference(reference, circuit.cores, circuit.size_mm2);
        let (ampacity, resistance, reactance) = match reference_row {
            Some(row) => (row.ampacity_a, row.resistance_ohm_km, row.reactance_ohm_km),
            None => {
                warnings.push(CableWarning {
                    code: "cable_not_in_table",
                    message: format!(
                        "{} x {} mm2 is not in the cable reference table - ampacity and impedance unknown",
                        circuit.cores.key(),
                        circuit.size_mm2
                    ),
                });
                (0.0, 0.0, 0.0)
            }
        };

        let derating = circuit.grouping_factor * circuit.temperature_factor;
        let iz = ampacity * derating;
        let runs = circuit.runs_selected.max(1) as f64;
        let runs_required = if iz > 0.0 { iflc / iz } else { 0.0 };
        let impedance = resistance * pf + reactance * sin_phi;
        let vd_v = k_drop * iflc * (circuit.length_m / 1000.0) / runs * impedance;
        let vd_pct = if circuit.voltage_v > 0.0 {
            vd_v / circuit.voltage_v * 100.0
        } else {
            0.0
        };
        let max_length_m = circuit.allowable_vd_pct.and_then(|allowable| {
            let denominator = k_drop * iflc / runs * (1.0 / 1000.0) * impedance;
            (denominator > 0.0).then(|| (allowable / 100.0 * circuit.voltage_v) / denominator)
        });

        if circuit.load_kw <= 0.0 {
            warnings.push(CableWarning {
                code: "no_load",
                message: "No load entered - the current and voltage drop are zero".into(),
            });
        }
        if iz > 0.0 && iflc > iz {
            warnings.push(CableWarning {
                code: "ampacity_exceeded",
                message: format!(
                    "Design current {iflc:.1} A exceeds the derated capacity {iz:.1} A",
                ),
            });
        }
        if iz > 0.0 && circuit.protection_rating_a > 0.0 && iz <= circuit.protection_rating_a {
            warnings.push(CableWarning {
                code: "protection_not_coordinated",
                message: format!(
                    "Derated capacity {iz:.1} A does not exceed the protection setting {:.1} A",
                    circuit.protection_rating_a
                ),
            });
        }

        let checks = CableChecks {
            ampacity: iz > 0.0 && iz >= iflc,
            protection: iz > 0.0 && iz > circuit.protection_rating_a,
            voltage_drop: None,
            max_length: None,
        };

        partials.push(Partial {
            derived: CableDerived {
                iflc_a: iflc,
                ampacity_a: ampacity,
                derating,
                iz_a: iz,
                runs_required,
                vd_v,
                vd_pct,
                vd_last_pct: None,
                chain_length_m: None,
                max_length_m,
                voltage_at_end_v: circuit.voltage_v - vd_v,
                checks,
                acceptable: false,
                warnings,
            },
        });
    }

    // Second pass: chain aggregation, in circuit order.
    let mut rows: Vec<CableRow> = circuits
        .iter()
        .zip(partials)
        .map(|(circuit, partial)| CableRow {
            circuit: circuit.clone(),
            derived: partial.derived,
        })
        .collect();

    let groups: Vec<String> = {
        let mut seen = Vec::new();
        for row in &rows {
            if let Some(group) = &row.circuit.group {
                if !seen.contains(group) {
                    seen.push(group.clone());
                }
            }
        }
        seen
    };

    for group in groups {
        let members: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.circuit.group.as_deref() == Some(group.as_str()))
            .map(|(index, _)| index)
            .collect();
        if members.is_empty() {
            continue;
        }
        let total_pct: f64 = members
            .iter()
            .map(|index| rows[*index].derived.vd_pct)
            .sum();
        let total_length: f64 = members
            .iter()
            .map(|index| rows[*index].circuit.length_m)
            .sum();
        let head = members[0];
        rows[head].derived.vd_last_pct = Some(total_pct);
        rows[head].derived.chain_length_m = Some(total_length);

        // Voltage at the end of each circuit, accumulated along the chain.
        let nominal = rows[head].circuit.voltage_v;
        let mut drop = 0.0;
        for index in &members {
            drop += rows[*index].derived.vd_v;
            rows[*index].derived.voltage_at_end_v = nominal - drop;
        }
    }

    // A circuit that is not part of a chain is its own chain.
    for row in &mut rows {
        if row.circuit.group.is_none() {
            row.derived.vd_last_pct = Some(row.derived.vd_pct);
            row.derived.chain_length_m = Some(row.circuit.length_m);
        }
    }

    // Third pass: the checks that depend on the chain.
    for row in &mut rows {
        if let (Some(allowable), Some(last)) =
            (row.circuit.allowable_vd_pct, row.derived.vd_last_pct)
        {
            row.derived.checks.voltage_drop = Some(allowable >= last);
        }
        if let (Some(max), Some(total)) = (row.derived.max_length_m, row.derived.chain_length_m) {
            row.derived.checks.max_length = Some(max >= total);
        }
        let checks = row.derived.checks;
        row.derived.acceptable = checks.ampacity
            && checks.protection
            && checks.voltage_drop.unwrap_or(true)
            && checks.max_length.unwrap_or(true)
            && !row
                .derived
                .warnings
                .iter()
                .any(|warning| warning.code == "cable_not_in_table");
    }

    rows
}

pub fn stats(rows: &[CableRow], reference: &[CableReference]) -> CableStats {
    let mut groups: Vec<&str> = Vec::new();
    for row in rows {
        if let Some(group) = row.circuit.group.as_deref() {
            if !groups.contains(&group) {
                groups.push(group);
            }
        }
    }
    let acceptable = rows.iter().filter(|row| row.derived.acceptable).count();
    CableStats {
        circuits: rows.len(),
        acceptable,
        failed: rows.len() - acceptable,
        groups: groups.len(),
        total_length_m: rows.iter().map(|row| row.circuit.length_m).sum(),
        reference_rows: reference.len(),
    }
}

pub fn build_state(
    project: &crate::models::ProjectMeta,
    circuits: &[CableCircuit],
    reference: &[CableReference],
) -> CableStateView {
    let rows = evaluate(circuits, reference);
    CableStateView {
        project: project.clone(),
        reference: reference.to_vec(),
        stats: stats(&rows, reference),
        circuits: rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> Vec<CableReference> {
        vec![CableReference {
            cores: CableCores::TwoCoreEarth,
            size_mm2: 2.5,
            ampacity_a: 26.0,
            resistance_ohm_km: 9.45,
            reactance_ohm_km: 0.0988,
            source: None,
        }]
    }

    fn circuit(load_kw: f64, length_m: f64, allowable: Option<f64>) -> CableCircuit {
        CableCircuit {
            cores: CableCores::TwoCoreEarth,
            size_mm2: 2.5,
            load_kw,
            length_m,
            voltage_v: 230.0,
            phases: Phases::Single,
            power_factor: 0.9,
            protection_rating_a: 10.0,
            grouping_factor: 0.66,
            temperature_factor: 1.0,
            allowable_vd_pct: allowable,
            ..Default::default()
        }
    }

    #[test]
    fn single_phase_values_match_the_workbook() {
        let rows = evaluate(&[circuit(0.36, 50.0, Some(3.0))], &reference());
        let derived = &rows[0].derived;
        assert!((derived.iflc_a - 1.7391304347826086).abs() < 1e-9);
        assert!((derived.iz_a - 17.16).abs() < 1e-9);
        assert!((derived.runs_required - 0.10134792743488395).abs() < 1e-12);
        assert!((derived.vd_v - 1.4866201602716835).abs() < 1e-9);
        assert!((derived.vd_pct - 0.646356591422471).abs() < 1e-9);
        assert!((derived.max_length_m.unwrap() - 232.07003996027498).abs() < 1e-6);
        assert!(derived.acceptable);
    }

    #[test]
    fn three_phase_uses_the_root_three_factors() {
        let mut circuit = circuit(25.0, 25.0, Some(1.0));
        circuit.voltage_v = 400.0;
        circuit.phases = Phases::Three;
        circuit.power_factor = 1.0;
        circuit.cores = CableCores::TwoCoreEarth;
        circuit.size_mm2 = 2.5;
        let mut reference = reference();
        reference[0].ampacity_a = 137.0;
        reference[0].resistance_ohm_km = 0.669;
        reference[0].reactance_ohm_km = 0.0786;
        let rows = evaluate(&[circuit], &reference);
        let derived = &rows[0].derived;
        assert!((derived.iflc_a - 36.08545034642032).abs() < 1e-9);
        assert!((derived.vd_v - 1.0453125).abs() < 1e-9);
        assert!((derived.vd_pct - 0.261328125).abs() < 1e-9);
    }

    #[test]
    fn chain_drop_and_length_accumulate_on_the_head() {
        let mut head = circuit(0.36, 50.0, Some(3.0));
        head.group = Some("NLP".into());
        let mut tail = circuit(0.3, 20.0, None);
        tail.group = Some("NLP".into());
        let rows = evaluate(&[head, tail], &reference());
        let expected: f64 = rows.iter().map(|row| row.derived.vd_pct).sum();
        assert!((rows[0].derived.vd_last_pct.unwrap() - expected).abs() < 1e-12);
        assert!((rows[0].derived.chain_length_m.unwrap() - 70.0).abs() < 1e-9);
        assert!(rows[1].derived.vd_last_pct.is_none());
        assert!(
            (rows[1].derived.voltage_at_end_v
                - (230.0 - rows[0].derived.vd_v - rows[1].derived.vd_v))
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn oversized_load_fails_the_ampacity_check() {
        let mut circuit = circuit(20.0, 10.0, Some(3.0));
        circuit.protection_rating_a = 100.0;
        let rows = evaluate(&[circuit], &reference());
        assert!(!rows[0].derived.checks.ampacity);
        assert!(!rows[0].derived.checks.protection);
        assert!(!rows[0].derived.acceptable);
        assert!(rows[0]
            .derived
            .warnings
            .iter()
            .any(|warning| warning.code == "ampacity_exceeded"));
    }

    #[test]
    fn unknown_size_is_flagged() {
        let mut circuit = circuit(1.0, 10.0, Some(3.0));
        circuit.size_mm2 = 400.0;
        let rows = evaluate(&[circuit], &reference());
        assert!(!rows[0].derived.acceptable);
        assert_eq!(rows[0].derived.iz_a, 0.0);
        assert!(rows[0]
            .derived
            .warnings
            .iter()
            .any(|warning| warning.code == "cable_not_in_table"));
    }
}
