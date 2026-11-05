//! Renders the interface into an in-memory terminal and checks what is on screen.

use super::app::{App, Tab};
use super::ui;
use crate::analysis::Analysis;
use crate::assess::Urgency;
use crate::finding::{Confidence, QuantumRisk};
use crate::report::fixtures;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, MouseButton, MouseEventKind};

fn app_with(assessments: Vec<crate::assess::Assessment>) -> App {
    let analysis = Analysis {
        files: fixtures::files(),
        assessments,
    };
    let mut app = App::new(
        "project".into(),
        analysis,
        "2026-11".into(),
        false,
        Confidence::Low,
    );
    app.tab = Tab::Findings;
    app
}

fn app() -> App {
    app_with(fixtures::assessments())
}

fn screen(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut out = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn draw(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| ui::render(frame, app)).unwrap();
    screen(terminal.backend().buffer())
}

#[test]
fn wide_layout_shows_header_list_details_and_footer() {
    let screen = draw(&mut app(), 120, 32);
    for expected in [
        "agentq",
        "project",
        "knowledge base 2026-11",
        "Findings (3)",
        "ECDH (NIST curves)",
        "HIGH",
        "Details",
        "Why it matters",
        "Replace with",
        "ML-KEM (CRYSTALS-Kyber)",
        "3 use(s)",
        "2 broken",
        "hide safe: off",
        "quit",
    ] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
}

#[test]
fn narrow_layout_still_shows_the_list_and_the_details() {
    let screen = draw(&mut app(), 70, 30);
    assert!(screen.contains("Findings (3)"), "{screen}");
    assert!(screen.contains("Details"), "{screen}");
    assert!(screen.contains("ECDH (NIST curves)"), "{screen}");
}

#[test]
fn details_follow_the_selection() {
    let mut app = app();
    let first = draw(&mut app, 120, 32);
    assert!(first.contains("broken by Shor's algorithm"));
    assert!(first.contains("key exchange"));

    app.handle_key(KeyCode::Down);
    let second = draw(&mut app, 120, 32);
    assert!(second.contains("ECDSA"), "{second}");
    assert!(second.contains("ML-DSA"), "{second}");

    app.handle_key(KeyCode::Char('G'));
    let last = draw(&mut app, 120, 32);
    assert!(
        last.contains("SHA-256") || last.contains("sha2-256") || last.contains("OK"),
        "{last}"
    );
}

#[test]
fn safe_findings_show_no_replacement() {
    let mut app = app();
    app.handle_key(KeyCode::Char('G'));
    let screen = draw(&mut app, 120, 32);
    assert!(!screen.contains("Replace with"), "{screen}");
}

#[test]
fn empty_analysis_says_nothing_was_found() {
    let mut app = app_with(vec![]);
    let screen = draw(&mut app, 100, 24);
    assert!(screen.contains("No cryptographic use found."), "{screen}");
    assert!(screen.contains("Nothing selected."), "{screen}");
}

#[test]
fn filters_that_hide_everything_explain_why() {
    let safe_only: Vec<_> = fixtures::assessments()
        .into_iter()
        .filter(|a| a.risk == QuantumRisk::Safe)
        .collect();
    let mut app = app_with(safe_only);
    app.handle_key(KeyCode::Char('s'));
    let screen = draw(&mut app, 100, 24);
    assert!(screen.contains("hidden by the filters"), "{screen}");
    assert!(screen.contains("hide safe: on"), "{screen}");
}

#[test]
fn help_overlay_lists_the_keys() {
    let mut app = app();
    app.handle_key(KeyCode::Char('?'));
    let screen = draw(&mut app, 100, 30);
    for expected in [
        "Help",
        "select a finding",
        "scan the directory again",
        "quit",
    ] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
}

