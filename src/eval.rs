//! Evaluation harness: scores the analyzer against a corpus annotated with
//! ground truth, and against a naive keyword search for comparison.
//!
//! A corpus file marks every cryptographic use with a trailing comment on the
//! line where the call starts:
//!
//! ```text
//! createHash('md5'); // @expect md5:hash
//! x.sign(); // @expect rsa:signature, sha2-256:hash
//! ```
//!
//! The purpose after the colon is optional. Lines without a marker are
//! expected to produce no finding, which is how false positives are counted.

use crate::analysis::analyze_files;
use crate::detect::scan_source_files;
use crate::finding::Purpose;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// Number of times the analysis is repeated to measure its speed.
const TIMING_RUNS: u32 = 20;

/// One annotated use of cryptography.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectation {
    pub file: String,
    pub line: usize,
    pub algorithm: String,
    pub purpose: Option<Purpose>,
}

/// A use identified by location and algorithm, as predicted by a tool.
pub type Hit = (String, usize, String);

const MARKER: &str = "// @expect";
const PYTHON_MARKER: &str = "# @expect";

/// Removes the annotation from a line so it cannot influence a tool.
pub fn strip_marker(line: &str) -> &str {
    [MARKER, PYTHON_MARKER]
        .into_iter()
        .filter_map(|marker| line.find(marker))
        .min()
        .map_or(line, |index| &line[..index])
}

/// Expectations declared in the markers of `source`, which belongs to `file`.
pub fn parse_expectations(file: &str, source: &str) -> Vec<Expectation> {
    let mut expectations = Vec::new();
    let marker = if file.ends_with(".py") {
        PYTHON_MARKER
    } else {
        MARKER
    };
    for (index, line) in source.lines().enumerate() {
        let Some(position) = line.find(marker) else {
            continue;
        };
        for item in line[position + marker.len()..].split(',') {
            let (algorithm, purpose) = item.trim().split_once(':').unwrap_or((item.trim(), ""));
            if algorithm.is_empty() {
                continue;
            }
            expectations.push(Expectation {
                file: file.to_string(),
                line: index + 1,
                algorithm: algorithm.to_string(),
                purpose: serde_json::from_str(&format!("\"{purpose}\"")).ok(),
            });
        }
    }
    expectations
}

/// A deliberately naive detector: any word on a line that names an algorithm.
/// It stands for what a developer would find with `grep`.
pub fn keyword_baseline(file: &str, source: &str) -> BTreeSet<Hit> {
    let mut hits = BTreeSet::new();
    for (index, line) in source.lines().enumerate() {
        let lowered = strip_marker(line).to_ascii_lowercase();
        let tokens: Vec<&str> = lowered
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|token| !token.is_empty())
            .collect();
        for (position, token) in tokens.iter().enumerate() {
            let next = tokens.get(position + 1).copied().unwrap_or("");
            let algorithm = match (*token, next) {
                ("rsa", _) => "rsa",
                ("dsa", _) => "dsa",
                ("ecdsa", _) => "ecdsa",
                ("ecdh", _) => "ecdh",
                ("ed25519", _) => "ed25519",
                ("x25519", _) => "x25519",
                ("md5", _) => "md5",
                ("sha1", _) | ("sha", "1") => "sha1",
                ("sha256", _) | ("sha", "256") => "sha2-256",
                ("sha384", _) | ("sha", "384") => "sha2-384",
                ("sha512", _) | ("sha", "512") => "sha2-512",
                ("aes128", _) | ("aes", "128") => "aes-128",
                ("aes192", _) | ("aes", "192") => "aes-192",
                ("aes256", _) | ("aes", "256") => "aes-256",
                ("3des", _) | ("des3", _) | ("tripledes", _) => "3des",
                ("chacha20", _) => "chacha20",
                _ => continue,
            };
            hits.insert((file.to_string(), index + 1, algorithm.to_string()));
        }
    }
    hits
}

