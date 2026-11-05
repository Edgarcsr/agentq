//! Interactive terminal interface to browse the findings of an analysis.

mod app;
mod browser;
mod overview;
mod ui;

pub use app::{App, Command, Counts, Tab};
pub use browser::Browser;

use crate::analysis::analyze_directory;
use crate::finding::Confidence;
use crate::knowledge::KnowledgeBase;
use browser::Action;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind,
};
use ratatui::crossterm::execute;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

/// The screen being shown.
enum Screen {
    /// Choosing the project to analyze.
    Browse(Browser),
    /// Looking at the findings of a project.
    Results(Box<App>),
}

/// The whole interface: which screen is up and the settings carried between them.
pub struct Tui {
    screen: Screen,
    hide_safe: bool,
    min_confidence: Confidence,
    quit: bool,
}

impl Tui {
    /// Starts on the results for `start`, or on the folder browser when there is none.
    pub fn start(start: Option<&Path>, hide_safe: bool, min_confidence: Confidence) -> Tui {
        let mut tui = Tui {
            screen: Screen::Browse(Browser::new(
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            )),
            hide_safe,
            min_confidence,
            quit: false,
        };
        if let Some(path) = start {
            tui.open_results(path);
        }
        tui
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    fn open_results(&mut self, path: &Path) {
        let app = App::new(
            path.display().to_string(),
            analyze_directory(path),
            KnowledgeBase::embedded().version.clone(),
            self.hide_safe,
            self.min_confidence,
        );
        self.screen = Screen::Results(Box::new(app));
    }

    /// Goes back to choosing a project, on the folder that holds the current one.
    fn open_browser(&mut self, current: &Path) {
        let parent = current.parent().filter(|p| !p.as_os_str().is_empty());
        let mut browser = Browser::new(parent.unwrap_or(current).to_path_buf());
        // Matching needs the same absolute form the browser lists.
        if let Ok(absolute) = std::fs::canonicalize(current) {
            browser.select_path(&absolute);
        }
        self.screen = Screen::Browse(browser);
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        match &mut self.screen {
            Screen::Browse(browser) => match browser.handle_key(code) {
                Some(Action::Analyze(path)) => self.open_results(&path),
                Some(Action::Quit) => self.quit = true,
                None => {}
            },
            Screen::Results(app) => {
                let command = app.handle_key(code);
                if app.should_quit {
                    self.quit = true;
                    return;
                }
                let root = PathBuf::from(&app.root);
                match command {
                    Some(Command::Rescan) => app.replace(analyze_directory(&root)),
                    Some(Command::OpenBrowser) => self.open_browser(&root),
                    None => {}
                }
            }
        }
    }

    pub fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) {
        match &mut self.screen {
            Screen::Browse(browser) => match browser.handle_mouse(kind, column, row) {
                Some(Action::Analyze(path)) => self.open_results(&path),
                Some(Action::Quit) => self.quit = true,
                None => {}
            },
            Screen::Results(app) => app.handle_mouse(kind, column, row),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match &mut self.screen {
            Screen::Browse(browser) => browser::render(frame, browser),
            Screen::Results(app) => ui::render(frame, app),
        }
    }
}

/// Runs the interface until the user quits.
///
/// With `start` it opens on that project's results; without it, on a browser
/// to pick the project.
pub fn run(start: Option<&Path>, hide_safe: bool, min_confidence: Confidence) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "the interactive interface needs a terminal; use --format for piped output",
        ));
    }
    let mut tui = Tui::start(start, hide_safe, min_confidence);
    let mut terminal = ratatui::try_init()?;
    // Turn mouse reporting off again even if the program panics.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        previous_hook(info);
    }));
    let result = execute!(io::stdout(), EnableMouseCapture)
        .and_then(|()| event_loop(&mut terminal, &mut tui));
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, tui: &mut Tui) -> io::Result<()> {
    while !tui.should_quit() {
        terminal.draw(|frame| tui.render(frame))?;
        // Resize events fall through and simply trigger a redraw.
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => tui.handle_key(key.code),
            Event::Mouse(mouse) => tui.handle_mouse(mouse.kind, mouse.column, mouse.row),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
