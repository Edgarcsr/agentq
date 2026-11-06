//! Drawing of the terminal interface.

use super::app::{App, Areas, Tab};
use super::overview;
use crate::assess::{Assessment, Urgency};
use crate::i18n::{Lang, fill};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Clear, List, ListItem, Padding, Paragraph, Wrap};

// The agentq palette: white text on a neutral background, with orange kept for
// whatever has focus (the selected row, the active tab, the focused panel).
// The other colors only carry meaning (risk), never decoration.
pub(super) const ACCENT: Color = Color::Rgb(255, 138, 61);
pub(super) const FG: Color = Color::Rgb(245, 245, 245);
pub(super) const MUTED: Color = Color::Rgb(138, 138, 138);
pub(super) const BORDER: Color = Color::Rgb(70, 70, 70);
pub(super) const KEY: Color = Color::Rgb(200, 200, 200);
pub(super) const DIM: Color = Color::Rgb(98, 98, 98);
// Risk is told by weight, not by hue: the worst level is an inverted badge and
// the others fade through shades of gray, so orange is the only color.
pub(super) const BAR: Color = Color::Rgb(176, 176, 176);
pub(super) const ON_ACCENT: Color = Color::Rgb(20, 20, 20);

/// Terminals narrower than this stack the list above the details.
const WIDE: u16 = 100;

pub(super) fn urgency_color(urgency: Urgency) -> Color {
    match urgency {
        Urgency::High => FG,
        Urgency::Medium => KEY,
        Urgency::Low => MUTED,
        Urgency::None => DIM,
    }
}

pub(super) fn badge(urgency: Urgency, lang: Lang) -> Span<'static> {
    badge_for(urgency, lang, false)
}

/// The badge of an urgency; on the selected row it turns orange.
pub(super) fn badge_for(urgency: Urgency, lang: Lang, selected: bool) -> Span<'static> {
    let color = if selected {
        ACCENT
    } else {
        urgency_color(urgency)
    };
    let style = Style::new().fg(color).add_modifier(Modifier::BOLD);
    Span::styled(
        lang.badge(urgency),
        if urgency == Urgency::High {
            style.fg(ON_ACCENT).bg(color)
        } else {
            style
        },
    )
}

pub(super) fn panel(title: &str) -> Block<'static> {
    framed(title, BORDER, MUTED)
}

/// A panel that has the focus: the only kind that is drawn in orange.
pub(super) fn focused_panel(title: &str) -> Block<'static> {
    framed(title, ACCENT, ACCENT)
}

fn framed(title: &str, border: Color, text: Color) -> Block<'static> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
        .border_style(Style::new().fg(border))
        .title(Span::styled(
            format!(" {title} "),
            Style::new().fg(text).add_modifier(Modifier::BOLD),
        ))
}

/// Marker drawn before the selected row of a list.
pub(super) fn selected_symbol() -> Line<'static> {
    Line::from("")
}

/// The bar drawn at the left of every row of a list: orange on the selected
/// one, blank on the others, as in the Bubbles list.
pub(super) fn row_bar(selected: bool) -> Span<'static> {
    if selected {
        Span::styled("┃ ", Style::new().fg(ACCENT))
    } else {
        Span::raw("  ")
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.areas = Areas::default();
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(4),
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
        render_help(frame, area, app.lang);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.lang.t();
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
                "  {}",
                fill(
                    t.header_info,
                    &[
                        ("files", &app.file_count),
                        ("version", &app.knowledge_version)
                    ]
                )
            ),
            Style::new().fg(MUTED),
        ),
    ]);

    let labels = [
        (format!("1 {}", t.tab_overview), Tab::Overview),
        (
            format!("2 {} ({})", t.tab_findings, app.visible().len()),
            Tab::Findings,
        ),
    ];
    // Flat tabs on a rule: the active one is orange and its stretch of the rule
    // is thick, so it reads as the open page.
    let mut tabs = vec![Span::raw(" ")];
    let mut rule = vec![Span::styled("─", Style::new().fg(BORDER))];
    let mut x = area.x.saturating_add(1);
    for (label, which) in &labels {
        let active = app.tab == *which;
        let text = format!(" {label} ");
        let width = text.chars().count();
        tabs.push(Span::styled(
            text,
            if active {
                Style::new()
                    .fg(ON_ACCENT)
                    .bg(ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(MUTED)
            },
        ));
        tabs.push(Span::raw(" "));
        rule.push(Span::styled(
            if active { "━" } else { "─" }.repeat(width),
            Style::new().fg(if active { ACCENT } else { BORDER }),
        ));
        rule.push(Span::styled("─", Style::new().fg(BORDER)));
        let total = (width + 1) as u16;
        app.areas.tabs.push((
            Rect::new(x, area.y.saturating_add(2), total, 1).intersection(area),
            *which,
        ));
        x = x.saturating_add(total);
    }
    let used = x.saturating_sub(area.x) as usize;
    rule.push(Span::styled(
        "─".repeat((area.width as usize).saturating_sub(used)),
        Style::new().fg(BORDER),
    ));

    frame.render_widget(
        Paragraph::new(vec![
            title,
            Line::raw(""),
            Line::from(tabs),
            Line::from(rule),
        ]),
        area,
    );
}