struct LexicalRule {
    pattern: &'static str,
    algorithm: &'static str,
}

const PYTHON_LEXICAL_RULES: &[LexicalRule] = &[
    LexicalRule {
        pattern: "rsa.generate_private_key(",
        algorithm: "rsa",
    },
    LexicalRule {
        pattern: "ec.generate_private_key(",
        algorithm: "ecdsa",
    },
    LexicalRule {
        pattern: "dh.generate_parameters(",
        algorithm: "dh",
    },
    LexicalRule {
        pattern: "RSA.generate(",
        algorithm: "rsa",
    },
    LexicalRule {
        pattern: "ECC.generate(",
        algorithm: "ecdsa",
    },
    LexicalRule {
        pattern: "DSA.generate(",
        algorithm: "dsa",
    },
];

const PYTHON_OPERATIONS: &[(&str, &str)] = &[
    (".sign(", "signature"),
    (".verify(", "signature"),
    (".exchange(", "key_exchange"),
    (".encrypt(", "encryption"),
    (".decrypt(", "encryption"),
];

/// Legacy line-based Python detection, retained only as an evaluation baseline.
pub fn lexical_python_baseline(file: &str, source: &str) -> BTreeSet<Hit> {
    let mut hits = BTreeSet::new();
    let mut tracked = BTreeMap::new();

    for (index, raw_line) in source.lines().enumerate() {
        let line = strip_marker(raw_line);
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let has_api = PYTHON_LEXICAL_RULES
            .iter()
            .any(|rule| line.contains(rule.pattern));
        let target = python_assignment_target(line);
        if let Some(target) = target
            && !has_api
        {
            tracked.remove(target);
        }

        for rule in PYTHON_LEXICAL_RULES {
            if line.contains(rule.pattern) {
                hits.insert((file.to_string(), index + 1, rule.algorithm.to_string()));
                if let Some(target) = target {
                    tracked.insert(target.to_string(), rule.algorithm);
                }
            }
        }

        for (variable, algorithm) in &tracked {
            for (method, purpose) in PYTHON_OPERATIONS {
                if !python_method_call(line, variable, method) {
                    continue;
                }
                let algorithm = match (*purpose, *algorithm) {
                    ("signature", "rsa" | "ecdsa" | "dsa") => *algorithm,
                    ("key_exchange", "dh") => "dh",
                    ("key_exchange", "ecdsa") => "ecdh",
                    ("encryption", "rsa") => "rsa",
                    _ => continue,
                };
                hits.insert((file.to_string(), index + 1, algorithm.to_string()));
            }
        }
    }
    hits
}

fn python_assignment_target(line: &str) -> Option<&str> {
    let (target, _) = line.split_once('=')?;
    let target = target.trim();
    let mut characters = target.chars();
    let first = characters.next()?;
    (first == '_' || first.is_ascii_alphabetic())
        .then_some(target)
        .filter(|_| {
            characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
        })
}

