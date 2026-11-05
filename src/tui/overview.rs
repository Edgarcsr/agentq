//! The overview page: totals and rankings across all findings.

use super::app::App;
use super::ui::{ACCENT, ACCENT_LIGHT, DANGER, FG, MUTED, SAFE, badge, panel, urgency_color};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::Path;

/// Width of the badge column plus the spaces around a bar.
const BADGE_WIDTH: usize = 7;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.lang.t();
    let counts = app.counts();
    let [cards, middle, bottom] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .areas(area);

    let columns: [Rect; 4] = Layout::horizontal([Constraint::Fill(1); 4]).areas(cards);
    card(
        frame,
        columns[0],
        t.ov_files,
        app.file_count,
        t.ov_scanned,
        FG,
    );
    card(
        frame,
        columns[1],
        t.ov_vulnerable,
        counts.broken,
        t.ov_broken_by_shor,
        DANGER,
    );
    card(
        frame,
        columns[2],
        t.ov_weakened,
        counts.weakened,
        t.ov_weakened_by_grover,
        ACCENT_LIGHT,
    );
    card(
        frame,
        columns[3],
        t.ov_safe,
        counts.safe,
        t.ov_no_migration,
        SAFE,
    );

    let [urgency_area, algorithms_area] =
        Layout::horizontal([Constraint::Percentage(50); 2]).areas(middle);
    render_urgency(frame, urgency_area, app);
    render_algorithms(frame, algorithms_area, app);

    let [files_area, languages_area] =
        Layout::horizontal([Constraint::Percentage(50); 2]).areas(bottom);
    render_files(frame, files_area, app);
    render_languages(frame, languages_area, app);
}

fn card(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: usize,
    caption: &str,
    color: ratatui::style::Color,
) {
    let lines = vec![
        Line::from(Span::styled(
            value.to_string(),
            Style::new().fg(color).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(caption.to_string(), Style::new().fg(MUTED))),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(panel(title)),
        area,
    );
}

/// A bar of `count` out of `max`, at most `width` cells wide; any non-zero
/// count gets at least one cell so it stays visible.
fn bar(count: usize, max: usize, width: usize) -> String {
    if count == 0 || max == 0 || width == 0 {
        return String::new();
    }
    let cells = (count * width / max).clamp(1, width);
    "█".repeat(cells)
}

/// Shortens `text` from the left so it fits `width` columns.
fn fit_left(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    let tail: String = chars[chars.len() - keep..].iter().collect();
    format!("…{tail}")
}

fn empty(message: &str) -> Vec<Line<'static>> {
    vec![Line::from(Span::styled(
        message.to_string(),
        Style::new().fg(MUTED),
    ))]
}

fn render_urgency(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.lang.t();
    let block = panel(t.ov_by_urgency);
    let inner = block.inner(area);
    let counts = app.urgency_counts();
    let max = counts.iter().map(|(_, count)| *count).max().unwrap_or(0);
    let width = usize::from(inner.width).saturating_sub(BADGE_WIDTH + 6);
    for (row, (urgency, _)) in (0u16..).zip(counts.iter()) {
        if row < inner.height {
            app.areas
                .urgency_rows
                .push((Rect::new(inner.x, inner.y + row, inner.width, 1), *urgency));
        }
    }

    let lines: Vec<Line> = if max == 0 {
        empty(t.ov_none_to_show)
    } else {
        counts
            .iter()
            .map(|(urgency, count)| {
                Line::from(vec![
                    badge(*urgency, app.lang),
                    Span::raw(" "),
                    Span::styled(
                        bar(*count, max, width),
                        Style::new().fg(urgency_color(*urgency)),
                    ),
                    Span::styled(format!(" {count}"), Style::new().fg(FG)),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_algorithms(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.lang.t();
    let block = panel(t.ov_top_algorithms);
    let inner = block.inner(area);
    let ranked = app.top_algorithms(inner.height.into());
    let max = ranked.first().map_or(0, |(_, count)| *count);
    let name_width = ranked
        .iter()
        .map(|(name, _)| name.chars().count())
        .max()
        .unwrap_or(0)
        .min(usize::from(inner.width) / 2);
    let bar_width = usize::from(inner.width).saturating_sub(name_width + 6);

    let lines: Vec<Line> = if ranked.is_empty() {
        empty(t.ov_nothing)
    } else {
        ranked
            .iter()
            .map(|(name, count)| {
                Line::from(vec![
                    Span::styled(
                        format!("{:<name_width$} ", fit_left(name, name_width)),
                        Style::new().fg(FG),
                    ),
                    Span::styled(bar(*count, max, bar_width), Style::new().fg(ACCENT)),
                    Span::styled(format!(" {count}"), Style::new().fg(FG)),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_files(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.lang.t();
    let block = panel(t.ov_review_first);
    let inner = block.inner(area);
    let files = app.top_files(inner.height.into());
    let path_width = usize::from(inner.width).saturating_sub(BADGE_WIDTH + 5);

    let lines: Vec<Line> = if files.is_empty() {
        empty(t.ov_nothing)
    } else {
        files
            .iter()
            .map(|(file, count, urgency)| {
                let shown = file.strip_prefix(Path::new(&app.root)).unwrap_or(file);
                Line::from(vec![
                    badge(*urgency, app.lang),
                    Span::raw(" "),
                    Span::styled(
                        format!(
                            "{:<path_width$}",
                            fit_left(&shown.display().to_string(), path_width)
                        ),
                        Style::new().fg(FG),
                    ),
                    Span::styled(format!(" {count:>2}"), Style::new().fg(MUTED)),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn render_languages(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.lang.t();
    let block = panel(t.ov_languages);
    let inner = block.inner(area);
    let languages = app.languages();
    let total: usize = languages.iter().map(|(_, count)| *count).sum();
    let max = languages.first().map_or(0, |(_, count)| *count);
    let bar_width = usize::from(inner.width).saturating_sub(8 + 12);

    let lines: Vec<Line> = if languages.is_empty() {
        empty(t.ov_no_sources)
    } else {
        languages
            .iter()
            .map(|(label, count)| {
                let percent = *count as f64 * 100.0 / total as f64;
                Line::from(vec![
                    Span::styled(format!("{label:<8}"), Style::new().fg(FG)),
                    Span::styled(bar(*count, max, bar_width), Style::new().fg(ACCENT_LIGHT)),
                    Span::styled(format!(" {count} ({percent:.0}%)"), Style::new().fg(MUTED)),
                ])
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines).block(block), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assess::Urgency;

    #[test]
    fn bars_scale_and_keep_small_counts_visible() {
        assert_eq!(bar(10, 10, 8).chars().count(), 8);
        assert_eq!(bar(5, 10, 8).chars().count(), 4);
        assert_eq!(bar(1, 100, 8).chars().count(), 1);
        assert_eq!(bar(0, 10, 8), "");
        assert_eq!(bar(3, 0, 8), "");
        assert_eq!(bar(3, 10, 0), "");
    }

    #[test]
    fn long_paths_are_cut_from_the_left() {
        assert_eq!(fit_left("src/auth/tokens.ts", 18), "src/auth/tokens.ts");
        assert_eq!(fit_left("src/auth/tokens.ts", 10), "…tokens.ts");
        assert_eq!(fit_left("abc", 1), "…");
    }

    #[test]
    fn urgency_colors_are_distinct() {
        let colors: Vec<_> = [Urgency::High, Urgency::Medium, Urgency::Low, Urgency::None]
            .iter()
            .map(|u| urgency_color(*u))
            .collect();
        for (i, a) in colors.iter().enumerate() {
            for b in &colors[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
