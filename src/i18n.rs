//! Translations of everything agentq says to the user.
//!
//! Every message lives in one table, written once per language. The table is
//! expanded by a macro into a struct and one constant per language, so a
//! message that is missing in a language does not compile.

use crate::assess::Urgency;
use crate::finding::{Confidence, Purpose, QuantumRisk};
use crate::knowledge::Attack;
use std::fmt::Display;

/// Languages the interface and reports can be shown in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    PtBr,
}

impl Lang {
    /// Reads a language tag such as `en`, `pt`, `pt-BR` or `pt_BR.UTF-8`.
    pub fn parse(tag: &str) -> Option<Lang> {
        let tag = tag.split('.').next().unwrap_or(tag).replace('_', "-");
        match tag.to_ascii_lowercase().as_str() {
            "en" | "en-us" | "en-gb" => Some(Lang::En),
            "pt" | "pt-br" | "pt-pt" => Some(Lang::PtBr),
            _ => None,
        }
    }

    /// The language of the environment (`LC_ALL`, `LC_MESSAGES`, then `LANG`),
    /// or English when it is not one we translate to.
    pub fn detect() -> Lang {
        let value = ["LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty());
        Lang::from_locale(value.as_deref())
    }

    /// Picks the language for a locale value such as `pt_BR.UTF-8`.
    pub fn from_locale(locale: Option<&str>) -> Lang {
        match locale {
            Some(value) if value.to_ascii_lowercase().starts_with("pt") => Lang::PtBr,
            _ => Lang::En,
        }
    }

    /// The other language, for a key that toggles between them.
    pub fn other(self) -> Lang {
        match self {
            Lang::En => Lang::PtBr,
            Lang::PtBr => Lang::En,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::PtBr => "pt-BR",
        }
    }

    pub fn t(self) -> &'static Text {
        match self {
            Lang::En => &EN,
            Lang::PtBr => &PT_BR,
        }
    }

    pub fn purpose(self, purpose: Purpose) -> &'static str {
        let t = self.t();
        match purpose {
            Purpose::KeyExchange => t.purpose_key_exchange,
            Purpose::Signature => t.purpose_signature,
            Purpose::Encryption => t.purpose_encryption,
            Purpose::Hash => t.purpose_hash,
            Purpose::Unknown => t.purpose_unknown,
        }
    }

    /// Lowercase name of an urgency, as in "high".
    pub fn urgency(self, urgency: Urgency) -> &'static str {
        let t = self.t();
        match urgency {
            Urgency::High => t.urgency_high,
            Urgency::Medium => t.urgency_medium,
            Urgency::Low => t.urgency_low,
            Urgency::None => t.urgency_none,
        }
    }

    /// Six-column badge for an urgency, as in " HIGH ".
    pub fn badge(self, urgency: Urgency) -> &'static str {
        let t = self.t();
        match urgency {
            Urgency::High => t.badge_high,
            Urgency::Medium => t.badge_medium,
            Urgency::Low => t.badge_low,
            Urgency::None => t.badge_none,
        }
    }

    pub fn risk(self, risk: QuantumRisk) -> &'static str {
        let t = self.t();
        match risk {
            QuantumRisk::Broken => t.risk_broken,
            QuantumRisk::Weakened => t.risk_weakened,
            QuantumRisk::Safe => t.risk_safe,
        }
    }

    /// Translates the machine name of a risk (`broken`, `weakened`, `safe`);
    /// anything else is returned as is.
    pub fn risk_key(self, key: &str) -> String {
        match key {
            "broken" => self.t().risk_broken,
            "weakened" => self.t().risk_weakened,
            "safe" => self.t().risk_safe,
            other => other,
        }
        .to_string()
    }

    pub fn confidence(self, confidence: Confidence) -> &'static str {
        let t = self.t();
        match confidence {
            Confidence::Low => t.confidence_low,
            Confidence::Medium => t.confidence_medium,
            Confidence::High => t.confidence_high,
        }
    }

    /// "broken by Shor's algorithm", "safe", ...
    pub fn risk_sentence(self, risk: QuantumRisk, attack: &Attack) -> String {
        let t = self.t();
        let attack = match attack {
            Attack::Shor => t.attack_shor,
            Attack::Grover => t.attack_grover,
            Attack::None => t.attack_none,
        };
        match risk {
            QuantumRisk::Broken => fill(t.risk_broken_by, &[("attack", &attack)]),
            QuantumRisk::Weakened => fill(t.risk_weakened_by, &[("attack", &attack)]),
            QuantumRisk::Safe => t.risk_safe.to_string(),
        }
    }

    pub fn on_off(self, on: bool) -> &'static str {
        if on { self.t().on } else { self.t().off }
    }
}

