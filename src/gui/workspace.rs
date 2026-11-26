//! The window: a sidebar with the projects, and a main area that is either the
//! projects table (with scheduled scans) or one project, switched between a
//! dashboard and its findings by tabs. Layout follows the window width.

use crate::ui_text::Ui;
use agentq::analysis::{Analysis, analyze_directory, is_analyzed};
use agentq::assess::{Assessment, Urgency, relocalize};
use agentq::config::Settings;
use agentq::detect::Language;
use agentq::finding::QuantumRisk;
use agentq::i18n::Lang;
use agentq::knowledge::KnowledgeBase;
use agentq::recents::{Kind, Recent, Recents, Summary, project_name};
use agentq::schedule::{Repeat, Schedules, next_at};
use agentq::snapshot::Snapshot;
use chrono::Timelike;
use gpui_kit::assets::IconName as Lucide;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::chart::PieChart;
use gpui_kit::component::notification::{Notification, NotificationType};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::time_field::{HourCycle, TimeField, TimeFieldState};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{
    ActiveTheme, Disableable, Icon, IconName, ResizableState, Sizable, StyledExt, WindowExt,
    h_flex, h_resizable, resizable_panel, v_flex,
};
use gpui_kit::{
    Anchor, Animation, AnimationExt, AnyElement, App, AppContext, Context, Hsla,
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Render,
    StatefulInteractiveElement, Styled, Window, WindowControlArea, div, ease_in_out, hsla, img,
    prelude::FluentBuilder as _, px, radians, relative,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SIDEBAR_W: f32 = 212.;

// ---- model ----------------------------------------------------------------

/// Where a project is in its life: waiting for a scan, being scanned, or done.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Pending,
    Running,
    Done,
}

enum Scan {
    Running,
    Done {
        summary: Summary,
        /// Most urgent first.
        findings: Vec<Assessment>,
        /// Source languages found, most files first.
        languages: Vec<(Language, usize)>,
    },
}

struct Project {
    path: PathBuf,
    scan: Scan,
    selected: usize,
    /// When the analysis on screen was made; `None` while it is the first one.
    analyzed_at: Option<u64>,
    /// When the scan under way started, to tell the user how long ago.
    scan_started: Option<u64>,
}

impl Project {
    fn status(&self) -> Status {
        match self.scan {
            Scan::Running => Status::Running,
            Scan::Done { .. } => Status::Done,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Projects,
    Dashboard,
    Findings,
}

/// The "schedule a scan" dialog while it is open.
struct Draft {
    path: Option<PathBuf>,
    /// The time of day the user types in.
    time: gpui_kit::Entity<TimeFieldState>,
    repeat: Repeat,
    /// The list of projects is open, to pick another one.
    changing: bool,
    /// Redraws the dialog while the time is being typed, so the summary of
    /// when it will run stays right.
    _watch: gpui_kit::Subscription,
}

pub struct Workspace {
    open: Vec<Project>,
    active: Option<usize>,
    page: Page,
    recents: Recents,
    schedules: Schedules,
    /// When each saved analysis of a project was made, newest first.
    versions: HashMap<PathBuf, Vec<u64>>,
    /// Projects whose dates are unfolded in the sidebar.
    expanded: HashSet<PathBuf>,
    /// How many times each project was folded or unfolded; the arrow only
    /// animates after the first, so nothing moves when the app starts.
    fold_turns: HashMap<PathBuf, u32>,
    draft: Option<Draft>,
    lang_menu: bool,
    /// How the findings list and its details share the width.
    split: gpui_kit::Entity<ResizableState>,
    kinds: HashMap<PathBuf, Kind>,
    /// Toasts waiting for the next frame, which has a window to show them in.
    toasts: Vec<(NotificationType, String, String)>,
    lang: Lang,
}

/// The colours the pages draw with, copied out of the theme so they can be
/// used while the view is borrowed for event listeners.
#[derive(Clone, Copy)]
struct Pal {
    bg: Hsla,
    sidebar: Hsla,
    fg: Hsla,
    muted: Hsla,
    border: Hsla,
    hover: Hsla,
    active: Hsla,
    red: Hsla,
    orange: Hsla,
    yellow: Hsla,
    green: Hsla,
    soft: Hsla,
    ui: &'static Ui,
    lang: Lang,
}

impl Pal {
    fn new(cx: &App, lang: Lang) -> Pal {
        let t = cx.theme();
        Pal {
            bg: t.background,
            sidebar: t.sidebar,
            fg: t.foreground,
            muted: t.muted_foreground,
            border: t.border,
            hover: t.accent,
            active: t.list_active,
            red: hsla(0. / 360., 0.78, 0.60, 1.),
            orange: hsla(25. / 360., 0.92, 0.56, 1.),
            yellow: hsla(48. / 360., 0.92, 0.55, 1.),
            green: hsla(152. / 360., 0.50, 0.52, 1.),
            soft: hsla(0., 0., 0.93, 1.),
            ui: Ui::of(lang),
            lang,
        }
    }

    /// Red, orange, yellow, green: how soon a use should be migrated.
    fn urgency(&self, urgency: Urgency) -> Hsla {
        match urgency {
            Urgency::High => self.red,
            Urgency::Medium => self.orange,
            Urgency::Low => self.yellow,
            Urgency::None => self.green,
        }
    }
}

// ---- small pieces ---------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn dot(color: Hsla) -> impl IntoElement {
    div().size(px(8.)).flex_shrink_0().rounded_full().bg(color)
}

/// The icon of a project's kind, drawn in its own colours.
fn kind_icon(kind: Kind) -> impl IntoElement {
    let file = match kind {
        Kind::Node => "node",
        Kind::TypeScript => "typescript",
        Kind::Python => "python",
        Kind::Go => "go",
        Kind::Rust => "rust",
        Kind::Java => "java",
        Kind::Ruby => "ruby",
        Kind::Php => "php",
        Kind::Folder => "folder",
    };
    img(format!("icons/{file}.svg"))
        .size(px(16.))
        .flex_shrink_0()
}

fn tip(text: String) -> impl Fn(&mut Window, &mut App) -> gpui_kit::AnyView + 'static {
    move |window, cx| Tooltip::new(text.clone()).build(window, cx)
}

/// Yellow dot for pending, the Lucide loader (spinning) for running, green
/// dot for done.
fn status_icon(status: Status, pal: Pal) -> AnyElement {
    match status {
        Status::Pending => dot(pal.yellow).into_any_element(),
        Status::Running => Spinner::new()
            .icon(IconName::Loader)
            .small()
            .color(pal.soft)
            .into_any_element(),
        Status::Done => dot(pal.green).into_any_element(),
    }
}

fn status_badge(status: Status, pal: Pal) -> AnyElement {
    let label = match status {
        Status::Pending => pal.ui.pending,
        Status::Running => pal.ui.running,
        Status::Done => pal.ui.done,
    };
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .w(px(16.))
                .flex()
                .justify_center()
                .child(status_icon(status, pal)),
        )
        .child(label)
        .into_any_element()
}

/// Coloured counts by urgency: high, medium, low, none.
fn issues(summary: Option<Summary>, pal: Pal) -> AnyElement {
    let Some(s) = summary else {
        return div().text_color(pal.muted).child("—").into_any_element();
    };
    let item = |color: Hsla, n: usize| {
        h_flex()
            .gap_1p5()
            .items_center()
            .child(dot(color))
            .child(div().text_color(pal.muted).child(n.to_string()))
    };
    h_flex()
        .gap_3()
        .child(item(pal.red, s.urgent))
        .child(item(pal.orange, s.medium))
        .child(item(pal.yellow, s.low))
        .child(item(pal.green, s.none))
        .into_any_element()
}

/// A titled block of the dashboard: no box, just a heading over content.
fn section(title: &'static str, pal: Pal, body: impl IntoElement) -> impl IntoElement {
    v_flex()
        .min_w_0()
        .gap_3()
        .child(
            div()
                .text_sm()
                .font_medium()
                .text_color(pal.muted)
                .child(title),
        )
        .child(body)
}

fn caption(text: &'static str, pal: Pal) -> impl IntoElement {
    div().text_xs().text_color(pal.muted).child(text)
}

fn rel(file: &Path, root: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string()
}

fn legend(color: Hsla, label: &str) -> impl IntoElement {
    h_flex()
        .gap_2()
        .items_center()
        .child(dot(color))
        .child(label.to_string())
}

fn empty(title: &'static str, text: &'static str, pal: Pal) -> AnyElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_1()
        .py_16()
        .child(div().font_semibold().child(title))
        .child(div().text_sm().text_color(pal.muted).child(text))
        .into_any_element()
}

