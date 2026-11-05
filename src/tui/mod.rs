//! Interactive terminal interface to browse the findings of an analysis.
//!
//! It follows the architecture of [Bubble Tea](https://github.com/charmbracelet/bubbletea),
//! itself taken from Elm:
//!
//! - [`Tui`] is the model, the whole state of the interface.
//! - [`Msg`] is anything that happened: a key, a mouse event, a tick of the
//!   clock, a finished scan.
//! - [`Tui::update`] is the only place the state changes. It takes a message
//!   and may ask for work by leaving [`Cmd`]s in an outbox.
//! - [`Tui::render`] draws the model and changes nothing else.
//! - The runtime in [`run`] reads input, performs the commands on background
//!   threads and feeds their results back as messages, so a slow scan never
//!   freezes the screen.

mod app;
mod browser;
mod loading;
mod overview;
mod ui;

pub use app::{App, Command, Counts, Tab};
pub use browser::Browser;

use crate::analysis::{Analysis, analyze_directory};
use crate::finding::Confidence;
use crate::i18n::Lang;
use crate::knowledge::KnowledgeBase;
use browser::Action;
use ratatui::DefaultTerminal;
use ratatui::Frame;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseEventKind,
};
use ratatui::crossterm::execute;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

/// Something that happened.
pub enum Msg {
    Key(KeyCode),
    Mouse {
        kind: MouseEventKind,
        column: u16,
        row: u16,
    },
    /// The user asked to leave from anywhere (Ctrl-C).
    Quit,
    /// Time passed while something was running; the spinner moves on.
    Tick,
    /// A background scan finished.
    Scanned {
        id: u64,
        path: PathBuf,
        /// The language the advice was written in.
        lang: Lang,
        analysis: Analysis,
    },
}

/// Work the runtime must do for the interface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// Analyze a folder in the background and report back with [`Msg::Scanned`].
    Scan { id: u64, path: PathBuf, lang: Lang },
}

/// The screen being shown.
enum Screen {
    /// Choosing the project to analyze.
    Browse(Browser),
    /// Waiting for the analysis of a folder.
    Loading(PathBuf),
    /// Looking at the findings of a project.
    Results(Box<App>),
}

/// The whole interface: which screen is up and the settings carried between them.
pub struct Tui {
    screen: Screen,
    hide_safe: bool,
    min_confidence: Confidence,
    lang: Lang,
    quit: bool,
    spinner: usize,
    next_scan: u64,
    /// The scan whose result is still wanted; older ones are ignored.
    pending: Option<u64>,
    /// Commands left by [`Tui::update`] for the runtime.
    outbox: Vec<Cmd>,
}

impl Tui {
    /// Starts by analyzing `start`, or on the folder browser when there is none.
    pub fn start(
        start: Option<&Path>,
        hide_safe: bool,
        min_confidence: Confidence,
        lang: Lang,
    ) -> Tui {
        let mut tui = Tui {
            screen: Screen::Browse(Browser::new(
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            )),
            hide_safe,
            min_confidence,
            lang,
            quit: false,
            spinner: 0,
            next_scan: 0,
            pending: None,
            outbox: Vec::new(),
        };
        if let Some(path) = start {
            tui.scan(path.to_path_buf(), false);
        }
        tui
    }

    pub fn should_quit(&self) -> bool {
        self.quit
    }

    /// Whether something is running in the background.
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Takes the commands the last updates asked for.
    pub fn drain_commands(&mut self) -> Vec<Cmd> {
        std::mem::take(&mut self.outbox)
    }

