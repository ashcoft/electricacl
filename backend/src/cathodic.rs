//! Cathodic protection calculation (calculation 03).
//!
//! Reproduces the internal-tank SACP sheets of
//! `Basic Cal SACP for Internal Tank - PT Pertamina MOR VII Sulawesi.xls`
//! following DNV RP B401 (2005) and GS EP COR 201:
//!
//! ```text
//! Cdu  = (Cd + (T - 25) x 0.001) x CBf          (DNV temperature correction)
//! Cap  = epsilon - 27 x (T - 20)                 Ah/kg at operating temperature
//! TWA  = shell area + head/bottom area           from the structure geometry
//! SA   = TWA + round(TWA x discrepancy, 2)       workbook rounds the allowance
//! Ip   = SA x Cdu
//! M    = Ip x Y x 8760 / Cap / U x safety        kg of anode material
//! N    = ceil(M / anode unit weight)
//! ```
//!
//! Piping, pipelines, external surfaces and ICCP reuse the same current demand
//! and mass balance; ICCP adds a rectifier rating from the groundbed loop.

use serde::{Deserialize, Serialize};

pub const TEMP_COEFFICIENT: f64 = 0.001;
pub const CAPACITY_SLOPE: f64 = 27.0;
pub const HOURS_PER_YEAR: f64 = 8760.0;