#[test]
fn long_details_can_be_scrolled_to_the_very_end() {
    let mut app = app();
    let top = draw(&mut app, 100, 16);
    assert!(
        app.detail_max_scroll > 0,
        "the details should not fit in 16 rows"
    );
    assert!(top.contains("Urgency"));
    assert!(
        !top.contains("Advice"),
        "the end should be off screen:\n{top}"
    );

    while app.detail_scroll < app.detail_max_scroll {
        app.handle_key(KeyCode::PageDown);
        draw(&mut app, 100, 16);
    }
    let bottom = draw(&mut app, 100, 16);
    assert!(!bottom.contains("Urgency"), "{bottom}");
    // The last advice line is fully visible: nothing is cut off at the end.
    assert!(bottom.contains("ciphertext 1088 bytes"), "{bottom}");

    app.handle_key(KeyCode::PageUp);
    assert_ne!(draw(&mut app, 100, 16), bottom);
}

#[test]
fn rescan_message_appears_in_the_footer() {
    let mut app = app();
    app.replace(Analysis {
        files: fixtures::files(),
        assessments: fixtures::assessments(),
    });
    let screen = draw(&mut app, 140, 24);
    assert!(
        screen.contains("Scanned again: 3 use(s) in 4 file(s)"),
        "{screen}"
    );
}

#[test]
fn tiny_terminals_do_not_panic() {
    for (width, height) in [(1, 1), (10, 3), (30, 8), (200, 5)] {
        let mut app = app();
        app.handle_key(KeyCode::Char('?'));
        draw(&mut app, width, height);
        app.handle_key(KeyCode::Char('?'));
        draw(&mut app, width, height);
    }
}

#[test]
fn help_overlay_explains_every_urgency_level() {
    let mut app = app();
    app.handle_key(KeyCode::Char('?'));
    let screen = draw(&mut app, 100, 30);
    for expected in ["HIGH", "MED ", "LOW ", "OK  ", "already post-quantum"] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
}

#[test]
fn overview_shows_totals_and_rankings() {
    let mut app = app();
    app.handle_key(KeyCode::Char('1'));
    let screen = draw(&mut app, 120, 34);
    for expected in [
        "1 Overview",
        "2 Findings (3)",
        "Quantum-vulnerable",
        "Weakened",
        "Already safe",
        "By urgency",
        "Top algorithms",
        "ECDH (NIST curves)",
        "Review first",
        "src/b.ts",
        "Languages",
        "JS/TS",
        "(50%)",
    ] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
    // The findings list belongs to the other tab.
    assert!(!screen.contains("Why it matters"), "{screen}");
}

#[test]
fn overview_has_a_message_when_there_is_nothing_to_show() {
    let mut app = app_with(vec![]);
    app.handle_key(KeyCode::Char('1'));
    let screen = draw(&mut app, 110, 30);
    assert!(screen.contains("No cryptographic use to show."), "{screen}");
}

#[test]
fn overview_survives_small_terminals() {
    for (width, height) in [(1, 1), (20, 6), (50, 14), (90, 40)] {
        let mut app = app();
        app.handle_key(KeyCode::Char('1'));
        draw(&mut app, width, height);
    }
}

fn click(app: &mut App, column: u16, row: u16) {
    app.handle_mouse(MouseEventKind::Down(MouseButton::Left), column, row);
}

#[test]
fn clicking_a_tab_switches_to_it() {
    let mut app = app();
    draw(&mut app, 120, 32);
    let overview = app
        .areas
        .tabs
        .iter()
        .find(|(_, t)| *t == Tab::Overview)
        .unwrap()
        .0;
    click(&mut app, overview.x + 1, overview.y);
    assert_eq!(app.tab, Tab::Overview);

    draw(&mut app, 120, 32);
    let findings = app
        .areas
        .tabs
        .iter()
        .find(|(_, t)| *t == Tab::Findings)
        .unwrap()
        .0;
    click(&mut app, findings.x + findings.width - 1, findings.y);
    assert_eq!(app.tab, Tab::Findings);
}