fn render_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let root = app.root.clone();
    let lang = app.lang;
    let t = lang.t();
    let selected = app.selected;
    let inner_width = usize::from(panel("").inner(area).width);
    let visible = app.visible();
    // Columns are as wide as their widest cell (capped), so rows line up.
    let name_width = visible
        .iter()
        .map(|a| a.algorithm_name.chars().count())
        .max()
        .unwrap_or(0)
        .min(22);
    let purpose_width = visible
        .iter()
        .map(|a| lang.purpose(a.finding.purpose).chars().count())
        .max()
        .unwrap_or(0);
    // bar (2) + badge (6) + gaps; whatever is left goes to the location.
    let used = 2 + 6 + 1 + name_width + 2 + purpose_width + 2;
    let location_width = inner_width.saturating_sub(used);
    let items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(index, a)| {
            let on = index == selected;
            let finding = &a.finding;
            let location = finding
                .file
                .strip_prefix(&root)
                .unwrap_or(&finding.file)
                .display();
            let mut spans = vec![
                row_bar(on),
                badge_for(a.urgency, lang, on),
                Span::raw(" "),
                Span::styled(
                    format!(
                        "{:<name_width$}",
                        a.algorithm_name
                            .chars()
                            .take(name_width)
                            .collect::<String>()
                    ),
                    Style::new()
                        .fg(if on { ACCENT } else { FG })
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {:<purpose_width$}", lang.purpose(finding.purpose)),
                    Style::new().fg(MUTED),
                ),
            ];
            if location_width >= 12 {
                let place = format!("{location}:{}", finding.line);
                spans.push(Span::raw("  "));
                spans.extend(super::overview::path_spans(&place, location_width));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = fill(t.findings_title, &[("n", &items.len())]);
    app.areas.list = panel(&title).inner(area);
    if items.is_empty() {
        let hidden = app.hidden();
        let message = if hidden > 0 {
            fill(t.hidden_by_filters, &[("n", &hidden)])
        } else {
            t.none_found.to_string()
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
        .block(focused_panel(&title))
        .highlight_symbol(selected_symbol());
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
    Line::from(vec![
        Span::styled("── ", Style::new().fg(DIM)),
        Span::styled(
            text.to_string(),
            Style::new().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" ──", Style::new().fg(DIM)),
    ])
}

fn bullet(text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("  • ", Style::new().fg(MUTED)),
        Span::styled(text.to_string(), Style::new().fg(FG)),
    ])
}

/// The lines of the details pane for one finding.
fn detail_lines(a: &Assessment, root: &str, lang: Lang) -> Vec<Line<'static>> {
    let t = lang.t();
    let finding = &a.finding;
    let location = finding.file.strip_prefix(root).unwrap_or(&finding.file);
    let mut lines = vec![
        Line::from(vec![
            badge(a.urgency, lang),
            Span::raw(" "),
            Span::styled(
                a.algorithm_name.clone(),
                Style::new().fg(FG).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("{}:{}:{}", location.display(), finding.line, finding.column),
            Style::new().fg(MUTED),
        )),
        Line::raw(""),
        field(
            t.label_urgency,
            Span::styled(
                lang.urgency(a.urgency),
                Style::new()
                    .fg(urgency_color(a.urgency))
                    .add_modifier(Modifier::BOLD),
            ),
        ),
        field(
            t.label_quantum,
            Span::styled(lang.risk_sentence(a.risk, &a.attack), Style::new().fg(FG)),
        ),
        field(
            t.label_purpose,
            Span::styled(lang.purpose(finding.purpose), Style::new().fg(FG)),
        ),
    ];
    if let Some(bits) = finding.key_size {
        lines.push(field(
            t.label_key_size,
            Span::styled(format!("{bits}-bit"), Style::new().fg(FG)),
        ));
    }
    lines.push(field(
        t.label_library,
        Span::styled(finding.library.clone(), Style::new().fg(FG)),
    ));
    lines.push(field(
        t.label_confidence,
        Span::styled(lang.confidence(finding.confidence), Style::new().fg(FG)),
    ));
    lines.push(Line::raw(""));
    lines.push(heading(t.h_code));
    lines.push(Line::from(vec![
        Span::styled("  ▎ ", Style::new().fg(ACCENT)),
        Span::styled(finding.snippet.clone(), Style::new().fg(FG)),
    ]));

    lines.push(Line::raw(""));
    lines.push(heading(t.h_why));
    lines.push(Line::from(Span::styled(
        a.explanation.clone(),
        Style::new().fg(FG),
    )));

    if !a.recommendations.is_empty() {
        lines.push(Line::raw(""));
        lines.push(heading(t.h_replace));
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
            lines.push(heading(t.h_implementations));
            for implementation in seen {
                lines.push(bullet(implementation));
            }
        }
    }
    if !a.advice.is_empty() {
        lines.push(Line::raw(""));
        lines.push(heading(t.h_advice));
        for advice in &a.advice {
            lines.push(bullet(advice));
        }
    }
    lines
}

