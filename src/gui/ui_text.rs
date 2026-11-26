//! Everything the desktop interface says, in each language. The advice text
//! itself comes from the library (`agentq::i18n`); this is the chrome around it.

use agentq::assess::Urgency;
use agentq::i18n::Lang;
use agentq::schedule::Repeat;

pub struct Ui {
    /// Portuguese rather than English.
    is_pt: bool,
    pub now: &'static str,
    pub just_now: &'static str,
    pub pending: &'static str,
    pub running: &'static str,
    pub done: &'static str,
    pub scanning: &'static str,

    pub broken: &'static str,
    pub weakened: &'static str,
    pub safe: &'static str,
    pub broken_by_quantum: &'static str,

    pub why_it_matters: &'static str,
    pub alternatives: &'static str,
    pub advice: &'static str,

    pub projects: &'static str,
    pub projects_tip: &'static str,
    pub open_caption: &'static str,
    pub open_folder: &'static str,
    pub open_folder_tip: &'static str,
    pub nothing_open: &'static str,
    pub language_tip: &'static str,
    pub languages: &'static str,
    pub choose: &'static str,

    pub dashboard: &'static str,
    pub findings: &'static str,
    pub rescan: &'static str,
    pub rescan_tip: &'static str,
    pub schedule: &'static str,
    pub schedule_tip: &'static str,

    pub no_projects: &'static str,
    pub no_projects_hint: &'static str,
    pub col_project: &'static str,
    pub col_status: &'static str,
    pub col_issues: &'static str,
    pub col_when: &'static str,
    pub scheduled_scans: &'static str,
    pub new: &'static str,
    pub nothing_scheduled: &'static str,
    pub cancel_scan: &'static str,

    pub no_project_open: &'static str,
    pub pick_project: &'static str,
    pub pick_project_first: &'static str,
    pub no_crypto: &'static str,
    pub looks_clean: &'static str,
    pub nothing_at_risk: &'static str,
    pub nothing_urgent: &'static str,
    pub migrate_first: &'static str,
    pub algorithms: &'static str,
    pub purposes: &'static str,
    pub files: &'static str,
    pub languages_title: &'static str,
    pub analyzed: &'static str,
    pub not_analyzed: &'static str,
    pub not_analyzed_hint: &'static str,
    pub coverage_note: &'static str,
    pub no_source_files: &'static str,
    pub uses_word: &'static str,
    pub use_word: &'static str,

    pub schedule_a_scan: &'static str,
    pub project: &'static str,
    pub choose_other: &'static str,
    pub at_time: &'static str,
    pub schedule_subtitle: &'static str,
    pub change_project: &'static str,
    pub remove_project: &'static str,
    pub remove_analysis: &'static str,
    pub toast_analysis_removed: &'static str,
    pub show_dates: &'static str,
    pub hide_dates: &'static str,
    pub toast_project_removed: &'static str,
    pub repeat: &'static str,
    pub cancel: &'static str,

    pub toast_scan_done: &'static str,
    pub toast_scan_risks: &'static str,
    pub toast_scheduled: &'static str,
    pub toast_removed: &'static str,
    pub toast_started: &'static str,
    pub toast_skipped: &'static str,
    pub toast_missing: &'static str,
    pub toast_language: &'static str,
}