#[test]
fn clicking_a_finding_selects_it() {
    let mut app = app();
    draw(&mut app, 120, 32);
    let list = app.areas.list;
    // Each finding takes two rows: the second row of the third one is still the third.
    click(&mut app, list.x + 2, list.y + 5);
    assert_eq!(app.selected, 2);
    click(&mut app, list.x + 2, list.y);
    assert_eq!(app.selected, 0);
    // Below the last finding nothing changes.
    click(&mut app, list.x + 2, list.y + 8);
    assert_eq!(app.selected, 0);
    // Neither does a click outside the list.
    click(&mut app, 0, 0);
    assert_eq!(app.selected, 0);
    assert!(draw(&mut app, 120, 32).contains("ECDH (NIST curves)"));
}

#[test]
fn the_wheel_moves_the_selection_over_the_list_and_scrolls_the_details() {
    let mut app = app();
    draw(&mut app, 100, 16);
    let list = app.areas.list;
    app.handle_mouse(MouseEventKind::ScrollDown, list.x + 1, list.y + 1);
    app.handle_mouse(MouseEventKind::ScrollDown, list.x + 1, list.y + 1);
    assert_eq!(app.selected, 2);
    app.handle_mouse(MouseEventKind::ScrollUp, list.x + 1, list.y + 1);
    assert_eq!(app.selected, 1);

    draw(&mut app, 100, 16);
    assert!(app.detail_max_scroll > 0);
    let details = app.areas.details;
    app.handle_mouse(MouseEventKind::ScrollDown, details.x + 1, details.y + 1);
    assert_eq!(app.detail_scroll, 3.min(app.detail_max_scroll));
    app.handle_mouse(MouseEventKind::ScrollUp, details.x + 1, details.y + 1);
    assert_eq!(app.detail_scroll, 0);
}

#[test]
fn clicking_an_urgency_on_the_overview_opens_its_first_finding() {
    let mut app = app();
    app.handle_key(KeyCode::Char('1'));
    draw(&mut app, 120, 34);
    let row = |u: Urgency| {
        app.areas
            .urgency_rows
            .iter()
            .find(|(_, x)| *x == u)
            .unwrap()
            .0
    };

    let medium = row(Urgency::Medium);
    click(&mut app, medium.x + 2, medium.y);
    assert_eq!(app.tab, Tab::Findings);
    assert_eq!(
        app.selected_assessment().unwrap().finding.algorithm,
        "ecdsa"
    );

    app.handle_key(KeyCode::Char('1'));
    draw(&mut app, 120, 34);
    let low = app
        .areas
        .urgency_rows
        .iter()
        .find(|(_, x)| *x == Urgency::Low)
        .unwrap()
        .0;
    click(&mut app, low.x + 2, low.y);
    // Nothing is LOW in the fixtures: stay on the overview and say so.
    assert_eq!(app.tab, Tab::Overview);
    assert!(draw(&mut app, 120, 34).contains("No finding with that urgency"));
}

#[test]
fn a_click_closes_the_help_and_the_wheel_is_ignored_meanwhile() {
    let mut app = app();
    app.handle_key(KeyCode::Char('?'));
    draw(&mut app, 100, 30);
    let list = app.areas.list;
    app.handle_mouse(MouseEventKind::ScrollDown, list.x + 1, list.y + 1);
    assert_eq!(app.selected, 0);
    click(&mut app, list.x + 1, list.y + 3);
    assert!(!app.show_help);
    assert_eq!(
        app.selected, 0,
        "the click that closes the help does not also select"
    );
}

#[test]
fn mouse_events_on_a_tiny_terminal_do_not_panic() {
    let mut app = app();
    draw(&mut app, 5, 3);
    for (column, row) in [(0, 0), (4, 2), (200, 200)] {
        click(&mut app, column, row);
        app.handle_mouse(MouseEventKind::ScrollDown, column, row);
    }
}
