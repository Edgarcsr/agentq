//! The screen shown while a folder is being analyzed in the background.

use super::ui::{ACCENT, FG, MUTED, panel};
use crate::i18n::{Lang, fill};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::path::Path;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// The spinner character for `tick`, which wraps around.
pub fn frame(tick: usize) -> &'static str {
    FRAMES[tick % FRAMES.len()]
}

pub fn render(frame_: &mut Frame, path: &Path, tick: usize, lang: Lang) {
    let t = lang.t();
    let area = frame_.area();
    let [row] = Layout::vertical([Constraint::Length(5)])
        .flex(Flex::Center)
        .areas(area);
    let [box_area] = Layout::horizontal([Constraint::Max(72)])
        .flex(Flex::Center)
        .areas(row);

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", frame(tick)),
                Style::new().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                fill(t.loading, &[("path", &path.display())]),
                Style::new().fg(FG),
            ),
        ]),
        Line::raw(""),
        Line::from(Span::styled(t.loading_hint, Style::new().fg(MUTED))),
    ];
    frame_.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(panel("agentq")),
        box_area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spinner_cycles_through_its_frames() {
        assert_eq!(frame(0), "⠋");
        assert_eq!(frame(9), "⠏");
        assert_eq!(frame(10), frame(0));
        assert_ne!(frame(0), frame(1));
    }
}
