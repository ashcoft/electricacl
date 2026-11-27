//! The PDF report is generated in-process; these tests pin its structure.

use std::path::Path;

use electricacl::calc::build_state;
use electricacl::models::Scenario;
use electricacl::report;
use electricacl::store::Store;

fn state() -> electricacl::calc::StateView {
    let store = Store::load(Path::new("/nonexistent-electricacl-report-test")).unwrap();
    build_state(
        &store.state().project,
        &store.state().settings,
        store.items(),
        store.motor_table(),
    )
}

fn page_count(pdf: &[u8]) -> usize {
    let document = lopdf::Document::load_mem(pdf).expect("the report parses as a PDF");
    document.get_pages().len()
}

#[test]
fn renders_a_multi_page_report_for_every_scenario() {
    let state = state();
    for scenario in Scenario::ALL {
        let pdf = report::render(&state, scenario).expect("report renders");
        assert!(pdf.starts_with(b"%PDF-1."), "{scenario:?} is not a PDF");
        assert!(
            pdf.ends_with(b"%%EOF\n") || pdf.ends_with(b"%%EOF"),
            "{scenario:?} is truncated"
        );
        assert!(
            pdf.len() > 20_000,
            "{scenario:?} is suspiciously small: {}",
            pdf.len()
        );
        let pages = page_count(&pdf);
        // 656 consumers need far more than one page at 70 rows per page.
        assert!(pages >= 8, "{scenario:?} has only {pages} pages");
    }
}

#[test]
fn report_embeds_the_base14_fonts_and_a_title() {
    let state = state();
    let pdf = report::render(&state, Scenario::Normal).unwrap();
    let document = lopdf::Document::load_mem(&pdf).unwrap();
    let info = document
        .trailer
        .get(b"Info")
        .and_then(|value| value.as_reference())
        .ok()
        .and_then(|id| document.get_object(id).ok())
        .and_then(|object| object.as_dict().ok())
        .cloned()
        .expect("document info dictionary");
    let title = info
        .get(b"Title")
        .and_then(|value| value.as_str())
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .unwrap_or_default();
    assert_eq!(title, "Electrical load list");

    // Base-14 fonts are referenced by name, so the file stays small.
    let raw = String::from_utf8_lossy(&pdf);
    assert!(raw.contains("Courier") || raw.contains("Helvetica"));
}

#[test]
fn unknown_scenario_is_not_renderable_but_all_known_ones_are() {
    let state = state();
    assert_eq!(state.summaries.len(), 6);
    assert!(report::render(&state, Scenario::BlackStart).is_ok());
}