fn running(project: &Project, pal: Pal) -> AnyElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap_3()
        .py_16()
        .text_color(pal.muted)
        .child(Spinner::new().icon(IconName::Loader).color(pal.soft))
        .child(format!("{} {}…", pal.ui.scanning, project.path.display()))
        .into_any_element()
}

/// How many findings fall in each urgency: high, medium, low, none.
type Mix = [usize; 4];

fn slot(urgency: Urgency) -> usize {
    match urgency {
        Urgency::High => 0,
        Urgency::Medium => 1,
        Urgency::Low => 2,
        Urgency::None => 3,
    }
}

/// Horizontal bars: label, a track whose fill is as long as the count and is
/// split by urgency, and the count.
fn bars(rows: Vec<(String, Mix)>, pal: Pal) -> AnyElement {
    let colors = [pal.red, pal.orange, pal.yellow, pal.green];
    let max = rows
        .iter()
        .map(|(_, mix)| mix.iter().sum::<usize>())
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    v_flex()
        .gap_3()
        .children(rows.into_iter().map(|(label, mix)| {
            let n: usize = mix.iter().sum();
            h_flex()
                .gap_3()
                .items_center()
                .text_sm()
                .child(div().w(px(112.)).flex_shrink_0().truncate().child(label))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(4.))
                        .rounded_full()
                        .bg(pal.border)
                        .child(
                            h_flex()
                                .h_full()
                                .w(relative(n as f32 / max))
                                .rounded_full()
                                .overflow_hidden()
                                // Green first, red last, so the red ends up on the right.
                                .children(
                                    mix.iter().enumerate().rev().filter(|(_, c)| **c > 0).map(
                                        |(i, c)| {
                                            div()
                                                .h_full()
                                                .bg(colors[i])
                                                .w(relative(*c as f32 / n as f32))
                                        },
                                    ),
                                ),
                        ),
                )
                .child(
                    div()
                        .w(px(22.))
                        .flex_shrink_0()
                        .text_right()
                        .text_color(pal.muted)
                        .child(n.to_string()),
                )
        }))
        .into_any_element()
}

/// The source languages found: one row each with its share of the files, and a
/// tag saying whether agentq analyzes it. The ones it does not are flagged,
/// since the findings above do not cover them.
fn languages_block(languages: &[(Language, usize)], narrow: bool, pal: Pal) -> AnyElement {
    if languages.is_empty() {
        return div()
            .text_sm()
            .text_color(pal.muted)
            .child(pal.ui.no_source_files)
            .into_any_element();
    }
    let total = languages.iter().map(|(_, n)| *n).sum::<usize>().max(1);
    let unsupported = languages.iter().any(|(l, _)| !is_analyzed(*l));

    let rows = languages.iter().map(|(language, count)| {
        let analyzed = is_analyzed(*language);
        let (name, icon) = match language {
            Language::JavaScriptTypeScript => ("JavaScript / TypeScript", "typescript"),
            Language::Python => ("Python", "python"),
            Language::Java => ("Java", "java"),
            Language::Go => ("Go", "go"),
        };
        let share = *count as f32 / total as f32;
        let tag = if analyzed {
            h_flex()
                .gap_1p5()
                .items_center()
                .text_xs()
                .text_color(pal.muted)
                .child(Icon::new(Lucide::Check).size_3p5())
                .child(pal.ui.analyzed)
                .into_any_element()
        } else {
            h_flex()
                .id(("language-tag", *language as usize))
                .gap_1p5()
                .px_2()
                .py_0p5()
                .items_center()
                .rounded_full()
                .bg(pal.yellow.opacity(0.14))
                .text_xs()
                .text_color(pal.yellow)
                .tooltip(tip(pal.ui.not_analyzed_hint.into()))
                .child(Icon::new(Lucide::TriangleAlert).size_3p5())
                .child(pal.ui.not_analyzed)
                .into_any_element()
        };
        h_flex()
            .h(px(44.))
            .gap_4()
            .items_center()
            .border_b_1()
            .border_color(pal.border)
            .text_sm()
            .child(
                img(format!("icons/{icon}.svg"))
                    .size(px(18.))
                    .flex_shrink_0(),
            )
            .child(
                div()
                    .w(px(if narrow { 120. } else { 180. }))
                    .flex_shrink_0()
                    .truncate()
                    .font_medium()
                    .child(name),
            )
            .when(!narrow, |s| {
                s.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(px(4.))
                        .rounded_full()
                        .bg(pal.border)
                        .child(
                            div()
                                .h_full()
                                .w(relative(share))
                                .rounded_full()
                                .bg(if analyzed { pal.soft } else { pal.muted }),
                        ),
                )
            })
            .when(narrow, |s| s.child(div().flex_1()))
            .child(
                div()
                    .w(px(84.))
                    .flex_shrink_0()
                    .text_right()
                    .text_color(pal.muted)
                    .child(pal.ui.files_count(*count)),
            )
            .child(div().flex_shrink_0().child(tag))
    });

    v_flex()
        .gap_1()
        .when(languages.len() > 1, |s| {
            s.child(
                div()
                    .pb_2()
                    .text_sm()
                    .text_color(pal.muted)
                    .child(pal.ui.languages_detected(languages.len())),
            )
        })
        .children(rows)
        .when(unsupported, |s| {
            s.child(
                div()
                    .pt_3()
                    .text_sm()
                    .text_color(pal.muted)
                    .child(pal.ui.coverage_note),
            )
        })
        .into_any_element()
}