fn render_details(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.lang.t();
    let block = panel(t.details_title);
    let inner = block.inner(area);
    app.areas.details = inner;
    let Some(assessment) = app.selected_assessment() else {
        let empty = Paragraph::new(t.nothing_selected)
            .style(Style::new().fg(MUTED))
            .block(block);
        frame.render_widget(empty, area);
        app.detail_max_scroll = 0;
        return;
    };

    let lines = detail_lines(assessment, &app.root, app.lang);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    // Measured without the border, so it is the exact number of rows once wrapped.
    let rows = u16::try_from(paragraph.line_count(inner.width)).unwrap_or(u16::MAX);
    app.detail_max_scroll = rows.saturating_sub(inner.height);
    let scroll = app.detail_scroll.min(app.detail_max_scroll);

    frame.render_widget(paragraph.scroll((scroll, 0)).block(block), area);
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let lang = app.lang;
    let t = lang.t();
    let counts = app.counts();
    let mut summary = vec![
        Span::styled(
            format!(" {} ", fill(t.footer_uses, &[("n", &counts.total)])),
            Style::new().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", fill(t.footer_broken, &[("n", &counts.broken)])),
            Style::new().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", fill(t.footer_weakened, &[("n", &counts.weakened)])),
            Style::new().fg(FG),
        ),
        Span::styled(
            fill(t.footer_safe, &[("n", &counts.safe)]),
            Style::new().fg(MUTED),
        ),
        Span::styled(
            format!(
                "   {}",
                fill(
                    t.footer_filters,
                    &[
                        ("safe", &lang.on_off(app.hide_safe)),
                        ("confidence", &lang.confidence(app.min_confidence)),
                    ]
                )
            ),
            Style::new().fg(MUTED),
        ),
    ];
    // A message about the last action replaces the totals until the next key,
    // so it is never pushed off the line by them.
    if app.scanning {
        summary = vec![Span::styled(
            format!(" {} {}", super::loading::frame(app.spinner), t.scanning),
            Style::new().fg(ACCENT),
        )];
    } else if let Some(status) = &app.status {
        summary = vec![Span::styled(format!(" {status}"), Style::new().fg(ACCENT))];
    }

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(summary),
            key_hints(&[
                ("↑↓", t.key_select),
                ("1 2", t.key_tabs),
                ("s", t.key_hide_safe),
                ("c", t.key_confidence),
                ("r", t.key_rescan),
                ("b", t.key_browse),
                ("m", t.key_menu),
                ("L", t.key_language),
                ("?", t.key_help),
                ("q", t.key_back),
            ]),
        ]),
        area,
    );
}

/// A line of `key  meaning` pairs, as shown at the bottom of every screen.
pub(super) fn key_hints(pairs: &[(&str, &str)]) -> Line<'static> {
    let mut spans = vec![Span::styled(" ", Style::new().fg(MUTED))];
    for (index, (key, meaning)) in pairs.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" • ", Style::new().fg(DIM)));
        }
        spans.push(Span::styled(key.to_string(), Style::new().fg(KEY)));
        spans.push(Span::styled(format!(" {meaning}"), Style::new().fg(DIM)));
    }
    Line::from(spans)
}

fn render_help(frame: &mut Frame, area: Rect, lang: Lang) {
    let t = lang.t();
    let rows = [
        ("↑ ↓  j k", t.help_select),
        ("g  G", t.help_first_last),
        ("PgUp  PgDn  space", t.help_scroll),
        ("s", t.help_hide_safe),
        ("c", t.help_confidence),
        ("r", t.help_rescan),
        ("b", t.help_browse),
        ("m", t.key_menu),
        ("1  2  Tab", t.help_tabs),
        ("mouse", t.help_mouse),
        ("L", t.help_language),
        ("?", t.help_close),
        ("q  Esc", t.help_quit),
    ];
    let mut lines = vec![Line::raw("")];
    for (keys, what) in rows {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {keys:<20}"),
                Style::new().fg(FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled(what, Style::new().fg(MUTED)),
        ]));
    }
    lines.push(Line::raw(""));
    for legend in [t.legend_high, t.legend_medium, t.legend_low, t.legend_none] {
        lines.push(Line::from(Span::styled(legend, Style::new().fg(MUTED))));
    }

    let width = 74.min(area.width);
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
            .block(focused_panel(t.help_title)),
        popup,
    );
}