static EN: Ui = Ui {
    is_pt: false,
    now: "Now",
    just_now: "Just now",
    pending: "Pending",
    running: "Running",
    done: "Done",
    scanning: "Scanning",

    broken: "Broken",
    weakened: "Weakened",
    safe: "Safe",
    broken_by_quantum: "broken by quantum",

    why_it_matters: "Why it matters",
    alternatives: "Alternatives",
    advice: "Advice",

    projects: "Projects",
    projects_tip: "All projects and scheduled scans",
    open_caption: "History",
    open_folder: "Open folder",
    open_folder_tip: "Open a folder and scan it",
    nothing_open: "No projects yet.",
    language_tip: "Language of the advice and the interface",
    languages: "Languages",
    choose: "Choose",

    dashboard: "Dashboard",
    findings: "Findings",
    rescan: "Rescan",
    rescan_tip: "Scan this project again",
    schedule: "Schedule",
    schedule_tip: "Plan a scan for later",

    no_projects: "No projects yet",
    no_projects_hint: "Open a folder to scan it for quantum-vulnerable cryptography.",
    col_project: "Project",
    col_status: "Status",
    col_issues: "Issues",
    col_when: "When",
    scheduled_scans: "Scheduled scans",
    new: "New",
    nothing_scheduled: "Nothing scheduled. Plan scans to run later, once or on repeat.",
    cancel_scan: "Cancel this scan",

    no_project_open: "No project open",
    pick_project: "Pick one from Projects, or open a folder.",
    pick_project_first: "Pick one from Projects first.",
    no_crypto: "No cryptography found.",
    looks_clean: "This project looks clean.",
    nothing_at_risk: "Nothing here is at risk.",
    nothing_urgent: "Nothing urgent.",
    migrate_first: "Migrate first",
    algorithms: "Algorithms",
    purposes: "Purposes",
    files: "Files",
    languages_title: "Languages",
    analyzed: "analyzed",
    not_analyzed: "not analyzed yet",
    not_analyzed_hint: "agentq does not analyze this language yet, so these files were listed but not examined.",
    coverage_note: "The findings cover only the analyzed languages.",
    no_source_files: "No source files found.",
    uses_word: "uses",
    use_word: "use",

    schedule_a_scan: "Schedule a scan",
    project: "Project",
    choose_other: "Choose another folder…",
    at_time: "Time",
    schedule_subtitle: "Choose a project and the time of day it should run.",
    change_project: "Change",
    remove_project: "Remove from history",
    remove_analysis: "Remove this analysis",
    toast_analysis_removed: "Analysis removed",
    show_dates: "Show dates",
    hide_dates: "Hide dates",
    toast_project_removed: "Removed from history",
    repeat: "Repeat",
    cancel: "Cancel",

    toast_scan_done: "Scan finished",
    toast_scan_risks: "Scan finished with risks",
    toast_scheduled: "Scan scheduled",
    toast_removed: "Schedule removed",
    toast_started: "Scheduled scan started",
    toast_skipped: "Scheduled scan skipped",
    toast_missing: "no longer exists",
    toast_language: "Language: English",
};

static PT: Ui = Ui {
    is_pt: true,
    now: "Agora",
    just_now: "Há pouco",
    pending: "Pendente",
    running: "Em execução",
    done: "Concluído",
    scanning: "Escaneando",

    broken: "Quebrado",
    weakened: "Enfraquecido",
    safe: "Seguro",
    broken_by_quantum: "quebrado por quântico",

    why_it_matters: "Por que importa",
    alternatives: "Alternativas",
    advice: "Conselhos",

    projects: "Projetos",
    projects_tip: "Todos os projetos e scans agendados",
    open_caption: "Histórico",
    open_folder: "Abrir pasta",
    open_folder_tip: "Abrir uma pasta e escaneá-la",
    nothing_open: "Nenhum projeto ainda.",
    language_tip: "Idioma dos conselhos e da interface",
    languages: "Idiomas",
    choose: "Escolher",

    dashboard: "Painel",
    findings: "Achados",
    rescan: "Reescanear",
    rescan_tip: "Escanear este projeto de novo",
    schedule: "Agendar",
    schedule_tip: "Planejar um scan para depois",

    no_projects: "Nenhum projeto ainda",
    no_projects_hint: "Abra uma pasta para procurar criptografia vulnerável a computação quântica.",
    col_project: "Projeto",
    col_status: "Status",
    col_issues: "Problemas",
    col_when: "Quando",
    scheduled_scans: "Scans agendados",
    new: "Novo",
    nothing_scheduled: "Nada agendado. Planeje scans para rodar depois, uma vez ou repetidos.",
    cancel_scan: "Cancelar este scan",

    no_project_open: "Nenhum projeto aberto",
    pick_project: "Escolha um em Projetos ou abra uma pasta.",
    pick_project_first: "Escolha um projeto em Projetos primeiro.",
    no_crypto: "Nenhuma criptografia encontrada.",
    looks_clean: "Este projeto parece limpo.",
    nothing_at_risk: "Nada aqui está em risco.",
    nothing_urgent: "Nada urgente.",
    migrate_first: "Migrar primeiro",
    algorithms: "Algoritmos",
    purposes: "Propósitos",
    files: "Arquivos",
    languages_title: "Linguagens",
    analyzed: "analisada",
    not_analyzed: "ainda não analisada",
    not_analyzed_hint: "O agentq ainda não analisa esta linguagem, então esses arquivos foram listados mas não examinados.",
    coverage_note: "Os achados cobrem só as linguagens analisadas.",
    no_source_files: "Nenhum arquivo de código encontrado.",
    uses_word: "usos",
    use_word: "uso",

    schedule_a_scan: "Agendar um scan",
    project: "Projeto",
    choose_other: "Escolher outra pasta…",
    at_time: "Horário",
    schedule_subtitle: "Escolha um projeto e o horário em que ele deve rodar.",
    change_project: "Trocar",
    remove_project: "Remover do histórico",
    remove_analysis: "Remover esta análise",
    toast_analysis_removed: "Análise removida",
    show_dates: "Mostrar datas",
    hide_dates: "Ocultar datas",
    toast_project_removed: "Removido do histórico",
    repeat: "Repetir",
    cancel: "Cancelar",

    toast_scan_done: "Scan concluído",
    toast_scan_risks: "Scan concluído com riscos",
    toast_scheduled: "Scan agendado",
    toast_removed: "Agendamento removido",
    toast_started: "Scan agendado iniciado",
    toast_skipped: "Scan agendado ignorado",
    toast_missing: "não existe mais",
    toast_language: "Idioma: Português",
};

