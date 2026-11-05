//! Renders the interface into an in-memory terminal and checks what is on screen.

use super::app::{App, Tab};
use super::browser::{Browser, fixture::TempTree};
use super::ui;
use super::{Cmd, Msg, Screen, Tui, perform};
use crate::analysis::Analysis;
use crate::analysis::analyze_directory;
use crate::assess::Urgency;
use crate::finding::{Confidence, QuantumRisk};
use crate::i18n::Lang;
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
        Lang::En,
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

fn draw_tui(tui: &mut Tui, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| tui.render(frame)).unwrap();
    screen(terminal.backend().buffer())
}

/// Does what the commands ask for, right here, and delivers the results.
fn settle(tui: &mut Tui) {
    loop {
        let commands = tui.drain_commands();
        if commands.is_empty() {
            break;
        }
        for command in commands {
            let Cmd::Scan { id, path, lang } = command;
            let analysis = analyze_directory(&path, lang);
            tui.update(Msg::Scanned {
                id,
                path,
                lang,
                analysis,
            });
        }
    }
}

fn press(tui: &mut Tui, code: KeyCode) {
    tui.update(Msg::Key(code));
    settle(tui);
}

/// Starts the interface and lets the first scan finish.
fn started(
    path: Option<&std::path::Path>,
    hide_safe: bool,
    confidence: Confidence,
    lang: Lang,
) -> Tui {
    let mut tui = Tui::start(path, hide_safe, confidence, lang);
    settle(&mut tui);
    tui
}

fn sample() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("samples/node-shop")
}

#[test]
fn the_browser_shows_folders_hints_and_the_way_to_analyze() {
    let tree = TempTree::new();
    let mut tui = Tui::start(None, false, Confidence::Low, Lang::En);
    tui.screen = Screen::Browse(Browser::new(tree.0.clone()));
    let screen = draw_tui(&mut tui, 110, 22);
    for expected in [
        "Choose a project to analyze",
        "Analyze this folder",
        "parent folder",
        "alpha/",
        "Node project",
        "Beta/",
        "git repository",
        "gamma/",
        "analyze here",
    ] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
    assert!(!screen.contains(".hidden"), "{screen}");
}

#[test]
fn the_browser_explains_when_a_folder_cannot_be_opened() {
    let tree = TempTree::new();
    let mut browser = Browser::new(tree.0.clone());
    browser.open(tree.0.join("missing"));
    let mut tui = Tui::start(None, false, Confidence::Low, Lang::En);
    tui.screen = Screen::Browse(browser);
    assert!(draw_tui(&mut tui, 110, 22).contains("Cannot open"));
}

#[test]
fn a_path_skips_the_browser() {
    let tui = started(Some(&sample()), false, Confidence::Low, Lang::En);
    let Screen::Results(app) = &tui.screen else {
        panic!("expected the results")
    };
    assert_eq!(app.visible().len(), 22);
}

#[test]
fn without_a_path_it_starts_on_the_browser() {
    let tui = Tui::start(None, false, Confidence::Low, Lang::En);
    assert!(matches!(tui.screen, Screen::Browse(_)));
}

#[test]
fn choosing_a_folder_shows_its_results_and_b_goes_back() {
    let samples = sample().parent().unwrap().to_path_buf();
    let mut tui = Tui::start(None, true, Confidence::Medium, Lang::En);
    tui.screen = Screen::Browse(Browser::new(samples.clone()));

    // Go into node-shop and analyze it.
    let Screen::Browse(browser) = &mut tui.screen else {
        unreachable!()
    };
    browser.select_path(&std::fs::canonicalize(sample()).unwrap());
    press(&mut tui, KeyCode::Enter);
    press(&mut tui, KeyCode::Char('a'));

    let Screen::Results(app) = &tui.screen else {
        panic!("expected the results")
    };
    assert!(app.root.ends_with("node-shop"));
    // The settings given on the command line carry over.
    assert!(app.hide_safe);
    assert_eq!(app.min_confidence, Confidence::Medium);
    assert!(draw_tui(&mut tui, 120, 34).contains("Overview"));

    // "b" returns to the folder that holds the project, with it selected.
    press(&mut tui, KeyCode::Char('b'));
    let Screen::Browse(browser) = &tui.screen else {
        panic!("expected the browser")
    };
    assert_eq!(browser.dir, std::fs::canonicalize(&samples).unwrap());
    assert_eq!(browser.entries[browser.selected].name, "node-shop");
}

