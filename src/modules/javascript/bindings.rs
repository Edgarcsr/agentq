use super::syntax::{string_literal, text, unwrap_expression, walk};
use super::value::{Value, eval};
use std::collections::HashMap;
use tree_sitter::{Node, Tree};

/// A name traced back to a module and a property path inside it.
///
/// `const { createHash: h } = require('node:crypto')` binds `h` to module
/// `crypto` and path `["createHash"]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub module: String,
    pub path: Vec<String>,
}

/// Module-level view of a file: what every imported name refers to, and which
/// names are bound to constant values. Scoping is ignored: the first
/// declaration of a name wins, wherever it appears in the file.
#[derive(Debug, Default)]
pub struct Bindings {
    names: HashMap<String, Resolved>,
    consts: HashMap<String, Value>,
}

/// Pseudo-module for the global object (`globalThis`, `window`, `self`).
const GLOBAL: &str = "global";
/// Pseudo-module for the Web Crypto API.
pub const WEBCRYPTO: &str = "webcrypto";

impl Bindings {
    pub fn collect(tree: &Tree, source: &str) -> Bindings {
        let mut bindings = Bindings::default();
        walk(tree.root_node(), |node| match node.kind() {
            "import_statement" => bindings.collect_import(node, source),
            "variable_declarator" => bindings.collect_declarator(node, source),
            _ => {}
        });
        bindings
    }

    pub fn consts(&self) -> &HashMap<String, Value> {
        &self.consts
    }

    fn bind(&mut self, name: &str, resolved: Resolved) {
        self.names.entry(name.to_string()).or_insert(resolved);
    }