fn python_method_call(line: &str, variable: &str, method: &str) -> bool {
    let pattern = format!("{variable}{method}");
    line.match_indices(&pattern).any(|(start, _)| {
        !line[..start].chars().next_back().is_some_and(|character| {
            character == '.' || character == '_' || character.is_ascii_alphanumeric()
        })
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
}

impl Counts {
    pub fn precision(&self) -> Option<f64> {
        ratio(
            self.true_positives,
            self.true_positives + self.false_positives,
        )
    }

    pub fn recall(&self) -> Option<f64> {
        ratio(
            self.true_positives,
            self.true_positives + self.false_negatives,
        )
    }

    pub fn f1(&self) -> Option<f64> {
        let (p, r) = (self.precision()?, self.recall()?);
        (p + r > 0.0).then(|| 2.0 * p * r / (p + r))
    }

    fn add(&mut self, other: Counts) {
        self.true_positives += other.true_positives;
        self.false_positives += other.false_positives;
        self.false_negatives += other.false_negatives;
    }
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

/// Scores of one detector.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scores {
    pub overall: Counts,
    pub by_algorithm: BTreeMap<String, Counts>,
    pub missed: BTreeSet<Hit>,
    pub spurious: BTreeSet<Hit>,
}

/// Compares predicted hits with the expected ones.
pub fn score(expected: &BTreeSet<Hit>, predicted: &BTreeSet<Hit>) -> Scores {
    let mut scores = Scores::default();
    for hit in expected.union(predicted) {
        let entry = scores.by_algorithm.entry(hit.2.clone()).or_default();
        match (expected.contains(hit), predicted.contains(hit)) {
            (true, true) => entry.true_positives += 1,
            (false, true) => {
                entry.false_positives += 1;
                scores.spurious.insert(hit.clone());
            }
            (true, false) => {
                entry.false_negatives += 1;
                scores.missed.insert(hit.clone());
            }
            (false, false) => unreachable!("hit comes from the union"),
        }
    }
    for counts in scores.by_algorithm.values() {
        scores.overall.add(*counts);
    }
    scores
}

/// Everything measured in one evaluation run.
#[derive(Debug)]
pub struct Evaluation {
    pub files: usize,
    pub lines: usize,
    pub expected: usize,
    pub tool: Scores,
    pub baseline: Scores,
    /// Scores of the legacy lexical detector on Python files only.
    pub python_lexical: Option<Scores>,
    /// Correctly located uses whose purpose was annotated, and how many of them the tool got right.
    pub purpose_correct: usize,
    pub purpose_total: usize,
    /// Average time of one analysis of the whole corpus.
    pub analysis_time: Duration,
}

/// Evaluates the analyzer and the baseline on the corpus under `root`.
pub fn evaluate(root: &Path) -> Evaluation {
    let files = scan_source_files(root);
    let relative = |path: &Path| {
        path.strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string()
    };

    let mut expectations = Vec::new();
    let mut baseline_hits = BTreeSet::new();
    let mut python_expectations = BTreeSet::new();
    let mut python_lexical_hits = BTreeSet::new();
    let mut has_python_files = false;
    let mut lines = 0;
    for (path, _) in &files {
        let Ok(source) = fs::read_to_string(path) else {
            continue;
        };
        let name = relative(path);
        lines += source.lines().count();
        let file_expectations = parse_expectations(&name, &source);
        if path.extension().is_some_and(|extension| extension == "py") {
            has_python_files = true;
            python_expectations.extend(
                file_expectations
                    .iter()
                    .map(|e| (e.file.clone(), e.line, e.algorithm.clone())),
            );
            python_lexical_hits.extend(lexical_python_baseline(&name, &source));
        }
        expectations.extend(file_expectations);
        baseline_hits.extend(keyword_baseline(&name, &source));
    }

    let started = Instant::now();
    let mut findings = Vec::new();
    for _ in 0..TIMING_RUNS {
        findings = analyze_files(&files);
    }
    let analysis_time = started.elapsed() / TIMING_RUNS;

    let expected: BTreeSet<Hit> = expectations
        .iter()
        .map(|e| (e.file.clone(), e.line, e.algorithm.clone()))
        .collect();
    let predicted: BTreeSet<Hit> = findings
        .iter()
        .map(|f| (relative(&f.file), f.line, f.algorithm.clone()))
        .collect();

    let mut purpose_total = 0;
    let mut purpose_correct = 0;
    for expectation in &expectations {
        let Some(purpose) = expectation.purpose else {
            continue;
        };
        let found = findings.iter().find(|f| {
            relative(&f.file) == expectation.file
                && f.line == expectation.line
                && f.algorithm == expectation.algorithm
        });
        if let Some(finding) = found {
            purpose_total += 1;
            if finding.purpose == purpose {
                purpose_correct += 1;
            }
        }
    }

    Evaluation {
        files: files.len(),
        lines,
        expected: expected.len(),
        tool: score(&expected, &predicted),
        baseline: score(&expected, &baseline_hits),
        python_lexical: has_python_files.then(|| score(&python_expectations, &python_lexical_hits)),
        purpose_correct,
        purpose_total,
        analysis_time,
    }
}

fn percent(value: Option<f64>) -> String {
    value.map_or("-".to_string(), |v| format!("{:.1}%", v * 100.0))
}

fn row(name: &str, counts: &Counts) -> String {
    format!(
        "| {name} | {} | {} | {} | {} | {} | {} |",
        counts.true_positives,
        counts.false_positives,
        counts.false_negatives,
        percent(counts.precision()),
        percent(counts.recall()),
        percent(counts.f1()),
    )
}

/// Markdown report, ready to paste into the thesis.
pub fn render_markdown(evaluation: &Evaluation) -> String {
    let mut out = String::new();
    let header = "| | TP | FP | FN | Precision | Recall | F1 |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: |";

    writeln!(out, "# Evaluation\n").unwrap();
    writeln!(
        out,
        "Corpus: {} files, {} lines, {} annotated cryptographic uses.\n",
        evaluation.files, evaluation.lines, evaluation.expected
    )
    .unwrap();

    writeln!(out, "## Overall\n\n{header}").unwrap();
    writeln!(out, "{}", row("agentq", &evaluation.tool.overall)).unwrap();
    writeln!(
        out,
        "{}",
        row("keyword search", &evaluation.baseline.overall)
    )
    .unwrap();
    if let Some(lexical) = &evaluation.python_lexical {
        writeln!(out, "{}", row("Python lexical baseline", &lexical.overall)).unwrap();
    }

    writeln!(out, "\n## By algorithm (agentq)\n\n{header}").unwrap();
    for (algorithm, counts) in &evaluation.tool.by_algorithm {
        writeln!(out, "{}", row(algorithm, counts)).unwrap();
    }

    writeln!(out, "\n## By algorithm (keyword search)\n\n{header}").unwrap();
    for (algorithm, counts) in &evaluation.baseline.by_algorithm {
        writeln!(out, "{}", row(algorithm, counts)).unwrap();
    }

    if let Some(lexical) = &evaluation.python_lexical {
        writeln!(
            out,
            "\n## By algorithm (Python lexical baseline)\n\n{header}"
        )
        .unwrap();
        for (algorithm, counts) in &lexical.by_algorithm {
            writeln!(out, "{}", row(algorithm, counts)).unwrap();
        }
    }

    writeln!(out, "\n## Purpose\n").unwrap();
    writeln!(
        out,
        "Of {} located uses with an annotated purpose, agentq assigned the right purpose to {} ({}).",
        evaluation.purpose_total,
        evaluation.purpose_correct,
        percent(ratio(evaluation.purpose_correct, evaluation.purpose_total)),
    )
    .unwrap();

    writeln!(out, "\n## Speed\n").unwrap();
    let seconds = evaluation.analysis_time.as_secs_f64();
    writeln!(
        out,
        "Analyzing the corpus takes {:.1} ms on average over {TIMING_RUNS} runs ({:.0} files/s, {:.0} lines/s).",
        seconds * 1000.0,
        evaluation.files as f64 / seconds,
        evaluation.lines as f64 / seconds,
    )
    .unwrap();

    for (title, hits) in [
        (
            "Missed by agentq (false negatives)",
            &evaluation.tool.missed,
        ),
        (
            "Reported by agentq but not annotated (false positives)",
            &evaluation.tool.spurious,
        ),
    ] {
        writeln!(out, "\n## {title}\n").unwrap();
        if hits.is_empty() {
            writeln!(out, "None.").unwrap();
        }
        for (file, line, algorithm) in hits {
            writeln!(out, "- `{file}:{line}` {algorithm}").unwrap();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(file: &str, line: usize, algorithm: &str) -> Hit {
        (file.into(), line, algorithm.into())
    }

    #[test]
    fn parses_markers_with_and_without_purpose() {
        let source = "a(); // @expect rsa\nb();\nc(); // @expect sha1:hash, ecdsa:signature\n";
        let parsed = parse_expectations("f.js", source);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].algorithm, "rsa");
        assert_eq!(parsed[0].purpose, None);
        assert_eq!((parsed[1].line, parsed[1].algorithm.as_str()), (3, "sha1"));
        assert_eq!(parsed[1].purpose, Some(Purpose::Hash));
        assert_eq!(parsed[2].purpose, Some(Purpose::Signature));
    }

    #[test]
    fn markers_are_hidden_from_the_baseline() {
        assert_eq!(strip_marker("x(); // @expect rsa"), "x(); ");
        assert!(keyword_baseline("f.js", "x(); // @expect rsa:signature").is_empty());
    }

    #[test]
    fn parses_python_markers_and_hides_them_from_baselines() {
        let source = "noop()  # @expect rsa:signature\n";
        let parsed = parse_expectations("f.py", source);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].purpose, Some(Purpose::Signature));
        assert!(keyword_baseline("f.py", source).is_empty());
        assert!(lexical_python_baseline("f.py", source).is_empty());
    }

    #[test]
    fn lexical_python_baseline_tracks_and_invalidates_simple_assignments() {
        let source = "key = rsa.generate_private_key()\nkey = load_key()\nkey.sign(data)\n";
        let hits = lexical_python_baseline("f.py", source);
        assert_eq!(hits, [hit("f.py", 1, "rsa")].into_iter().collect());
    }

    #[test]
    fn baseline_matches_keywords_anywhere_on_a_line() {
        let source =
            "// uses RSA and md5\ncreateHash('SHA-256');\nconst k = 'aes-256-gcm';\nplain();";
        let hits = keyword_baseline("f.js", source);
        assert!(hits.contains(&hit("f.js", 1, "rsa")));
        assert!(hits.contains(&hit("f.js", 1, "md5")));
        assert!(hits.contains(&hit("f.js", 2, "sha2-256")));
        assert!(hits.contains(&hit("f.js", 3, "aes-256")));
        assert_eq!(hits.len(), 4);
    }

    #[test]
    fn scoring_counts_true_false_positives_and_negatives() {
        let expected: BTreeSet<Hit> = [hit("a", 1, "rsa"), hit("a", 2, "md5"), hit("a", 3, "rsa")]
            .into_iter()
            .collect();
        let predicted: BTreeSet<Hit> = [hit("a", 1, "rsa"), hit("a", 3, "rsa"), hit("a", 9, "md5")]
            .into_iter()
            .collect();
        let scores = score(&expected, &predicted);
        assert_eq!(
            scores.overall,
            Counts {
                true_positives: 2,
                false_positives: 1,
                false_negatives: 1
            }
        );
        assert_eq!(scores.by_algorithm["rsa"].true_positives, 2);
        assert_eq!(scores.by_algorithm["md5"].false_positives, 1);
        assert_eq!(scores.missed, [hit("a", 2, "md5")].into_iter().collect());
        assert_eq!(scores.spurious, [hit("a", 9, "md5")].into_iter().collect());
    }

    #[test]
    fn metrics_follow_their_definitions() {
        let counts = Counts {
            true_positives: 6,
            false_positives: 2,
            false_negatives: 4,
        };
        assert_eq!(counts.precision(), Some(0.75));
        assert_eq!(counts.recall(), Some(0.6));
        let f1 = counts.f1().unwrap();
        assert!((f1 - 2.0 * 0.75 * 0.6 / 1.35).abs() < 1e-12);
        assert_eq!(Counts::default().precision(), None);
        assert_eq!(Counts::default().f1(), None);
    }
}
