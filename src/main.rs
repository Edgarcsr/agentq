use agentq::analysis::analyze_directory;
use agentq::finding::{Confidence, QuantumRisk};
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

    /// Browse the findings in an interactive terminal interface.
    #[arg(short, long, conflicts_with_all = ["format", "output"])]
    tui: bool,
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

    if args.tui {
        return match agentq::tui::run(
            args.path.as_deref(),
            args.hide_safe,
            args.min_confidence.into(),
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        };
    }

    let path = args.path.unwrap_or_else(|| PathBuf::from("."));
    let analysis = analyze_directory(&path);

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
        knowledge_version: knowledge.version.clone(),
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