#[test]
fn quitting_works_from_both_screens() {
    let mut tui = Tui::start(None, false, Confidence::Low, Lang::En);
    press(&mut tui, KeyCode::Char('q'));
    assert!(tui.should_quit());

    let mut tui = started(Some(&sample()), false, Confidence::Low, Lang::En);
    press(&mut tui, KeyCode::Char('q'));
    assert!(tui.should_quit());
}

#[test]
fn rescan_keeps_the_results_screen() {
    let mut tui = started(Some(&sample()), false, Confidence::Low, Lang::En);
    press(&mut tui, KeyCode::Char('r'));
    let Screen::Results(app) = &tui.screen else {
        panic!("expected the results")
    };
    assert_eq!(app.visible().len(), 22);
    assert!(app.status.as_deref().unwrap().contains("22 use(s)"));
}

#[test]
fn mouse_events_reach_the_active_screen() {
    let tree = TempTree::new();
    let mut tui = Tui::start(None, false, Confidence::Low, Lang::En);
    tui.screen = Screen::Browse(Browser::new(tree.0.clone()));
    draw_tui(&mut tui, 110, 22);
    let Screen::Browse(browser) = &tui.screen else {
        unreachable!()
    };
    let list = browser.list_area;
    tui.update(Msg::Mouse {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: list.x + 3,
        row: list.y + 2,
    });
    let Screen::Browse(browser) = &tui.screen else {
        unreachable!()
    };
    assert_eq!(browser.selected, 2);
}

fn app_in(lang: Lang) -> App {
    let mut app = App::new(
        "project".into(),
        Analysis {
            files: fixtures::files(),
            assessments: crate::assess::assess_all(
                &fixtures::assessments()
                    .iter()
                    .map(|a| a.finding.clone())
                    .collect::<Vec<_>>(),
                crate::knowledge::KnowledgeBase::embedded(),
                lang,
            ),
        },
        "2026-11".into(),
        false,
        Confidence::Low,
        lang,
    );
    app.tab = Tab::Findings;
    app
}

#[test]
fn the_findings_page_in_portuguese() {
    let screen = draw(&mut app_in(Lang::PtBr), 130, 34);
    for expected in [
        "1 Visão geral",
        "2 Achados (3)",
        "4 arquivo(s) · base de conhecimento 2026-11",
        "Achados (3)",
        "ALTA",
        "troca de chaves",
        "Detalhes",
        "Urgência",
        "alta",
        "quebrado pelo algoritmo de Shor",
        "Propósito",
        "Biblioteca",
        "Confiança",
        "Alta",
        "Local",
        "Código",
        "Por que importa",
        "Substituir por",
        "3 uso(s)",
        "2 quebrado(s)",
        "ocultar seguros: desativado · confiança mínima: Baixa",
        "sair",
    ] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
    assert!(!screen.contains("Why it matters"), "{screen}");
}

#[test]
fn the_overview_in_portuguese() {
    let mut app = app_in(Lang::PtBr);
    app.handle_key(KeyCode::Char('1'));
    let screen = draw(&mut app, 130, 34);
    for expected in [
        "Vulneráveis a quânticos",
        "quebrados por Shor",
        "Enfraquecidos",
        "Já seguros",
        "sem necessidade de migração",
        "Por urgência",
        "Algoritmos mais usados",
        "Revisar primeiro",
        "Linguagens",
    ] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
}

#[test]
fn the_help_in_portuguese_fits_and_lists_the_language_key() {
    let mut app = app_in(Lang::PtBr);
    app.handle_key(KeyCode::Char('?'));
    let screen = draw(&mut app, 100, 34);
    for expected in [
        "Ajuda",
        "selecionar um achado",
        "analisar a pasta de novo",
        "trocar o idioma",
        "ALTA  troca de chaves",
        "BAIXA enfraquecido pelo algoritmo de Grover",
        "seguro, ou já pós-quântico",
    ] {
        assert!(
            screen.contains(expected),
            "missing `{expected}` in:\n{screen}"
        );
    }
}