    /// Applies a message to the state.
    pub fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Key(code) => self.on_key(code),
            Msg::Mouse { kind, column, row } => self.on_mouse(kind, column, row),
            Msg::Quit => self.quit = true,
            Msg::Tick => self.spinner = self.spinner.wrapping_add(1),
            Msg::Scanned {
                id,
                path,
                lang,
                analysis,
            } => self.on_scanned(id, path, lang, analysis),
        }
    }

    /// Asks for an analysis of `path`. With `rescan` the current results stay
    /// on screen and are replaced when it finishes; otherwise a loading screen
    /// is shown.
    fn scan(&mut self, path: PathBuf, rescan: bool) {
        self.next_scan += 1;
        let id = self.next_scan;
        self.pending = Some(id);
        match &mut self.screen {
            Screen::Results(app) if rescan => app.scanning = true,
            _ => self.screen = Screen::Loading(path.clone()),
        }
        self.outbox.push(Cmd::Scan {
            id,
            path,
            lang: self.lang,
        });
    }

    fn on_scanned(&mut self, id: u64, path: PathBuf, scanned_in: Lang, analysis: Analysis) {
        if self.pending != Some(id) {
            return; // cancelled, or superseded by a newer scan
        }
        self.pending = None;
        if let Screen::Results(app) = &mut self.screen
            && app.scanning
        {
            app.scanning = false;
            app.replace(analysis);
            // The language may have been switched while it was running.
            app.set_lang(self.lang);
            return;
        }
        let mut app = App::new(
            path.display().to_string(),
            analysis,
            KnowledgeBase::embedded().version.clone(),
            self.hide_safe,
            self.min_confidence,
            scanned_in,
        );
        app.set_lang(self.lang);
        self.screen = Screen::Results(Box::new(app));
    }

    /// Goes back to choosing a project, on the folder that holds the current one.
    fn open_browser(&mut self, current: &Path) {
        self.pending = None;
        let parent = current.parent().filter(|p| !p.as_os_str().is_empty());
        let mut browser = Browser::new(parent.unwrap_or(current).to_path_buf());
        // Matching needs the same absolute form the browser lists.
        if let Ok(absolute) = std::fs::canonicalize(current) {
            browser.select_path(&absolute);
        }
        self.screen = Screen::Browse(browser);
    }

    fn on_key(&mut self, code: KeyCode) {
        match &mut self.screen {
            Screen::Browse(_) if code == KeyCode::Char('L') => self.lang = self.lang.other(),
            Screen::Browse(browser) => match browser.handle_key(code) {
                Some(Action::Analyze(path)) => self.scan(path, false),
                Some(Action::Quit) => self.quit = true,
                None => {}
            },
            Screen::Loading(path) => {
                let path = path.clone();
                match code {
                    KeyCode::Esc => self.open_browser(&path),
                    KeyCode::Char('q') => self.quit = true,
                    KeyCode::Char('L') => self.lang = self.lang.other(),
                    _ => {}
                }
            }
            Screen::Results(app) => {
                let command = app.handle_key(code);
                // The language can be switched from the results.
                self.lang = app.lang;
                if app.should_quit {
                    self.quit = true;
                    return;
                }
                let root = PathBuf::from(&app.root);
                match command {
                    Some(Command::Rescan) if !app.scanning => self.scan(root, true),
                    Some(Command::OpenBrowser) => self.open_browser(&root),
                    _ => {}
                }
            }
        }
    }

    fn on_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) {
        match &mut self.screen {
            Screen::Browse(browser) => match browser.handle_mouse(kind, column, row) {
                Some(Action::Analyze(path)) => self.scan(path, false),
                Some(Action::Quit) => self.quit = true,
                None => {}
            },
            Screen::Loading(_) => {}
            Screen::Results(app) => app.handle_mouse(kind, column, row),
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match &mut self.screen {
            Screen::Browse(browser) => browser::render(frame, browser, self.lang),
            Screen::Loading(path) => loading::render(frame, path, self.spinner, self.lang),
            Screen::Results(app) => {
                app.spinner = self.spinner;
                ui::render(frame, app)
            }
        }
    }
}

/// Does the work of a command on a background thread; the result comes back
/// through `tx` as a message.
fn perform(cmd: Cmd, tx: &Sender<Msg>) {
    match cmd {
        Cmd::Scan { id, path, lang } => {
            let tx = tx.clone();
            thread::spawn(move || {
                let analysis = analyze_directory(&path, lang);
                // The receiver is gone when the interface has already quit.
                let _ = tx.send(Msg::Scanned {
                    id,
                    path,
                    lang,
                    analysis,
                });
            });
        }
    }
}

/// Runs the interface until the user quits.
///
/// With `start` it opens analyzing that project; without it, on a browser to
/// pick the project.
pub fn run(
    start: Option<&Path>,
    hide_safe: bool,
    min_confidence: Confidence,
    lang: Lang,
) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "the interactive interface needs a terminal; use --format for piped output",
        ));
    }
    let mut tui = Tui::start(start, hide_safe, min_confidence, lang);
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

/// How long to wait for input while something runs, so the spinner keeps moving.
const BUSY_POLL: Duration = Duration::from_millis(80);
/// How long to wait for input when nothing runs.
const IDLE_POLL: Duration = Duration::from_millis(250);

fn event_loop(terminal: &mut DefaultTerminal, tui: &mut Tui) -> io::Result<()> {
    let (tx, rx) = mpsc::channel();
    while !tui.should_quit() {
        for cmd in tui.drain_commands() {
            perform(cmd, &tx);
        }
        terminal.draw(|frame| tui.render(frame))?;

        let wait = if tui.is_busy() { BUSY_POLL } else { IDLE_POLL };
        if event::poll(wait)? {
            // Resize and other events fall through and just trigger a redraw.
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c');
                    tui.update(if ctrl_c {
                        Msg::Quit
                    } else {
                        Msg::Key(key.code)
                    });
                }
                Event::Mouse(mouse) => tui.update(Msg::Mouse {
                    kind: mouse.kind,
                    column: mouse.column,
                    row: mouse.row,
                }),
                _ => {}
            }
        } else if tui.is_busy() {
            tui.update(Msg::Tick);
        }
        while let Ok(msg) = rx.try_recv() {
            tui.update(msg);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
