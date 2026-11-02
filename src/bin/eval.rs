//! Scores agentq on an annotated corpus: `cargo run --bin eval -- eval/corpus/node`.

use agentq::eval::{evaluate, render_markdown};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// Measure precision, recall and speed of agentq against a corpus with ground truth.
#[derive(Parser)]
struct Args {
    /// Corpus directory, annotated with `// @expect` markers.
    #[arg(default_value = "eval/corpus/node")]
    corpus: PathBuf,

    /// Write the Markdown report to a file instead of standard output.
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = Args::parse();
    if !args.corpus.is_dir() {
        eprintln!("error: `{}` is not a directory", args.corpus.display());
        return ExitCode::from(2);
    }
    let report = render_markdown(&evaluate(&args.corpus));
    match args.output {
        Some(path) => {
            if let Err(error) = fs::write(&path, report) {
                eprintln!("error: cannot write `{}`: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
        None => println!("{report}"),
    }
    ExitCode::SUCCESS
}