#[test]
fn the_l_key_switches_the_language_and_rewrites_the_advice() {
    let mut app = app();
    assert!(draw(&mut app, 130, 34).contains("Why it matters"));
    let before = app.selected_assessment().unwrap().advice.clone();

    app.handle_key(KeyCode::Char('L'));
    assert_eq!(app.lang, Lang::PtBr);
    let screen = draw(&mut app, 130, 34);
    assert!(screen.contains("Por que importa"), "{screen}");
    assert_ne!(app.selected_assessment().unwrap().advice, before);
    assert!(
        app.selected_assessment().unwrap().advice[0].contains("esquema híbrido"),
        "{:?}",
        app.selected_assessment().unwrap().advice
    );
    // The order and the selection do not change with the language.
    assert_eq!(app.selected_assessment().unwrap().finding.algorithm, "ecdh");

    app.handle_key(KeyCode::Char('L'));
    assert_eq!(app.lang, Lang::En);
    assert!(draw(&mut app, 130, 34).contains("Why it matters"));
}

#[test]
fn the_language_survives_moving_between_screens() {
    let mut tui = started(Some(&sample()), false, Confidence::Low, Lang::En);
    press(&mut tui, KeyCode::Char('L'));
    assert!(draw_tui(&mut tui, 130, 34).contains("Visão geral"));

    press(&mut tui, KeyCode::Char('b'));
    let browser = draw_tui(&mut tui, 110, 24);
    for expected in [
        "Escolha um projeto para analisar",
        "Analisar esta pasta",
        "pasta acima",
        "analisar aqui",
    ] {
        assert!(
            browser.contains(expected),
            "missing `{expected}` in:\n{browser}"
        );
    }

    // Switching back in the browser sticks when a project is opened again.
    press(&mut tui, KeyCode::Char('L'));
    assert!(draw_tui(&mut tui, 110, 24).contains("Choose a project to analyze"));
    press(&mut tui, KeyCode::Char('a'));
    assert!(draw_tui(&mut tui, 130, 34).contains("Overview"));
}

#[test]
fn the_browser_hints_and_errors_are_translated() {
    let tree = TempTree::new();
    let mut browser = Browser::new(tree.0.clone());
    browser.open(tree.0.join("missing"));
    let mut tui = Tui::start(None, false, Confidence::Low, Lang::PtBr);
    tui.screen = Screen::Browse(browser);
    let screen = draw_tui(&mut tui, 120, 24);
    assert!(screen.contains("Projeto Node"), "{screen}");
    assert!(screen.contains("repositório git"), "{screen}");
    assert!(screen.contains("Não foi possível abrir"), "{screen}");
    assert!(screen.contains("Pastas (3)"), "{screen}");
}

#[test]
fn the_rescan_status_is_translated() {
    let mut app = app_in(Lang::PtBr);
    app.replace(Analysis {
        files: fixtures::files(),
        assessments: fixtures::assessments(),
    });
    // It is visible even on a terminal too narrow for the whole summary.
    assert!(draw(&mut app, 80, 24).contains("Análise refeita: 3 uso(s) em 4 arquivo(s)"));
}

#[test]
fn a_path_shows_a_loading_screen_until_the_scan_arrives() {
    let mut tui = Tui::start(Some(&sample()), false, Confidence::Low, Lang::En);
    assert!(tui.is_busy());
    let loading = draw_tui(&mut tui, 100, 20);
    assert!(loading.contains("Analyzing"), "{loading}");
    assert!(loading.contains("node-shop"), "{loading}");
    assert!(loading.contains("Esc cancels"), "{loading}");
    assert!(loading.contains("⠋"), "{loading}");

    let commands = tui.drain_commands();
    assert_eq!(commands.len(), 1);
    let Cmd::Scan { path, .. } = &commands[0];
    assert_eq!(path, &sample());

    // The scan finishes: the results replace the loading screen.
    let Cmd::Scan { id, path, lang } = commands.into_iter().next().unwrap();
    let analysis = analyze_directory(&path, lang);
    tui.update(Msg::Scanned {
        id,
        path,
        lang,
        analysis,
    });
    assert!(!tui.is_busy());
    assert!(matches!(tui.screen, Screen::Results(_)));
}

#[test]
fn ticks_move_the_spinner() {
    let mut tui = Tui::start(Some(&sample()), false, Confidence::Low, Lang::En);
    let first = draw_tui(&mut tui, 100, 20);
    tui.update(Msg::Tick);
    let second = draw_tui(&mut tui, 100, 20);
    assert!(first.contains("⠋") && !first.contains("⠙"));
    assert!(second.contains("⠙") && !second.contains("⠋"));
}

