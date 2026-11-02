use crate::detect::Language;
use std::path::Path;
use tree_sitter::{Parser, Tree};

/// Parses `source` with the tree-sitter grammar for `language`.
///
/// `path` selects between the JavaScript, TypeScript and TSX grammars, which
/// all map to [`Language::JavaScriptTypeScript`]. Returns `None` when the
/// parser gives up.
pub fn parse(language: Language, path: &Path, source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    match language {
        Language::JavaScriptTypeScript => {
            let grammar = match path.extension().and_then(|ext| ext.to_str()) {
                Some("ts" | "mts" | "cts") => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
                Some("tsx") => tree_sitter_typescript::LANGUAGE_TSX,
                _ => tree_sitter_javascript::LANGUAGE,
            };
            parser.set_language(&grammar.into()).ok()?;
        }
        Language::Python => parser
            .set_language(&tree_sitter_python::LANGUAGE.into())
            .ok()?,
        Language::Java => parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .ok()?,
        Language::Go => parser.set_language(&tree_sitter_go::LANGUAGE.into()).ok()?,
    }
    parser.parse(source, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JS_TS: Language = Language::JavaScriptTypeScript;

    #[test]
    fn parses_javascript() {
        let tree = parse(JS_TS, Path::new("a.js"), "const x = require('crypto');").unwrap();
        assert_eq!(tree.root_node().kind(), "program");
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn parses_jsx_with_the_javascript_grammar() {
        let tree = parse(JS_TS, Path::new("a.jsx"), "const el = <div id=\"a\" />;").unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn parses_typescript_type_annotations() {
        let source = "const n: number = 1; interface A { b: string }";
        let tree = parse(JS_TS, Path::new("a.ts"), source).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn parses_module_specific_typescript_extensions() {
        for file in ["a.mts", "a.cts"] {
            let tree = parse(JS_TS, Path::new(file), "const n: number = 1;").unwrap();
            assert!(!tree.root_node().has_error(), "{file}");
        }
    }

    #[test]
    fn parses_tsx() {
        let source = "const el = <A<string> prop={1 as number} />;";
        let tree = parse(JS_TS, Path::new("a.tsx"), source).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn typescript_syntax_is_not_valid_javascript() {
        let tree = parse(JS_TS, Path::new("a.js"), "const n: number = 1;").unwrap();
        assert!(tree.root_node().has_error());
    }

    #[test]
    fn parses_python_java_and_go() {
        let cases = [
            (
                Language::Python,
                "a.py",
                "import hashlib\nhashlib.md5(b'x')\n",
            ),
            (
                Language::Java,
                "A.java",
                "class A { void f() { MessageDigest.getInstance(\"MD5\"); } }",
            ),
            (
                Language::Go,
                "a.go",
                "package main\nimport \"crypto/md5\"\nfunc main() { md5.New() }\n",
            ),
        ];
        for (language, file, source) in cases {
            let tree = parse(language, Path::new(file), source).unwrap();
            assert!(!tree.root_node().has_error(), "{file}");
        }
    }
}
