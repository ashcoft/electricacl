use crate::models::{EquipmentClass, MotorRef};

const MATCH_EPS: f64 = 1e-9;

/// Efficiency and power factor basis used for a consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// Interpolated from the motor reference table (Appendix-11).
    MotorTable,
    /// Fixed 0.95 efficiency / 0.90 power factor for feeder type loads.
    Feeder,
    /// Fixed unity values for "other" electrical users.
    Other,
    /// Motor rating missing from the reference table, feeder defaults used.
    Fallback,
}

impl Basis {
    pub fn key(self) -> &'static str {
        match self {
            Basis::MotorTable => "motor_table",
            Basis::Feeder => "feeder",
            Basis::Other => "other",
            Basis::Fallback => "fallback",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EfficiencyPf {
    pub efficiency: f64,
    pub power_factor: f64,
    pub basis: Basis,
}

#[derive(Debug, Clone, Default)]
pub struct MotorTable {
    rows: Vec<MotorRef>,
}

impl MotorTable {
    pub fn new(mut rows: Vec<MotorRef>) -> Self {
        rows.sort_by(|a, b| a.rated_kw.total_cmp(&b.rated_kw));
        Self { rows }
    }

    pub fn rows(&self) -> &[MotorRef] {
        &self.rows
    }

    pub fn find(&self, rated_kw: f64) -> Option<&MotorRef> {
        self.rows
            .iter()
            .find(|row| (row.rated_kw - rated_kw).abs() < MATCH_EPS)
    }

    /// Efficiency and power factor at the actual load factor.
    ///
    /// Feeders use 0.95 / 0.90, other users 1.00 / 1.00. Motors interpolate
    /// between the 100 %, 75 % and 50 % columns of the reference table; below
    /// 50 % the curve is extrapolated. The workbook selects that last branch
    /// on the absorbed power, not on the load factor.
    pub fn efficiency_pf(
        &self,
        class: EquipmentClass,
        rated_kw: f64,
        absorbed_kw: f64,
    ) -> EfficiencyPf {
        match class {
            EquipmentClass::Feeder => EfficiencyPf {
                efficiency: 0.95,
                power_factor: 0.90,
                basis: Basis::Feeder,
            },
            EquipmentClass::Other => EfficiencyPf {
                efficiency: 1.0,
                power_factor: 1.0,
                basis: Basis::Other,
            },
            EquipmentClass::Motor => {
                let Some(row) = self.find(rated_kw) else {
                    return EfficiencyPf {
                        efficiency: 0.95,
                        power_factor: 0.90,
                        basis: Basis::Fallback,
                    };
                };
                let (Some(eff_full), Some(eff_75), Some(eff_50)) =
                    (row.eff_full, row.eff_75, row.eff_50)
                else {
                    return EfficiencyPf {
                        efficiency: 0.95,
                        power_factor: 0.90,
                        basis: Basis::Fallback,
                    };
                };
                let (Some(pf_full), Some(pf_75), Some(pf_50)) = (row.pf_full, row.pf_75, row.pf_50)
                else {
                    return EfficiencyPf {
                        efficiency: 0.95,
                        power_factor: 0.90,
                        basis: Basis::Fallback,
                    };
                };
                if rated_kw <= 0.0 {
                    return EfficiencyPf {
                        efficiency: eff_full,
                        power_factor: pf_full,
                        basis: Basis::MotorTable,
                    };
                }
                let lf = absorbed_kw / rated_kw;
                let (efficiency, power_factor) = if lf > 0.75 {
                    let t = (lf - 0.75) / 0.25;
                    (
                        eff_75 + (eff_full - eff_75) * t,
                        pf_75 + (pf_full - pf_75) * t,
                    )
                } else if absorbed_kw > 0.5 {
                    let t = (lf - 0.5) / 0.25;
                    (eff_50 + (eff_75 - eff_50) * t, pf_50 + (pf_75 - pf_50) * t)
                } else {
                    let t = (0.5 - lf) / 0.25;
                    (eff_50 + (eff_75 - eff_50) * t, pf_50 - (pf_75 - pf_50) * t)
                };
                EfficiencyPf {
                    efficiency: efficiency.clamp(0.01, 1.0),
                    power_factor: power_factor.clamp(0.01, 1.0),
                    basis: Basis::MotorTable,
                }
            }
        }
    }

    pub fn starting_ratio(&self, rated_kw: f64) -> Option<f64> {
        self.find(rated_kw).and_then(|row| row.starting_ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> MotorTable {
        MotorTable::new(vec![
            MotorRef {
                rated_kw: 0.55,
                starting_ratio: Some(6.3),
                eff_full: Some(0.771),
                eff_75: Some(0.754),
                eff_50: Some(0.713),
                pf_full: Some(0.81),
                pf_75: Some(0.75),
                pf_50: Some(0.65),
            },
            MotorRef {
                rated_kw: 15.0,
                starting_ratio: Some(7.9),
                eff_full: Some(0.906),
                eff_75: Some(0.911),
                eff_50: Some(0.905),
                pf_full: Some(0.84),
                pf_75: Some(0.79),
                pf_50: Some(0.69),
            },
        ])
    }

    #[test]
    fn feeder_defaults() {
        let got = table().efficiency_pf(EquipmentClass::Feeder, 150.0, 150.0);
        assert_eq!(got.efficiency, 0.95);
        assert_eq!(got.power_factor, 0.90);
        assert_eq!(got.basis, Basis::Feeder);
    }

    #[test]
    fn interpolates_between_50_and_75_percent() {
        // 11 kW absorbed on a 15 kW motor -> 73.3 % load factor.
        let got = table().efficiency_pf(EquipmentClass::Motor, 15.0, 11.0);
        assert!((got.efficiency - 0.9106).abs() < 1e-9, "{got:?}");
        assert!(
            (got.power_factor - 0.7833333333333333).abs() < 1e-9,
            "{got:?}"
        );
    }

    #[test]
    fn extrapolates_below_50_percent() {
        // 0.3 kW absorbed on a 0.55 kW motor -> 54.5 % load factor but the
        // workbook branches on the absorbed power being below 0.5 kW.
        let got = table().efficiency_pf(EquipmentClass::Motor, 0.55, 0.3);
        assert!(
            (got.efficiency - 0.7055454545454545).abs() < 1e-9,
            "{got:?}"
        );
        assert!(
            (got.power_factor - 0.6681818181818182).abs() < 1e-9,
            "{got:?}"
        );
    }

    #[test]
    fn unknown_motor_rating_falls_back() {
        let got = table().efficiency_pf(EquipmentClass::Motor, 3.0, 3.0);
        assert_eq!(got.basis, Basis::Fallback);
        assert_eq!(got.efficiency, 0.95);
    }
}