fn round_half_up(value: f64, digits: i32) -> f64 {
    let scale = 10f64.powi(digits);
    (value * scale + 0.5).floor() / scale
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Structure {
    #[default]
    Tank,
    Vessel,
    Piping,
    Pipeline,
}

impl Structure {
    pub const ALL: [Structure; 4] = [
        Structure::Tank,
        Structure::Vessel,
        Structure::Piping,
        Structure::Pipeline,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Structure::Tank => "tank",
            Structure::Vessel => "vessel",
            Structure::Piping => "piping",
            Structure::Pipeline => "pipeline",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Structure::Tank => "Tank",
            Structure::Vessel => "Vessel",
            Structure::Piping => "Piping",
            Structure::Pipeline => "Pipeline",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    #[default]
    Vertical,
    Horizontal,
    Rectangular,
}

impl Orientation {
    pub fn key(self) -> &'static str {
        match self {
            Orientation::Vertical => "vertical",
            Orientation::Horizontal => "horizontal",
            Orientation::Rectangular => "rectangular",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Orientation::Vertical => "Vertical",
            Orientation::Horizontal => "Horizontal",
            Orientation::Rectangular => "Rectangular",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    #[default]
    Internal,
    External,
}

impl Surface {
    pub fn key(self) -> &'static str {
        match self {
            Surface::Internal => "internal",
            Surface::External => "external",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Surface::Internal => "Internal",
            Surface::External => "External",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    #[default]
    Sacp,
    Iccp,
}

impl Method {
    pub fn key(self) -> &'static str {
        match self {
            Method::Sacp => "sacp",
            Method::Iccp => "iccp",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Method::Sacp => "Sacrificial anode (SACP)",
            Method::Iccp => "Impressed current (ICCP)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RectifierInputs {
    pub anode_output_a: f64,
    pub groundbed_resistance_ohm: f64,
    pub cable_resistance_ohm: f64,
    pub back_emf_v: f64,
    pub margin_pct: f64,
}

impl Default for RectifierInputs {
    fn default() -> Self {
        Self {
            anode_output_a: 5.0,
            groundbed_resistance_ohm: 1.0,
            cable_resistance_ohm: 0.2,
            back_emf_v: 2.0,
            margin_pct: 20.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CathodicItem {
    pub id: String,
    pub tag: Option<String>,
    pub description: Option<String>,
    pub source_sheet: Option<String>,
    pub structure: Structure,
    pub orientation: Orientation,
    pub surface: Surface,
    pub method: Method,
    pub design_life_years: f64,
    pub coating_breakdown: f64,
    pub apply_coating_breakdown: bool,
    pub current_density_a_m2: f64,
    pub temp_coefficient: f64,
    pub design_temp_c: f64,
    pub operating_temp_c: f64,
    pub capacity_temp_c: Option<f64>,
    pub capacity_ah_kg: f64,
    pub utilization: f64,
    pub safety_factor: f64,
    pub diameter_m: f64,
    pub length_m: f64,
    pub width_m: f64,
    pub liquid_height_m: f64,
    pub discrepancy_pct: f64,
    pub current_density_override: Option<f64>,
    pub anode_type: Option<String>,
    pub anode_weight_kg: f64,
    pub anode_output_a: Option<f64>,
    pub anodes_selected: u32,
    pub rectifier: Option<RectifierInputs>,
    pub remark: Option<String>,
}

impl Default for CathodicItem {
    fn default() -> Self {
        Self {
            id: String::new(),
            tag: None,
            description: None,
            source_sheet: None,
            structure: Structure::Tank,
            orientation: Orientation::Vertical,
            surface: Surface::Internal,
            method: Method::Sacp,
            design_life_years: 20.0,
            coating_breakdown: 0.1,
            apply_coating_breakdown: true,
            current_density_a_m2: 0.07,
            temp_coefficient: TEMP_COEFFICIENT,
            design_temp_c: 25.0,
            operating_temp_c: 25.0,
            capacity_temp_c: None,
            capacity_ah_kg: 2500.0,
            utilization: 0.85,
            safety_factor: 1.0,
            diameter_m: 1.0,
            length_m: 1.0,
            width_m: 1.0,
            liquid_height_m: 1.0,
            discrepancy_pct: 10.0,
            current_density_override: None,
            anode_type: Some("ICS 40 AWB".to_string()),
            anode_weight_kg: 4.0,
            anode_output_a: Some(0.43),
            anodes_selected: 1,
            rectifier: None,
            remark: None,
        }
    }
}

impl CathodicItem {
    pub fn new_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    pub fn sanitize(&mut self) {
        if self.id.trim().is_empty() {
            self.id = Self::new_id();
        }
        self.design_life_years = self.design_life_years.clamp(0.0, 100.0);
        self.coating_breakdown = self.coating_breakdown.clamp(0.0, 2.0);
        self.current_density_a_m2 = self.current_density_a_m2.max(0.0);
        self.temp_coefficient = self.temp_coefficient.clamp(0.0, 1.0);
        self.design_temp_c = self.design_temp_c.clamp(-10.0, 200.0);
        self.operating_temp_c = self.operating_temp_c.clamp(-10.0, 200.0);
        if let Some(value) = self.capacity_temp_c {
            self.capacity_temp_c = Some(value.clamp(-10.0, 200.0));
        }
        self.capacity_ah_kg = self.capacity_ah_kg.max(0.0);
        self.utilization = self.utilization.clamp(0.0, 1.0);
        self.safety_factor = self.safety_factor.clamp(0.1, 5.0);
        for value in [
            &mut self.diameter_m,
            &mut self.length_m,
            &mut self.width_m,
            &mut self.liquid_height_m,
        ] {
            *value = value.max(0.0);
        }
        self.discrepancy_pct = self.discrepancy_pct.clamp(0.0, 100.0);
        self.anode_weight_kg = self.anode_weight_kg.max(0.0);
        if let Some(value) = self.anode_output_a {
            self.anode_output_a = Some(value.max(0.0));
        }
        if let Some(rectifier) = self.rectifier.as_mut() {
            rectifier.anode_output_a = rectifier.anode_output_a.max(0.0);
            rectifier.groundbed_resistance_ohm = rectifier.groundbed_resistance_ohm.max(0.0);
            rectifier.cable_resistance_ohm = rectifier.cable_resistance_ohm.max(0.0);
            rectifier.back_emf_v = rectifier.back_emf_v.max(0.0);
            rectifier.margin_pct = rectifier.margin_pct.clamp(0.0, 200.0);
        }
        for field in [
            &mut self.tag,
            &mut self.description,
            &mut self.source_sheet,
            &mut self.anode_type,
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

    pub fn capacity_temp(&self) -> f64 {
        self.capacity_temp_c.unwrap_or(self.operating_temp_c)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CathodicWarning {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CathodicChecks {
    pub capacity: bool,
    pub current_density: bool,
    pub anode_count: bool,
    pub design_life: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CathodicDerived {
    pub current_density_a_m2: f64,
    pub capacity_at_temp_ah_kg: f64,
    pub area_shell_m2: f64,
    pub area_ends_m2: f64,
    pub wetted_area_m2: f64,
    pub discrepancy_area_m2: f64,
    pub exposed_area_m2: f64,
    pub protection_current_a: f64,
    pub anode_mass_kg: Option<f64>,
    pub anodes_by_mass: Option<u32>,
    pub anodes_by_current: Option<u32>,
    pub anodes_required: u32,
    pub anodes_selected: u32,
    pub life_years: Option<f64>,
    pub rectifier_current_a: Option<f64>,
    pub rectifier_voltage_v: Option<f64>,
    pub rectifier_kva: Option<f64>,
    pub checks: CathodicChecks,
    pub acceptable: bool,
    pub warnings: Vec<CathodicWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CathodicRow {
    #[serde(flatten)]
    pub item: CathodicItem,
    pub derived: CathodicDerived,
}

#[derive(Debug, Clone, Serialize)]
pub struct CathodicStats {
    pub items: usize,
    pub sacp: usize,
    pub iccp: usize,
    pub acceptable: usize,
    pub failed: usize,
    pub warnings: usize,
    pub total_protection_current_a: f64,
    pub total_anode_mass_kg: f64,
    pub total_anodes: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CathodicStateView {
    pub project: crate::models::ProjectMeta,
    pub items: Vec<CathodicRow>,
    pub stats: CathodicStats,
}

/// Shell, head/bottom and total wetted area for the item geometry.
fn geometry(item: &CathodicItem) -> (f64, f64, f64) {
    let diameter = item.diameter_m;
    let height = if item.surface == Surface::External {
        item.length_m
    } else {
        item.liquid_height_m
    };
    match item.structure {
        Structure::Piping | Structure::Pipeline => {
            let shell = std::f64::consts::PI * diameter * item.length_m;
            (shell, 0.0, shell)
        }
        Structure::Tank | Structure::Vessel => match item.orientation {
            Orientation::Rectangular => {
                let shell = 2.0 * (item.length_m + item.width_m) * height;
                let ends = 2.0 * item.length_m * item.width_m;
                (shell, ends, shell + ends)
            }
            Orientation::Vertical => {
                let shell = std::f64::consts::PI * diameter * height;
                let ends = if item.structure == Structure::Vessel {
                    1.5 * std::f64::consts::PI * diameter * diameter / 4.0
                } else {
                    std::f64::consts::PI * diameter * diameter / 4.0
                };
                (shell, ends, shell + ends)
            }
            Orientation::Horizontal => {
                let fill = if item.surface == Surface::External {
                    1.0
                } else if diameter > 0.0 {
                    (item.liquid_height_m / diameter).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let shell = std::f64::consts::PI * diameter * item.length_m * fill;
                let ends = 3.0 * std::f64::consts::PI * diameter * diameter / 4.0 * fill;
                (shell, ends, shell + ends)
            }
        },
    }
}

pub fn derive(item: &CathodicItem) -> CathodicDerived {
    let mut warnings = Vec::new();

    let capacity_temp = item.capacity_temp();
    let capacity_at_temp = item.capacity_ah_kg - CAPACITY_SLOPE * (capacity_temp - 20.0);
    let computed_density = (item.current_density_a_m2
        + (item.design_temp_c - 25.0) * item.temp_coefficient)
        * if item.apply_coating_breakdown {
            item.coating_breakdown
        } else {
            1.0
        };
    let current_density = item.current_density_override.unwrap_or(computed_density);

    let (area_shell, area_ends, wetted) = geometry(item);
    let discrepancy_area = round_half_up(wetted * item.discrepancy_pct / 100.0, 2);
    let exposed = wetted + discrepancy_area;
    let protection_current = exposed * current_density;

    if capacity_at_temp <= 0.0 {
        warnings.push(CathodicWarning {
            code: "capacity_not_positive",
            message: format!(
                "Anode capacity at {capacity_temp:.1} °C is {capacity_at_temp:.1} Ah/kg - the mass balance cannot be closed"
            ),
        });
    }
    if current_density <= 0.0 {
        warnings.push(CathodicWarning {
            code: "no_current_density",
            message: "Design current density is zero - check the coating breakdown and temperature"
                .to_string(),
        });
    }
    if exposed <= 0.0 {
        warnings.push(CathodicWarning {
            code: "no_area",
            message: "Exposed area is zero - check the geometry and liquid height".to_string(),
        });
    }

    let mut anode_mass = None;
    let mut anodes_by_mass = None;
    let mut anodes_by_current = None;
    let mut life_years = None;
    let mut rectifier_current = None;
    let mut rectifier_voltage = None;
    let mut rectifier_kva = None;

    let anode_count = match item.method {
        Method::Sacp => {
            let mass = if capacity_at_temp > 0.0 && item.utilization > 0.0 {
                let value = protection_current * item.design_life_years * HOURS_PER_YEAR
                    / capacity_at_temp
                    / item.utilization
                    * item.safety_factor;
                anode_mass = Some(value);
                Some(value)
            } else {
                None
            };
            let by_mass = mass.map(|value| {
                if item.anode_weight_kg > 0.0 {
                    (value / item.anode_weight_kg - 1e-9).ceil().max(0.0) as u32
                } else {
                    0
                }
            });
            let by_current = item
                .anode_output_a
                .filter(|value| *value > 0.0)
                .map(|output| (protection_current / output - 1e-9).ceil().max(0.0) as u32);
            anodes_by_mass = by_mass;
            anodes_by_current = by_current;
            if item.anode_weight_kg <= 0.0 {
                warnings.push(CathodicWarning {
                    code: "no_anode_weight",
                    message: "Anode unit weight is zero - the number of anodes cannot be sized"
                        .to_string(),
                });
            }
            if let Some(required) = by_mass {
                if item.anodes_selected < required {
                    warnings.push(CathodicWarning {
                        code: "anodes_short",
                        message: format!(
                            "{} anodes selected, {required} required by the mass balance",
                            item.anodes_selected
                        ),
                    });
                }
            }
            if mass.is_some()
                && item.anodes_selected > 0
                && item.anode_weight_kg > 0.0
                && protection_current > 0.0
                && capacity_at_temp > 0.0
            {
                let actual = item.anodes_selected as f64
                    * item.anode_weight_kg
                    * capacity_at_temp
                    * item.utilization
                    / (protection_current * HOURS_PER_YEAR * item.safety_factor);
                life_years = Some(actual);
                if actual < item.design_life_years {
                    warnings.push(CathodicWarning {
                        code: "life_short",
                        message: format!(
                            "Selected anodes give {actual:.1} years, below the {:.1}-year design life",
                            item.design_life_years
                        ),
                    });
                }
            }
            by_mass.into_iter().chain(by_current).max().unwrap_or(0)
        }
        Method::Iccp => {
            let rectifier = item.rectifier.clone().unwrap_or_default();
            let current = protection_current;
            let voltage = (current
                * (rectifier.groundbed_resistance_ohm + rectifier.cable_resistance_ohm)
                + rectifier.back_emf_v)
                * (1.0 + rectifier.margin_pct / 100.0);
            rectifier_current = Some(current);
            rectifier_voltage = Some(voltage);
            rectifier_kva = Some(voltage * current / 1000.0);
            let output = rectifier.anode_output_a;
            let required = if output > 0.0 {
                (current / output - 1e-9).ceil().max(0.0) as u32
            } else {
                warnings.push(CathodicWarning {
                    code: "no_groundbed_output",
                    message: "Groundbed anode output is zero - the anode count cannot be sized"
                        .to_string(),
                });
                0
            };
            if item.anodes_selected < required {
                warnings.push(CathodicWarning {
                    code: "anodes_short",
                    message: format!(
                        "{} groundbed anodes selected, {required} required for {current:.1} A",
                        item.anodes_selected
                    ),
                });
            }
            required
        }
    };

    let checks = CathodicChecks {
        capacity: capacity_at_temp > 0.0,
        current_density: current_density > 0.0,
        anode_count: item.anodes_selected >= anode_count,
        design_life: life_years
            .map(|value| value >= item.design_life_years)
            .unwrap_or(item.method == Method::Iccp),
    };
    let acceptable = checks.capacity
        && checks.current_density
        && checks.anode_count
        && checks.design_life
        && exposed > 0.0;

    CathodicDerived {
        current_density_a_m2: current_density,
        capacity_at_temp_ah_kg: capacity_at_temp,
        area_shell_m2: area_shell,
        area_ends_m2: area_ends,
        wetted_area_m2: wetted,
        discrepancy_area_m2: discrepancy_area,
        exposed_area_m2: exposed,
        protection_current_a: protection_current,
        anode_mass_kg: anode_mass,
        anodes_by_mass,
        anodes_by_current,
        anodes_required: anode_count,
        anodes_selected: item.anodes_selected,
        life_years,
        rectifier_current_a: rectifier_current,
        rectifier_voltage_v: rectifier_voltage,
        rectifier_kva,
        checks,
        acceptable,
        warnings,
    }
}

pub fn build_state(
    project: &crate::models::ProjectMeta,
    items: &[CathodicItem],
) -> CathodicStateView {
    let mut rows = Vec::with_capacity(items.len());
    let mut stats = CathodicStats {
        items: items.len(),
        sacp: 0,
        iccp: 0,
        acceptable: 0,
        failed: 0,
        warnings: 0,
        total_protection_current_a: 0.0,
        total_anode_mass_kg: 0.0,
        total_anodes: 0,
    };
    for item in items {
        let derived = derive(item);
        match item.method {
            Method::Sacp => stats.sacp += 1,
            Method::Iccp => stats.iccp += 1,
        }
        if derived.acceptable {
            stats.acceptable += 1;
        } else {
            stats.failed += 1;
        }
        stats.warnings += derived.warnings.len();
        stats.total_protection_current_a += derived.protection_current_a;
        stats.total_anode_mass_kg += derived.anode_mass_kg.unwrap_or(0.0);
        stats.total_anodes += derived.anodes_selected;
        rows.push(CathodicRow {
            item: item.clone(),
            derived,
        });
    }
    CathodicStateView {
        project: project.clone(),
        items: rows,
        stats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_tank_matches_hand_calculation() {
        let item = CathodicItem {
            structure: Structure::Tank,
            orientation: Orientation::Vertical,
            diameter_m: 16.8,
            liquid_height_m: 17.2,
            design_life_years: 20.0,
            coating_breakdown: 0.05,
            current_density_a_m2: 0.07,
            design_temp_c: 27.0,
            operating_temp_c: 27.0,
            capacity_ah_kg: 780.0,
            utilization: 0.85,
            discrepancy_pct: 10.0,
            anode_weight_kg: 40.0,
            anodes_selected: 40,
            ..CathodicItem::default()
        };
        let derived = derive(&item);
        assert!((derived.current_density_a_m2 - 0.0036).abs() < 1e-12);
        assert!((derived.capacity_at_temp_ah_kg - 591.0).abs() < 1e-9);
        assert!((derived.wetted_area_m2 - 1129.4653908186024).abs() < 1e-6);
        assert!((derived.exposed_area_m2 - 1242.4153908186024).abs() < 1e-6);
        assert!((derived.protection_current_a - 4.472695406946969).abs() < 1e-9);
        assert_eq!(derived.anodes_required, 39);
        assert!(derived.acceptable);
    }

    #[test]
    fn iccp_sizes_the_rectifier() {
        let item = CathodicItem {
            method: Method::Iccp,
            structure: Structure::Pipeline,
            diameter_m: 0.5,
            length_m: 1000.0,
            current_density_a_m2: 0.05,
            apply_coating_breakdown: false,
            design_temp_c: 25.0,
            operating_temp_c: 25.0,
            discrepancy_pct: 0.0,
            anodes_selected: 3,
            rectifier: Some(RectifierInputs {
                anode_output_a: 10.0,
                groundbed_resistance_ohm: 1.0,
                cable_resistance_ohm: 0.2,
                back_emf_v: 2.0,
                margin_pct: 20.0,
            }),
            ..CathodicItem::default()
        };
        let derived = derive(&item);
        assert!((derived.exposed_area_m2 - std::f64::consts::PI * 0.5 * 1000.0).abs() < 1e-6);
        assert!((derived.protection_current_a - std::f64::consts::PI * 25.0).abs() < 1e-6);
        assert_eq!(derived.anodes_required, 8);
        let voltage = (derived.protection_current_a * 1.2 + 2.0) * 1.2;
        assert!((derived.rectifier_voltage_v.unwrap() - voltage).abs() < 1e-9);
    }
}
