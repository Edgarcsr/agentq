//! Drawing of the terminal interface.

use super::app::{App, Areas, Tab};
use super::overview;
use crate::assess::{Assessment, Urgency};
use crate::finding::QuantumRisk;
use crate::knowledge::Attack;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Wrap};

// The agentq palette.
pub(super) const ACCENT: Color = Color::Rgb(255, 138, 61);
pub(super) const ACCENT_LIGHT: Color = Color::Rgb(255, 210, 161);
pub(super) const FG: Color = Color::Rgb(245, 236, 228);
pub(super) const MUTED: Color = Color::Rgb(160, 143, 128);
pub(super) const BORDER: Color = Color::Rgb(51, 39, 29);
pub(super) const DANGER: Color = Color::Rgb(255, 107, 91);
pub(super) const SAFE: Color = Color::Rgb(142, 207, 160);
pub(super) const ON_ACCENT: Color = Color::Rgb(26, 8, 0);

/// Terminals narrower than this stack the list above the details.
const WIDE: u16 = 100;

pub(super) fn urgency_color(urgency: Urgency) -> Color {
    match urgency {
        Urgency::High => DANGER,
        Urgency::Medium => ACCENT,
        Urgency::Low => ACCENT_LIGHT,
        Urgency::None => SAFE,
    }
}

pub(super) fn urgency_label(urgency: Urgency) -> &'static str {
    match urgency {
        Urgency::High => " HIGH ",
        Urgency::Medium => " MED  ",
        Urgency::Low => " LOW  ",
        Urgency::None => " OK   ",
    }
}

pub(super) fn badge(urgency: Urgency) -> Span<'static> {
    Span::styled(
        urgency_label(urgency),
        Style::new()
            .fg(ON_ACCENT)
            .bg(urgency_color(urgency))
            .add_modifier(Modifier::BOLD),
    )
}

pub(super) fn panel(title: &str) -> Block<'static> {
    Block::bordered()
        .padding(Padding::horizontal(1))
        .border_style(Style::new().fg(BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.areas = Areas::default();
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(2),
    ])
    .areas(area);

    render_header(frame, header, app);

    match app.tab {
        Tab::Overview => overview::render(frame, body, app),
        Tab::Findings => {
            let [list_area, detail_area] = if area.width >= WIDE {
                Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
                    .areas(body)
            } else {
                Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .areas(body)
            };
            render_list(frame, list_area, app);
            render_details(frame, detail_area, app);
        }
    }
    render_footer(frame, footer, app);

    if app.show_help {
        render_help(frame, area);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &mut App) {
    let title = Line::from(vec![
        Span::styled(
            " agentq ",
            Style::new()
                .fg(ON_ACCENT)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  {}", app.root), Style::new().fg(FG)),
        Span::styled(
            format!(
                "  {} file(s) · knowledge base {}",
                app.file_count, app.knowledge_version
            ),
            Style::new().fg(MUTED),
        ),
    ]);

    let tab = |label: String, active: bool| {
        let style = if active {
            Style::new()
                .fg(ON_ACCENT)
                .bg(ACCENT_LIGHT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(MUTED)
        };
        Span::styled(format!(" {label} "), style)
    };
    let labels = [
        ("1 Overview".to_string(), Tab::Overview),
        (
            format!("2 Findings ({})", app.visible().len()),
            Tab::Findings,
        ),
    ];
    let mut spans = vec![Span::raw(" ")];
    let mut x = area.x.saturating_add(1);
    for (label, which) in &labels {
        let span = tab(label.clone(), app.tab == *which);
        let width = span.width() as u16;
        app.areas.tabs.push((
            Rect::new(x, area.y.saturating_add(1), width, 1).intersection(area),
            *which,
        ));
        x = x.saturating_add(width).saturating_add(1);
        spans.push(span);
        spans.push(Span::raw(" "));
    }
    let tabs = Line::from(spans);

    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_style(Style::new().fg(BORDER));
    frame.render_widget(Paragraph::new(vec![title, tabs]).block(block), area);
}

fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let root = app.root.clone();
    let items: Vec<ListItem> = app
        .visible()
        .iter()
        .map(|a| {
            let finding = &a.finding;
            let location = finding
                .file
                .strip_prefix(&root)
                .unwrap_or(&finding.file)
                .display();
            let first = Line::from(vec![
                badge(a.urgency),
                Span::raw(" "),
                Span::styled(
                    a.algorithm_name.clone(),
                    Style::new().fg(FG).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}", finding.purpose.label()),
                    Style::new().fg(MUTED),
                ),
            ]);
            let second = Line::from(Span::styled(
                format!("       {location}:{}", finding.line),
                Style::new().fg(MUTED),
            ));
            ListItem::new(Text::from(vec![first, second]))
        })
        .collect();

    let title = format!("Findings ({})", items.len());
    app.areas.list = panel(&title).inner(area);
    if items.is_empty() {
        let hidden = app.hidden();
        let message = if hidden > 0 {
            format!("{hidden} finding(s) hidden by the filters.\nPress s or c to change them.")
        } else {
            "No cryptographic use found.".to_string()
        };
        let text = Paragraph::new(message)
            .style(Style::new().fg(MUTED))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(panel(&title));
        frame.render_widget(text, area);
        return;
    }

    let list = List::new(items)
        .block(panel(&title))
        .highlight_style(Style::new().bg(BORDER).add_modifier(Modifier::BOLD))
        .highlight_symbol("▸ ");
    app.list_state.select(Some(app.selected));
    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn field(label: &str, value: Span<'static>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<11}"), Style::new().fg(MUTED)),
        value,
    ])
}

