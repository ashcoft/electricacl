//! Reproduces the numbers of `LOAD LIST.xlsx` from the embedded seed data.
//!
//! The expected values are read from the workbook itself (Overall Load List
//! totals and Summary sheet), so any change in the calculation chain breaks
//! these tests.

use electricacl::calc::{summarize, MotorTable, ScenarioSummary};
use electricacl::models::{CalcSettings, LoadItem, Scenario};
use electricacl::store::Store;

const TOL: f64 = 1e-6;

fn seed() -> (Vec<LoadItem>, MotorTable) {
    let store = Store::load(std::path::Path::new("/nonexistent-electricacl-test")).unwrap();
    (store.items().to_vec(), store.motor_table().clone())
}

fn summary(summaries: &[ScenarioSummary], scenario: Scenario) -> &ScenarioSummary {
    summaries
        .iter()
        .find(|entry| entry.scenario == scenario)
        .expect("scenario summary present")
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOL,
        "{what}: expected {expected}, got {actual}"
    );
}

#[test]
fn normal_holding_scenario_matches_workbook() {
    let (items, table) = seed();
    let summaries = summarize(&items, &table, &CalcSettings::default());
    let normal = summary(&summaries, Scenario::Normal);

    assert_close(normal.continuous.kw, 23884.462013822522, "continuous kW");
    assert_close(
        normal.continuous.kvar,
        12249.711911076838,
        "continuous kVAr",
    );
    assert_close(normal.continuous.kva, 26842.558886851843, "continuous kVA");
    assert_close(
        normal.intermittent.kw,
        2086.1037492127393,
        "intermittent kW",
    );
    assert_close(normal.standby.kw, 3348.85831668436, "standby kW");
    assert_close(normal.spare.kw, 16262.530950872902, "spare kW");
    assert_close(
        normal.max_normal_running.kw,
        24510.293138586345,
        "max running kW",
    );
    assert_close(normal.peak.kw, 25950.62235669334, "peak kW");
    assert_close(normal.peak.kvar, 13283.061006613985, "peak kVAr");
    assert_close(normal.peak.kva, 29152.607265991515, "peak kVA");
    assert_close(normal.demand.kw, 31140.746828032006, "demand kW");
    assert_close(normal.demand.kva, 34983.128719189815, "demand kVA");
    assert_close(
        normal.largest.continuous.kw,
        4393.03471292029,
        "largest continuous",
    );
    assert_close(
        normal.largest.standby.kw,
        1440.329218106996,
        "largest standby",
    );
}

#[test]
fn maintenance_essential_and_emergency_match_workbook() {
    let (items, table) = seed();
    let summaries = summarize(&items, &table, &CalcSettings::default());

    let maintenance = summary(&summaries, Scenario::Maintenance);
    assert_close(
        maintenance.continuous.kw,
        5712.609852187783,
        "maintenance continuous",
    );
    assert_close(
        maintenance.max_normal_running.kw,
        6317.613400074892,
        "maintenance max running",
    );
    assert_close(maintenance.peak.kw, 6528.139715864365, "maintenance peak");
    assert_close(
        maintenance.demand.kw,
        7833.7676590372375,
        "maintenance demand",
    );

    let essential = summary(&summaries, Scenario::Essential);
    assert_close(
        essential.continuous.kw,
        8859.485766208509,
        "essential continuous",
    );
    assert_close(
        essential.max_normal_running.kw,
        9467.962998306144,
        "essential max running",
    );
    assert_close(essential.demand.kw, 11614.18717691474, "essential demand");

    let emergency = summary(&summaries, Scenario::Emergency);
    assert_close(emergency.demand.kw, 1929.787556240612, "emergency demand");
}

#[test]
fn feeder_item_matches_workbook_row() {
    let (items, table) = seed();
    let heater = items
        .iter()
        .find(|item| item.tag.as_deref() == Some("F-165-E-0001"))
        .expect("start-up heater present");
    let derived = electricacl::calc::derive(heater, &table);
    assert_close(derived.consumed_kw, 1578.9473684210527, "consumed kW");
    assert_close(derived.consumed_kvar, 764.7191129018724, "consumed kVAr");
    assert_close(derived.flc_a, 1468.0071333402623, "full load current");
}

#[test]
fn motor_efficiency_and_power_factor_follow_reference_table() {
    let (items, table) = seed();
    let motor = items
        .iter()
        .find(|item| {
            item.equipment_class == electricacl::models::EquipmentClass::Motor
                && item.rated_kw == 15.0
                && item.absorbed_kw == 11.0
        })
        .expect("15 kW motor at 11 kW absorbed");
    let derived = electricacl::calc::derive(motor, &table);
    assert_close(derived.efficiency, 0.9106, "efficiency");
    assert_close(derived.power_factor, 0.7833333333333333, "power factor");
    assert_close(derived.flc_a, 17.59624608987188, "full load current");
}

#[test]
fn contingency_and_diversity_settings_are_applied() {
    let (items, table) = seed();
    let settings = CalcSettings {
        contingency_pct: 0.0,
        intermittent_factor_pct: 0.0,
        standby_factor_pct: 0.0,
    };
    let summaries = summarize(&items, &table, &settings);
    let normal = summary(&summaries, Scenario::Normal);
    assert_close(normal.demand.kw, normal.peak.kw, "zero contingency");
    // With zero diversity the largest single intermittent load still governs.
    assert_close(
        normal.max_normal_running.kw,
        normal.continuous.kw + normal.largest.intermittent.kw,
        "zero diversity",
    );
    assert_close(
        normal.peak.kw,
        normal.continuous.kw + normal.largest.standby.kw,
        "peak uses the largest stand-by load",
    );
}
