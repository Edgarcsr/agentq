//! Interactive terminal interface to browse the findings of an analysis.

mod app;
mod overview;
mod ui;

pub use app::{App, Command, Counts, Tab};

use crate::analysis::analyze_directory;
use crate::finding::Confidence;
use crate::knowledge::KnowledgeBase;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind,
};
use ratatui::crossterm::execute;
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
    // Turn mouse reporting off again even if the program panics.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        previous_hook(info);
    }));
    let result = execute!(io::stdout(), EnableMouseCapture)
        .and_then(|()| event_loop(&mut terminal, &mut app, root));
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, root: &Path) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, app))?;
        // Resize events fall through and simply trigger a redraw.
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if let Some(Command::Rescan) = app.handle_key(key.code) {
                    app.replace(analyze_directory(root));
                }
            }
            Event::Mouse(mouse) => app.handle_mouse(mouse.kind, mouse.column, mouse.row),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
