//! Verify the cathodic engine against the cached values of the source workbook.

use serde::Deserialize;

use electricacl::cathodic::{build_state, derive, CathodicItem};

#[derive(Deserialize)]
struct Seed {
    project: electricacl::models::ProjectMeta,
    items: Vec<CathodicItem>,
    verification: Vec<Verification>,
}

#[derive(Deserialize)]
struct Verification {
    wetted_area_m2: f64,
    exposed_area_m2: f64,
    current_density_a_m2: f64,
    capacity_at_temp_ah_kg: f64,
    protection_current_a: f64,
    anode_mass_kg: f64,
    anodes_required: u32,
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-6 * expected.abs().max(1.0)
}

#[test]
fn reproduces_every_extracted_workbook_sheet() {
    let seed: Seed = serde_json::from_str(include_str!("../data/cathodic_seed.json"))
        .expect("cathodic seed is valid JSON");
    assert_eq!(seed.items.len(), 23);
    assert_eq!(seed.items.len(), seed.verification.len());

    for (item, expected) in seed.items.iter().zip(seed.verification.iter()) {
        let derived = derive(item);
        let label = item.source_sheet.as_deref().unwrap_or("?");
        assert!(
            close(derived.wetted_area_m2, expected.wetted_area_m2),
            "{label}: wetted area {} vs {}",
            derived.wetted_area_m2,
            expected.wetted_area_m2
        );
        assert!(
            close(derived.exposed_area_m2, expected.exposed_area_m2),
            "{label}: exposed area {} vs {}",
            derived.exposed_area_m2,
            expected.exposed_area_m2
        );
        assert!(
            close(derived.current_density_a_m2, expected.current_density_a_m2),
            "{label}: current density {} vs {}",
            derived.current_density_a_m2,
            expected.current_density_a_m2
        );
        assert!(
            close(
                derived.capacity_at_temp_ah_kg,
                expected.capacity_at_temp_ah_kg
            ),
            "{label}: capacity {} vs {}",
            derived.capacity_at_temp_ah_kg,
            expected.capacity_at_temp_ah_kg
        );
        assert!(
            close(derived.protection_current_a, expected.protection_current_a),
            "{label}: protection current {} vs {}",
            derived.protection_current_a,
            expected.protection_current_a
        );
        let mass = derived.anode_mass_kg.expect("SACP mass");
        assert!(
            close(mass, expected.anode_mass_kg),
            "{label}: anode mass {mass} vs {}",
            expected.anode_mass_kg
        );
        assert_eq!(
            derived.anodes_required, expected.anodes_required,
            "{label}: required anodes"
        );
        assert!(
            derived.anodes_selected >= derived.anodes_required,
            "{label}: selected fewer anodes than required"
        );
        assert!(derived.acceptable, "{label}: item should be acceptable");
    }

    let state = build_state(&seed.project, &seed.items);
    assert_eq!(state.stats.items, 23);
    assert_eq!(state.stats.sacp, 23);
    assert_eq!(state.stats.iccp, 0);
    assert_eq!(state.stats.acceptable, 23);
    assert_eq!(state.stats.failed, 0);
}
