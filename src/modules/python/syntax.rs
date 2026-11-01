use std::error::Error;
use std::fmt;

use tree_sitter::{Language, Node, Parser, Point, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureKind {
    Import,
    Call,
    Assignment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxStructure {
    pub kind: StructureKind,
    pub range: SourceRange,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxIssueKind {
    ErrorNode,
    MissingNode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxIssue {
    pub kind: SyntaxIssueKind,
    pub node_kind: String,
    pub range: SourceRange,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyntaxAnalysis {
    pub structures: Vec<SyntaxStructure>,
    pub issues: Vec<SyntaxIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxLayerError {
    IncompatibleLanguage(String),
    ParseCancelled,
}

impl fmt::Display for SyntaxLayerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IncompatibleLanguage(message) => {
                write!(
                    formatter,
                    "Tree-sitter rejected the Python grammar: {message}"
                )
            }
            Self::ParseCancelled => formatter.write_str("Tree-sitter did not return a syntax tree"),
        }
    }
}

impl Error for SyntaxLayerError {}

/// Parses Python source without executing it and collects basic syntax structures.
///
/// Lines and columns are 1-based; columns count UTF-8 bytes. End positions are exclusive.
/// Recoverable syntax errors are returned in `issues` alongside structures from the partial tree.
pub fn parse_source(source: &str) -> Result<SyntaxAnalysis, SyntaxLayerError> {
    let mut parser = Parser::new();
    let language: Language = tree_sitter_python::LANGUAGE.into();
    parser
        .set_language(&language)
        .map_err(|error| SyntaxLayerError::IncompatibleLanguage(error.to_string()))?;

    let tree = parser
        .parse(source.as_bytes(), None)
        .ok_or(SyntaxLayerError::ParseCancelled)?;

    Ok(collect_structures(&tree, source))
}

fn collect_structures(tree: &Tree, source: &str) -> SyntaxAnalysis {
    let mut analysis = SyntaxAnalysis::default();
    let mut pending = vec![tree.root_node()];

    while let Some(node) = pending.pop() {
        if let Some(kind) = structure_kind(node.kind()) {
            analysis.structures.push(SyntaxStructure {
                kind,
                range: source_range(node.start_position(), node.end_position()),
                text: node_text(node, source),
            });
        }

        if let Some(kind) = syntax_issue_kind(node.is_error(), node.is_missing()) {
            analysis.issues.push(SyntaxIssue {
                kind,
                node_kind: node.kind().to_string(),
                range: source_range(node.start_position(), node.end_position()),
                text: node_text(node, source),
            });
        }

        let mut cursor = node.walk();
        pending.extend(
            node.children(&mut cursor)
                .collect::<Vec<_>>()
                .into_iter()
                .rev(),
        );
    }

    analysis
}

fn syntax_issue_kind(is_error: bool, is_missing: bool) -> Option<SyntaxIssueKind> {
    if is_missing {
        Some(SyntaxIssueKind::MissingNode)
    } else if is_error {
        Some(SyntaxIssueKind::ErrorNode)
    } else {
        None
    }
}

fn structure_kind(node_kind: &str) -> Option<StructureKind> {
    match node_kind {
        "import_statement" | "import_from_statement" => Some(StructureKind::Import),
        "call" => Some(StructureKind::Call),
        "assignment" | "augmented_assignment" | "named_expression" => {
            Some(StructureKind::Assignment)
        }
        _ => None,
    }
}

fn source_range(start: Point, end: Point) -> SourceRange {
    SourceRange {
        start_line: start.row + 1,
        start_column: start.column + 1,
        end_line: end.row + 1,
        end_column: end.column + 1,
    }
}

fn node_text(node: Node<'_>, source: &str) -> String {
    source
        .get(node.byte_range())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{StructureKind, SyntaxIssueKind, parse_source, syntax_issue_kind};

    #[test]
    fn recognizes_import_forms() {
        let analysis = parse_source(
            "import hashlib\n\
import cryptography.hazmat.primitives.asymmetric as asymmetric\n\
from Crypto.PublicKey import RSA as ImportedRSA\n",
        )
        .expect("Python grammar should initialize");

        let imports: Vec<_> = analysis
            .structures
            .iter()
            .filter(|structure| structure.kind == StructureKind::Import)
            .collect();

        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].text, "import hashlib");
        assert!(imports[1].text.ends_with("as asymmetric"));
        assert!(imports[2].text.ends_with("as ImportedRSA"));
        assert!(analysis.issues.is_empty());
    }

    #[test]
    fn recognizes_calls_and_assignments_across_multiple_lines() {
        let source = "result = client.sign(\n    payload,\n    padding,\n)\n";
        let analysis = parse_source(source).expect("Python grammar should initialize");
        let call = analysis
            .structures
            .iter()
            .find(|structure| structure.kind == StructureKind::Call)
            .expect("method call should be present");
        let assignment = analysis
            .structures
            .iter()
            .find(|structure| structure.kind == StructureKind::Assignment)
            .expect("assignment should be present");

        assert_eq!(call.range.start_line, 1);
        assert_eq!(call.range.start_column, 10);
        assert_eq!(call.range.end_line, 4);
        assert_eq!(call.range.end_column, 2);
        assert_eq!(call.text, "client.sign(\n    payload,\n    padding,\n)");
        assert_eq!(assignment.range.start_line, 1);
        assert_eq!(assignment.range.start_column, 1);
        assert_eq!(assignment.range.end_line, 4);
        assert_eq!(assignment.range.end_column, 2);
        assert!(analysis.issues.is_empty());
    }

    #[test]
    fn recognizes_simple_function_calls_and_method_calls() {
        let analysis = parse_source("value = derive_key(secret)\nclient.sign(value)\n")
            .expect("Python grammar should initialize");
        let calls: Vec<_> = analysis
            .structures
            .iter()
            .filter(|structure| structure.kind == StructureKind::Call)
            .collect();

        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].text, "derive_key(secret)");
        assert_eq!(calls[1].text, "client.sign(value)");
    }

    #[test]
    fn columns_count_utf8_bytes_for_unicode_source() {
        let analysis = parse_source("label = \"π\"; client.sign(data)\n")
            .expect("Python grammar should initialize");
        let call = analysis
            .structures
            .iter()
            .find(|structure| structure.kind == StructureKind::Call)
            .expect("method call should be present");

        assert_eq!(call.range.start_column, 15);
        assert_eq!(call.range.end_column, 32);
    }

    #[test]
    fn does_not_treat_comments_or_string_contents_as_calls() {
        let source = "# client.sign(data)\nmessage = \"client.sign(data)\"\n";
        let analysis = parse_source(source).expect("Python grammar should initialize");

        assert!(
            !analysis
                .structures
                .iter()
                .any(|structure| structure.kind == StructureKind::Call)
        );
        assert!(
            analysis
                .structures
                .iter()
                .any(|structure| structure.kind == StructureKind::Assignment)
        );
        assert!(analysis.issues.is_empty());
    }

    #[test]
    fn reports_error_nodes_and_recovers_valid_structures() {
        let analysis = parse_source("kept = valid_call()\ndef broken(:\n    result = call(\n")
            .expect("parser should return a partial tree");

        assert!(
            analysis
                .issues
                .iter()
                .any(|issue| issue.kind == SyntaxIssueKind::ErrorNode)
        );
        assert!(
            analysis
                .structures
                .iter()
                .any(|structure| structure.kind == StructureKind::Call
                    && structure.text == "valid_call()")
        );
    }

    #[test]
    fn maps_error_and_missing_flags_to_distinct_issue_kinds() {
        assert_eq!(
            syntax_issue_kind(true, false),
            Some(SyntaxIssueKind::ErrorNode)
        );
        assert_eq!(
            syntax_issue_kind(false, true),
            Some(SyntaxIssueKind::MissingNode)
        );
        assert_eq!(syntax_issue_kind(false, false), None);
    }

    #[test]
    fn python_grammar_abi_is_accepted_by_the_tree_sitter_runtime() {
        use tree_sitter::{Language, Parser};

        let mut parser = Parser::new();
        let language: Language = tree_sitter_python::LANGUAGE.into();

        parser
            .set_language(&language)
            .expect("Python grammar ABI should match the Tree-sitter runtime");
        assert!(parser.parse("pass", None).is_some());
    }
}
