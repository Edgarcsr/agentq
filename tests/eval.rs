use agentq::eval::evaluate;
use std::path::Path;

fn corpus() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("eval/corpus/node")
}

/// Regression guard for the quality measured on the annotated Node corpus.
/// If the analyzer gets better, raise the thresholds; if it gets worse, fix it.
#[test]
fn node_corpus_quality_does_not_regress() {
    let evaluation = evaluate(&corpus());
    assert_eq!(evaluation.files, 12);
    assert_eq!(evaluation.expected, 49);

    let tool = &evaluation.tool.overall;
    assert!(tool.precision().unwrap() >= 0.99, "{tool:?}");
    assert!(tool.recall().unwrap() >= 0.95, "{tool:?}");

    let baseline = evaluation.baseline.overall.f1().unwrap();
    assert!(
        tool.f1().unwrap() > baseline + 0.2,
        "agentq should clearly beat keyword search"
    );
}

/// The only misses are the documented limitations in `limitations.js`.
#[test]
fn node_corpus_misses_are_the_documented_limitations() {
    let evaluation = evaluate(&corpus());
    let missed: Vec<_> = evaluation
        .tool
        .missed
        .iter()
        .map(|(file, line, algorithm)| (file.as_str(), *line, algorithm.as_str()))
        .collect();
    assert_eq!(
        missed,
        vec![("limitations.js", 7, "md5"), ("limitations.js", 10, "rsa")]
    );
    assert!(evaluation.tool.spurious.is_empty());
}

#[test]
fn node_corpus_purposes_are_mostly_right() {
    let evaluation = evaluate(&corpus());
    assert!(evaluation.purpose_total >= 40);
    assert!(evaluation.purpose_correct * 100 >= evaluation.purpose_total * 90);
}
