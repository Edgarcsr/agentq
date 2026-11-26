use super::bindings::Bindings;
use super::keys::KeyFlow;
use super::{rules, value};
use crate::analyzer::Analyzer;
use crate::detect::Language;
use crate::finding::{Finding, Purpose};
use crate::modules::common::{
    Detection, Value,
    syntax::{text, walk},
};
use crate::parse::parse;
use crate::purpose::{Evidence, infer_purposes};
use std::path::Path;
use tree_sitter::Node;

/// Library given to all Java evidence. JCA is one API split over two
/// packages: a `KeyPairGenerator` (`java.security`) is used by a `KeyAgreement`
/// or `Cipher` (`javax.crypto`), and the file-level guess of
/// [`infer_purposes`] only pairs evidence with keys of the same library.
const JCA: &str = "java.security";

/// Finds explicit JCA/JCE factories and secret-key specifications in Java.
pub struct JavaAnalyzer;

impl Analyzer for JavaAnalyzer {
    fn analyze(&self, path: &Path, source: &str) -> Vec<Finding> {
        let Some(tree) = parse(Language::Java, path, source) else {
            return vec![];
        };
        let bindings = Bindings::collect(&tree, source);
        let mut calls = Vec::new();
        walk(tree.root_node(), |node| {
            if node.kind() == "method_invocation" {
                calls.push(node);
            }
        });
        let context = Context {
            source,
            bindings: &bindings,
            calls: &calls,
        };
        let mut declarators = Vec::new();
        walk(tree.root_node(), |node| {
            if node.kind() == "variable_declarator" {
                declarators.push(node);
            }
        });
        let keys = KeyFlow::new(source, &bindings, &calls, &declarators);
        let mut findings = Vec::new();
        // Variables holding the key behind each finding, by finding index.
        let mut bound = Vec::new();
        let mut evidence = Vec::new();
        walk(tree.root_node(), |node| {
            let detection = if let Some(class) = bindings.factory_class(node, source) {
                let Some(arg) = value::arguments(node).first().copied() else {
                    return;
                };
                let name = value::eval(arg, source, &bindings);
                let detection = name.as_str().and_then(|name| {
                    let parameter = context.configuration(node, class);
                    rules::detect(class, name, &parameter)
                });
                if detection.is_none() {
                    // `Signature.getInstance(config.algorithm())` names no
                    // algorithm, but the key it receives still signs.
                    let purpose = match class {
                        "java.security.Signature" => Some(Purpose::Signature),
                        "javax.crypto.KeyAgreement" => Some(Purpose::KeyExchange),
                        _ => None,
                    };
                    let names = keys.used(node);
                    if let Some(purpose) = purpose
                        && !names.is_empty()
                    {
                        evidence.push(Evidence {
                            purpose,
                            library: JCA.to_string(),
                            algorithm: None,
                            names,
                            source: None,
                        });
                    }
                }
                detection
            } else if context.is_secret_key(node) {
                context.secret_key(node)
            } else {
                None
            };
            let Some(detection) = detection else {
                return;
            };
            if detection.purpose != Purpose::Unknown {
                evidence.push(Evidence {
                    purpose: detection.purpose,
                    library: JCA.to_string(),
                    algorithm: Some(detection.algorithm.to_string()),
                    names: keys.used(node),
                    source: Some(findings.len()),
                });
            }
            bound.push(keys.bound(node));
            findings.push(finding(path, source, node, detection));
        });
        infer_purposes(&mut findings, &bound, &evidence);
        findings
    }
}

struct Context<'a, 't> {
    source: &'a str,
    bindings: &'a Bindings<'t>,
    calls: &'a [Node<'t>],
}

