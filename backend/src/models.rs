use serde::{Deserialize, Serialize};

/// Operating scenarios evaluated by the load list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    Normal,
    Offloading,
    Maintenance,
    Essential,
    Emergency,
    BlackStart,
}

impl Scenario {
    pub const ALL: [Scenario; 6] = [
        Scenario::Normal,
        Scenario::Offloading,
        Scenario::Maintenance,
        Scenario::Essential,
        Scenario::Emergency,
        Scenario::BlackStart,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Scenario::Normal => "normal",
            Scenario::Offloading => "offloading",
            Scenario::Maintenance => "maintenance",
            Scenario::Essential => "essential",
            Scenario::Emergency => "emergency",
            Scenario::BlackStart => "black_start",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Scenario::Normal => "Normal / Holding",
            Scenario::Offloading => "Offloading",
            Scenario::Maintenance => "Maintenance",
            Scenario::Essential => "Essential",
            Scenario::Emergency => "Emergency",
            Scenario::BlackStart => "Black Start",
        }
    }
}

/// Service duty of a consumer. The duty decides which load column the consumer
/// is counted in (continuous / intermittent / standby / spare).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Duty {
    Continuous,
    Intermittent,
    Standby,
    Spare,
}

impl Duty {
    pub const ALL: [Duty; 4] = [
        Duty::Continuous,
        Duty::Intermittent,
        Duty::Standby,
        Duty::Spare,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Duty::Continuous => "continuous",
            Duty::Intermittent => "intermittent",
            Duty::Standby => "standby",
            Duty::Spare => "spare",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Duty::Continuous => "Continuous",
            Duty::Intermittent => "Intermittent",
            Duty::Standby => "Stand-by",
            Duty::Spare => "Spare",
        }
    }

    /// Compact code used in dense reports: C / I / SB / SP.
    pub fn short(self) -> &'static str {
        match self {
            Duty::Continuous => "C",
            Duty::Intermittent => "I",
            Duty::Standby => "SB",
            Duty::Spare => "SP",
        }
    }
}

/// Motor / feeder / other electrical user. Decides the efficiency and power
/// factor basis of the consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentClass {
    Motor,
    Feeder,
    #[default]
    Other,
}

impl EquipmentClass {
    pub fn key(self) -> &'static str {
        match self {
            EquipmentClass::Motor => "motor",
            EquipmentClass::Feeder => "feeder",
            EquipmentClass::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            EquipmentClass::Motor => "Motor",
            EquipmentClass::Feeder => "Feeder",
            EquipmentClass::Other => "Other",
        }
    }
}

fn default_voltage() -> f64 {
    0.69
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoadItem {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub line_no: Option<i64>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub area: Option<String>,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub equipment_class: EquipmentClass,
    #[serde(default)]
    pub feeder_type: Option<String>,
    #[serde(default)]
    pub duty: Option<Duty>,
    #[serde(default)]
    pub load_type: Option<String>,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
    #[serde(default = "default_voltage")]
    pub voltage_kv: f64,
    #[serde(default)]
    pub phase: Option<String>,
    #[serde(default)]
    pub substation: Option<String>,
    #[serde(default)]
    pub switchgear: Option<String>,
    #[serde(default)]
    pub bus: Option<String>,
    #[serde(default)]
    pub drive_type: Option<String>,
    #[serde(default)]
    pub breaker_a: Option<f64>,
    /// Absorbed (shaft / process) power in kW.
    #[serde(default)]
    pub absorbed_kw: f64,
    /// Nameplate rating in kW.
    #[serde(default)]
    pub rated_kw: f64,
    #[serde(default)]
    pub remark: Option<String>,
}

impl LoadItem {
    pub fn new_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Clamp values that would otherwise break the calculation.
    pub fn sanitize(&mut self) {
        if self.id.trim().is_empty() {
            self.id = Self::new_id();
        }
        if self.voltage_kv <= 0.0 {
            self.voltage_kv = default_voltage();
        }
        self.absorbed_kw = self.absorbed_kw.max(0.0);
        self.rated_kw = self.rated_kw.max(0.0);
        if self.rated_kw == 0.0 && self.absorbed_kw > 0.0 {
            self.rated_kw = self.absorbed_kw;
        }
        let mut seen = Vec::new();
        self.scenarios.retain(|s| {
            if seen.contains(s) {
                false
            } else {
                seen.push(*s);
                true
            }
        });
        for field in [
            &mut self.revision,
            &mut self.status,
            &mut self.unit,
            &mut self.area,
            &mut self.system,
            &mut self.tag,
            &mut self.description,
            &mut self.feeder_type,
            &mut self.load_type,
            &mut self.phase,
            &mut self.substation,
            &mut self.switchgear,
            &mut self.bus,
            &mut self.drive_type,
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

/// Motor efficiency / power factor reference row (workbook Appendix-11).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotorRef {
    pub rated_kw: f64,
    #[serde(default)]
    pub starting_ratio: Option<f64>,
    #[serde(default)]
    pub eff_full: Option<f64>,
    #[serde(default)]
    pub eff_75: Option<f64>,
    #[serde(default)]
    pub eff_50: Option<f64>,
    #[serde(default)]
    pub pf_full: Option<f64>,
    #[serde(default)]
    pub pf_75: Option<f64>,
    #[serde(default)]
    pub pf_50: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalcSettings {
    /// Spare contingency applied to the peak load (%).
    #[serde(default = "default_contingency")]
    pub contingency_pct: f64,
    /// Diversity applied to the intermittent total (%).
    #[serde(default = "default_intermittent")]
    pub intermittent_factor_pct: f64,
    /// Diversity applied to the stand-by total (%).
    #[serde(default = "default_standby")]
    pub standby_factor_pct: f64,
}

fn default_contingency() -> f64 {
    20.0
}
fn default_intermittent() -> f64 {
    30.0
}
fn default_standby() -> f64 {
    10.0
}

impl Default for CalcSettings {
    fn default() -> Self {
        Self {
            contingency_pct: default_contingency(),
            intermittent_factor_pct: default_intermittent(),
            standby_factor_pct: default_standby(),
        }
    }
}

impl CalcSettings {
    pub fn sanitize(&mut self) {
        self.contingency_pct = self.contingency_pct.clamp(0.0, 200.0);
        self.intermittent_factor_pct = self.intermittent_factor_pct.clamp(0.0, 100.0);
        self.standby_factor_pct = self.standby_factor_pct.clamp(0.0, 100.0);
    }

    pub fn contingency(&self) -> f64 {
        self.contingency_pct / 100.0
    }
    pub fn intermittent_factor(&self) -> f64 {
        self.intermittent_factor_pct / 100.0
    }
    pub fn standby_factor(&self) -> f64 {
        self.standby_factor_pct / 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    #[serde(default = "default_project")]
    pub name: String,
    #[serde(default = "default_document")]
    pub document: String,
    #[serde(default)]
    pub revision: String,
}

fn default_project() -> String {
    "Electrical Load List".to_string()
}
fn default_document() -> String {
    "Load List Calculation".to_string()
}

impl Default for ProjectMeta {
    fn default() -> Self {
        Self {
            name: default_project(),
            document: default_document(),
            revision: "01".to_string(),
        }
    }
}