#[test]
fn escape_cancels_the_loading_and_a_late_result_is_ignored() {
    let mut tui = Tui::start(Some(&sample()), false, Confidence::Low, Lang::En);
    let Cmd::Scan { id, path, lang } = tui.drain_commands().into_iter().next().unwrap();

    tui.update(Msg::Key(KeyCode::Esc));
    assert!(!tui.is_busy());
    let Screen::Browse(browser) = &tui.screen else {
        panic!("expected the browser")
    };
    assert_eq!(browser.entries[browser.selected].name, "node-shop");

    // The cancelled scan still finishes in the background; nobody wants it.
    let analysis = analyze_directory(&path, lang);
    tui.update(Msg::Scanned {
        id,
        path,
        lang,
        analysis,
    });
    assert!(matches!(tui.screen, Screen::Browse(_)));
}

#[test]
fn only_the_latest_scan_counts() {
    let mut tui = started(Some(&sample()), false, Confidence::Low, Lang::En);
    // Two rescans in a row: the second is ignored while the first runs.
    tui.update(Msg::Key(KeyCode::Char('r')));
    tui.update(Msg::Key(KeyCode::Char('r')));
    assert_eq!(tui.drain_commands().len(), 1);
}

#[test]
fn a_rescan_keeps_the_results_on_screen_while_it_runs() {
    let mut tui = started(Some(&sample()), false, Confidence::Low, Lang::En);
    tui.update(Msg::Key(KeyCode::Down));
    tui.update(Msg::Key(KeyCode::Char('r')));
    assert!(tui.is_busy());
    let screen = draw_tui(&mut tui, 140, 34);
    assert!(screen.contains("Scanning…"), "{screen}");
    assert!(
        screen.contains("Overview"),
        "the results stay visible:\n{screen}"
    );

    settle(&mut tui);
    assert!(!tui.is_busy());
    let screen = draw_tui(&mut tui, 140, 34);
    assert!(!screen.contains("Scanning…"), "{screen}");
    let Screen::Results(app) = &tui.screen else {
        panic!("expected the results")
    };
    assert!(!app.scanning);
    assert!(app.status.as_deref().unwrap().contains("22 use(s)"));
}

#[test]
fn ctrl_c_quits_from_every_screen() {
    for mut tui in [
        Tui::start(None, false, Confidence::Low, Lang::En),
        Tui::start(Some(&sample()), false, Confidence::Low, Lang::En),
        started(Some(&sample()), false, Confidence::Low, Lang::En),
    ] {
        tui.update(Msg::Quit);
        assert!(tui.should_quit());
    }
}

#[test]
fn the_language_chosen_while_loading_wins() {
    let mut tui = Tui::start(Some(&sample()), false, Confidence::Low, Lang::En);
    tui.update(Msg::Key(KeyCode::Char('L')));
    assert!(draw_tui(&mut tui, 100, 20).contains("Analisando"));
    // The scan was started in English, but the user has moved to Portuguese.
    settle(&mut tui);
    let Screen::Results(app) = &tui.screen else {
        panic!("expected the results")
    };
    assert_eq!(app.lang, Lang::PtBr);
    assert!(
        app.selected_assessment()
            .is_some_and(|a| a.explanation.contains("colher agora")
                || a.advice.iter().any(|t| t.contains("híbrido")))
    );
}

#[test]
fn a_scan_runs_on_a_background_thread_and_reports_back() {
    let (tx, rx) = std::sync::mpsc::channel();
    perform(
        Cmd::Scan {
            id: 7,
            path: sample(),
            lang: Lang::En,
        },
        &tx,
    );
    let msg = rx
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the scan should finish");
    let Msg::Scanned { id, analysis, .. } = msg else {
        panic!("expected Scanned")
    };
    assert_eq!(id, 7);
    assert_eq!(analysis.assessments.len(), 22);
}

#[test]
fn the_busy_state_follows_the_scans() {
    let mut tui = Tui::start(None, false, Confidence::Low, Lang::En);
    assert!(!tui.is_busy());
    let Screen::Browse(browser) = &mut tui.screen else {
        unreachable!()
    };
    browser.select_path(&std::fs::canonicalize(sample().parent().unwrap()).unwrap());
    // Pressing "a" in the browser analyzes the folder being shown.
    tui.update(Msg::Key(KeyCode::Char('a')));
    assert!(tui.is_busy());
    assert!(matches!(tui.screen, Screen::Loading(_)));
    settle(&mut tui);
    assert!(!tui.is_busy());
}