fn chip(
    id: (&'static str, usize),
    label: &'static str,
    on: bool,
    pal: Pal,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(if on { pal.fg } else { pal.border })
        .text_sm()
        .cursor_pointer()
        .when(on, |s| s.bg(pal.fg).text_color(pal.bg))
        .when(!on, |s| s.text_color(pal.muted).hover(|s| s.bg(pal.hover)))
        .child(label)
}

fn details(a: &Assessment, stacked: bool, pal: Pal) -> AnyElement {
    let risk = match a.risk {
        QuantumRisk::Broken => pal.ui.broken_by_quantum.to_string(),
        QuantumRisk::Weakened => pal.ui.weakened.to_lowercase(),
        QuantumRisk::Safe => pal.ui.safe.to_lowercase(),
    };
    v_flex()
        .id("details")
        .when(!stacked, |s| s.size_full())
        .when(stacked, |s| s.w_full().flex_1().min_h_0().border_t_1())
        .border_color(pal.border)
        .overflow_y_scroll()
        .gap_4()
        .p_6()
        .text_sm()
        .child(
            div()
                .text_base()
                .font_semibold()
                .child(a.algorithm_name.clone()),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .text_color(pal.muted)
                .child(dot(pal.urgency(a.urgency)))
                .child(format!(
                    "{} · {} · {risk}",
                    a.finding.library,
                    pal.lang.purpose(a.finding.purpose)
                )),
        )
        .child(
            div()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(pal.border)
                .font_family("Menlo")
                .text_xs()
                .overflow_x_hidden()
                .child(a.finding.snippet.clone()),
        )
        .child(caption(pal.ui.why_it_matters, pal))
        .child(a.explanation.clone())
        .when(!a.recommendations.is_empty(), |s| {
            s.child(caption(pal.ui.alternatives, pal)).children(
                a.recommendations.iter().map(|r| {
                    div().child(format!("• {} ({})", r.name, r.implementations.join(", ")))
                }),
            )
        })
        .when(!a.advice.is_empty(), |s| {
            s.child(caption(pal.ui.advice, pal))
                .children(a.advice.iter().map(|t| div().child(format!("• {t}"))))
        })
        .into_any_element()
}

// ---- behaviour ------------------------------------------------------------

impl Workspace {
    pub fn new(initial: Option<PathBuf>, cx: &mut Context<Self>) -> Workspace {
        let lang = agentq::config::Settings::load()
            .lang()
            .unwrap_or_else(Lang::detect);
        let mut workspace = Workspace {
            open: Vec::new(),
            active: None,
            page: Page::Projects,
            recents: Recents::load(),
            schedules: Schedules::load(),
            versions: HashMap::new(),
            expanded: HashSet::new(),
            fold_turns: HashMap::new(),
            draft: None,
            lang_menu: false,
            split: cx.new(|_| ResizableState::default()),
            kinds: HashMap::new(),
            toasts: Vec::new(),
            lang,
        };
        for recent in &workspace.recents.projects {
            let found = Snapshot::versions(&recent.path);
            workspace.versions.insert(recent.path.clone(), found);
        }
        // Lists saved by older versions follow the order of opening.
        workspace.sort_history();
        let _ = workspace.recents.save();
        if let Some(path) = initial {
            workspace.open_project(path, cx);
        }
        workspace.run_due(cx);

        // Look for scheduled scans that came due.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(15))
                    .await;
                if this.update(cx, |this, cx| this.run_due(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        workspace
    }

    fn toast(&mut self, kind: NotificationType, title: impl Into<String>, text: impl Into<String>) {
        self.toasts.push((kind, title.into(), text.into()));
    }

    fn kind_of(&self, path: &Path) -> Kind {
        self.kinds.get(path).copied().unwrap_or(Kind::Folder)
    }

    /// Opens `path` (or focuses it) and scans it.
    fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let path = path.canonicalize().unwrap_or(path);
        self.page = Page::Dashboard;
        if let Some(index) = self.open.iter().position(|p| p.path == path) {
            self.active = Some(index);
            // It may be showing an older date: the project opens on the latest.
            let latest = self.dates_of(&path).first().copied();
            let project = &self.open[index];
            if let Some(latest) = latest
                && project.status() == Status::Done
                && project.analyzed_at != Some(latest)
            {
                self.view_snapshot(path, latest, cx);
            }
            cx.notify();
            return;
        }
        // Opening a project does not move it in the history: the list follows
        // the dates of the analyses.
        self.recents.add(&path);
        let _ = self.recents.save();
        // What was found the last time is shown at once; scanning again is
        // up to the user.
        let saved = Snapshot::load(&path);
        let (scan, analyzed_at) = match saved {
            Some(snap) => (
                Scan::Done {
                    summary: snap.summary,
                    findings: self.in_current_language(snap.findings, &snap.lang),
                    languages: snap.languages,
                },
                Some(snap.taken_at),
            ),
            None => (Scan::Running, None),
        };
        let needs_scan = matches!(scan, Scan::Running);
        self.open.push(Project {
            path: path.clone(),
            scan,
            selected: 0,
            analyzed_at,
            scan_started: None,
        });
        self.active = Some(self.open.len() - 1);
        if needs_scan {
            self.scan(path, cx);
        }
        cx.notify();
    }

    /// The advice of a saved analysis, in the language of the interface. It is
    /// written again from the findings, which takes no time, so a language
    /// change never costs a scan.
    fn in_current_language(&self, findings: Vec<Assessment>, saved_in: &str) -> Vec<Assessment> {
        if saved_in == self.lang.code() {
            findings
        } else {
            relocalize(findings, KnowledgeBase::embedded(), self.lang)
        }
    }

    /// Starts every scheduled scan that is due, in the background, without
    /// changing what the user is looking at.
    fn run_due(&mut self, cx: &mut Context<Self>) {
        let now = now_secs();
        let due = self.schedules.due(now);
        for item in due {
            self.schedules.completed(item.id, now);
            if !item.path.is_dir() {
                self.toast(
                    NotificationType::Warning,
                    Ui::of(self.lang).toast_skipped,
                    format!(
                        "{} {}",
                        item.path.display(),
                        Ui::of(self.lang).toast_missing
                    ),
                );
                continue;
            }
            self.toast(
                NotificationType::Info,
                Ui::of(self.lang).toast_started,
                project_name(&item.path),
            );
            self.recents.add(&item.path);
            match self.open.iter().find(|p| p.path == item.path) {
                // Already being scanned: that run covers this one.
                Some(p) if p.status() == Status::Running => {}
                Some(_) => self.scan(item.path.clone(), cx),
                None => {
                    self.open.push(Project {
                        path: item.path.clone(),
                        scan: Scan::Running,
                        selected: 0,
                        analyzed_at: None,
                        scan_started: None,
                    });
                    self.scan(item.path.clone(), cx);
                }
            }
        }
        let _ = self.schedules.save();
        let _ = self.recents.save();
        cx.notify();
    }

    fn scan(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Some(project) = self.open.iter_mut().find(|p| p.path == path) {
            project.scan = Scan::Running;
            project.scan_started = Some(now_secs());
        }
        let lang = self.lang;
        let background = cx.background_spawn({
            let path = path.clone();
            async move { analyze_directory(&path, lang) }
        });
        cx.spawn(async move |this, cx| {
            let analysis = background.await;
            this.update(cx, |this, cx| this.finish_scan(&path, analysis, cx))
                .ok();
        })
        .detach();
        cx.notify();
    }

    fn finish_scan(&mut self, path: &Path, analysis: Analysis, cx: &mut Context<Self>) {
        let taken_at = now_secs();
        let snapshot = Snapshot::from_analysis(path, &analysis, self.lang, taken_at);
        let summary = snapshot.summary;
        // Keep this analysis: it is what the history shows next time.
        let _ = snapshot.save();
        self.versions
            .insert(path.to_path_buf(), Snapshot::versions(path));
        if let Some(project) = self.open.iter_mut().find(|p| p.path == path) {
            project.scan = Scan::Done {
                summary,
                findings: snapshot.findings,
                languages: snapshot.languages,
            };
            project.selected = 0;
            project.analyzed_at = Some(taken_at);
        }
        // A new analysis is what puts a project at the top of the history.
        self.recents.touch(path);
        self.recents.record(path, summary);
        let _ = self.recents.save();
        let ui = Ui::of(self.lang);
        let (kind, title) = if summary.broken > 0 {
            (NotificationType::Warning, ui.toast_scan_risks)
        } else {
            (NotificationType::Success, ui.toast_scan_done)
        };
        self.toast(
            kind,
            title,
            format!(
                "{}: {} {} · {} {} · {} {}",
                project_name(path),
                summary.broken,
                ui.broken.to_lowercase(),
                summary.weakened,
                ui.weakened.to_lowercase(),
                summary.safe,
                ui.safe.to_lowercase()
            ),
        );
        cx.notify();
    }

    fn close_project(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.open.len() {
            return;
        }
        self.open.remove(index);
        self.active = match self.active {
            _ if self.open.is_empty() => None,
            Some(a) if a > index => Some(a - 1),
            Some(a) => Some(a.min(self.open.len() - 1)),
            None => None,
        };
        if self.active.is_none() {
            self.page = Page::Projects;
        }
        cx.notify();
    }

    fn pick_folder(
        &mut self,
        then: fn(&mut Self, PathBuf, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(Ui::of(self.lang).choose.into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = picked.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update(cx, |this, cx| then(this, path, cx)).ok();
            }
        })
        .detach();
    }

    /// Changes the language of the advice, remembers it, and scans the open
    /// projects again, since the advice is written during the scan.
    fn set_lang(&mut self, lang: Lang, cx: &mut Context<Self>) {
        if self.lang == lang {
            return;
        }
        self.lang = lang;
        let mut settings = Settings::load();
        settings.language = Some(lang.code().to_string());
        let _ = settings.save();
        // Only the words change, so the open analyses are written again in
        // the new language instead of being scanned again.
        for project in &mut self.open {
            if let Scan::Done { findings, .. } = &mut project.scan {
                *findings = relocalize(std::mem::take(findings), KnowledgeBase::embedded(), lang);
            }
        }
        self.toast(NotificationType::Info, Ui::of(lang).toast_language, "");
        cx.notify();
    }

    fn go(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    fn open_schedule_dialog(
        &mut self,
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Starts at the next full hour, which the user then edits.
        let start = chrono::Local::now() + chrono::Duration::hours(1);
        let start = chrono::NaiveTime::from_hms_opt(start.hour(), 0, 0).unwrap_or_default();
        let time = cx.new(|cx| {
            let mut state = TimeFieldState::new(window, cx).hour_cycle(HourCycle::H23);
            state.set_time(start, window, cx);
            state
        });
        let watch = cx.observe(&time, |_, _, cx| cx.notify());
        self.draft = Some(Draft {
            changing: path.is_none(),
            path,
            time,
            repeat: Repeat::Once,
            _watch: watch,
        });
        cx.notify();
    }

    fn confirm_schedule(&mut self, cx: &mut Context<Self>) {
        if let Some(Draft {
            path: Some(path),
            time,
            repeat,
            ..
        }) = self.draft.take()
        {
            let time = time.read(cx).time();
            let now = now_secs();
            let Some(at) = next_at(time.hour(), time.minute(), now) else {
                return;
            };
            self.schedules.add(&path, at, repeat);
            self.toast(
                NotificationType::Info,
                Ui::of(self.lang).toast_scheduled,
                format!(
                    "{} · {} · {}",
                    project_name(&path),
                    time.format("%H:%M"),
                    Ui::of(self.lang).repeat_label(repeat)
                ),
            );
            // Scheduled projects show up in the table even if never opened.
            self.recents.add(&path);
            let _ = self.schedules.save();
            let _ = self.recents.save();
        }
        cx.notify();
    }

    /// Drops one saved analysis of a project; the project and its other dates
    /// stay. If it was the one on screen, the latest one that is left takes
    /// its place.
    fn forget_analysis(&mut self, path: &Path, at: u64, cx: &mut Context<Self>) {
        Snapshot::remove_at(path, at);
        let left = Snapshot::versions(path);
        let viewing = self
            .open
            .iter()
            .any(|p| p.path == path && p.analyzed_at == Some(at));
        if left.len() < 2 {
            self.expanded.remove(path);
        }
        self.versions.insert(path.to_path_buf(), left.clone());
        self.sort_history();
        // The numbers in the list are those of the latest analysis left.
        if let Some(snap) = left.first().and_then(|at| Snapshot::load_at(path, *at)) {
            self.recents.record(path, snap.summary);
            let _ = self.recents.save();
        }
        if viewing && let Some(latest) = left.first() {
            self.view_snapshot(path.to_path_buf(), *latest, cx);
        }
        self.toast(
            NotificationType::Info,
            Ui::of(self.lang).toast_analysis_removed,
            Ui::of(self.lang).exact(at),
        );
        cx.notify();
    }

    /// Shows the analysis made at `at`, as it was, without scanning again.
    fn view_snapshot(&mut self, path: PathBuf, at: u64, cx: &mut Context<Self>) {
        let Some(snap) = Snapshot::load_at(&path, at) else {
            return;
        };
        self.page = Page::Dashboard;
        let scan = Scan::Done {
            summary: snap.summary,
            findings: self.in_current_language(snap.findings, &snap.lang),
            languages: snap.languages,
        };
        match self.open.iter().position(|p| p.path == path) {
            Some(index) => {
                let project = &mut self.open[index];
                project.scan = scan;
                project.selected = 0;
                project.analyzed_at = Some(at);
                self.active = Some(index);
            }
            None => {
                self.open.push(Project {
                    path,
                    scan,
                    selected: 0,
                    analyzed_at: Some(at),
                    scan_started: None,
                });
                self.active = Some(self.open.len() - 1);
            }
        }
        cx.notify();
    }

    /// Drops a project from the history, with any scans planned for it.
    fn forget_project(&mut self, path: &Path, cx: &mut Context<Self>) {
        let planned: Vec<u64> = self
            .schedules
            .items
            .iter()
            .filter(|s| s.path == path)
            .map(|s| s.id)
            .collect();
        for id in planned {
            self.schedules.cancel(id);
        }
        self.recents.remove(path);
        Snapshot::remove(path);
        self.versions.remove(path);
        self.expanded.remove(path);
        self.fold_turns.remove(path);
        if let Some(index) = self.open.iter().position(|p| p.path == path) {
            self.close_project(index, cx);
        }
        let _ = self.schedules.save();
        let _ = self.recents.save();
        self.toast(
            NotificationType::Info,
            Ui::of(self.lang).toast_project_removed,
            project_name(path),
        );
        cx.notify();
    }

    /// Status, last known numbers and the next planned run of a project.
    fn state_of(&self, recent: &Recent) -> (Status, Option<Summary>) {
        let running = self
            .open
            .iter()
            .any(|p| p.path == recent.path && p.status() == Status::Running);
        // The numbers of the latest analysis. An open project may be showing
        // an older date, so what it holds is only the fallback.
        let summary = recent.summary.or_else(|| {
            self.open
                .iter()
                .find_map(|p| match (&p.path == &recent.path, &p.scan) {
                    (true, Scan::Done { summary, .. }) => Some(*summary),
                    _ => None,
                })
        });
        let status = if running {
            Status::Running
        } else if self.schedules.next_for(&recent.path).is_some() || summary.is_none() {
            Status::Pending
        } else {
            Status::Done
        };
        (status, summary)
    }

    fn current(&self) -> Option<&Project> {
        self.active.and_then(|a| self.open.get(a))
    }

    // ---- sidebar -----------------------------------------------------------

    /// The date of the latest analysis of a project; for one that was never
    /// analyzed, when it was added.
    fn analysis_date(&self, recent: &Recent) -> u64 {
        self.dates_of(&recent.path)
            .first()
            .copied()
            .unwrap_or(recent.last_opened)
    }

    /// Puts the history in the order of the dates of the analyses, newest
    /// first. Projects with the same date keep their places.
    fn sort_history(&mut self) {
        let mut projects = std::mem::take(&mut self.recents.projects);
        projects.sort_by_key(|r| std::cmp::Reverse(self.analysis_date(r)));
        self.recents.projects = projects;
    }

    /// The saved analyses of a project, newest first.
    fn dates_of(&self, path: &Path) -> &[u64] {
        self.versions.get(path).map_or(&[], Vec::as_slice)
    }

    /// Opens or closes the list of dates under a project.
    fn toggle_fold(&mut self, path: &Path, cx: &mut Context<Self>) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_path_buf());
        }
        *self.fold_turns.entry(path.to_path_buf()).or_insert(0) += 1;
        cx.notify();
    }

    /// The arrow of a project that has more than one analysis. It points right
    /// when folded and turns down, smoothly, when the dates are shown.
    fn chevron(&self, i: usize, path: &Path, pal: Pal, cx: &mut Context<Self>) -> AnyElement {
        use std::f32::consts::FRAC_PI_2;
        let open = self.expanded.contains(path);
        let turns = self.fold_turns.get(path).copied().unwrap_or(0);
        let (from, to) = if open {
            (0., FRAC_PI_2)
        } else {
            (FRAC_PI_2, 0.)
        };
        let arrow = Icon::new(IconName::ChevronRight).size_4();
        let arrow = if turns == 0 {
            arrow.rotate(radians(to)).into_any_element()
        } else {
            // A new id for each turn makes the animation start again.
            arrow
                .with_animation(
                    ("fold-turn", turns as usize),
                    Animation::new(Duration::from_millis(160)).with_easing(ease_in_out),
                    move |icon, delta| icon.rotate(radians(from + (to - from) * delta)),
                )
                .into_any_element()
        };
        let path = path.to_path_buf();
        div()
            .id(("fold", i))
            .w(px(20.))
            .h(px(20.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .text_color(pal.muted)
            .hover(|s| s.bg(pal.active).text_color(pal.fg))
            .tooltip(tip(if open {
                pal.ui.hide_dates
            } else {
                pal.ui.show_dates
            }
            .to_string()))
            .child(arrow)
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.toggle_fold(&path, cx);
            }))
            .into_any_element()
    }

    /// One saved analysis of a project, listed under it by date.
    fn date_row(
        &self,
        key: usize,
        path: &Path,
        at: u64,
        pal: Pal,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let viewing = self.page != Page::Projects
            && self
                .current()
                .is_some_and(|p| p.path == path && p.analyzed_at == Some(at));
        let path = path.to_path_buf();
        let forget = (path.to_path_buf(), at);
        h_flex()
            .id(("date", key))
            .group("date-row")
            .gap_2p5()
            .pl_12()
            .pr_3()
            .h(px(26.))
            .rounded_md()
            .items_center()
            .cursor_pointer()
            .text_xs()
            .text_color(if viewing { pal.fg } else { pal.muted })
            .when(viewing, |s| s.bg(pal.active))
            .hover(|s| s.bg(pal.hover).text_color(pal.fg))
            .tooltip(tip(pal.ui.exact(at)))
            .child(div().flex_1().truncate().child(pal.ui.short_date(at)))
            .child(
                div()
                    .w(px(20.))
                    .flex_shrink_0()
                    .opacity(0.)
                    .group_hover("date-row", |s| s.opacity(1.))
                    .child(
                        Button::new(("forget-date", key))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip(pal.ui.remove_analysis)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.forget_analysis(&forget.0, forget.1, cx);
                            })),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.view_snapshot(path.clone(), at, cx);
            }))
            .into_any_element()
    }

    /// One project of the history: its kind, its name, and where its scan
    /// stands when it is open. The one being looked at is highlighted.
    fn history_row(
        &self,
        i: usize,
        r: &Recent,
        pal: Pal,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.open.iter().find(|p| p.path == r.path);
        let active =
            self.page != Page::Projects && self.current().is_some_and(|p| p.path == r.path);
        let status = open.map(|p| p.status());
        let hint = r.path.display().to_string();
        let (open_path, forget_path) = (r.path.clone(), r.path.clone());
        h_flex()
            .id(("history", i))
            .group("history-row")
            .gap_2p5()
            .pl_3()
            .pr_1()
            .h(px(30.))
            .rounded_md()
            .items_center()
            .cursor_pointer()
            .text_sm()
            .text_color(if active || open.is_some() {
                pal.fg
            } else {
                pal.muted
            })
            .when(active, |s| s.bg(pal.active))
            .hover(|s| s.bg(pal.hover).text_color(pal.fg))
            .tooltip(tip(hint))
            .child(kind_icon(self.kind_of(&r.path)))
            .child(div().flex_1().min_w_0().truncate().child(r.name()))
            // The last slot: the spinner while it scans (with when it
            // began on hover), the remove button otherwise.
            .child(if status == Some(Status::Running) {
                let started = open.and_then(|p| p.scan_started);
                let since = started.map(|at| now_secs().saturating_sub(at));
                let mut text = pal.ui.scanning_since(since);
                if let Some(at) = started {
                    text = format!("{text}\n{}", pal.ui.exact(at));
                }
                div()
                    .id(("scanning", i))
                    .w(px(20.))
                    .h(px(20.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .tooltip(tip(text))
                    .child(status_icon(Status::Running, pal))
                    .into_any_element()
            } else if self.dates_of(&r.path).len() > 1 {
                // A project with dates under it has no remove button of its
                // own: each date has one.
                self.chevron(i, &r.path, pal, cx)
            } else {
                div()
                    .w(px(20.))
                    .flex_shrink_0()
                    .opacity(0.)
                    .group_hover("history-row", |s| s.opacity(1.))
                    .child(
                        Button::new(("forget-side", i))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip(pal.ui.remove_project)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.forget_project(&forget_path, cx);
                            })),
                    )
                    .into_any_element()
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                // The row is the folder: it opens the latest analysis and
                // also folds or unfolds the dates beneath it.
                if this.dates_of(&open_path).len() > 1 {
                    this.toggle_fold(&open_path, cx);
                }
                this.open_project(open_path.clone(), cx);
            }))
    }

    fn sidebar(&self, pal: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let mut history_rows: Vec<AnyElement> = Vec::new();
        for (i, r) in self.recents.projects.iter().enumerate() {
            history_rows.push(self.history_row(i, r, pal, cx).into_any_element());
            if self.expanded.contains(&r.path) {
                for (n, at) in self.dates_of(&r.path).iter().enumerate() {
                    history_rows.push(self.date_row(i * 100 + n, &r.path, *at, pal, cx));
                }
            }
        }
        let selected = self.page == Page::Projects;

        v_flex()
            .w(px(SIDEBAR_W))
            .flex_shrink_0()
            .h_full()
            .bg(pal.sidebar)
            .border_r_1()
            .border_color(pal.border)
            // Room for the traffic lights; also drags the window.
            .child(
                div()
                    .h(px(44.))
                    .flex_shrink_0()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                div().px_2().child(
                    h_flex()
                        .id("nav-proj")
                        .gap_2p5()
                        .px_3()
                        .h(px(30.))
                        .rounded_md()
                        .items_center()
                        .cursor_pointer()
                        .text_sm()
                        .text_color(if selected { pal.fg } else { pal.muted })
                        .when(selected, |s| s.bg(pal.active))
                        .hover(|s| s.bg(pal.hover).text_color(pal.fg))
                        .tooltip(tip(pal.ui.projects_tip.into()))
                        .child(Icon::new(Lucide::Folders).size_4())
                        .child(div().flex_1().child(pal.ui.projects))
                        .child(
                            div()
                                .text_xs()
                                .child(self.recents.projects.len().to_string()),
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.go(Page::Projects, cx))),
                ),
            )
            .child(
                h_flex()
                    .pl_5()
                    .pr_3()
                    .pt_4()
                    .pb_1()
                    .items_center()
                    .child(div().flex_1().child(caption(pal.ui.open_caption, pal)))
                    .child(
                        Button::new("open-folder")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .tooltip(pal.ui.open_folder_tip)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pick_folder(Self::open_project, cx)
                            })),
                    ),
            )
            .child(
                v_flex()
                    .id("open-list")
                    .flex_1()
                    .min_h_0()
                    .px_2()
                    .gap_0p5()
                    .overflow_y_scroll()
                    .children(history_rows)
                    .when(self.recents.projects.is_empty(), |s| {
                        s.child(
                            div()
                                .px_3()
                                .text_xs()
                                .text_color(pal.muted)
                                .child(pal.ui.nothing_open),
                        )
                    }),
            )
            .child(self.language_bar(pal, cx))
    }

    /// Footer row that opens the language menu, drawn beside the sidebar.
    fn language_bar(&self, pal: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.lang_menu;
        div()
            .px_2()
            .py_2()
            .border_t_1()
            .border_color(pal.border)
            .child(
                h_flex()
                    .id("lang-row")
                    .gap_2p5()
                    .px_3()
                    .h(px(30.))
                    .rounded_md()
                    .items_center()
                    .cursor_pointer()
                    .text_sm()
                    .text_color(if open { pal.fg } else { pal.muted })
                    .when(open, |s| s.bg(pal.active))
                    .hover(|s| s.bg(pal.hover).text_color(pal.fg))
                    .tooltip(tip(pal.ui.language_tip.into()))
                    .child(Icon::new(Lucide::Languages).size_4())
                    .child(div().flex_1().child(pal.ui.languages))
                    .child(Icon::new(Lucide::ChevronRight).size_4())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.lang_menu = !this.lang_menu;
                        cx.notify();
                    })),
            )
    }

    /// The language menu: a small panel to the right of the sidebar, level
    /// with the footer row, and a transparent layer that closes it.
    fn language_menu(&self, pal: Pal, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.lang_menu {
            return None;
        }
        let item = |id: &'static str, label: &'static str, lang: Lang| {
            let on = self.lang == lang;
            h_flex()
                .id(id)
                .gap_2()
                .px_3()
                .h(px(30.))
                .rounded_md()
                .items_center()
                .cursor_pointer()
                .text_sm()
                .hover(|s| s.bg(pal.hover))
                .child(
                    div()
                        .w(px(16.))
                        .flex_shrink_0()
                        .when(on, |s| s.child(Icon::new(Lucide::Check).size_4())),
                )
                .child(div().flex_1().child(label))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.lang_menu = false;
                    this.set_lang(lang, cx);
                    cx.notify();
                }))
        };
        Some(
            div()
                .absolute()
                .inset_0()
                .child(
                    div()
                        .id("lang-scrim")
                        .absolute()
                        .inset_0()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.lang_menu = false;
                            cx.notify();
                        })),
                )
                .child(
                    v_flex()
                        .id("lang-card")
                        .occlude()
                        .absolute()
                        .left(px(SIDEBAR_W + 4.))
                        .bottom(px(8.))
                        .w(px(184.))
                        .p_1()
                        .rounded_lg()
                        .border_1()
                        .border_color(pal.border)
                        .bg(pal.sidebar)
                        .shadow_lg()
                        .child(item("lang-en", "English", Lang::En))
                        .child(item("lang-pt", "Português", Lang::PtBr)),
                )
                .into_any_element(),
        )
    }

    // ---- header and tabs ---------------------------------------------------

    /// Dashboard / Findings switch of the open project.
    fn tabs(&self, pal: Pal, cx: &mut Context<Self>) -> impl IntoElement {
        let count = self.current().and_then(|p| match &p.scan {
            Scan::Done { summary, .. } => Some(summary.total()),
            Scan::Running => None,
        });
        let tab = |id: &'static str, label: &'static str, n: Option<usize>, page: Page| {
            let on = self.page == page;
            h_flex()
                .id(id)
                .gap_2()
                .h(px(38.))
                .px_1()
                .mr_5()
                .items_center()
                .cursor_pointer()
                .text_sm()
                .border_b_2()
                .border_color(if on {
                    pal.fg
                } else {
                    gpui_kit::transparent_black()
                })
                .text_color(if on { pal.fg } else { pal.muted })
                .hover(|s| s.text_color(pal.fg))
                .child(label)
                .when_some(n, |s, n| {
                    s.child(div().text_xs().text_color(pal.muted).child(n.to_string()))
                })
                .on_click(cx.listener(move |this, _, _, cx| this.go(page, cx)))
        };
        h_flex()
            .flex_shrink_0()
            .px_8()
            .border_b_1()
            .border_color(pal.border)
            .child(tab("tab-dash", pal.ui.dashboard, None, Page::Dashboard))
            .child(tab("tab-find", pal.ui.findings, count, Page::Findings))
    }

    fn header(&self, pal: Pal, compact: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let on_project = self.page != Page::Projects && self.current().is_some();
        let title = if on_project {
            self.current()
                .map_or_else(String::new, |p| project_name(&p.path))
        } else {
            pal.ui.projects.to_string()
        };
        let path = on_project
            .then(|| self.current().map(|p| p.path.clone()))
            .flatten();
        let rescan = path
            .clone()
            .filter(|_| self.current().is_some_and(|p| p.status() == Status::Done));
        let analyzed_at = on_project
            .then(|| self.current().and_then(|p| p.analyzed_at))
            .flatten();
        let analyzed = analyzed_at.map(|at| pal.ui.analyzed(now_secs().saturating_sub(at)));
        let analyzed_exact = analyzed_at.map(|at| pal.ui.exact(at));

        h_flex()
            .h(px(60.))
            .flex_shrink_0()
            .px_8()
            .gap_3()
            .items_center()
            .window_control_area(WindowControlArea::Drag)
            .child(
                div()
                    .flex_shrink_0()
                    .max_w(px(280.))
                    .truncate()
                    .text_base()
                    .font_semibold()
                    .child(title),
            )
            .child(
                div()
                    .id("header-path")
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .text_color(pal.muted)
                    .when_some(analyzed_exact, |s, text| s.tooltip(tip(text)))
                    .when(!compact, |s| {
                        let place = path
                            .clone()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        s.child(match &analyzed {
                            Some(when) => format!("{place} · {when}"),
                            None => place,
                        })
                    }),
            )
            .when_some(rescan, |s, p| {
                s.child(
                    Button::new("rescan")
                        .small()
                        .icon(IconName::Redo2)
                        .label(pal.ui.rescan)
                        .tooltip(pal.ui.rescan_tip)
                        .on_click(cx.listener(move |this, _, _, cx| this.scan(p.clone(), cx))),
                )
            })
            .child({
                let p = path.clone();
                Button::new("schedule")
                    .small()
                    .icon(Icon::new(Lucide::Hourglass))
                    .label(pal.ui.schedule)
                    .tooltip(pal.ui.schedule_tip)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_schedule_dialog(p.clone(), window, cx)
                    }))
            })
            .child(
                Button::new("open-folder-top")
                    .primary()
                    .small()
                    .icon(IconName::Plus)
                    .label(pal.ui.open_folder)
                    .tooltip(pal.ui.open_folder_tip)
                    .on_click(
                        cx.listener(|this, _, _, cx| this.pick_folder(Self::open_project, cx)),
                    ),
            )
    }

    // ---- pages ---------------------------------------------------------------

    fn projects_page(&self, pal: Pal, main_w: f32, cx: &mut Context<Self>) -> AnyElement {
        if self.recents.projects.is_empty() {
            return empty(pal.ui.no_projects, pal.ui.no_projects_hint, pal);
        }
        let wide = main_w >= 760.;
        let medium = main_w >= 560.;
        let now = now_secs();

        let rows = self
            .recents
            .projects
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let (status, summary) = self.state_of(r);
                let path = r.path.clone();
                let next = self.schedules.next_for(&r.path);
                // The date of the latest analysis, which is what the column
                // and the order of the list follow.
                let analyzed = self.analysis_date(r);
                // The text of the column and the exact moment it stands for.
                let (when, exact) = match (status, next) {
                    (Status::Running, _) => (pal.ui.now.to_string(), None),
                    (_, Some(n)) => (pal.ui.until(n.at, now), Some(pal.ui.exact(n.at))),
                    _ => (
                        pal.ui.ago(now.saturating_sub(analyzed)),
                        Some(pal.ui.exact(analyzed)),
                    ),
                };
                h_flex()
                    .id(("row", i))
                    .h(px(44.))
                    .px_4()
                    .gap_4()
                    .items_center()
                    .border_b_1()
                    .border_color(pal.border)
                    .cursor_pointer()
                    .hover(|s| s.bg(pal.hover))
                    .tooltip(tip(r.path.display().to_string()))
                    .child(
                        div()
                            .flex_shrink_0()
                            .child(kind_icon(self.kind_of(&r.path))),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_medium()
                            .child(r.name()),
                    )
                    .when(medium, |s| {
                        s.child(
                            div()
                                .w(px(110.))
                                .flex_shrink_0()
                                .child(status_badge(status, pal)),
                        )
                    })
                    .when(wide, |s| {
                        s.child(
                            div()
                                .w(px(170.))
                                .flex_shrink_0()
                                .child(issues(summary, pal)),
                        )
                    })
                    .child(
                        div()
                            .id(("when", i))
                            .w(px(90.))
                            .flex_shrink_0()
                            .whitespace_nowrap()
                            .text_right()
                            .text_sm()
                            .text_color(pal.muted)
                            .when_some(exact, |s, text| s.tooltip(tip(text)))
                            .child(when),
                    )
                    .child({
                        let path = r.path.clone();
                        Button::new(("forget", i))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip(pal.ui.remove_project)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.forget_project(&path, cx);
                            }))
                    })
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.open_project(path.clone(), cx)),
                    )
            })
            .collect::<Vec<_>>();

        let head = |label: &'static str, w: f32| {
            div()
                .w(px(w))
                .flex_shrink_0()
                .text_xs()
                .text_color(pal.muted)
                .child(label)
        };

        // Scheduled scans, soonest first.
        let scheduled = self
            .schedules
            .items
            .iter()
            .map(|s| {
                let id = s.id;
                h_flex()
                    .id(("sched", s.id as usize))
                    .h(px(44.))
                    .px_4()
                    .gap_3()
                    .items_center()
                    .border_b_1()
                    .border_color(pal.border)
                    .child(kind_icon(self.kind_of(&s.path)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .child(project_name(&s.path)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .flex_shrink_0()
                            .text_sm()
                            .text_color(pal.muted)
                            .child(dot(pal.yellow))
                            .child(format!(
                                "{} · {}",
                                pal.ui.until(s.at, now),
                                pal.ui.repeat_label(s.repeat)
                            )),
                    )
                    .child(
                        Button::new(("unsched", s.id as usize))
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .tooltip(pal.ui.cancel_scan)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.schedules.cancel(id);
                                let _ = this.schedules.save();
                                let title = Ui::of(this.lang).toast_removed;
                                this.toast(NotificationType::Info, title, "");
                                cx.notify();
                            })),
                    )
            })
            .collect::<Vec<_>>();

        v_flex()
            .px_8()
            .pt_2()
            .pb_8()
            .gap_8()
            .child(
                v_flex()
                    .child(
                        h_flex()
                            .h(px(36.))
                            .px_4()
                            .gap_4()
                            .items_center()
                            .border_b_1()
                            .border_color(pal.border)
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .text_color(pal.muted)
                                    .child(pal.ui.col_project),
                            )
                            .when(medium, |s| s.child(head(pal.ui.col_status, 110.)))
                            .when(wide, |s| s.child(head(pal.ui.col_issues, 170.)))
                            .child(
                                div()
                                    .w(px(90.))
                                    .flex_shrink_0()
                                    .text_right()
                                    .text_xs()
                                    .text_color(pal.muted)
                                    .child(pal.ui.col_when),
                            )
                            .child(div().w(px(24.)).flex_shrink_0()),
                    )
                    .children(rows)
                    .child(
                        h_flex()
                            .gap_4()
                            .px_4()
                            .pt_3()
                            .text_xs()
                            .text_color(pal.muted)
                            .child(legend(pal.red, pal.ui.urgency(Urgency::High)))
                            .child(legend(pal.orange, pal.ui.urgency(Urgency::Medium)))
                            .child(legend(pal.yellow, pal.ui.urgency(Urgency::Low)))
                            .child(legend(pal.green, pal.ui.urgency(Urgency::None))),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(pal.muted)
                                    .child(pal.ui.scheduled_scans),
                            )
                            .child(
                                Button::new("schedule-new")
                                    .ghost()
                                    .small()
                                    .icon(IconName::Plus)
                                    .label(pal.ui.new)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_schedule_dialog(None, window, cx)
                                    })),
                            ),
                    )
                    .children(scheduled)
                    .when(self.schedules.items.is_empty(), |s| {
                        s.child(
                            div()
                                .text_sm()
                                .text_color(pal.muted)
                                .child(pal.ui.nothing_scheduled),
                        )
                    }),
            )
            .into_any_element()
    }

    fn dashboard_page(&self, pal: Pal, main_w: f32) -> AnyElement {
        let Some(project) = self.current() else {
            return empty(pal.ui.no_project_open, pal.ui.pick_project, pal);
        };
        let Scan::Done {
            summary,
            findings,
            languages,
        } = &project.scan
        else {
            return running(project, pal);
        };
        let narrow = main_w < 640.;

        // The verdict first, in plain words.
        let verdict = if summary.total() == 0 {
            pal.ui.no_crypto.to_string()
        } else if summary.broken > 0 {
            pal.ui.verdict_broken(summary.broken)
        } else if summary.weakened > 0 {
            pal.ui.verdict_weakened(summary.weakened)
        } else {
            pal.ui.nothing_at_risk.to_string()
        };
        let detail = pal.ui.summary_line(
            summary.weakened,
            summary.safe,
            summary.files,
            summary.urgent,
        );

        // The split between broken, weakened and safe, as a ring.
        let slices = vec![
            (pal.ui.urgency(Urgency::None), summary.none, pal.green),
            (pal.ui.urgency(Urgency::Low), summary.low, pal.yellow),
            (pal.ui.urgency(Urgency::Medium), summary.medium, pal.orange),
            (pal.ui.urgency(Urgency::High), summary.urgent, pal.red),
        ];
        let donut = h_flex()
            .gap_5()
            .items_center()
            .child(
                div()
                    .relative()
                    .size(px(132.))
                    .flex_shrink_0()
                    .child(
                        PieChart::new(slices.clone())
                            .value(|d| d.1 as f32)
                            .color(|d| d.2)
                            .tooltip_name(|d| d.0.into())
                            .tooltip_value({
                                let ui = pal.ui;
                                move |d, _, pct| ui.tooltip_uses(d.1, pct).into()
                            })
                            .inner_radius(42.)
                            .outer_radius(62.)
                            .pad_angle(0.03),
                    )
                    .child(
                        v_flex()
                            .absolute()
                            .inset_0()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .child(summary.total().to_string()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(pal.muted)
                                    .child(pal.ui.uses_word),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_1p5()
                    .text_sm()
                    .children(slices.iter().map(|(l, n, c)| {
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(dot(*c))
                            .child(div().w(px(70.)).text_color(pal.muted).child(*l))
                            .child(n.to_string())
                    })),
            );

        let head = h_flex()
            .flex_wrap()
            .gap_x_10()
            .gap_y_6()
            .items_center()
            .justify_between()
            .child(
                v_flex()
                    .flex_1()
                    .min_w(px(260.))
                    .gap_2p5()
                    .child(div().text_xl().font_semibold().child(verdict))
                    .child(div().text_sm().text_color(pal.muted).child(detail)),
            )
            .when(summary.total() > 0, |s| s.child(donut));

        // Migrate first.
        let top = findings
            .iter()
            .filter(|a| a.urgency != Urgency::None)
            .take(6)
            .map(|a| {
                let file = rel(&a.finding.file, &project.path);
                h_flex()
                    .h(px(40.))
                    .gap_4()
                    .items_center()
                    .border_b_1()
                    .border_color(pal.border)
                    .text_sm()
                    .child(dot(pal.urgency(a.urgency)))
                    .child(
                        div()
                            .w(px(if narrow { 120. } else { 160. }))
                            .flex_shrink_0()
                            .truncate()
                            .child(a.algorithm_name.clone()),
                    )
                    .when(!narrow, |s| {
                        s.child(
                            div()
                                .w(px(130.))
                                .flex_shrink_0()
                                .truncate()
                                .text_color(pal.muted)
                                .child(pal.lang.purpose(a.finding.purpose)),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(pal.muted)
                            .child(format!("{file}:{}", a.finding.line)),
                    )
                    .child(
                        div()
                            .w(px(64.))
                            .flex_shrink_0()
                            .text_right()
                            .text_color(pal.muted)
                            .child(pal.ui.urgency(a.urgency)),
                    )
            })
            .collect::<Vec<_>>();
        let migrate = if top.is_empty() {
            div()
                .text_sm()
                .text_color(pal.muted)
                .child(pal.ui.nothing_urgent)
                .into_any_element()
        } else {
            v_flex().children(top).into_any_element()
        };

        // Where, what and why.
        let mut per_file: BTreeMap<String, Mix> = BTreeMap::new();
        let mut by_purpose: BTreeMap<&'static str, Mix> = BTreeMap::new();
        let mut by_algo: BTreeMap<String, Mix> = BTreeMap::new();
        for a in findings {
            let i = slot(a.urgency);
            per_file
                .entry(rel(&a.finding.file, &project.path))
                .or_default()[i] += 1;
            by_purpose
                .entry(pal.lang.purpose(a.finding.purpose))
                .or_default()[i] += 1;
            by_algo.entry(a.algorithm_name.clone()).or_default()[i] += 1;
        }
        let ranked = |map: Vec<(String, Mix)>, take: usize| {
            let mut rows = map;
            rows.sort_by_key(|(_, mix)| std::cmp::Reverse(mix.iter().sum::<usize>()));
            rows.truncate(take);
            rows
        };
        let file_bars = bars(ranked(per_file.into_iter().collect(), 6), pal);
        let algo_bars = bars(ranked(by_algo.into_iter().collect(), 6), pal);
        let purpose_bars = bars(
            ranked(
                by_purpose
                    .into_iter()
                    .map(|(l, m)| (l.to_string(), m))
                    .collect(),
                usize::MAX,
            ),
            pal,
        );

        let col = |c: AnyElement| div().flex_1().min_w(px(240.)).child(c);

        v_flex()
            .w_full()
            .min_w_0()
            .px_8()
            .pt_8()
            .pb_12()
            .gap_12()
            .child(head)
            .child(section(pal.ui.migrate_first, pal, migrate))
            .child(
                h_flex()
                    .flex_wrap()
                    .gap_x_8()
                    .gap_y_8()
                    .items_start()
                    .child(col(
                        section(pal.ui.algorithms, pal, algo_bars).into_any_element()
                    ))
                    .child(col(
                        section(pal.ui.purposes, pal, purpose_bars).into_any_element()
                    ))
                    .child(col(section(pal.ui.files, pal, file_bars).into_any_element())),
            )
            .child(section(
                pal.ui.languages_title,
                pal,
                languages_block(languages, narrow, pal),
            ))
            .into_any_element()
    }

    fn findings_page(&self, pal: Pal, main_w: f32, cx: &mut Context<Self>) -> AnyElement {
        let Some(project) = self.current() else {
            return empty(pal.ui.no_project_open, pal.ui.pick_project_first, pal);
        };
        let Scan::Done { findings, .. } = &project.scan else {
            return running(project, pal);
        };
        if findings.is_empty() {
            return empty(pal.ui.no_crypto, pal.ui.looks_clean, pal);
        }
        let selected = project.selected.min(findings.len() - 1);
        let stacked = main_w < 860.;
        let narrow = main_w < 520.;

        let rows = findings
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let f = &a.finding;
                h_flex()
                    .id(("finding", i))
                    .gap_3()
                    .px_3()
                    .h(px(36.))
                    .rounded_md()
                    .items_center()
                    .cursor_pointer()
                    .text_sm()
                    .when(i == selected, |s| s.bg(pal.active))
                    .hover(|s| s.bg(pal.hover))
                    .child(dot(pal.urgency(a.urgency)))
                    .child(
                        div()
                            .w(px(if narrow { 110. } else { 130. }))
                            .flex_shrink_0()
                            .truncate()
                            .child(a.algorithm_name.clone()),
                    )
                    .when(!narrow, |s| {
                        s.child(
                            div()
                                .w(px(110.))
                                .flex_shrink_0()
                                .truncate()
                                .text_color(pal.muted)
                                .child(pal.lang.purpose(f.purpose)),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(pal.muted)
                            .child(format!("{}:{}", rel(&f.file, &project.path), f.line)),
                    )
                    .when(!narrow, |s| {
                        s.child(
                            div()
                                .w(px(64.))
                                .flex_shrink_0()
                                .text_right()
                                .text_color(pal.muted)
                                .child(pal.ui.urgency(a.urgency)),
                        )
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(p) = this.active.and_then(|a| this.open.get_mut(a)) {
                            p.selected = i;
                        }
                        cx.notify();
                    }))
            })
            .collect::<Vec<_>>();

        let list = v_flex()
            .id("findings")
            .p_4()
            .overflow_y_scroll()
            .when(stacked, |s| s.w_full().h(px(220.)).flex_shrink_0())
            .when(!stacked, |s| s.flex_1().min_w_0().h_full())
            .children(rows);

        let detail = details(&findings[selected], stacked, pal);
        if stacked {
            return div()
                .size_full()
                .min_h_0()
                .flex()
                .flex_col()
                .child(list)
                .child(detail)
                .into_any_element();
        }
        // Wide: the divider can be dragged to give the explanation more room.
        h_resizable("findings-split")
            .with_state(&self.split)
            .child(
                resizable_panel()
                    .size_range(px(260.)..px(2000.))
                    .child(list),
            )
            .child(
                resizable_panel()
                    .size(px(420.))
                    .size_range(px(280.)..px(900.))
                    .child(detail),
            )
            .into_any_element()
    }

    // ---- schedule dialog ---------------------------------------------------

    fn schedule_dialog(&self, pal: Pal, cx: &mut Context<Self>) -> Option<AnyElement> {
        let draft = self.draft.as_ref()?;
        let chosen = draft.path.clone();
        let picking = draft.changing || chosen.is_none();

        // The project: the one chosen, as a card, or the list to choose from.
        let project: AnyElement = match (&chosen, picking) {
            (Some(path), false) => h_flex()
                .gap_3()
                .px_3()
                .h(px(52.))
                .items_center()
                .rounded_lg()
                .border_1()
                .border_color(pal.border)
                .child(kind_icon(self.kind_of(path)))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_sm()
                                .font_medium()
                                .child(project_name(path)),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_xs()
                                .text_color(pal.muted)
                                .child(path.display().to_string()),
                        ),
                )
                .child(
                    Button::new("sched-change")
                        .ghost()
                        .xsmall()
                        .label(pal.ui.change_project)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(d) = this.draft.as_mut() {
                                d.changing = true;
                            }
                            cx.notify();
                        })),
                )
                .into_any_element(),
            _ => {
                let rows = self
                    .recents
                    .projects
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        let on = chosen.as_ref() == Some(&r.path);
                        let path = r.path.clone();
                        h_flex()
                            .id(("pick", i))
                            .h(px(34.))
                            .gap_3()
                            .px_3()
                            .rounded_md()
                            .items_center()
                            .cursor_pointer()
                            .text_sm()
                            .when(on, |s| s.bg(pal.active))
                            .hover(|s| s.bg(pal.hover))
                            .child(kind_icon(self.kind_of(&r.path)))
                            .child(div().flex_1().min_w_0().truncate().child(r.name()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(d) = this.draft.as_mut() {
                                    d.path = Some(path.clone());
                                    d.changing = false;
                                }
                                cx.notify();
                            }))
                    })
                    .collect::<Vec<_>>();
                v_flex()
                    .gap_1()
                    .child(
                        v_flex()
                            .id("pick-list")
                            .max_h(px(168.))
                            .overflow_y_scroll()
                            .p_1()
                            .rounded_lg()
                            .border_1()
                            .border_color(pal.border)
                            .children(rows),
                    )
                    .child(
                        Button::new("pick-other")
                            .ghost()
                            .small()
                            .icon(IconName::FolderOpen)
                            .label(pal.ui.choose_other)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.pick_folder(
                                    |this, path, cx| {
                                        let path = path.canonicalize().unwrap_or(path);
                                        if let Some(d) = this.draft.as_mut() {
                                            d.path = Some(path.clone());
                                            d.changing = false;
                                        }
                                        this.recents.add(&path);
                                        let _ = this.recents.save();
                                        cx.notify();
                                    },
                                    cx,
                                )
                            })),
                    )
                    .into_any_element()
            }
        };

        // How often it repeats: three equal choices side by side.
        let repeats = [Repeat::Once, Repeat::Daily, Repeat::Weekly]
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                chip(
                    ("repeat", i),
                    pal.ui.repeat_label(r),
                    draft.repeat == r,
                    pal,
                )
                .flex_1()
                .justify_center()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(d) = this.draft.as_mut() {
                        d.repeat = r;
                    }
                    cx.notify();
                }))
            })
            .collect::<Vec<_>>();

        // In plain words, when the first run will be.
        let now = now_secs();
        let time = draft.time.read(cx).time();
        let summary = next_at(time.hour(), time.minute(), now)
            .map(|at| pal.ui.plan_summary(at, now, draft.repeat))
            .unwrap_or_default();

        let field = |label: &'static str, body: AnyElement| {
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_medium()
                        .text_color(pal.muted)
                        .child(label),
                )
                .child(body)
        };

        let card = v_flex()
            .id("schedule-card")
            .w(px(460.))
            .max_w_full()
            .max_h_full()
            .overflow_y_scroll()
            .gap_5()
            .p_6()
            .rounded_xl()
            .border_1()
            .border_color(pal.border)
            .bg(pal.sidebar)
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                h_flex()
                    .items_start()
                    .gap_3()
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_semibold()
                                    .child(pal.ui.schedule_a_scan),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(pal.muted)
                                    .child(pal.ui.schedule_subtitle),
                            ),
                    )
                    .child(
                        Button::new("sched-close")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.draft = None;
                                cx.notify();
                            })),
                    ),
            )
            .child(field(pal.ui.project, project))
            .child(field(
                pal.ui.at_time,
                TimeField::new(&draft.time)
                    .large()
                    .w(px(150.))
                    .into_any_element(),
            ))
            .child(field(
                pal.ui.repeat,
                h_flex().gap_2().children(repeats).into_any_element(),
            ))
            .child(
                h_flex()
                    .gap_2p5()
                    .px_3()
                    .py_2p5()
                    .items_center()
                    .rounded_lg()
                    .bg(pal.hover)
                    .text_sm()
                    .child(Icon::new(Lucide::Hourglass).size_4().text_color(pal.muted))
                    .child(div().flex_1().child(summary)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("sched-cancel")
                            .ghost()
                            .small()
                            .label(pal.ui.cancel)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.draft = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("sched-ok")
                            .primary()
                            .small()
                            .icon(Icon::new(Lucide::Hourglass))
                            .label(pal.ui.schedule)
                            .disabled(chosen.is_none())
                            .on_click(cx.listener(|this, _, _, cx| this.confirm_schedule(cx))),
                    ),
            );

        Some(
            div()
                .id("scrim")
                .absolute()
                .inset_0()
                .p_6()
                .flex()
                .items_center()
                .justify_center()
                .bg(hsla(0., 0., 0., 0.6))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.draft = None;
                    cx.notify();
                }))
                .child(card)
                .into_any_element(),
        )
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = Pal::new(cx, self.lang);

        // Kinds are read from disk once per project, not on every frame.
        let paths: Vec<PathBuf> = self
            .recents
            .projects
            .iter()
            .map(|r| r.path.clone())
            .chain(self.open.iter().map(|p| p.path.clone()))
            .chain(self.schedules.items.iter().map(|s| s.path.clone()))
            .collect();
        for path in paths {
            self.kinds
                .entry(path.clone())
                .or_insert_with(|| Kind::detect(&path));
        }
        for (kind, title, text) in std::mem::take(&mut self.toasts) {
            // No notification type: the type would colour the icon. The icon is
            // set by hand instead, in the muted foreground.
            let icon = match kind {
                NotificationType::Success => IconName::CircleCheck,
                NotificationType::Warning | NotificationType::Error => IconName::TriangleAlert,
                NotificationType::Info => IconName::Info,
            };
            let note = Notification::new()
                .title(title)
                .icon(Icon::new(icon).text_color(pal.muted))
                .placement(Anchor::BottomCenter);
            let note = if text.is_empty() {
                note
            } else {
                note.message(text)
            };
            window.push_notification(note, cx);
        }

        let width = f32::from(window.viewport_size().width);
        let main_w = (width - SIDEBAR_W).max(320.);
        let compact = main_w < 700.;
        let on_project = self.page != Page::Projects && self.current().is_some();

        let page = match self.page {
            Page::Projects => self.projects_page(pal, main_w, cx),
            Page::Dashboard => self.dashboard_page(pal, main_w),
            Page::Findings => self.findings_page(pal, main_w, cx),
        };
        let scrolls = self.page != Page::Findings;
        let dialog = self.schedule_dialog(pal, cx);
        let lang_menu = self.language_menu(pal, cx);

        div()
            .size_full()
            .relative()
            .text_sm()
            .bg(pal.bg)
            .text_color(pal.fg)
            .child(
                h_flex().size_full().child(self.sidebar(pal, cx)).child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .child(self.header(pal, compact, cx))
                        .when(on_project, |s| s.child(self.tabs(pal, cx)))
                        .child(
                            div()
                                .id("page")
                                .flex_1()
                                .min_h_0()
                                .min_w_0()
                                .overflow_x_hidden()
                                .when(scrolls, |s| s.overflow_y_scroll())
                                .child(page),
                        ),
                ),
            )
            .children(lang_menu)
            .children(dialog)
    }
}