fn heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
    ))
}

fn bullet(text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  • ", Style::new().fg(ACCENT)),
        Span::styled(text.to_string(), Style::new().fg(FG)),
    ])
}

fn risk_text(a: &Assessment) -> String {
    let attack = match a.attack {
        Attack::Shor => "Shor's algorithm",
        Attack::Grover => "Grover's algorithm",
        Attack::None => "no known quantum attack",
    };
    match a.risk {
        QuantumRisk::Broken => format!("broken by {attack}"),
        QuantumRisk::Weakened => format!("weakened by {attack}"),
        QuantumRisk::Safe => "safe".to_string(),
    }
}

/// The lines of the details pane for one finding.
fn detail_lines(a: &Assessment, root: &str) -> Vec<Line<'static>> {
    let finding = &a.finding;
    let location = finding.file.strip_prefix(root).unwrap_or(&finding.file);
    let urgency_name = match a.urgency {
        Urgency::High => "high",
        Urgency::Medium => "medium",
        Urgency::Low => "low",
        Urgency::None => "none",
    };

    let mut lines = vec![
        Line::from(Span::styled(
            a.algorithm_name.clone(),
            Style::new().fg(FG).add_modifier(Modifier::BOLD),
        )),
        Line::raw(""),
        field(
            "Urgency",
            Span::styled(
                urgency_name,
                Style::new()
                    .fg(urgency_color(a.urgency))
                    .add_modifier(Modifier::BOLD),
            ),
        ),
        field("Quantum", Span::styled(risk_text(a), Style::new().fg(FG))),
        field(
            "Purpose",
            Span::styled(finding.purpose.label(), Style::new().fg(FG)),
        ),
    ];
    if let Some(bits) = finding.key_size {
        lines.push(field(
            "Key size",
            Span::styled(format!("{bits}-bit"), Style::new().fg(FG)),
        ));
    }
    lines.push(field(
        "Library",
        Span::styled(finding.library.clone(), Style::new().fg(FG)),
    ));
    lines.push(field(
        "Confidence",
        Span::styled(format!("{:?}", finding.confidence), Style::new().fg(FG)),
    ));
    lines.push(field(
        "Location",
        Span::styled(
            format!("{}:{}:{}", location.display(), finding.line, finding.column),
            Style::new().fg(FG),
        ),
    ));

    lines.push(Line::raw(""));
    lines.push(heading("Code"));
    lines.push(Line::from(Span::styled(
        format!("  {}", finding.snippet),
        Style::new().fg(ACCENT_LIGHT),
    )));

    lines.push(Line::raw(""));
    lines.push(heading("Why it matters"));
    lines.push(Line::from(Span::styled(
        a.explanation.clone(),
        Style::new().fg(FG),
    )));

    if !a.recommendations.is_empty() {
        lines.push(Line::raw(""));
        lines.push(heading("Replace with"));
        for recommendation in &a.recommendations {
            lines.push(bullet(&recommendation.name));
        }
        let mut seen: Vec<&str> = Vec::new();
        for recommendation in &a.recommendations {
            for implementation in &recommendation.implementations {
                if !seen.contains(&implementation.as_str()) {
                    seen.push(implementation);
                }
            }
        }
        if !seen.is_empty() {
            lines.push(Line::raw(""));
            lines.push(heading("Implementations"));
            for implementation in seen {
                lines.push(bullet(implementation));
            }
        }
    }
    if !a.advice.is_empty() {
        lines.push(Line::raw(""));
        lines.push(heading("Advice"));
        for advice in &a.advice {
            lines.push(bullet(advice));
        }
    }
    lines
}

