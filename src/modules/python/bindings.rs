use crate::modules::common::Value;
use crate::modules::common::syntax::{text, walk};
use std::collections::HashMap;
use tree_sitter::{Node, Tree};

/// Names bound by imports (to their fully qualified name) and by constant
/// assignments. Scoping is ignored: the first binding of a name wins.
#[derive(Debug, Default)]
pub struct Bindings {
    names: HashMap<String, String>,
    consts: HashMap<String, Value>,
}

impl Bindings {
    pub fn collect(tree: &Tree, source: &str) -> Bindings {
        let mut bindings = Bindings::default();
        walk(tree.root_node(), |node| match node.kind() {
            "import_statement" => bindings.collect_import(node, source),
            "import_from_statement" => bindings.collect_import_from(node, source),
            "assignment" => bindings.collect_assignment(node, source),
            _ => {}
        });
        bindings
    }

    fn bind(&mut self, local: &str, qualified: String) {
        self.names.entry(local.to_string()).or_insert(qualified);
    }

    /// `import a.b` binds `a`; `import a.b as c` binds `c` to `a.b`.
    fn collect_import(&mut self, node: Node, source: &str) {
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            match name.kind() {
                "dotted_name" => {
                    let full = text(name, source);
                    let first = full.split('.').next().unwrap_or(full);
                    self.bind(first, first.to_string());
                }
                "aliased_import" => {
                    if let (Some(original), Some(alias)) = (
                        name.child_by_field_name("name"),
                        name.child_by_field_name("alias"),
                    ) {
                        self.bind(text(alias, source), text(original, source).to_string());
                    }
                }
                _ => {}
            }
        }
    }

    /// `from a.b import c as d` binds `d` to `a.b.c`.
    fn collect_import_from(&mut self, node: Node, source: &str) {
        let Some(module) = node.child_by_field_name("module_name") else {
            return;
        };
        if module.kind() != "dotted_name" {
            return; // relative import
        }
        let module = text(module, source);
        let mut cursor = node.walk();
        for name in node.children_by_field_name("name", &mut cursor) {
            let (original, local) = match name.kind() {
                "dotted_name" => (text(name, source), text(name, source)),
                "aliased_import" => {
                    let (Some(original), Some(alias)) = (
                        name.child_by_field_name("name"),
                        name.child_by_field_name("alias"),
                    ) else {
                        continue;
                    };
                    (text(original, source), text(alias, source))
                }
                _ => continue,
            };
            self.bind(local, format!("{module}.{original}"));
        }
    }

    fn collect_assignment(&mut self, node: Node, source: &str) {
        let (Some(left), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) else {
            return;
        };
        if left.kind() != "identifier" {
            return;
        }
        let local = text(left, source);
        if let Some(qualified) = self.resolve(right, source) {
            self.bind(local, qualified);
        } else {
            let value = self.eval(right, source);
            if !value.is_unknown() {
                self.consts.entry(local.to_string()).or_insert(value);
            }
        }
    }

    /// Fully qualified name of an identifier or attribute chain, following imports.
    pub fn resolve(&self, node: Node, source: &str) -> Option<String> {
        let qualified = match node.kind() {
            "identifier" => self.names.get(text(node, source))?.clone(),
            "attribute" => {
                let object = self.resolve(node.child_by_field_name("object")?, source)?;
                format!(
                    "{object}.{}",
                    text(node.child_by_field_name("attribute")?, source)
                )
            }
            _ => return None,
        };
        // PyCryptodome is also published as `Cryptodome`.
        Some(match qualified.strip_prefix("Cryptodome") {
            Some(rest) => format!("Crypto{rest}"),
            None => qualified,
        })
    }

    /// Evaluates literals, lists, constants and references to library constants.
    ///
    /// A reference to something imported (`ec.SECP256R1()`, `ecdsa.NIST256p`)
    /// evaluates to the last component of its name, which is how curves are told apart.
    pub fn eval(&self, node: Node, source: &str) -> Value {
        match node.kind() {
            "string" => string_value(node, source).map_or(Value::Unknown, Value::Str),
            "integer" => text(node, source)
                .replace('_', "")
                .parse::<f64>()
                .map_or(Value::Unknown, Value::Num),
            "identifier" => match self.consts.get(text(node, source)) {
                Some(value) => value.clone(),
                None => self.last_component(node, source),
            },
            "attribute" => self.last_component(node, source),
            "call" => match node.child_by_field_name("function") {
                Some(function) => self.last_component(function, source),
                None => Value::Unknown,
            },
            "list" | "tuple" => {
                let mut cursor = node.walk();
                Value::Array(
                    node.named_children(&mut cursor)
                        .filter(|child| child.kind() != "comment")
                        .map(|child| self.eval(child, source))
                        .collect(),
                )
            }
            "parenthesized_expression" => node
                .named_child(0)
                .map_or(Value::Unknown, |inner| self.eval(inner, source)),
            _ => Value::Unknown,
        }
    }

    fn last_component(&self, node: Node, source: &str) -> Value {
        match self.resolve(node, source) {
            Some(qualified) => Value::Str(qualified.rsplit('.').next().unwrap_or("").to_string()),
            None => Value::Unknown,
        }
    }
}