impl<'t> Context<'_, 't> {
    /// Size/curve evidence must belong to this instance and its straight-line
    /// block. Unknown or conflicting initializations do not imply a default.
    fn configuration(&self, factory: Node<'t>, class: &str) -> Value {
        let method = match class {
            "java.security.KeyPairGenerator" => "initialize",
            "javax.crypto.KeyGenerator" | "javax.crypto.Cipher" => "init",
            _ => return Value::Unknown,
        };
        let mut first_initialization = usize::MAX;
        let mut values = self.calls.iter().filter_map(|call| {
            if text(call.child_by_field_name("name")?, self.source) != method
                || call.start_byte() < factory.start_byte()
            {
                return None;
            }
            let origin = self
                .bindings
                .origin(call.child_by_field_name("object")?, self.source)?;
            if origin.id() != factory.id() {
                return None;
            }
            first_initialization = first_initialization.min(call.start_byte());
            if block(*call).is_none() || block(*call) != block(factory) {
                return Some(Value::Unknown);
            }
            let args = value::arguments(*call);
            Some(if class == "javax.crypto.Cipher" {
                args.get(1).map_or(Value::Unknown, |key| {
                    self.key_bits(*key)
                        .map_or(Value::Unknown, |bits| Value::Num(bits as f64))
                })
            } else {
                args.first().map_or(Value::Unknown, |arg| {
                    value::parameter(*arg, self.source, self.bindings)
                })
            })
        });
        let Some(first) = values.next() else {
            return Value::Unknown;
        };
        if !values.all(|value| value == first) {
            return Value::Unknown;
        }
        // A later initialization cannot describe a key already generated with
        // provider defaults, nor a cipher operation performed before `init`.
        let used_before_init = self.calls.iter().any(|call| {
            call.start_byte() < first_initialization
                && call.child_by_field_name("name").is_some_and(|name| {
                    matches!(
                        text(name, self.source),
                        "generateKey"
                            | "generateKeyPair"
                            | "genKeyPair"
                            | "doFinal"
                            | "wrap"
                            | "unwrap"
                    )
                })
                && call
                    .child_by_field_name("object")
                    .and_then(|object| self.bindings.origin(object, self.source))
                    .is_some_and(|origin| origin.id() == factory.id())
        });
        if used_before_init {
            Value::Unknown
        } else {
            first
        }
    }

    fn is_secret_key(&self, node: Node) -> bool {
        node.kind() == "object_creation_expression"
            && node
                .child_by_field_name("type")
                .and_then(|t| self.bindings.class(t, self.source))
                == Some("javax.crypto.spec.SecretKeySpec")
    }

    fn secret_key(&self, node: Node<'t>) -> Option<Detection> {
        let args = value::arguments(node);
        let name = value::eval(*args.last()?, self.source, self.bindings);
        let bytes = match args.len() {
            2 => self.array_length(args[0]),
            4 => value::eval(args[2], self.source, self.bindings).as_u32(),
            _ => None,
        };
        let parameter = bytes
            .and_then(|n| n.checked_mul(8))
            .map_or(Value::Unknown, |n| Value::Num(n as f64));
        rules::detect(
            "javax.crypto.spec.SecretKeySpec",
            name.as_str()?,
            &parameter,
        )
    }

    fn array_length(&self, node: Node<'t>) -> Option<u32> {
        let node = self.bindings.origin(node, self.source)?;
        let initializer = if node.kind() == "array_creation_expression" {
            if text(node.child_by_field_name("type")?, self.source) != "byte" {
                return None;
            }
            if let Some(initializer) = node.child_by_field_name("value") {
                initializer
            } else {
                let mut cursor = node.walk();
                let dimensions: Vec<_> = node
                    .children_by_field_name("dimensions", &mut cursor)
                    .collect();
                if dimensions.len() != 1 {
                    return None;
                }
                let length = dimensions[0].named_child(0)?;
                return value::eval(length, self.source, self.bindings).as_u32();
            }
        } else {
            node
        };
        if initializer.kind() != "array_initializer" {
            return None;
        }
        let mut cursor = initializer.walk();
        let count = initializer
            .named_children(&mut cursor)
            .filter(|n| !matches!(n.kind(), "line_comment" | "block_comment"))
            .count();
        u32::try_from(count).ok()
    }

    fn key_bits(&self, key: Node<'t>) -> Option<u32> {
        let origin = self.bindings.origin(key, self.source)?;
        if self.is_secret_key(origin) {
            return self
                .secret_key(origin)
                .filter(|d| d.algorithm.starts_with("aes-"))?
                .key_size;
        }
        if origin.kind() == "method_invocation"
            && text(origin.child_by_field_name("name")?, self.source) == "generateKey"
        {
            let factory = self
                .bindings
                .origin(origin.child_by_field_name("object")?, self.source)?;
            if self.bindings.factory_class(factory, self.source)
                != Some("javax.crypto.KeyGenerator")
            {
                return None;
            }
            let args = value::arguments(factory);
            let name = value::eval(*args.first()?, self.source, self.bindings);
            if !name.as_str()?.eq_ignore_ascii_case("AES") {
                return None;
            }
            return self
                .configuration(factory, "javax.crypto.KeyGenerator")
                .as_u32();
        }
        None
    }
}

fn block(mut node: Node) -> Option<usize> {
    loop {
        if matches!(
            node.kind(),
            "block" | "constructor_body" | "lambda_expression"
        ) {
            return Some(node.id());
        }
        if matches!(
            node.kind(),
            "if_statement"
                | "for_statement"
                | "enhanced_for_statement"
                | "while_statement"
                | "do_statement"
                | "ternary_expression"
                | "switch_expression"
        ) {
            return None;
        }
        node = node.parent()?;
    }
}

fn finding(path: &Path, source: &str, node: Node, detection: Detection) -> Finding {
    let position = node.start_position();
    Finding {
        file: path.to_path_buf(),
        line: position.row + 1,
        column: position.column + 1,
        snippet: text(node, source)
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(120)
            .collect(),
        library: detection.library.to_string(),
        algorithm: detection.algorithm.to_string(),
        purpose: detection.purpose,
        key_size: detection.key_size,
        confidence: detection.confidence,
    }
}