fn render_details(frame: &mut Frame, area: Rect, app: &mut App) {
    let block = panel("Details");
    let inner = block.inner(area);
    app.areas.details = inner;
    let Some(assessment) = app.selected_assessment() else {
        let empty = Paragraph::new("Nothing selected.")
            .style(Style::new().fg(MUTED))
            .block(block);
        frame.render_widget(empty, area);
        app.detail_max_scroll = 0;
        return;
    };

    let paragraph = Paragraph::new(detail_lines(assessment, &app.root)).wrap(Wrap { trim: false });
    // Measured without the border, so it is the exact number of rows once wrapped.
    let rows = u16::try_from(paragraph.line_count(inner.width)).unwrap_or(u16::MAX);
    app.detail_max_scroll = rows.saturating_sub(inner.height);
    let scroll = app.detail_scroll.min(app.detail_max_scroll);

    frame.render_widget(paragraph.scroll((scroll, 0)).block(block), area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let counts = app.counts();
    let mut summary = vec![
        Span::styled(
            format!(" {} use(s) ", counts.total),
            Style::new().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} broken ", counts.broken),
            Style::new().fg(DANGER),
        ),
        Span::styled(
            format!("{} weakened ", counts.weakened),
            Style::new().fg(ACCENT_LIGHT),
        ),
        Span::styled(format!("{} safe", counts.safe), Style::new().fg(MUTED)),
        Span::styled(
            format!(
                "   hide safe: {} · min confidence: {:?}",
                if app.hide_safe { "on" } else { "off" },
                app.min_confidence
            ),
            Style::new().fg(MUTED),
        ),
    ];
    if let Some(status) = &app.status {
        summary.push(Span::styled(
            format!("   {status}"),
            Style::new().fg(ACCENT),
        ));
    }

    let key =
        |k: &'static str| Span::styled(k, Style::new().fg(ACCENT).add_modifier(Modifier::BOLD));
    let text = |t: &'static str| Span::styled(t, Style::new().fg(MUTED));
    let keys = Line::from(vec![
        text(" "),
        key("↑↓"),
        text(" select  "),
        key("1 2"),
        text(" tabs  "),
        key("PgUp/PgDn"),
        text(" scroll  "),
        key("s"),
        text(" hide safe  "),
        key("c"),
        text(" confidence  "),
        key("r"),
        text(" rescan  "),
        key("b"),
        text(" browse  "),
        key("?"),
        text(" help  "),
        key("q"),
        text(" quit"),
    ]);
    frame.render_widget(Paragraph::new(vec![Line::from(summary), keys]), area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let rows = [
        ("↑ ↓  j k", "select a finding"),
        ("g  G", "first / last finding"),
        ("PgUp  PgDn  space", "scroll the details"),
        ("s", "hide or show safe uses"),
        ("c", "cycle the minimum confidence"),
        ("r", "scan the directory again"),
        ("b", "choose another project"),
        ("1  2  Tab", "switch between overview and findings"),
        ("mouse", "click tabs and findings, wheel to scroll"),
        ("?", "close this help"),
        ("q  Esc", "quit"),
    ];
    let mut lines = vec![Line::raw("")];
    for (keys, what) in rows {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {keys:<20}"),
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(what, Style::new().fg(FG)),
        ]));
    }
    lines.push(Line::raw(""));
    for legend in [
        "  HIGH  key exchange or encryption that quantum computers break",
        "  MED   signatures that quantum computers could forge",
        "  LOW   weakened by Grover's algorithm",
        "  OK    safe, or already post-quantum",
    ] {
        lines.push(Line::from(Span::styled(legend, Style::new().fg(MUTED))));
    }

    let width = 68.min(area.width);
    let height = (lines.len() as u16 + 2).min(area.height);
    let [popup] = Layout::horizontal([Constraint::Length(width)])
        .flex(Flex::Center)
        .areas(area);
    let [popup] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel("Help")),
        popup,
    );
}