impl Ui {
    pub fn of(lang: Lang) -> &'static Ui {
        match lang {
            Lang::En => &EN,
            Lang::PtBr => &PT,
        }
    }

    fn pt(&self) -> bool {
        self.is_pt
    }

    /// High, Medium, Low, None.
    pub fn urgency(&self, urgency: Urgency) -> &'static str {
        match (urgency, self.pt()) {
            (Urgency::High, false) => "High",
            (Urgency::Medium, false) => "Medium",
            (Urgency::Low, false) => "Low",
            (Urgency::None, false) => "None",
            (Urgency::High, true) => "Alta",
            (Urgency::Medium, true) => "Média",
            (Urgency::Low, true) => "Baixa",
            (Urgency::None, true) => "Nenhuma",
        }
    }

    pub fn repeat_label(&self, repeat: Repeat) -> &'static str {
        match (repeat, self.pt()) {
            (Repeat::Once, false) => "Once",
            (Repeat::Daily, false) => "Daily",
            (Repeat::Weekly, false) => "Weekly",
            (Repeat::Once, true) => "Uma vez",
            (Repeat::Daily, true) => "Diário",
            (Repeat::Weekly, true) => "Semanal",
        }
    }

    /// The exact moment in the local time zone, written the way date-fns
    /// writes its long form (`PPpp`): "Nov 26, 2026 at 3:45:12 PM" in English,
    /// "26 de nov. de 2026 às 15:45:12" in Portuguese.
    pub fn exact(&self, at: u64) -> String {
        use chrono::{Datelike, Local, TimeZone};
        const PT_MONTHS: [&str; 12] = [
            "jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.",
            "dez.",
        ];
        let Some(moment) = i64::try_from(at)
            .ok()
            .and_then(|at| Local.timestamp_opt(at, 0).single())
        else {
            return String::new();
        };
        if self.pt() {
            format!(
                "{} de {} de {} às {}",
                moment.day(),
                PT_MONTHS[moment.month0() as usize],
                moment.year(),
                moment.format("%H:%M:%S")
            )
        } else {
            moment.format("%b %-d, %Y at %-I:%M:%S %p").to_string()
        }
    }

    /// A short date for a list: "Nov 26, 3:45 PM", "26 nov., 15:45".
    pub fn short_date(&self, at: u64) -> String {
        use chrono::{Datelike, Local, TimeZone};
        const PT_MONTHS: [&str; 12] = [
            "jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.",
            "dez.",
        ];
        let Some(moment) = i64::try_from(at)
            .ok()
            .and_then(|at| Local.timestamp_opt(at, 0).single())
        else {
            return String::new();
        };
        if self.pt() {
            format!(
                "{} {}, {}",
                moment.day(),
                PT_MONTHS[moment.month0() as usize],
                moment.format("%H:%M")
            )
        } else {
            moment.format("%b %-d, %-I:%M %p").to_string()
        }
    }

    /// When the first run will be, in plain words: "Runs today at 15:00 · in
    /// 2h", "Runs every day at 09:30, starting tomorrow".
    pub fn plan_summary(&self, at: u64, now: u64, repeat: Repeat) -> String {
        use chrono::{Local, TimeZone};
        let local = |t: u64| {
            i64::try_from(t)
                .ok()
                .and_then(|t| Local.timestamp_opt(t, 0).single())
        };
        let (Some(first), Some(today)) = (local(at), local(now)) else {
            return String::new();
        };
        let time = first.format("%H:%M");
        let same_day = first.date_naive() == today.date_naive();
        if self.pt() {
            let day = if same_day { "hoje" } else { "amanhã" };
            match repeat {
                Repeat::Once => format!("Roda {day} às {time} · {}", self.until(at, now)),
                Repeat::Daily => format!("Roda todo dia às {time}, a partir de {day}"),
                Repeat::Weekly => format!("Roda toda semana às {time}, a partir de {day}"),
            }
        } else {
            let day = if same_day { "today" } else { "tomorrow" };
            match repeat {
                Repeat::Once => format!("Runs {day} at {time} · {}", self.until(at, now)),
                Repeat::Daily => format!("Runs every day at {time}, starting {day}"),
                Repeat::Weekly => format!("Runs every week at {time}, starting {day}"),
            }
        }
    }

    /// "Scanning · started 5m ago", "Analisando · iniciado há 5 min".
    pub fn scanning_since(&self, secs: Option<u64>) -> String {
        let (head, fresh, began) = if self.pt() {
            ("Analisando", "acabou de começar", "iniciado")
        } else {
            ("Scanning", "just started", "started")
        };
        match secs {
            Some(s) if s >= 60 => format!("{head} · {began} {}", self.ago(s)),
            _ => format!("{head} · {fresh}"),
        }
    }

    /// "Analyzed 5m ago", "Analisado há 5 min".
    pub fn analyzed(&self, secs: u64) -> String {
        if secs < 60 {
            return if self.pt() {
                "Analisado agora há pouco"
            } else {
                "Analyzed just now"
            }
            .to_string();
        }
        if self.pt() {
            format!("Analisado {}", self.ago(secs))
        } else {
            format!("Analyzed {}", self.ago(secs))
        }
    }

    /// "5m ago", "há 5 min".
    pub fn ago(&self, secs: u64) -> String {
        let (n, unit) = match secs {
            0..=59 => return self.just_now.to_string(),
            60..=3599 => (secs / 60, if self.pt() { "min" } else { "m" }),
            3600..=86_399 => (secs / 3600, if self.pt() { "h" } else { "h" }),
            _ => (secs / 86_400, if self.pt() { "d" } else { "d" }),
        };
        if self.pt() {
            format!("há {n} {unit}")
        } else {
            format!("{n}{unit} ago")
        }
    }

    /// "in 5m", "em 5 min"; "due"/"agora" when it is already time.
    pub fn until(&self, at: u64, now: u64) -> String {
        let s = at.saturating_sub(now);
        let (n, unit) = match s {
            0 => return if self.pt() { "agora" } else { "due" }.to_string(),
            1..=3599 => (s.div_ceil(60), if self.pt() { "min" } else { "m" }),
            3600..=86_399 => (s / 3600, "h"),
            _ => (s / 86_400, "d"),
        };
        if self.pt() {
            format!("em {n} {unit}")
        } else {
            format!("in {n}{unit}")
        }
    }

    pub fn verdict_broken(&self, n: usize) -> String {
        match (self.pt(), n) {
            (false, 1) => "1 use is broken by quantum computers.".into(),
            (false, n) => format!("{n} uses are broken by quantum computers."),
            (true, 1) => "1 uso é quebrado por computadores quânticos.".into(),
            (true, n) => format!("{n} usos são quebrados por computadores quânticos."),
        }
    }

    pub fn verdict_weakened(&self, n: usize) -> String {
        if self.pt() {
            format!("{n} enfraquecido(s), nenhum quebrado.")
        } else {
            format!("{n} weakened, none broken.")
        }
    }

    pub fn summary_line(
        &self,
        weakened: usize,
        safe: usize,
        files: usize,
        urgent: usize,
    ) -> String {
        let tail = |en: String, pt: String| if self.pt() { pt } else { en };
        let mut line = tail(
            format!("{weakened} weakened · {safe} safe · {files} files scanned"),
            format!("{weakened} enfraquecidos · {safe} seguros · {files} arquivos escaneados"),
        );
        if urgent > 0 {
            line.push_str(&tail(
                format!(" · {urgent} to migrate first"),
                format!(" · {urgent} para migrar primeiro"),
            ));
        }
        line
    }

    /// "1 file", "12 files" / "1 arquivo", "12 arquivos".
    pub fn files_count(&self, n: usize) -> String {
        match (self.pt(), n) {
            (false, 1) => "1 file".into(),
            (false, n) => format!("{n} files"),
            (true, 1) => "1 arquivo".into(),
            (true, n) => format!("{n} arquivos"),
        }
    }

    /// "2 languages detected" / "2 linguagens detectadas".
    pub fn languages_detected(&self, n: usize) -> String {
        if self.pt() {
            format!("{n} linguagens detectadas")
        } else {
            format!("{n} languages detected")
        }
    }

    pub fn tooltip_uses(&self, n: usize, pct: f32) -> String {
        let word = if n == 1 {
            self.use_word
        } else {
            self.uses_word
        };
        format!("{n} {word} ({pct:.0}%)")
    }
}
