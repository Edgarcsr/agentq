//! Interactive terminal interface to browse the findings of an analysis.

mod app;
mod overview;
mod ui;

pub use app::{App, Command, Counts, Tab};

use crate::analysis::analyze_directory;
use crate::finding::Confidence;
use crate::knowledge::KnowledgeBase;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use std::io::{self, IsTerminal};
use std::path::Path;

/// Analyzes `root` and shows the findings until the user quits.
pub fn run(root: &Path, hide_safe: bool, min_confidence: Confidence) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "the interactive interface needs a terminal; use --format for piped output",
        ));
    }
    let mut app = App::new(
        root.display().to_string(),
        analyze_directory(root),
        KnowledgeBase::embedded().version.clone(),
        hide_safe,
        min_confidence,
    );
    let mut terminal = ratatui::try_init()?;
    let result = event_loop(&mut terminal, &mut app, root);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, root: &Path) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, app))?;
        // Resize events fall through and simply trigger a redraw.
        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && let Some(Command::Rescan) = app.handle_key(key.code)
        {
            app.replace(analyze_directory(root));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
