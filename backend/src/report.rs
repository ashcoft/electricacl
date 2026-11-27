//! PDF report rendering.
//!
//! A deterministic report built with the PDF base-14 fonts: a summary page, a
//! scenario comparison and the full load list with a repeating header. No
//! browser or headless Chrome involved.
//!
//! Tables are laid out with a monospaced font and character-based column
//! widths, which keeps the columns aligned without a layout engine.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use printpdf::{
    BuiltinFont, Color, IndirectFontRef, Line, Mm, PdfDocument, PdfDocumentReference,
    PdfLayerReference, Point, Rgb,
};

use crate::calc::{ScenarioSummary, StateView};
use crate::models::{Duty, Scenario};

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN_L: f32 = 14.0;
const MARGIN_R: f32 = 14.0;
const MARGIN_TOP: f32 = 16.0;
const MARGIN_BOTTOM: f32 = 16.0;
const BODY: f32 = 8.0;
const LIST: f32 = 6.5;
const MONO_ADVANCE: f32 = 0.6; // Courier advance width in em
const MM_PER_PT: f32 = 0.352_778;

fn char_mm(size: f32) -> f32 {
    size * MONO_ADVANCE * MM_PER_PT
}

/// The PDF base-14 fonts use WinAnsi, so anything outside ASCII is folded to a
/// readable equivalent instead of producing garbage glyphs.
fn ascii(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        let replacement = match ch {
            '·' | '•' => "-",
            '×' => "x",
            '–' | '—' => "-",
            'φ' => "phi",
            '°' => " deg",
            '≤' => "<=",
            '≥' => ">=",
            '±' => "+/-",
            '\u{a0}' => " ",
            _ => {
                if ch.is_ascii() {
                    out.push(ch);
                } else {
                    out.push('?');
                }
                continue;
            }
        };
        out.push_str(replacement);
    }
    out
}

fn cell(text: &str, chars: usize, numeric: bool) -> String {
    let text = ascii(text);
    let mut clipped: String = if text.chars().count() > chars {
        let mut value: String = text.chars().take(chars.saturating_sub(1)).collect();
        value.push('~');
        value
    } else {
        text
    };
    let padding = chars.saturating_sub(clipped.chars().count());
    if numeric {
        clipped.insert_str(0, &" ".repeat(padding));
    } else {
        clipped.push_str(&" ".repeat(padding));
    }
    clipped
}

