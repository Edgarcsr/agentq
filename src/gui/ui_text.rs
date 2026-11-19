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
    pub close: &'static str,
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
    pub run_in: &'static str,
    pub repeat: &'static str,
    pub cancel: &'static str,
    pub delays: [&'static str; 4],

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
    open_caption: "Open",
    open_folder: "Open folder",
    open_folder_tip: "Open a folder and scan it",
    nothing_open: "Nothing open.",
    language_tip: "Language of the advice and the interface",
    languages: "Languages",
    close: "Close",
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
    not_analyzed_hint: "agentq only analyzes JavaScript and TypeScript for now, so these files were listed but not examined.",
    coverage_note: "The findings cover only the analyzed languages.",
    no_source_files: "No source files found.",
    uses_word: "uses",
    use_word: "use",

    schedule_a_scan: "Schedule a scan",
    project: "Project",
    choose_other: "Choose another folder…",
    run_in: "Run in",
    repeat: "Repeat",
    cancel: "Cancel",
    delays: ["15 min", "1 hour", "6 hours", "24 hours"],

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
    just_now: "Agora há pouco",
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
    open_caption: "Abertos",
    open_folder: "Abrir pasta",
    open_folder_tip: "Abrir uma pasta e escaneá-la",
    nothing_open: "Nada aberto.",
    language_tip: "Idioma dos conselhos e da interface",
    languages: "Idiomas",
    close: "Fechar",
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
    not_analyzed_hint: "O agentq só analisa JavaScript e TypeScript por enquanto, então esses arquivos foram listados mas não examinados.",
    coverage_note: "Os achados cobrem só as linguagens analisadas.",
    no_source_files: "Nenhum arquivo de código encontrado.",
    uses_word: "usos",
    use_word: "uso",

    schedule_a_scan: "Agendar um scan",
    project: "Projeto",
    choose_other: "Escolher outra pasta…",
    run_in: "Executar em",
    repeat: "Repetir",
    cancel: "Cancelar",
    delays: ["15 min", "1 hora", "6 horas", "24 horas"],

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