/// Contents of a string literal; `None` for f-strings with substitutions.
fn string_value(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    let mut content = String::new();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "string_content" => content.push_str(text(child, source)),
            "interpolation" => return None,
            _ => {}
        }
    }
    Some(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::Language;
    use crate::parse::parse;
    use std::path::Path;

    /// Collects bindings for `source`, then resolves and evaluates the expression `probe`.
    fn probe(source: &str, expression: &str) -> (Option<String>, Value) {
        let full = format!("{source}\n__probe = {expression}\n");
        let tree = parse(Language::Python, Path::new("t.py"), &full).unwrap();
        let bindings = Bindings::collect(&tree, &full);
        let mut result = (None, Value::Unknown);
        walk(tree.root_node(), |node| {
            if node.kind() == "assignment"
                && text(node.child_by_field_name("left").unwrap(), &full) == "__probe"
            {
                let right = node.child_by_field_name("right").unwrap();
                result = (bindings.resolve(right, &full), bindings.eval(right, &full));
            }
        });
        result
    }

    #[test]
    fn resolves_plain_aliased_and_from_imports() {
        let src = "import hashlib\nimport cryptography.hazmat as hz\n\
                   from cryptography.hazmat.primitives import hashes as h, serialization";
        assert_eq!(probe(src, "hashlib.md5").0.as_deref(), Some("hashlib.md5"));
        assert_eq!(
            probe(src, "hz.primitives").0.as_deref(),
            Some("cryptography.hazmat.primitives")
        );
        assert_eq!(
            probe(src, "h.SHA1").0.as_deref(),
            Some("cryptography.hazmat.primitives.hashes.SHA1")
        );
        assert_eq!(
            probe(src, "serialization").0.as_deref(),
            Some("cryptography.hazmat.primitives.serialization")
        );
    }

    #[test]
    fn dotted_import_binds_the_first_component() {
        let src = "import cryptography.hazmat.primitives.hashes";
        assert_eq!(
            probe(src, "cryptography.hazmat.primitives.hashes.MD5")
                .0
                .as_deref(),
            Some("cryptography.hazmat.primitives.hashes.MD5")
        );
    }

    #[test]
    fn cryptodome_is_normalized_to_crypto() {
        let src = "from Cryptodome.PublicKey import RSA";
        assert_eq!(
            probe(src, "RSA.generate").0.as_deref(),
            Some("Crypto.PublicKey.RSA.generate")
        );
    }

    #[test]
    fn follows_assignment_aliases() {
        let src = "from cryptography.hazmat.primitives.asymmetric import rsa\ngen = rsa.generate_private_key";
        assert_eq!(
            probe(src, "gen").0.as_deref(),
            Some("cryptography.hazmat.primitives.asymmetric.rsa.generate_private_key")
        );
    }

    #[test]
    fn evaluates_literals_constants_and_lists() {
        let src = "BITS = 2_048\nNAME = 'md5'";
        assert_eq!(probe(src, "BITS").1, Value::Num(2048.0));
        assert_eq!(probe(src, "NAME").1, Value::Str("md5".into()));
        assert_eq!(probe(src, "\"a\"").1, Value::Str("a".into()));
        assert_eq!(
            probe(src, "['RS256', 'ES256']").1,
            Value::Array(vec![Value::Str("RS256".into()), Value::Str("ES256".into())])
        );
    }

    #[test]
    fn f_strings_with_substitutions_are_unknown() {
        assert_eq!(probe("x = 1", "f'sha{x}'").1, Value::Unknown);
    }

    #[test]
    fn library_references_evaluate_to_their_last_component() {
        let src = "from cryptography.hazmat.primitives.asymmetric import ec\nimport ecdsa";
        assert_eq!(
            probe(src, "ec.SECP256R1()").1,
            Value::Str("SECP256R1".into())
        );
        assert_eq!(
            probe(src, "ecdsa.NIST256p").1,
            Value::Str("NIST256p".into())
        );
        assert_eq!(probe(src, "unknown.thing").1, Value::Unknown);
    }
}
