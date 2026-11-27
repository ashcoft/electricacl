use std::collections::BTreeMap;

use serde::Serialize;

use super::motor_table::{Basis, MotorTable};
use crate::models::{CalcSettings, Duty, LoadItem, MotorRef, ProjectMeta, Scenario};

const SQRT3_APPROX: f64 = 1.732;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Totals {
    pub kw: f64,
    pub kvar: f64,
    pub kva: f64,
}

impl Totals {
    pub fn new(kw: f64, kvar: f64) -> Self {
        Self {
            kw,
            kvar,
            kva: (kw * kw + kvar * kvar).sqrt(),
        }
    }

    fn add(&mut self, kw: f64, kvar: f64) {
        self.kw += kw;
        self.kvar += kvar;
        self.kva = (self.kw * self.kw + self.kvar * self.kvar).sqrt();
    }

    pub fn scaled(&self, factor: f64) -> Self {
        Totals::new(self.kw * factor, self.kvar * factor)
    }

    pub fn power_factor(&self) -> f64 {
        if self.kva > 0.0 {
            self.kw / self.kva
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Derived {
    pub load_factor: f64,
    pub efficiency: f64,
    pub power_factor: f64,
    pub basis: &'static str,
    /// Full load current in amperes (rated power basis, as per the workbook).
    pub flc_a: f64,
    /// Consumed active power in kW (`absorbed / efficiency`).
    pub consumed_kw: f64,
    pub consumed_kvar: f64,
    pub consumed_kva: f64,
    pub starting_current_a: Option<f64>,
    pub starting_kva: Option<f64>,
    pub warnings: Vec<Warning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemView {
    #[serde(flatten)]
    pub item: LoadItem,
    pub derived: Derived,
}

pub fn derive(item: &LoadItem, table: &MotorTable) -> Derived {
    let rated = item.rated_kw;
    let absorbed = item.absorbed_kw;
    let load_factor = if rated > 0.0 { absorbed / rated } else { 0.0 };
    let curve = table.efficiency_pf(item.equipment_class, rated, absorbed);

    let flc_a = if item.voltage_kv > 0.0 && curve.efficiency > 0.0 && curve.power_factor > 0.0 {
        rated / (SQRT3_APPROX * item.voltage_kv * curve.efficiency * curve.power_factor)
    } else {
        0.0
    };

    let consumed_kw = if curve.efficiency > 0.0 {
        absorbed / curve.efficiency
    } else {
        0.0
    };
    let consumed_kvar = consumed_kw
        * (1.0 - curve.power_factor * curve.power_factor)
            .max(0.0)
            .sqrt()
        / curve.power_factor;

    let starting_ratio = if item.equipment_class == crate::models::EquipmentClass::Motor {
        table.starting_ratio(rated)
    } else {
        None
    };
    let starting_current_a = starting_ratio.map(|ratio| flc_a * ratio);
    let starting_kva = starting_current_a.map(|current| 3f64.sqrt() * item.voltage_kv * current);

    let mut warnings = Vec::new();
    if item.duty.is_none() {
        warnings.push(Warning {
            code: "no_duty",
            message: "No service duty assigned - excluded from every load column".into(),
        });
    }
    if item.scenarios.is_empty() {
        warnings.push(Warning {
            code: "no_scenario",
            message: "No operating scenario selected - excluded from all scenarios".into(),
        });
    }
    if curve.basis == Basis::Fallback {
        warnings.push(Warning {
            code: "motor_not_in_table",
            message: format!(
                "Motor rating {rated} kW is not in the reference table - feeder defaults (0.95 / 0.90) applied"
            ),
        });
    }
    if load_factor > 1.02 {
        warnings.push(Warning {
            code: "load_factor_high",
            message: format!(
                "Absorbed power exceeds nameplate rating (load factor {:.0} %)",
                load_factor * 100.0
            ),
        });
    }
    if absorbed <= 0.0 && rated <= 0.0 {
        warnings.push(Warning {
            code: "zero_power",
            message: "No absorbed or rated power entered".into(),
        });
    }

    Derived {
        load_factor,
        efficiency: curve.efficiency,
        power_factor: curve.power_factor,
        basis: curve.basis.key(),
        flc_a,
        consumed_kw,
        consumed_kvar,
        consumed_kva: (consumed_kw * consumed_kw + consumed_kvar * consumed_kvar).sqrt(),
        starting_current_a,
        starting_kva,
        warnings,
    }
}

/// The workbook takes the maximum kW and the maximum kVAr independently, so
/// the "largest load" totals do the same.
fn update_largest(target: &mut Totals, kw: f64, kvar: f64) {
    *target = Totals::new(target.kw.max(kw), target.kvar.max(kvar));
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LargestLoads {
    pub continuous: Totals,
    pub intermittent: Totals,
    pub standby: Totals,
    pub spare: Totals,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BreakdownRow {
    pub key: String,
    pub items: usize,
    pub continuous_kw: f64,
    pub intermittent_kw: f64,
    pub standby_kw: f64,
    pub spare_kw: f64,
    pub connected_kw: f64,
    pub connected_kvar: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratorOption {
    pub label: String,
    pub units: u32,
    pub running: u32,
    pub unit_kw: f64,
    pub unit_kva: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioSummary {
    pub scenario: Scenario,
    pub label: String,
    pub continuous: Totals,
    pub intermittent: Totals,
    pub standby: Totals,
    pub spare: Totals,
    pub largest: LargestLoads,
    /// Continuous + diversity on intermittent, the load the plant runs at.
    pub max_normal_running: Totals,
    /// Maximum running load + the governing stand-by load.
    pub peak: Totals,
    /// Peak load with the spare contingency applied - the generation demand.
    pub demand: Totals,
    pub contingency_pct: f64,
    pub peak_power_factor: f64,
    pub active_items: usize,
    pub by_voltage: Vec<BreakdownRow>,
    pub by_area: Vec<BreakdownRow>,
    pub generator_sizing: Vec<GeneratorOption>,
}

fn breakdown_key_voltage(voltage: f64) -> String {
    if (voltage.fract()).abs() < 1e-9 {
        format!("{voltage:.0} kV")
    } else {
        let text = format!("{voltage:.3}");
        format!("{} kV", text.trim_end_matches('0').trim_end_matches('.'))
    }
}

struct BreakdownAccum {
    items: usize,
    continuous_kw: f64,
    intermittent_kw: f64,
    standby_kw: f64,
    spare_kw: f64,
    connected_kw: f64,
    connected_kvar: f64,
}

impl BreakdownAccum {
    fn new() -> Self {
        Self {
            items: 0,
            continuous_kw: 0.0,
            intermittent_kw: 0.0,
            standby_kw: 0.0,
            spare_kw: 0.0,
            connected_kw: 0.0,
            connected_kvar: 0.0,
        }
    }

    fn push(&mut self, duty: Option<Duty>, derived: &Derived) {
        self.items += 1;
        self.connected_kw += derived.consumed_kw;
        self.connected_kvar += derived.consumed_kvar;
        match duty {
            Some(Duty::Continuous) => self.continuous_kw += derived.consumed_kw,
            Some(Duty::Intermittent) => self.intermittent_kw += derived.consumed_kw,
            Some(Duty::Standby) => self.standby_kw += derived.consumed_kw,
            Some(Duty::Spare) => self.spare_kw += derived.consumed_kw,
            None => {}
        }
    }

    fn into_row(self, key: String) -> BreakdownRow {
        BreakdownRow {
            key,
            items: self.items,
            continuous_kw: self.continuous_kw,
            intermittent_kw: self.intermittent_kw,
            standby_kw: self.standby_kw,
            spare_kw: self.spare_kw,
            connected_kw: self.connected_kw,
            connected_kvar: self.connected_kvar,
        }
    }
}

fn generator_options(demand: &Totals) -> Vec<GeneratorOption> {
    let configs = [
        ("2 x 50 % + 1 stand-by", 3u32, 2u32),
        ("3 x 33.3 % + 1 stand-by", 4, 3),
        ("1 x 100 %", 1, 1),
    ];
    configs
        .iter()
        .map(|(label, units, running)| GeneratorOption {
            label: (*label).to_string(),
            units: *units,
            running: *running,
            unit_kw: demand.kw / *running as f64,
            unit_kva: demand.kva / *running as f64,
        })
        .collect()
}

pub fn summarize(
    items: &[LoadItem],
    table: &MotorTable,
    settings: &CalcSettings,
) -> Vec<ScenarioSummary> {
    let derived: Vec<(&LoadItem, Derived)> = items
        .iter()
        .map(|item| (item, derive(item, table)))
        .collect();

    Scenario::ALL
        .iter()
        .map(|&scenario| {
            let mut continuous = Totals::default();
            let mut intermittent = Totals::default();
            let mut standby = Totals::default();
            let mut spare = Totals::default();
            let mut largest = LargestLoads::default();
            let mut by_voltage: BTreeMap<String, BreakdownAccum> = BTreeMap::new();
            let mut by_area: BTreeMap<String, BreakdownAccum> = BTreeMap::new();
            let mut active_items = 0usize;

            for (item, derived) in &derived {
                if !item.scenarios.contains(&scenario) {
                    continue;
                }
                active_items += 1;
                let totals = match item.duty {
                    Some(Duty::Continuous) => Some(&mut continuous),
                    Some(Duty::Intermittent) => Some(&mut intermittent),
                    Some(Duty::Standby) => Some(&mut standby),
                    Some(Duty::Spare) => Some(&mut spare),
                    None => None,
                };
                if let Some(totals) = totals {
                    totals.add(derived.consumed_kw, derived.consumed_kvar);
                }
                match item.duty {
                    Some(Duty::Continuous) => update_largest(
                        &mut largest.continuous,
                        derived.consumed_kw,
                        derived.consumed_kvar,
                    ),
                    Some(Duty::Intermittent) => update_largest(
                        &mut largest.intermittent,
                        derived.consumed_kw,
                        derived.consumed_kvar,
                    ),
                    Some(Duty::Standby) => update_largest(
                        &mut largest.standby,
                        derived.consumed_kw,
                        derived.consumed_kvar,
                    ),
                    Some(Duty::Spare) => update_largest(
                        &mut largest.spare,
                        derived.consumed_kw,
                        derived.consumed_kvar,
                    ),
                    None => {}
                }

                by_voltage
                    .entry(breakdown_key_voltage(item.voltage_kv))
                    .or_insert_with(BreakdownAccum::new)
                    .push(item.duty, derived);
                by_area
                    .entry(item.area.clone().unwrap_or_else(|| "Unassigned".into()))
                    .or_insert_with(BreakdownAccum::new)
                    .push(item.duty, derived);
            }

            let int_factor = settings.intermittent_factor();
            let std_factor = settings.standby_factor();

            let mut max_normal_running = Totals::new(
                continuous.kw + (intermittent.kw * int_factor).max(largest.intermittent.kw),
                continuous.kvar + (intermittent.kvar * int_factor).max(largest.intermittent.kvar),
            );
            let peak = Totals::new(
                continuous.kw
                    + intermittent.kw * int_factor
                    + (standby.kw * std_factor).max(largest.standby.kw),
                continuous.kvar
                    + intermittent.kvar * int_factor
                    + (standby.kvar * std_factor).max(largest.standby.kvar),
            );
            let demand = peak.scaled(1.0 + settings.contingency());
            // Guard against a negative kVA from rounding noise on tiny loads.
            max_normal_running.kva =
                (max_normal_running.kw.powi(2) + max_normal_running.kvar.powi(2)).sqrt();

            let mut voltage_rows: Vec<BreakdownRow> = by_voltage
                .into_iter()
                .map(|(key, accum)| accum.into_row(key))
                .collect();
            voltage_rows.sort_by(|a, b| b.connected_kw.total_cmp(&a.connected_kw));
            let mut area_rows: Vec<BreakdownRow> = by_area
                .into_iter()
                .map(|(key, accum)| accum.into_row(key))
                .collect();
            area_rows.sort_by(|a, b| b.connected_kw.total_cmp(&a.connected_kw));

            ScenarioSummary {
                scenario,
                label: scenario.label().to_string(),
                continuous,
                intermittent,
                standby,
                spare,
                largest,
                max_normal_running,
                peak,
                demand,
                contingency_pct: settings.contingency_pct,
                peak_power_factor: peak.power_factor(),
                active_items,
                by_voltage: voltage_rows,
                by_area: area_rows,
                generator_sizing: generator_options(&demand),
            }
        })
        .collect()
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Options {
    pub areas: Vec<String>,
    pub systems: Vec<String>,
    pub voltages: Vec<f64>,
    pub statuses: Vec<String>,
    pub feeder_types: Vec<String>,
    pub load_types: Vec<String>,
    pub switchgears: Vec<String>,
}

fn push_unique<T: PartialEq>(list: &mut Vec<T>, value: T) {
    if !list.contains(&value) {
        list.push(value);
    }
}

pub fn options(items: &[LoadItem]) -> Options {
    let mut options = Options::default();
    for item in items {
        if let Some(area) = &item.area {
            push_unique(&mut options.areas, area.clone());
        }
        if let Some(system) = &item.system {
            push_unique(&mut options.systems, system.clone());
        }
        if let Some(status) = &item.status {
            push_unique(&mut options.statuses, status.clone());
        }
        if let Some(feeder_type) = &item.feeder_type {
            push_unique(&mut options.feeder_types, feeder_type.clone());
        }
        if let Some(load_type) = &item.load_type {
            push_unique(&mut options.load_types, load_type.clone());
        }
        if let Some(switchgear) = &item.switchgear {
            push_unique(&mut options.switchgears, switchgear.clone());
        }
        push_unique(&mut options.voltages, item.voltage_kv);
    }
    options.voltages.sort_by(|a, b| b.total_cmp(a));
    options.areas.sort();
    options.systems.sort();
    options.statuses.sort();
    options.feeder_types.sort();
    options.load_types.sort();
    options.switchgears.sort();
    options
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub items: usize,
    pub items_with_power: usize,
    pub warnings: usize,
    pub motor_table_rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateView {
    pub project: ProjectMeta,
    pub settings: CalcSettings,
    pub items: Vec<ItemView>,
    pub summaries: Vec<ScenarioSummary>,
    pub motor_table: Vec<MotorRef>,
    pub options: Options,
    pub stats: Stats,
}

pub fn build_state(
    project: &ProjectMeta,
    settings: &CalcSettings,
    items: &[LoadItem],
    table: &MotorTable,
) -> StateView {
    let views: Vec<ItemView> = items
        .iter()
        .map(|item| ItemView {
            item: item.clone(),
            derived: derive(item, table),
        })
        .collect();
    let stats = Stats {
        items: views.len(),
        items_with_power: views
            .iter()
            .filter(|view| view.item.absorbed_kw > 0.0 || view.item.rated_kw > 0.0)
            .count(),
        warnings: views.iter().map(|view| view.derived.warnings.len()).sum(),
        motor_table_rows: table.rows().len(),
    };
    StateView {
        project: project.clone(),
        settings: settings.clone(),
        items: views,
        summaries: summarize(items, table, settings),
        motor_table: table.rows().to_vec(),
        options: options(items),
        stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EquipmentClass;

    fn item(tag: &str, duty: Duty, absorbed: f64, rated: f64) -> LoadItem {
        LoadItem {
            id: tag.to_string(),
            tag: Some(tag.to_string()),
            equipment_class: EquipmentClass::Feeder,
            duty: Some(duty),
            scenarios: vec![Scenario::Normal],
            voltage_kv: 0.69,
            absorbed_kw: absorbed,
            rated_kw: rated,
            ..Default::default()
        }
    }

    #[test]
    fn consumed_power_uses_efficiency() {
        let derived = derive(
            &item("A", Duty::Continuous, 150.0, 150.0),
            &MotorTable::default(),
        );
        assert!((derived.consumed_kw - 157.89473684210526).abs() < 1e-9);
        assert!((derived.consumed_kvar - 76.47191129018724).abs() < 1e-9);
        assert!((derived.flc_a - 146.80071333402623).abs() < 1e-9);
    }

    #[test]
    fn peak_load_adds_diversified_intermittent_and_largest_standby() {
        let items = vec![
            item("A", Duty::Continuous, 100.0, 100.0),
            item("B", Duty::Intermittent, 10.0, 10.0),
            item("C", Duty::Standby, 50.0, 50.0),
        ];
        let summary = summarize(&items, &MotorTable::default(), &CalcSettings::default());
        let normal = summary
            .iter()
            .find(|s| s.scenario == Scenario::Normal)
            .unwrap();
        let cont = 100.0 / 0.95;
        let int = 10.0 / 0.95;
        let std = 50.0 / 0.95;
        // A single intermittent load governs: max(30 % of total, largest).
        assert!((normal.max_normal_running.kw - (cont + int)).abs() < 1e-9);
        assert!((normal.peak.kw - (cont + 0.3 * int + std)).abs() < 1e-9);
        assert!((normal.demand.kw - normal.peak.kw * 1.2).abs() < 1e-9);
    }

    #[test]
    fn items_without_duty_are_ignored_in_totals() {
        let mut unassigned = item("U", Duty::Continuous, 500.0, 500.0);
        unassigned.duty = None;
        let summary = summarize(
            &[item("A", Duty::Continuous, 100.0, 100.0), unassigned],
            &MotorTable::default(),
            &CalcSettings::default(),
        );
        let normal = summary
            .iter()
            .find(|s| s.scenario == Scenario::Normal)
            .unwrap();
        assert!((normal.continuous.kw - 100.0 / 0.95).abs() < 1e-9);
        assert_eq!(normal.active_items, 2);
    }
}
