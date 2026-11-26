//! Imports and lexical declarations. Reassigned values are deliberately unknown.

use crate::modules::common::syntax::{text, walk};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Tree};

const TYPES: &[&str] = &[
    "java.security.MessageDigest",
    "java.security.Signature",
    "java.security.KeyPairGenerator",
    "java.security.KeyFactory",
    "javax.crypto.Cipher",
    "javax.crypto.KeyAgreement",
    "javax.crypto.KeyGenerator",
    "javax.crypto.Mac",
    "javax.crypto.KEM",
    "javax.crypto.SecretKeyFactory",
    "javax.crypto.spec.SecretKeySpec",
    "java.security.spec.ECGenParameterSpec",
    "java.security.spec.NamedParameterSpec",
    "java.security.spec.RSAKeyGenParameterSpec",
];

struct Declaration<'t> {
    name: String,
    node: Node<'t>,
    scope: Node<'t>,
    value: Option<Node<'t>>,
    field: bool,
    mutable: bool,
}

pub struct Bindings<'t> {
    imports: HashMap<String, String>,
    wildcards: Vec<String>,
    static_factories: HashSet<String>,
    methods: HashSet<String>,
    declarations: Vec<Declaration<'t>>,
}

impl<'t> Bindings<'t> {
    pub fn collect(tree: &'t Tree, source: &str) -> Self {
        let mut bindings = Self {
            imports: HashMap::new(),
            wildcards: Vec::new(),
            static_factories: HashSet::new(),
            methods: HashSet::new(),
            declarations: Vec::new(),
        };
        walk(tree.root_node(), |node| match node.kind() {
            "import_declaration" => {
                let compact: String = text(node, source)
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect();
                let import = compact.trim_start_matches("import").trim_end_matches(';');
                if let Some(import) = import.strip_prefix("static") {
                    if let Some((owner, member)) = import.rsplit_once('.')
                        && matches!(member, "getInstance" | "*")
                    {
                        // Keep unknown owners too: ambiguous static imports must not match.
                        bindings.static_factories.insert(owner.to_string());
                    }
                } else if let Some(package) = import.strip_suffix(".*") {
                    bindings.wildcards.push(package.to_string());
                } else if let Some((_, name)) = import.rsplit_once('.') {
                    bindings
                        .imports
                        .insert(name.to_string(), import.to_string());
                }
            }
            "method_declaration" => {
                if let Some(name) = node.child_by_field_name("name") {
                    bindings.methods.insert(text(name, source).to_string());
                }
            }
            "variable_declarator"
            | "formal_parameter"
            | "catch_formal_parameter"
            | "enhanced_for_statement"
            | "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "type_parameter" => {
                if let Some(name) = node.child_by_field_name("name").or_else(|| {
                    (node.kind() == "type_parameter")
                        .then(|| node.named_child(0))
                        .flatten()
                }) {
                    bindings.declare(node, name, source);
                }
            }
            "lambda_expression" => {
                if let Some(parameters) = node.child_by_field_name("parameters") {
                    if parameters.kind() == "identifier" {
                        bindings.declare(parameters, parameters, source);
                    } else if parameters.kind() == "inferred_parameters" {
                        let mut cursor = parameters.walk();
                        for parameter in parameters.named_children(&mut cursor) {
                            bindings.declare(parameter, parameter, source);
                        }
                    }
                }
            }
            _ => {}
        });
        // Invalidate by declaration, rather than by name, so a write in another
        // method does not invalidate an unrelated local with the same spelling.
        let mut writes = HashSet::new();
        walk(tree.root_node(), |node| {
            let target = match node.kind() {
                "assignment_expression" => node.child_by_field_name("left"),
                "update_expression" => node.named_child(0),
                _ => None,
            };
            if let Some(target) = target
                && let Some(declaration) = bindings.declaration(target, source)
            {
                writes.insert(declaration.node.id());
            }
        });
        for declaration in &mut bindings.declarations {
            declaration.mutable |= writes.contains(&declaration.node.id());
        }
        bindings
    }

    fn declare(&mut self, node: Node<'t>, name: Node, source: &str) {
        let parent = node.parent();
        let field = parent
            .is_some_and(|p| matches!(p.kind(), "field_declaration" | "constant_declaration"));
        let final_field = parent.is_some_and(|p| {
            let mut cursor = p.walk();
            p.named_children(&mut cursor).any(|child| {
                child.kind() == "modifiers"
                    && text(child, source).split_whitespace().any(|s| s == "final")
            }) || p.kind() == "constant_declaration"
        });
        let mut scope = node.parent().unwrap_or(node);
        while !is_scope(scope) {
            let Some(parent) = scope.parent() else { break };
            scope = parent;
        }
        // The iteration variable is visible in the loop body, not outside it.
        if node.kind() == "enhanced_for_statement" {
            scope = node;
        }
        self.declarations.push(Declaration {
            name: text(name, source).to_string(),
            node,
            scope,
            value: (node.kind() == "variable_declarator")
                .then(|| node.child_by_field_name("value"))
                .flatten(),
            field,
            mutable: field && !final_field,
        });
    }

    fn lookup(&self, name: &str, at: Node, fields_only: bool) -> Option<&Declaration<'t>> {
        self.declarations
            .iter()
            .filter(|d| {
                d.name == name
                    && (!fields_only || d.field)
                    && d.scope.start_byte() <= at.start_byte()
                    && at.end_byte() <= d.scope.end_byte()
                    && (d.field
                        || matches!(
                            d.node.kind(),
                            "class_declaration"
                                | "interface_declaration"
                                | "enum_declaration"
                                | "record_declaration"
                                | "type_parameter"
                        )
                        || d.node.start_byte() <= at.start_byte())
            })
            .min_by_key(|d| d.scope.end_byte() - d.scope.start_byte())
    }

    fn declaration(&self, node: Node, source: &str) -> Option<&Declaration<'t>> {
        match node.kind() {
            "identifier" => self.lookup(text(node, source), node, false),
            "field_access" => {
                let object = node.child_by_field_name("object")?;
                let owner = text(object, source);
                if owner != "this"
                    && self.lookup(owner, node, false).is_some_and(|d| {
                        !matches!(
                            d.node.kind(),
                            "class_declaration"
                                | "interface_declaration"
                                | "enum_declaration"
                                | "record_declaration"
                        )
                    })
                {
                    return None;
                }
                let field = text(node.child_by_field_name("field")?, source);
                let mut parent = node.parent();
                while let Some(scope) = parent {
                    if matches!(scope.kind(), "class_body" | "interface_body" | "enum_body")
                        && (owner == "this"
                            || scope
                                .parent()
                                .and_then(|p| p.child_by_field_name("name"))
                                .is_some_and(|name| text(name, source) == owner))
                    {
                        return self
                            .declarations
                            .iter()
                            .find(|d| d.field && d.name == field && d.scope.id() == scope.id());
                    }
                    parent = scope.parent();
                }
                None
            }
            _ => None,
        }
    }

    /// Identity of the variable `node` names, unique in the file: two locals
    /// spelled `key` in different methods get different identities.
    pub fn variable(&self, node: Node, source: &str) -> Option<String> {
        let declaration = self.declaration(node, source)?;
        if matches!(
            declaration.node.kind(),
            "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "record_declaration"
                | "type_parameter"
        ) {
            return None;
        }
        Some(identity(&declaration.name, declaration.node))
    }

    /// Follows effectively-final locals, final fields and aliases; cycles stop.
    pub fn origin(&self, mut node: Node<'t>, source: &str) -> Option<Node<'t>> {
        for _ in 0..32 {
            match node.kind() {
                "parenthesized_expression" => node = node.named_child(0)?,
                "identifier" | "field_access" => {
                    let Some(declaration) = self.declaration(node, source) else {
                        return (node.kind() == "field_access").then_some(node);
                    };
                    if declaration.mutable {
                        return None;
                    }
                    node = declaration.value?;
                }
                _ => return Some(node),
            }
        }
        None
    }

    /// Resolves only known JDK types, with imports or fully qualified names.
    pub fn class(&self, node: Node, source: &str) -> Option<&'static str> {
        let name = text(node, source);
        let root = name.split('.').next()?;
        if self.lookup(root, node, false).is_some() {
            return None;
        }
        if name.contains('.') {
            return TYPES.iter().copied().find(|class| *class == name);
        }
        if let Some(import) = self.imports.get(name) {
            return TYPES.iter().copied().find(|class| *class == import);
        }
        let mut matches = TYPES.iter().copied().filter(|class| {
            class.rsplit_once('.').is_some_and(|(package, short)| {
                short == name && self.wildcards.iter().any(|p| p == package)
            })
        });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }

    pub fn factory_class(&self, call: Node, source: &str) -> Option<&'static str> {
        if call.kind() != "method_invocation"
            || text(call.child_by_field_name("name")?, source) != "getInstance"
        {
            return None;
        }
        if let Some(object) = call.child_by_field_name("object") {
            self.class(object, source)
        } else if self.static_factories.len() == 1 && !self.methods.contains("getInstance") {
            let owner = self.static_factories.iter().next()?;
            TYPES.iter().copied().find(|class| *class == owner)
        } else {
            None
        }
    }
}

/// Identity of the variable declared by `declaration` (see [`Bindings::variable`]).
pub fn identity(name: &str, declaration: Node) -> String {
    format!("{name}@{}", declaration.start_byte())
}

fn is_scope(node: Node) -> bool {
    matches!(
        node.kind(),
        "program"
            | "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "class_body"
            | "interface_body"
            | "enum_body"
            | "block"
            | "constructor_body"
            | "method_declaration"
            | "constructor_declaration"
            | "lambda_expression"
            | "for_statement"
            | "enhanced_for_statement"
            | "catch_clause"
            | "try_with_resources_statement"
            | "switch_block"
    )
}