    fn collect_import(&mut self, node: Node, source: &str) {
        let Some(module) = node
            .child_by_field_name("source")
            .and_then(|s| string_literal(s, source))
            .map(|s| normalize_module(&s))
        else {
            return;
        };
        let mut cursor = node.walk();
        for clause in node.named_children(&mut cursor) {
            if clause.kind() != "import_clause" {
                continue;
            }
            let mut inner = clause.walk();
            for part in clause.named_children(&mut inner) {
                match part.kind() {
                    "identifier" => self.bind(
                        text(part, source),
                        Resolved {
                            module: module.clone(),
                            path: vec![],
                        },
                    ),
                    "namespace_import" => {
                        let mut ns = part.walk();
                        if let Some(id) = part
                            .named_children(&mut ns)
                            .find(|c| c.kind() == "identifier")
                        {
                            self.bind(
                                text(id, source),
                                Resolved {
                                    module: module.clone(),
                                    path: vec![],
                                },
                            );
                        }
                    }
                    "named_imports" => {
                        let mut specs = part.walk();
                        for spec in part.named_children(&mut specs) {
                            let Some(name) = spec.child_by_field_name("name") else {
                                continue;
                            };
                            let imported = text(name, source);
                            let local = spec
                                .child_by_field_name("alias")
                                .map_or(imported, |alias| text(alias, source));
                            let path = if imported == "default" {
                                vec![]
                            } else {
                                vec![imported.to_string()]
                            };
                            self.bind(
                                local,
                                Resolved {
                                    module: module.clone(),
                                    path,
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn collect_declarator(&mut self, node: Node, source: &str) {
        let (Some(name), Some(value)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        ) else {
            return;
        };
        let resolved = self.resolve(value, source);
        match name.kind() {
            "identifier" => {
                let local = text(name, source);
                if let Some(resolved) = resolved {
                    self.bind(local, resolved);
                } else {
                    let value = eval(value, source, &self.consts);
                    if !value.is_unknown() {
                        self.consts.entry(local.to_string()).or_insert(value);
                    }
                }
            }
            "object_pattern" => {
                let Some(resolved) = resolved else { return };
                let mut cursor = name.walk();
                for part in name.named_children(&mut cursor) {
                    let (key, local) = match part.kind() {
                        "shorthand_property_identifier_pattern" => {
                            (text(part, source), text(part, source))
                        }
                        "pair_pattern" => {
                            let (Some(key), Some(local)) = (
                                part.child_by_field_name("key"),
                                part.child_by_field_name("value"),
                            ) else {
                                continue;
                            };
                            if local.kind() != "identifier" {
                                continue;
                            }
                            (text(key, source), text(local, source))
                        }
                        _ => continue,
                    };
                    let mut path = resolved.path.clone();
                    path.push(key.to_string());
                    self.bind(
                        local,
                        Resolved {
                            module: resolved.module.clone(),
                            path,
                        },
                    );
                }
            }
            _ => {}
        }
    }

    /// Traces an expression back to a module and property path, following
    /// `require`, dynamic `import()`, member access and previously bound names.
    pub fn resolve(&self, node: Node, source: &str) -> Option<Resolved> {
        let node = unwrap_expression(node);
        let resolved = match node.kind() {
            "identifier" => {
                let name = text(node, source);
                match self.names.get(name) {
                    Some(resolved) => resolved.clone(),
                    None => match name {
                        "globalThis" | "window" | "self" => Resolved {
                            module: GLOBAL.into(),
                            path: vec![],
                        },
                        // Unimported `crypto` is the Web Crypto global.
                        "crypto" => Resolved {
                            module: WEBCRYPTO.into(),
                            path: vec![],
                        },
                        _ => return None,
                    },
                }
            }
            "member_expression" => {
                let mut resolved = self.resolve(node.child_by_field_name("object")?, source)?;
                let property = text(node.child_by_field_name("property")?, source);
                if resolved.module == GLOBAL {
                    if property != "crypto" {
                        return None;
                    }
                    resolved = Resolved {
                        module: WEBCRYPTO.into(),
                        path: vec![],
                    };
                } else {
                    resolved.path.push(property.to_string());
                }
                resolved
            }
            "call_expression" => {
                let function = node.child_by_field_name("function")?;
                let is_loader = function.kind() == "import"
                    || (function.kind() == "identifier"
                        && text(function, source) == "require"
                        && !self.names.contains_key("require"));
                if !is_loader {
                    return None;
                }
                let arguments = node.child_by_field_name("arguments")?;
                let specifier = string_literal(arguments.named_child(0)?, source)?;
                Resolved {
                    module: normalize_module(&specifier),
                    path: vec![],
                }
            }
            _ => return None,
        };
        Some(normalize_webcrypto(resolved))
    }
}

/// `node:crypto` and `crypto` are the same module.
fn normalize_module(specifier: &str) -> String {
    specifier
        .strip_prefix("node:")
        .unwrap_or(specifier)
        .to_string()
}

/// `crypto.subtle` and `crypto.webcrypto.subtle` from `node:crypto` are the Web Crypto API.
fn normalize_webcrypto(mut resolved: Resolved) -> Resolved {
    if resolved.module == "crypto" {
        match resolved.path.first().map(String::as_str) {
            Some("webcrypto") => {
                resolved.module = WEBCRYPTO.into();
                resolved.path.remove(0);
            }
            Some("subtle") => resolved.module = WEBCRYPTO.into(),
            _ => {}
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detect::Language;
    use crate::parse::parse;
    use std::path::Path;

    /// Collects bindings for `source` and resolves the expression `probe`.
    fn resolve_in(source: &str, probe: &str) -> Option<Resolved> {
        let full = format!("{source}\nconst __probe = {probe};");
        let tree = parse(Language::JavaScriptTypeScript, Path::new("t.ts"), &full).unwrap();
        let bindings = Bindings::collect(&tree, &full);
        let mut result = None;
        walk(tree.root_node(), |node| {
            if node.kind() == "variable_declarator"
                && text(node.child_by_field_name("name").unwrap(), &full) == "__probe"
            {
                result = bindings.resolve(node.child_by_field_name("value").unwrap(), &full);
            }
        });
        result
    }

    fn resolved(module: &str, path: &[&str]) -> Option<Resolved> {
        Some(Resolved {
            module: module.into(),
            path: path.iter().map(|s| s.to_string()).collect(),
        })
    }

    #[test]
    fn resolves_require_and_member_access() {
        let src = "const crypto = require('crypto');";
        assert_eq!(resolve_in(src, "crypto"), resolved("crypto", &[]));
        assert_eq!(
            resolve_in(src, "crypto.createHash"),
            resolved("crypto", &["createHash"])
        );
    }

    #[test]
    fn strips_the_node_prefix() {
        let src = "const c = require('node:crypto');";
        assert_eq!(resolve_in(src, "c"), resolved("crypto", &[]));
    }

    #[test]
    fn resolves_default_namespace_and_named_imports() {
        let src = "import a from 'crypto'; import * as b from 'crypto'; \
                   import { createHash as h, sign } from 'node:crypto';";
        assert_eq!(resolve_in(src, "a"), resolved("crypto", &[]));
        assert_eq!(resolve_in(src, "b"), resolved("crypto", &[]));
        assert_eq!(resolve_in(src, "h"), resolved("crypto", &["createHash"]));
        assert_eq!(resolve_in(src, "sign"), resolved("crypto", &["sign"]));
    }

    #[test]
    fn resolves_destructured_requires_with_renames() {
        let src = "const { generateKeyPairSync, createHash: h } = require('crypto');";
        assert_eq!(
            resolve_in(src, "generateKeyPairSync"),
            resolved("crypto", &["generateKeyPairSync"])
        );
        assert_eq!(resolve_in(src, "h"), resolved("crypto", &["createHash"]));
    }

    #[test]
    fn resolves_dynamic_import_and_await() {
        let src = "const c = await import('crypto');";
        assert_eq!(resolve_in(src, "c.sign"), resolved("crypto", &["sign"]));
    }

    #[test]
    fn follows_aliases_of_members() {
        let src = "const forge = require('node-forge'); const rsa = forge.pki.rsa;";
        assert_eq!(
            resolve_in(src, "rsa.generateKeyPair"),
            resolved("node-forge", &["pki", "rsa", "generateKeyPair"])
        );
    }

    #[test]
    fn unimported_crypto_and_global_access_are_web_crypto() {
        assert_eq!(
            resolve_in("", "crypto.subtle.sign"),
            resolved("webcrypto", &["subtle", "sign"])
        );
        assert_eq!(
            resolve_in("", "globalThis.crypto.subtle"),
            resolved("webcrypto", &["subtle"])
        );
        assert_eq!(
            resolve_in("", "window.crypto.subtle"),
            resolved("webcrypto", &["subtle"])
        );
    }

    #[test]
    fn node_crypto_subtle_and_webcrypto_are_web_crypto() {
        let src = "import { webcrypto } from 'node:crypto'; import crypto from 'node:crypto';";
        assert_eq!(
            resolve_in(src, "webcrypto.subtle.sign"),
            resolved("webcrypto", &["subtle", "sign"])
        );
        assert_eq!(
            resolve_in(src, "crypto.subtle.digest"),
            resolved("webcrypto", &["subtle", "digest"])
        );
    }

    #[test]
    fn imported_crypto_is_not_the_web_crypto_global() {
        let src = "import crypto from 'node:crypto';";
        assert_eq!(
            resolve_in(src, "crypto.createHash"),
            resolved("crypto", &["createHash"])
        );
    }

    #[test]
    fn unknown_names_and_computed_values_do_not_resolve() {
        assert_eq!(resolve_in("", "foo.bar"), None);
        assert_eq!(resolve_in("const x = require(name);", "x"), None);
        assert_eq!(resolve_in("const x = 5;", "x"), None);
    }

    #[test]
    fn collects_constants_including_objects() {
        let src = "const ALG = 'rsa'; const BITS = 2048; const OPTS = { modulusLength: BITS };";
        let tree = parse(Language::JavaScriptTypeScript, Path::new("t.js"), src).unwrap();
        let bindings = Bindings::collect(&tree, src);
        assert_eq!(bindings.consts()["ALG"], Value::Str("rsa".into()));
        assert_eq!(bindings.consts()["BITS"], Value::Num(2048.0));
        assert_eq!(
            bindings.consts()["OPTS"].get("modulusLength").as_u32(),
            Some(2048)
        );
    }
}