/// Fills `{name}` placeholders in a message.
pub fn fill(template: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut text = template.to_string();
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), &value.to_string());
    }
    text
}

macro_rules! texts {
    ($($field:ident: $en:expr, $pt:expr;)*) => {
        /// Every message, in one language.
        pub struct Text {
            $(pub $field: &'static str,)*
        }
        const EN: Text = Text { $($field: $en,)* };
        const PT_BR: Text = Text { $($field: $pt,)* };
    };
}

texts! {
    // --- words used everywhere ---
    on: "on", "ativado";
    off: "off", "desativado";
    purpose_key_exchange: "key exchange", "troca de chaves";
    purpose_signature: "signature", "assinatura";
    purpose_encryption: "encryption", "cifragem";
    purpose_hash: "hash", "hash";
    purpose_unknown: "unknown", "desconhecido";
    urgency_high: "high", "alta";
    urgency_medium: "medium", "média";
    urgency_low: "low", "baixa";
    urgency_none: "none", "nenhuma";
    badge_high: " HIGH ", " ALTA ";
    badge_medium: " MED  ", " MÉDIA";
    badge_low: " LOW  ", " BAIXA";
    badge_none: " OK   ", " OK   ";
    risk_broken: "broken", "quebrado";
    risk_weakened: "weakened", "enfraquecido";
    risk_safe: "safe", "seguro";
    confidence_low: "Low", "Baixa";
    confidence_medium: "Medium", "Média";
    confidence_high: "High", "Alta";
    attack_shor: "Shor's algorithm", "algoritmo de Shor";
    attack_grover: "Grover's algorithm", "algoritmo de Grover";
    attack_none: "no known quantum attack", "nenhum ataque quântico conhecido";
    risk_broken_by: "broken by {attack}", "quebrado pelo {attack}";
    risk_weakened_by: "weakened by {attack}", "enfraquecido pelo {attack}";

    // --- text produced by the assessment ---
    ctx_harvest: "Traffic or data protected this way can be recorded today and decrypted once a quantum computer exists (harvest now, decrypt later).",
        "Tráfego ou dados protegidos assim podem ser gravados hoje e decifrados quando existir um computador quântico (colher agora, decifrar depois).";
    ctx_forge: "A quantum attacker able to recover private keys could forge signatures, which matters most for signatures that must stay trustworthy for years.",
        "Um atacante quântico capaz de recuperar chaves privadas poderia forjar assinaturas, o que pesa mais para assinaturas que precisam continuar confiáveis por anos.";
    advice_hybrid: "Use a hybrid scheme that combines the classical algorithm with ML-KEM, so security holds if either one is broken.",
        "Use um esquema híbrido que combine o algoritmo clássico com o ML-KEM, para que a segurança se mantenha se um dos dois for quebrado.";
    advice_signatures: "Migrate long-lived signatures first (certificates, firmware, signed archives). Post-quantum signatures and keys are much larger, so check size limits in protocols and storage.",
        "Migre primeiro as assinaturas de longa duração (certificados, firmware, arquivos assinados). Assinaturas e chaves pós-quânticas são bem maiores, então confira os limites de tamanho em protocolos e armazenamento.";
    advice_symmetric: "Prefer 256-bit symmetric keys and 256-bit or longer hash outputs.",
        "Prefira chaves simétricas de 256 bits e saídas de hash de 256 bits ou mais.";
    advice_unknown_purpose: "The purpose of this key could not be determined; check how it is used to choose between a key encapsulation mechanism (ML-KEM) and a signature scheme (ML-DSA).",
        "Não foi possível determinar para que serve esta chave; veja como ela é usada para escolher entre um mecanismo de encapsulamento de chaves (ML-KEM) e um esquema de assinatura (ML-DSA).";

    // --- text and Markdown reports ---
    report_title: "agentq report for {root}", "Relatório do agentq para {root}";
    report_kb: "knowledge base: {version}", "base de conhecimento: {version}";
    report_files: "Files", "Arquivos";
    report_files_detected: "{n} file(s) detected in {root}", "{n} arquivo(s) detectado(s) em {root}";
    report_summary: "Summary", "Resumo";
    report_uses_found: "{n} cryptographic use(s) found", "{n} uso(s) de criptografia encontrado(s)";
    report_need_urgent: "{n} need urgent attention", "{n} precisam de atenção urgente";
    report_findings_ranked: "Findings (most urgent first)", "Achados (mais urgentes primeiro)";
    confidence_label: "confidence", "confiança";
    why: "Why", "Por quê";
    replace_with: "Replace with", "Substituir por";
    implementations: "Implementations", "Implementações";
    note: "Note", "Nota";
    md_title: "agentq report", "Relatório do agentq";
    md_analyzed: "Analyzed", "Analisado";
    md_files_detected: "Files detected", "Arquivos detectados";
    md_uses_found: "Cryptographic uses found", "Usos de criptografia encontrados";
    md_knowledge_base: "Knowledge base", "Base de conhecimento";
    md_risk: "Risk", "Risco";
    md_count: "Count", "Qtde";
    md_language: "Language", "Linguagem";
    md_share: "Share", "Participação";
    md_findings: "Findings", "Achados";
    md_urgency: "Urgency", "Urgência";
    md_algorithm: "Algorithm", "Algoritmo";
    md_purpose: "Purpose", "Propósito";
    md_key_size: "Key size", "Tamanho da chave";
    md_location: "Location", "Local";
    md_details: "Details", "Detalhes";
    md_library: "Library", "Biblioteca";
    md_confidence: "Confidence", "Confiança";
    md_quantum_risk: "Quantum risk", "Risco quântico";
    md_at: "at", "em";

    // --- terminal interface: results ---
    tab_overview: "Overview", "Visão geral";
    tab_findings: "Findings", "Achados";
    header_info: "{files} file(s) · knowledge base {version}", "{files} arquivo(s) · base de conhecimento {version}";
    findings_title: "Findings ({n})", "Achados ({n})";
    hidden_by_filters: "{n} finding(s) hidden by the filters.\nPress s or c to change them.",
        "{n} achado(s) oculto(s) pelos filtros.\nAperte s ou c para mudá-los.";
    none_found: "No cryptographic use found.", "Nenhum uso de criptografia encontrado.";
    details_title: "Details", "Detalhes";
    nothing_selected: "Nothing selected.", "Nada selecionado.";
    label_urgency: "Urgency", "Urgência";
    label_quantum: "Quantum", "Quântico";
    label_purpose: "Purpose", "Propósito";
    label_key_size: "Key size", "Tam. chave";
    label_library: "Library", "Biblioteca";
    label_confidence: "Confidence", "Confiança";
    label_location: "Location", "Local";
    h_code: "Code", "Código";
    h_why: "Why it matters", "Por que importa";
    h_replace: "Replace with", "Substituir por";
    h_implementations: "Implementations", "Implementações";
    h_advice: "Advice", "Conselhos";
    footer_uses: "{n} use(s)", "{n} uso(s)";
    footer_broken: "{n} broken", "{n} quebrado(s)";
    footer_weakened: "{n} weakened", "{n} enfraquecido(s)";
    footer_safe: "{n} safe", "{n} seguro(s)";
    footer_filters: "hide safe: {safe} · min confidence: {confidence}",
        "ocultar seguros: {safe} · confiança mínima: {confidence}";
    key_select: "select", "selecionar";
    key_tabs: "tabs", "abas";
    key_scroll: "scroll", "rolar";
    key_hide_safe: "hide safe", "ocultar seguros";
    key_confidence: "confidence", "confiança";
    key_rescan: "rescan", "reanalisar";
    key_browse: "browse", "pastas";
    key_language: "language", "idioma";
    key_help: "help", "ajuda";
    key_quit: "quit", "sair";
    key_back: "back", "voltar";
    status_rescanned: "Scanned again: {uses} use(s) in {files} file(s)",
        "Análise refeita: {uses} uso(s) em {files} arquivo(s)";
    status_no_finding: "No finding with that urgency", "Nenhum achado com essa urgência";

    // --- terminal interface: background work ---
    loading: "Analyzing {path}", "Analisando {path}";
    loading_hint: "Esc cancels · shift+l language · q menu", "Esc cancela · shift+l idioma · q menu";
    scanning: "Scanning…", "Analisando…";

    // --- terminal interface: help ---
    help_title: "Help", "Ajuda";
    help_select: "select a finding", "selecionar um achado";
    help_first_last: "first / last finding", "primeiro / último achado";
    help_scroll: "scroll the details", "rolar os detalhes";
    help_hide_safe: "hide or show safe uses", "ocultar ou mostrar usos seguros";
    help_confidence: "cycle the minimum confidence", "alternar a confiança mínima";
    help_rescan: "scan the directory again", "analisar a pasta de novo";
    help_browse: "choose another project", "escolher outro projeto";
    help_tabs: "switch between overview and findings", "alternar entre visão geral e achados";
    help_mouse: "click tabs and findings, wheel to scroll", "clicar em abas e achados, roda para rolar";
    help_language: "switch the language", "trocar o idioma";
    help_close: "close this help", "fechar esta ajuda";
    help_quit: "back to the menu", "voltar ao menu";
    legend_high: "  HIGH  key exchange or encryption that quantum computers break",
        "  ALTA  troca de chaves ou cifragem que computadores quânticos quebram";
    legend_medium: "  MED   signatures that quantum computers could forge",
        "  MÉDIA assinaturas que computadores quânticos poderiam forjar";
    legend_low: "  LOW   weakened by Grover's algorithm", "  BAIXA enfraquecido pelo algoritmo de Grover";
    legend_none: "  OK    safe, or already post-quantum", "  OK    seguro, ou já pós-quântico";

    // --- terminal interface: overview ---
    ov_files: "Files", "Arquivos";
    ov_scanned: "scanned", "analisados";
    ov_vulnerable: "Quantum-vulnerable", "Vulneráveis a quânticos";
    ov_broken_by_shor: "broken by Shor", "quebrados por Shor";
    ov_weakened: "Weakened", "Enfraquecidos";
    ov_weakened_by_grover: "weakened by Grover", "enfraquecidos por Grover";
    ov_safe: "Already safe", "Já seguros";
    ov_no_migration: "no migration needed", "nada a migrar";
    ov_by_urgency: "By urgency", "Por urgência";
    ov_top_algorithms: "Top algorithms", "Algoritmos mais usados";
    ov_review_first: "Review first", "Revisar primeiro";
    ov_languages: "Languages", "Linguagens";
    ov_none_to_show: "No cryptographic use to show.", "Nenhum uso de criptografia para mostrar.";
    ov_nothing: "Nothing to show.", "Nada para mostrar.";
    ov_no_sources: "No source files found.", "Nenhum arquivo-fonte encontrado.";

    needs_terminal: "the interactive interface needs a terminal; use --format for piped output",
        "a interface interativa precisa de um terminal; use --format para saída redirecionada";

    // --- terminal interface: folder browser ---
    br_title: "Choose a project to analyze", "Escolha um projeto para analisar";
    br_analyze: "Analyze this folder", "Analisar esta pasta";
    br_parent: "parent folder", "pasta acima";
    br_folders: "Folders ({n})", "Pastas ({n})";
    hint_node: "Node project", "Projeto Node";
    hint_git: "git repository", "repositório git";
    br_tip: "Folders marked Node project or git repository are good candidates.",
        "Pastas marcadas como projeto Node ou repositório git são boas candidatas.";
    cannot_open: "Cannot open {path}: {error}", "Não foi possível abrir {path}: {error}";
    br_move: "move", "mover";
    br_open: "open or analyze", "abrir ou analisar";
    br_back: "parent", "voltar";
    br_analyze_here: "analyze here", "analisar aqui";
    br_hidden: "hidden", "ocultas";
    br_home: "home", "início";
    br_click: "click once to select, again to open", "clique para selecionar, de novo para abrir";
    br_menu: "menu", "menu";

    // --- terminal interface: main menu ---
    menu_tagline: "Find quantum-vulnerable cryptography in your code",
        "Ache criptografia vulnerável a computadores quânticos";
    menu_browse: "Choose a project", "Escolher um projeto";
    menu_browse_hint: "browse folders", "navegar pelas pastas";
    menu_here: "Analyze this folder", "Analisar esta pasta";
    menu_settings: "Settings", "Configurações";
    menu_settings_hint: "language and logo", "idioma e logo";
    menu_quit: "Quit", "Sair";
    menu_select: "select", "selecionar";
    menu_choose: "choose", "escolher";
    key_menu: "menu", "menu";

    // --- terminal interface: settings ---
    settings_title: "settings", "configurações";
    settings_language: "Language", "Idioma";
    settings_logo: "Logo", "Logo";
    settings_saved_in: "saved in", "salvo em";
    settings_drag: "drag the logo to turn it", "arraste o logo para girá-lo";
    key_change: "change", "alterar";
    logo_planet: "Planet", "Planeta";
    logo_lock: "Padlock", "Cadeado";
    logo_key: "Key", "Chave";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_tags() {
        for (tag, lang) in [
            ("en", Lang::En),
            ("EN-us", Lang::En),
            ("pt", Lang::PtBr),
            ("pt-BR", Lang::PtBr),
            ("pt_BR", Lang::PtBr),
            ("pt_BR.UTF-8", Lang::PtBr),
        ] {
            assert_eq!(Lang::parse(tag), Some(lang), "{tag}");
        }
        assert_eq!(Lang::parse("fr"), None);
        assert_eq!(Lang::parse(""), None);
    }

    #[test]
    fn locales_pick_portuguese_or_fall_back_to_english() {
        assert_eq!(Lang::from_locale(Some("pt_BR.UTF-8")), Lang::PtBr);
        assert_eq!(Lang::from_locale(Some("PT_PT")), Lang::PtBr);
        assert_eq!(Lang::from_locale(Some("en_US.UTF-8")), Lang::En);
        assert_eq!(Lang::from_locale(Some("de_DE")), Lang::En);
        assert_eq!(Lang::from_locale(Some("C")), Lang::En);
        assert_eq!(Lang::from_locale(None), Lang::En);
    }

    #[test]
    fn the_other_language_toggles() {
        assert_eq!(Lang::En.other(), Lang::PtBr);
        assert_eq!(Lang::PtBr.other(), Lang::En);
        assert_eq!(Lang::En.other().other(), Lang::En);
    }

    #[test]
    fn placeholders_are_filled() {
        assert_eq!(
            fill("{n} of {m} {n}", &[("n", &1), ("m", &"two")]),
            "1 of two 1"
        );
        assert_eq!(fill("nothing", &[("n", &1)]), "nothing");
        assert_eq!(fill("{missing}", &[("n", &1)]), "{missing}");
    }

    #[test]
    fn badges_are_six_columns_wide_in_every_language() {
        for lang in [Lang::En, Lang::PtBr] {
            for urgency in [Urgency::High, Urgency::Medium, Urgency::Low, Urgency::None] {
                assert_eq!(
                    lang.badge(urgency).chars().count(),
                    6,
                    "{lang:?} {urgency:?}"
                );
            }
        }
    }

    #[test]
    fn every_template_has_the_same_placeholders_in_both_languages() {
        let placeholders = |text: &str| {
            let mut found: Vec<String> = text
                .split('{')
                .skip(1)
                .filter_map(|rest| rest.split_once('}').map(|(name, _)| name.to_string()))
                .collect();
            found.sort();
            found
        };
        let pairs = [
            (EN.report_title, PT_BR.report_title),
            (EN.report_kb, PT_BR.report_kb),
            (EN.report_files_detected, PT_BR.report_files_detected),
            (EN.report_uses_found, PT_BR.report_uses_found),
            (EN.report_need_urgent, PT_BR.report_need_urgent),
            (EN.header_info, PT_BR.header_info),
            (EN.findings_title, PT_BR.findings_title),
            (EN.hidden_by_filters, PT_BR.hidden_by_filters),
            (EN.footer_uses, PT_BR.footer_uses),
            (EN.footer_broken, PT_BR.footer_broken),
            (EN.footer_weakened, PT_BR.footer_weakened),
            (EN.footer_safe, PT_BR.footer_safe),
            (EN.footer_filters, PT_BR.footer_filters),
            (EN.status_rescanned, PT_BR.status_rescanned),
            (EN.br_folders, PT_BR.br_folders),
            (EN.cannot_open, PT_BR.cannot_open),
            (EN.risk_broken_by, PT_BR.risk_broken_by),
            (EN.risk_weakened_by, PT_BR.risk_weakened_by),
        ];
        for (en, pt) in pairs {
            assert_eq!(placeholders(en), placeholders(pt), "{en} / {pt}");
        }
    }

    #[test]
    fn risk_sentences_name_the_attack() {
        assert_eq!(
            Lang::En.risk_sentence(QuantumRisk::Broken, &Attack::Shor),
            "broken by Shor's algorithm"
        );
        assert_eq!(
            Lang::PtBr.risk_sentence(QuantumRisk::Weakened, &Attack::Grover),
            "enfraquecido pelo algoritmo de Grover"
        );
        assert_eq!(
            Lang::PtBr.risk_sentence(QuantumRisk::Safe, &Attack::None),
            "seguro"
        );
    }

    #[test]
    fn machine_risk_names_are_translated() {
        assert_eq!(Lang::PtBr.risk_key("broken"), "quebrado");
        assert_eq!(Lang::En.risk_key("weakened"), "weakened");
        assert_eq!(Lang::PtBr.risk_key("other"), "other");
    }
}