fn num(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

/// `YYYY-MM-DD` without pulling in a date crate.
fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let days = seconds.div_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

struct Column {
    title: &'static str,
    chars: usize,
    numeric: bool,
}

fn row(columns: &[Column], values: &[String]) -> String {
    columns
        .iter()
        .zip(values)
        .map(|(column, value)| cell(value, column.chars, column.numeric))
        .collect::<Vec<_>>()
        .join(" ")
}

fn header_row(columns: &[Column]) -> String {
    let titles: Vec<String> = columns.iter().map(|c| c.title.to_string()).collect();
    row(columns, &titles)
}

fn width_mm(columns: &[Column], size: f32) -> f32 {
    let chars: usize = columns.iter().map(|c| c.chars).sum::<usize>() + columns.len() - 1;
    chars as f32 * char_mm(size)
}

struct Report {
    doc: PdfDocumentReference,
    mono: IndirectFontRef,
    sans: IndirectFontRef,
    bold: IndirectFontRef,
    layer: PdfLayerReference,
    y: f32,
    page_no: u32,
    footer: String,
}

impl Report {
    fn new(title: &str, footer: String) -> Result<Self> {
        let (doc, page, layer) =
            PdfDocument::new(title, Mm(PAGE_W), Mm(PAGE_H), "Load list report");
        let mono = doc.add_builtin_font(BuiltinFont::Courier)?;
        let sans = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
        let layer = doc.get_page(page).get_layer(layer);
        Ok(Self {
            doc,
            mono,
            sans,
            bold,
            layer,
            y: PAGE_H - MARGIN_TOP,
            page_no: 1,
            footer,
        })
    }

    fn text(&self, x: f32, y: f32, size: f32, text: &str, font: &IndirectFontRef) {
        if text.is_empty() {
            return;
        }
        self.layer.use_text(ascii(text), size, Mm(x), Mm(y), font);
    }

    fn rule(&self, y: f32, shade: f32) {
        self.rule_at(y, PAGE_W - MARGIN_R - MARGIN_L, shade);
    }

    fn rule_at(&self, y: f32, width_mm: f32, shade: f32) {
        self.layer
            .set_outline_color(Color::Rgb(Rgb::new(shade, shade, shade, None)));
        self.layer.set_outline_thickness(0.2);
        self.layer.add_line(Line {
            points: vec![
                (Point::new(Mm(MARGIN_L), Mm(y)), false),
                (Point::new(Mm(MARGIN_L + width_mm), Mm(y)), false),
            ],
            is_closed: false,
        });
    }

    fn draw_footer(&mut self) {
        self.rule(MARGIN_BOTTOM + 3.0, 0.75);
        let footer = self.footer.clone();
        self.text(MARGIN_L, MARGIN_BOTTOM, 7.0, &footer, &self.sans);
        let label = format!("Page {}", self.page_no);
        self.text(
            PAGE_W - MARGIN_R - label.chars().count() as f32 * 3.1,
            MARGIN_BOTTOM,
            7.0,
            &label,
            &self.sans,
        );
    }

    fn new_page(&mut self) {
        self.draw_footer();
        let (page, layer) = self
            .doc
            .add_page(Mm(PAGE_W), Mm(PAGE_H), "Load list report");
        self.layer = self.doc.get_page(page).get_layer(layer);
        self.y = PAGE_H - MARGIN_TOP;
        self.page_no += 1;
    }

    fn ensure(&mut self, needed: f32) {
        if self.y - needed < MARGIN_BOTTOM + 9.0 {
            self.new_page();
        }
    }

    fn line(&mut self, x: f32, size: f32, text: &str, font: &IndirectFontRef) {
        self.ensure(4.2);
        self.text(x, self.y, size, text, font);
        self.y -= 4.2;
    }

    fn heading(&mut self, title: &str) {
        self.ensure(18.0);
        self.y -= 4.0;
        self.text(MARGIN_L, self.y, 10.5, title, &self.bold);
        self.y -= 4.5;
        self.rule(self.y + 1.5, 0.45);
        self.y -= 1.5;
    }

    fn finish(mut self) -> Result<Vec<u8>> {
        self.draw_footer();
        self.doc.save_to_bytes().context("writing the PDF document")
    }
}

fn scenario_of(state: &StateView, scenario: Scenario) -> Option<&ScenarioSummary> {
    state
        .summaries
        .iter()
        .find(|entry| entry.scenario == scenario)
}

pub fn render(state: &StateView, scenario: Scenario) -> Result<Vec<u8>> {
    let generated = today();
    let footer = format!(
        "{} - {} - revision {} - generated {}",
        state.project.name, state.project.document, state.project.revision, generated
    );
    let mut report = Report::new("Electrical load list", footer)?;
    let bold = report.bold.clone();
    let mono = report.mono.clone();
    let sans = report.sans.clone();

    // ---- title block ----------------------------------------------------
    report.text(MARGIN_L, report.y, 17.0, &state.project.name, &bold);
    report.y -= 7.5;
    report.text(
        MARGIN_L,
        report.y,
        10.0,
        &format!(
            "{} - revision {}",
            state.project.document, state.project.revision
        ),
        &sans,
    );
    report.y -= 5.5;
    report.text(
        MARGIN_L,
        report.y,
        8.5,
        &format!(
            "Scenario: {}   -   {} consumers   -   generated {}",
            scenario.label(),
            state.items.len(),
            generated
        ),
        &sans,
    );
    report.y -= 9.0;

    let Some(summary) = scenario_of(state, scenario) else {
        anyhow::bail!("scenario summary is missing");
    };

    // ---- key figures ----------------------------------------------------
    report.heading("Key figures");
    let figures = [
        ("Total load demand (kVA)", num(summary.demand.kva, 1), "kVA"),
        ("Total load demand (kW)", num(summary.demand.kw, 1), "kW"),
        ("Peak load (kW)", num(summary.peak.kw, 1), "kW"),
        (
            "Maximum normal running (kW)",
            num(summary.max_normal_running.kw, 1),
            "kW",
        ),
        ("Continuous load (kW)", num(summary.continuous.kw, 1), "kW"),
        ("Peak power factor", num(summary.peak_power_factor, 3), ""),
    ];
    for (index, (label, value, unit)) in figures.iter().enumerate() {
        let x = MARGIN_L + (index % 2) as f32 * 94.0;
        let y = report.y - (index / 2) as f32 * 5.4;
        report.text(x, y, 8.5, label, &sans);
        report.text(x + 64.0, y, 8.5, &format!("{value} {unit}"), &mono);
    }
    report.y -= 3.0 * 5.4 + 3.0;

    // ---- load build-up --------------------------------------------------
    report.heading("Load build-up");
    let totals_columns = [
        Column {
            title: "Load category",
            chars: 38,
            numeric: false,
        },
        Column {
            title: "kW",
            chars: 12,
            numeric: true,
        },
        Column {
            title: "kVAr",
            chars: 12,
            numeric: true,
        },
        Column {
            title: "kVA",
            chars: 12,
            numeric: true,
        },
    ];
    report.line(MARGIN_L, BODY, &header_row(&totals_columns), &bold);
    report.rule_at(report.y + 2.4, width_mm(&totals_columns, BODY), 0.7);
    for (label, totals) in [
        ("Continuous", &summary.continuous),
        ("Intermittent", &summary.intermittent),
        ("Stand-by", &summary.standby),
        ("Spare (not included in peak)", &summary.spare),
        ("Maximum normal running", &summary.max_normal_running),
        ("Peak load", &summary.peak),
        ("Total demand (with contingency)", &summary.demand),
    ] {
        report.line(
            MARGIN_L,
            BODY,
            &row(
                &totals_columns,
                &[
                    label.to_string(),
                    num(totals.kw, 1),
                    num(totals.kvar, 1),
                    num(totals.kva, 1),
                ],
            ),
            &mono,
        );
    }
    report.y -= 1.5;
    report.line(
        MARGIN_L,
        BODY,
        &format!(
            "Contingency {} %  -  intermittent diversity {} %  -  stand-by diversity {} %",
            num(state.settings.contingency_pct, 0),
            num(state.settings.intermittent_factor_pct, 0),
            num(state.settings.standby_factor_pct, 0)
        ),
        &sans,
    );

    // ---- generator sizing ----------------------------------------------
    report.heading("Generator sizing (indicative)");
    let gen_columns = [
        Column {
            title: "Configuration",
            chars: 30,
            numeric: false,
        },
        Column {
            title: "Running",
            chars: 10,
            numeric: true,
        },
        Column {
            title: "Unit kW",
            chars: 12,
            numeric: true,
        },
        Column {
            title: "Unit kVA",
            chars: 12,
            numeric: true,
        },
    ];
    report.line(MARGIN_L, BODY, &header_row(&gen_columns), &bold);
    report.rule_at(report.y + 2.4, width_mm(&gen_columns, BODY), 0.7);
    for option in &summary.generator_sizing {
        report.line(
            MARGIN_L,
            BODY,
            &row(
                &gen_columns,
                &[
                    option.label.clone(),
                    format!("{}/{}", option.running, option.units),
                    num(option.unit_kw, 0),
                    num(option.unit_kva, 0),
                ],
            ),
            &mono,
        );
    }
    report.y -= 1.5;
    report.line(
        MARGIN_L,
        BODY,
        "Before site ambient derating, engine margins and motor start contribution.",
        &sans,
    );

    // ---- all scenarios --------------------------------------------------
    report.heading("Scenario comparison");
    let scenario_columns = [
        Column {
            title: "Scenario",
            chars: 22,
            numeric: false,
        },
        Column {
            title: "Continuous",
            chars: 11,
            numeric: true,
        },
        Column {
            title: "Intermittent",
            chars: 12,
            numeric: true,
        },
        Column {
            title: "Stand-by",
            chars: 10,
            numeric: true,
        },
        Column {
            title: "Spare",
            chars: 10,
            numeric: true,
        },
        Column {
            title: "Peak",
            chars: 10,
            numeric: true,
        },
        Column {
            title: "Demand",
            chars: 10,
            numeric: true,
        },
    ];
    report.line(MARGIN_L, BODY, &header_row(&scenario_columns), &bold);
    report.rule_at(report.y + 2.4, width_mm(&scenario_columns, BODY), 0.7);
    for entry in &state.summaries {
        report.line(
            MARGIN_L,
            BODY,
            &row(
                &scenario_columns,
                &[
                    entry.label.clone(),
                    num(entry.continuous.kw, 0),
                    num(entry.intermittent.kw, 0),
                    num(entry.standby.kw, 0),
                    num(entry.spare.kw, 0),
                    num(entry.peak.kw, 0),
                    num(entry.demand.kw, 0),
                ],
            ),
            &mono,
        );
    }
    report.y -= 1.5;
    report.line(
        MARGIN_L,
        BODY,
        "All values in kW. Demand includes the spare contingency.",
        &sans,
    );

    // ---- full load list -------------------------------------------------
    let columns = [
        Column {
            title: "Line",
            chars: 4,
            numeric: true,
        },
        Column {
            title: "Tag",
            chars: 14,
            numeric: false,
        },
        Column {
            title: "Description",
            chars: 30,
            numeric: false,
        },
        Column {
            title: "Area",
            chars: 4,
            numeric: false,
        },
        Column {
            title: "Class",
            chars: 6,
            numeric: false,
        },
        Column {
            title: "Duty",
            chars: 4,
            numeric: false,
        },
        Column {
            title: "kV",
            chars: 5,
            numeric: true,
        },
        Column {
            title: "Absorb.",
            chars: 8,
            numeric: true,
        },
        Column {
            title: "Rated",
            chars: 8,
            numeric: true,
        },
        Column {
            title: "Eff.",
            chars: 5,
            numeric: true,
        },
        Column {
            title: "cos",
            chars: 5,
            numeric: true,
        },
        Column {
            title: "FLC A",
            chars: 7,
            numeric: true,
        },
        Column {
            title: "kW",
            chars: 8,
            numeric: true,
        },
        Column {
            title: "kVAr",
            chars: 8,
            numeric: true,
        },
    ];
    let draw_header = |report: &mut Report, continued: bool| {
        if continued {
            report.new_page();
            report.text(
                MARGIN_L,
                report.y,
                11.0,
                "Overall load list (continued)",
                &bold,
            );
            report.y -= 6.0;
        } else {
            report.new_page();
            report.text(MARGIN_L, report.y, 11.0, "Overall load list", &bold);
            report.y -= 6.0;
        }
        report.line(MARGIN_L, LIST, &header_row(&columns), &bold);
        report.rule_at(report.y + 2.4, width_mm(&columns, LIST), 0.7);
        report.line(
            MARGIN_L,
            LIST,
            "C continuous - I intermittent - SB stand-by - SP spare",
            &sans,
        );
    };

    draw_header(&mut report, false);
    let mut rows_on_page = 0usize;
    for item in &state.items {
        if report.y - 3.8 < MARGIN_BOTTOM + 9.0 || rows_on_page >= 72 {
            draw_header(&mut report, true);
            rows_on_page = 0;
        }
        let derived = &item.derived;
        report.line(
            MARGIN_L,
            LIST,
            &row(
                &columns,
                &[
                    item.item.line_no.map(|n| n.to_string()).unwrap_or_default(),
                    item.item.tag.clone().unwrap_or_default(),
                    item.item.description.clone().unwrap_or_default(),
                    item.item.area.clone().unwrap_or_default(),
                    item.item.equipment_class.key().to_string(),
                    item.item.duty.map(Duty::short).unwrap_or("-").to_string(),
                    num(item.item.voltage_kv, 2),
                    num(item.item.absorbed_kw, 1),
                    num(item.item.rated_kw, 1),
                    num(derived.efficiency, 3),
                    num(derived.power_factor, 3),
                    num(derived.flc_a, 1),
                    num(derived.consumed_kw, 1),
                    num(derived.consumed_kvar, 1),
                ],
            ),
            &mono,
        );
        rows_on_page += 1;
    }

    report.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_folds_typographic_characters() {
        assert_eq!(ascii("10 x 20 mm"), "10 x 20 mm");
        assert_eq!(ascii("cos \u{3c6}"), "cos phi");
        assert_eq!(ascii("2\u{2013}3"), "2-3");
        assert_eq!(ascii("caf\u{e9}"), "caf?");
    }

    #[test]
    fn numbers_are_formatted_with_fixed_precision() {
        assert_eq!(num(1578.947368, 1), "1578.9");
        assert_eq!(num(0.9505, 3), "0.951");
    }

    #[test]
    fn today_is_an_iso_date() {
        let value = today();
        assert_eq!(value.len(), 10, "{value}");
        assert!(value.starts_with("20"), "{value}");
        assert_eq!(value.matches('-').count(), 2);
    }

    #[test]
    fn cells_fill_exactly_the_column_width() {
        assert_eq!(cell("abc", 5, false), "abc  ");
        assert_eq!(cell("abc", 5, true), "  abc");
        let clipped = cell("a very long description", 10, false);
        assert_eq!(clipped.chars().count(), 10);
        assert!(clipped.ends_with('~'));
    }

    #[test]
    fn rows_join_columns_with_a_single_space() {
        let columns = [
            Column {
                title: "A",
                chars: 3,
                numeric: false,
            },
            Column {
                title: "B",
                chars: 4,
                numeric: true,
            },
        ];
        assert_eq!(header_row(&columns), "A      B");
        assert_eq!(
            row(&columns, &["xy".to_string(), "12".to_string()]),
            "xy    12"
        );
    }
}
