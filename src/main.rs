use agentq::analysis::analyze_directory;
use agentq::finding::{Confidence, QuantumRisk};
use agentq::i18n::Lang;
use agentq::knowledge::KnowledgeBase;
use agentq::report::{Format, Report, render};
use clap::{Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// Find cryptography that quantum computers can break and suggest
/// post-quantum alternatives.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Directory to analyze (the current one, or with --tui a folder to pick).
    path: Option<PathBuf>,

    /// Report format.
    #[arg(short, long, value_enum, default_value_t = FormatArg::Text)]
    format: FormatArg,

    /// Write the report to a file instead of standard output.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Drop findings the analyzer is less sure about.
    #[arg(long, value_enum, default_value_t = ConfidenceArg::Low)]
    min_confidence: ConfidenceArg,

    /// Leave out uses that are already safe against quantum attacks.
    #[arg(long)]
    hide_safe: bool,

    /// Language of the interface and reports: en or pt-BR (default: from the
    /// environment, else en).
    #[arg(long, value_parser = parse_lang)]
    lang: Option<Lang>,

    /// Draw icons from a Nerd Font in the interface (also on with the
    /// AGENTQ_NERD_FONT environment variable). Needs that font in the terminal.
    #[arg(long)]
    nerd_font: bool,

    /// Browse the findings in an interactive terminal interface.
    #[arg(short, long, conflicts_with_all = ["format", "output"])]
    tui: bool,
}

fn parse_lang(tag: &str) -> Result<Lang, String> {
    Lang::parse(tag).ok_or_else(|| format!("unknown language `{tag}`; use en or pt-BR"))
}

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Text,
    Json,
    #[value(alias = "md")]
    Markdown,
}

#[derive(Clone, Copy, ValueEnum)]
enum ConfidenceArg {
    Low,
    Medium,
    High,
}

impl From<FormatArg> for Format {
    fn from(arg: FormatArg) -> Format {
        match arg {
            FormatArg::Text => Format::Text,
            FormatArg::Json => Format::Json,
            FormatArg::Markdown => Format::Markdown,
        }
    }
}

impl From<ConfidenceArg> for Confidence {
    fn from(arg: ConfidenceArg) -> Confidence {
        match arg {
            ConfidenceArg::Low => Confidence::Low,
            ConfidenceArg::Medium => Confidence::Medium,
            ConfidenceArg::High => Confidence::High,
        }
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    if let Some(path) = &args.path
        && !path.is_dir()
    {
        eprintln!("error: `{}` is not a directory", path.display());
        return ExitCode::from(2);
    }

    let lang = args
        .lang
        .or_else(|| agentq::config::Settings::load().lang())
        .unwrap_or_else(Lang::detect);

    if args.tui {
        let from_env =
            std::env::var_os("AGENTQ_NERD_FONT").is_some_and(|v| !v.is_empty() && v != "0");
        agentq::tui::set_nerd_font(args.nerd_font || from_env);
        return match agentq::tui::run(
            args.path.as_deref(),
            args.hide_safe,
            args.min_confidence.into(),
            lang,
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        };
    }

    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    let analysis = analyze_directory(&path, lang);

    let min_confidence = Confidence::from(args.min_confidence);
    let mut assessments = analysis.assessments;
    assessments.retain(|assessment| assessment.finding.confidence >= min_confidence);
    if args.hide_safe {
        assessments.retain(|assessment| assessment.risk != QuantumRisk::Safe);
    }

    let knowledge = KnowledgeBase::embedded();
    let report = Report {
        root: path.display().to_string(),
        files: &analysis.files,
        assessments,
        unassessed: &analysis.unassessed,
        knowledge_version: knowledge.version.clone(),
        lang,
    };
    let rendered = render(&report, args.format.into());

    match args.output {
        Some(path) => {
            if let Err(error) = fs::write(&path, rendered) {
                eprintln!("error: cannot write `{}`: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
        None => println!("{rendered}"),
    }
    ExitCode::SUCCESS
}
