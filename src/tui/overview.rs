//! The overview page: totals and rankings across all findings.

use super::app::App;
use super::ui::{BAR, BORDER, DIM, FG, MUTED, badge, panel, urgency_color};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::Path;

/// Width of the badge column plus the spaces around a bar.
const BADGE_WIDTH: usize = 7;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.lang.t();
    let counts = app.counts();
    let [stats, body] = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]).areas(area);

    // Totals as one strip: the number is what the eye should land on.
    let columns: [Rect; 4] = Layout::horizontal([Constraint::Fill(1); 4]).areas(stats);
    stat(frame, columns[0], t.ov_files, app.file_count, t.ov_scanned);
    stat(
        frame,
        columns[1],
        t.ov_vulnerable,
        counts.broken,
        t.ov_broken_by_shor,
    );
    stat(
        frame,
        columns[2],
        t.ov_weakened,
        counts.weakened,
        t.ov_weakened_by_grover,
    );
    stat(frame, columns[3], t.ov_safe, counts.safe, t.ov_no_migration);

    let [left, right] = Layout::horizontal([Constraint::Percentage(50); 2]).areas(body);
    let [urgency_area, algorithms_area] =
        Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]).areas(left);
    let languages_height = (app.languages().len().max(1) as u16 + 2).min(right.height / 2);
    let [files_area, languages_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(languages_height)]).areas(right);
    render_urgency(frame, urgency_area, app);
    render_algorithms(frame, algorithms_area, app);
    render_files(frame, files_area, app);
    render_languages(frame, languages_area, app);
}

/// One total: a colored rule at the left, the label, the number, a caption.
fn stat(frame: &mut Frame, area: Rect, title: &str, value: usize, caption: &str) {
    let rule = Span::styled("┃ ", Style::new().fg(BORDER));
    let lines = vec![
        Line::from(vec![
            rule.clone(),
            Span::styled(title.to_string(), Style::new().fg(MUTED)),
        ]),
        Line::from(vec![
            rule.clone(),
            Span::styled(
                value.to_string(),
                Style::new().fg(FG).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            rule,
            Span::styled(caption.to_string(), Style::new().fg(DIM)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
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

/// A bar with its empty track behind it, so the scale is always visible.
fn track(count: usize, max: usize, width: usize, color: Color) -> Vec<Span<'static>> {
    let filled = bar(count, max, width);
    let rest = width.saturating_sub(filled.chars().count());
    vec![
        Span::styled(filled, Style::new().fg(color)),
        Span::styled("░".repeat(rest), Style::new().fg(DIM)),
    ]
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

/// A path padded to `width`, with the folders dimmed and the file name bright.
pub(super) fn path_spans(path: &str, width: usize) -> Vec<Span<'static>> {
    let shown = fit_left(path, width);
    let pad = width.saturating_sub(shown.chars().count());
    let (dir, file) = match shown.rfind('/') {
        Some(at) => shown.split_at(at + 1),
        None => ("", shown.as_str()),
    };
    vec![
        Span::styled(dir.to_string(), Style::new().fg(MUTED)),
        Span::styled(file.to_string(), Style::new().fg(FG)),
        Span::raw(" ".repeat(pad)),
    ]
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
    let total: usize = counts.iter().map(|(_, count)| *count).sum();
    let width = usize::from(inner.width).saturating_sub(BADGE_WIDTH + 8);
    for (row, (urgency, _)) in (0u16..).zip(counts.iter()) {
        if row < inner.height {
            app.areas
                .urgency_rows
                .push((Rect::new(inner.x, inner.y + row, inner.width, 1), *urgency));
        }
    }

    let lines: Vec<Line> = if total == 0 {
        empty(t.ov_none_to_show)
    } else {
        counts
            .iter()
            .map(|(urgency, count)| {
                let mut spans = vec![badge(*urgency, app.lang), Span::raw(" ")];
                spans.extend(track(*count, total, width, urgency_color(*urgency)));
                spans.push(Span::styled(
                    format!(" {count:>3}"),
                    Style::new().fg(FG).add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled(
                    format!(" {:>3}%", count * 100 / total),
                    Style::new().fg(MUTED),
                ));
                Line::from(spans)
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
    let bar_width = usize::from(inner.width).saturating_sub(name_width + 5);

    let lines: Vec<Line> = if ranked.is_empty() {
        empty(t.ov_nothing)
    } else {
        ranked
            .iter()
            .map(|(name, count)| {
                let mut spans = vec![Span::styled(
                    format!("{:<name_width$} ", fit_left(name, name_width)),
                    Style::new().fg(FG),
                )];
                spans.extend(track(*count, max, bar_width, BAR));
                spans.push(Span::styled(format!(" {count:>3}"), Style::new().fg(MUTED)));
                Line::from(spans)
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
                let mut spans = vec![badge(*urgency, app.lang), Span::raw(" ")];
                spans.extend(path_spans(&shown.display().to_string(), path_width));
                spans.push(Span::styled(format!(" {count:>3}"), Style::new().fg(MUTED)));
                Line::from(spans)
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
    let bar_width = usize::from(inner.width).saturating_sub(8 + 12);

    let lines: Vec<Line> = if languages.is_empty() {
        empty(t.ov_no_sources)
    } else {
        languages
            .iter()
            .map(|(label, count)| {
                let percent = *count as f64 * 100.0 / total as f64;
                let mut spans = vec![Span::styled(format!("{label:<8}"), Style::new().fg(FG))];
                spans.extend(track(*count, total, bar_width, BAR));
                spans.push(Span::styled(
                    format!(" {count} ({percent:.0}%)"),
                    Style::new().fg(MUTED),
                ));
                Line::from(spans)
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
