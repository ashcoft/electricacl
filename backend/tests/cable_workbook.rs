//! Reproduces `0000-EF-D-41335_0 Power Cable Sizing Calc.XLSX`.
//!
//! Expected values are the cached results of the workbook's own formulas.

use std::path::Path;

use electricacl::cable::{build_state, CableStateView};
use electricacl::cable_store::CableStore;

const TOL: f64 = 1e-6;

fn state() -> CableStateView {
    let store = CableStore::load(Path::new("/nonexistent-electricacl-cable-test")).unwrap();
    store.state()
}

fn row<'a>(state: &'a CableStateView, tag: &str) -> &'a electricacl::cable::CableRow {
    state
        .circuits
        .iter()
        .find(|row| row.circuit.tag.as_deref() == Some(tag))
        .unwrap_or_else(|| panic!("circuit {tag} is missing"))
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOL,
        "{what}: expected {expected}, got {actual}"
    );
}

#[test]
fn seed_has_ten_circuits_and_three_cable_sizes() {
    let state = state();
    assert_eq!(state.circuits.len(), 10);
    assert_eq!(state.reference.len(), 3);
    assert_eq!(state.stats.acceptable, 10);
    assert_eq!(state.stats.failed, 0);
}

#[test]
fn transformer_to_panel_circuit_matches_the_workbook() {
    let state = state();
    let circuit = row(&state, "3000-NLP-2001-P01");
    let derived = &circuit.derived;

    close(derived.iflc_a, 36.08545034642032, "IFLC");
    close(derived.ampacity_a, 137.0, "It");
    close(derived.derating, 0.8, "CO");
    close(derived.iz_a, 109.6, "IZ");
    close(derived.runs_required, 0.3292468097301124, "runs required");
    close(derived.vd_v, 1.0453125, "voltage drop");
    close(derived.vd_pct, 0.261328125, "% voltage drop");
    close(derived.vd_last_pct.unwrap(), 0.261328125, "% VD last");
    close(derived.chain_length_m.unwrap(), 25.0, "chain length");
    // The workbook's maximum-length cell for this row points at the next row's
    // allowable drop and uses the single-phase factor; the engine uses the
    // circuit's own 1 % and 1.732, which is 95.7 m instead of 248.5 m.
    close(
        derived.max_length_m.unwrap(),
        95.66517189835575,
        "max length",
    );
    assert!(derived.acceptable);
}

#[test]
fn lighting_chain_matches_the_workbook() {
    let state = state();
    let head = row(&state, "3000-LTF-0001-P01");
    close(head.derived.iflc_a, 1.7391304347826086, "IFLC");
    close(head.derived.iz_a, 17.16, "IZ");
    close(head.derived.vd_v, 1.4866201602716835, "voltage drop");
    close(head.derived.vd_pct, 0.646356591422471, "% voltage drop");
    close(
        head.derived.vd_last_pct.unwrap(),
        1.29271318284494,
        "% VD last",
    );
    close(head.derived.chain_length_m.unwrap(), 150.0, "chain length");
    close(
        head.derived.max_length_m.unwrap(),
        232.07003996027498,
        "max length",
    );
    assert!(head.derived.checks.ampacity);
    assert!(head.derived.checks.protection);
    assert_eq!(head.derived.checks.voltage_drop, Some(true));
    assert_eq!(head.derived.checks.max_length, Some(true));

    // Downstream fixtures do not repeat the cumulative drop.
    let tail = row(&state, "3000-LTF-0006-P01");
    close(tail.derived.vd_pct, 0.04309043942816475, "% voltage drop");
    assert!(tail.derived.vd_last_pct.is_none());
    close(
        tail.derived.voltage_at_end_v,
        227.02675967945663,
        "voltage at end",
    );
}

#[test]
fn socket_chain_matches_the_workbook() {
    let state = state();
    let head = row(&state, "3000-JB-0002-P01");
    close(head.derived.iflc_a, 28.0, "IFLC");
    close(head.derived.iz_a, 40.92, "IZ");
    close(
        head.derived.runs_required,
        0.6842619745845552,
        "runs required",
    );
    close(head.derived.vd_v, 2.6096, "voltage drop");
    close(head.derived.vd_pct, 1.134608695652174, "% voltage drop");
    close(
        head.derived.vd_last_pct.unwrap(),
        2.1071304347826114,
        "% VD last",
    );
    close(
        head.derived.max_length_m.unwrap(),
        52.88166768853464,
        "max length",
    );
    assert!(head.derived.acceptable);

    let second = row(&state, "3000-GPO-0001-P01");
    close(second.derived.vd_pct, 0.4862608695652174, "% voltage drop");
}

#[test]
fn every_circuit_passes_the_workbook_checks() {
    let state = state();
    for row in &state.circuits {
        assert!(
            row.derived.checks.ampacity,
            "{} fails ampacity",
            row.circuit.tag.as_deref().unwrap_or("?")
        );
        assert!(
            row.derived.checks.protection,
            "{} fails protection coordination",
            row.circuit.tag.as_deref().unwrap_or("?")
        );
    }
    assert_eq!(state.stats.groups, 2);
    close(
        state.stats.total_length_m,
        25.0 + 50.0 + 20.0 * 5.0 + 20.0 + 15.0 + 15.0,
        "total length",
    );
}

#[test]
fn state_is_rebuilt_from_the_seed() {
    let store = CableStore::load(Path::new("/nonexistent-electricacl-cable-test")).unwrap();
    let view = build_state(
        &view_project(&store),
        store
            .state()
            .circuits
            .iter()
            .map(|row| row.circuit.clone())
            .collect::<Vec<_>>()
            .as_slice(),
        store.reference(),
    );
    assert_eq!(view.circuits.len(), 10);
    assert_eq!(view.stats.acceptable, 10);
}

fn view_project(store: &CableStore) -> electricacl::models::ProjectMeta {
    store.state().project
}
