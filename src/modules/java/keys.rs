//! Follows JCA keys through variables, so a generated key takes the purpose of
//! the operation that receives it.
//!
//! ```java
//! var generator = KeyPairGenerator.getInstance("RSA");   // key family, no purpose
//! KeyPair pair = generator.generateKeyPair();             // derived from `generator`
//! var signer = Signature.getInstance("SHA256withRSA");
//! signer.initSign(pair.getPrivate());                     // `pair` is used to sign
//! ```
//!
//! Variables are identified by their declaration (see [`Bindings::variable`]),
//! not by spelling, so `pair` in one method never links to `pair` in another.

use super::bindings::{Bindings, identity};
use crate::modules::common::syntax::text;
use tree_sitter::Node;

/// Calls that turn a generator, factory or key pair into keys.
const KEY_METHODS: &[&str] = &[
    "generateKeyPair",
    "genKeyPair",
    "getPrivate",
    "getPublic",
    "generatePrivate",
    "generatePublic",
];

/// Calls that hand a key to an operation: `signature.initSign(key)`,
/// `cipher.init(Cipher.ENCRYPT_MODE, key)`, `agreement.doPhase(key, true)`.
const USE_METHODS: &[&str] = &["initSign", "initVerify", "init", "doPhase"];

pub struct KeyFlow<'a, 't> {
    source: &'a str,
    bindings: &'a Bindings<'t>,
    calls: &'a [Node<'t>],
    declarators: &'a [Node<'t>],
}

impl<'a, 't> KeyFlow<'a, 't> {
    pub fn new(
        source: &'a str,
        bindings: &'a Bindings<'t>,
        calls: &'a [Node<'t>],
        declarators: &'a [Node<'t>],
    ) -> Self {
        Self {
            source,
            bindings,
            calls,
            declarators,
        }
    }

    /// Variables holding the instance created by `factory`, or keys derived
    /// from it: `generator`, then `pair = generator.generateKeyPair()`, then
    /// `privateKey = pair.getPrivate()`.
    pub fn bound(&self, factory: Node<'t>) -> Vec<String> {
        let mut bound: Vec<String> = self.receiver(factory).into_iter().collect();
        if bound.is_empty() {
            return bound;
        }
        // Each pass adds the variables derived from those already found; it
        // stops when a pass adds nothing, at most once per declarator.
        for _ in 0..self.declarators.len() {
            let before = bound.len();
            for declarator in self.declarators {
                let Some(name) = declarator.child_by_field_name("name") else {
                    continue;
                };
                let variable = identity(text(name, self.source), *declarator);
                if bound.contains(&variable) {
                    continue;
                }
                let Some(value) = declarator.child_by_field_name("value") else {
                    continue;
                };
                let Some(root) = key_source(value, self.source) else {
                    continue;
                };
                if self
                    .bindings
                    .variable(root, self.source)
                    .is_some_and(|root| bound.contains(&root))
                {
                    bound.push(variable);
                }
            }
            if bound.len() == before {
                break;
            }
        }
        bound
    }

    /// Variables passed to the operations of the instance created by
    /// `factory`, as `pair` in `signer.initSign(pair.getPrivate())`.
    pub fn used(&self, factory: Node<'t>) -> Vec<String> {
        let mut names = Vec::new();
        for call in self.calls {
            let is_use = call
                .child_by_field_name("name")
                .is_some_and(|name| USE_METHODS.contains(&text(name, self.source)));
            if !is_use {
                continue;
            }
            let on_instance = call
                .child_by_field_name("object")
                .and_then(|object| self.bindings.origin(object, self.source))
                .is_some_and(|origin| origin.id() == factory.id());
            if !on_instance {
                continue;
            }
            let Some(arguments) = call.child_by_field_name("arguments") else {
                continue;
            };
            crate::modules::common::syntax::walk(arguments, |node| {
                if node.kind() == "identifier"
                    && let Some(variable) = self.bindings.variable(node, self.source)
                    && !names.contains(&variable)
                {
                    names.push(variable);
                }
            });
        }
        names
    }

    /// The variable that receives `value`: `x` in `var x = value` or `x = value`.
    fn receiver(&self, value: Node<'t>) -> Option<String> {
        let mut value = value;
        while let Some(parent) = value.parent()
            && matches!(
                parent.kind(),
                "parenthesized_expression" | "cast_expression"
            )
        {
            value = parent;
        }
        let parent = value.parent()?;
        match parent.kind() {
            "variable_declarator" => {
                let name = parent.child_by_field_name("name")?;
                Some(identity(text(name, self.source), parent))
            }
            "assignment_expression" => self
                .bindings
                .variable(parent.child_by_field_name("left")?, self.source),
            _ => None,
        }
    }
}

/// The variable a key is taken from, as `pair` in `(PrivateKey) pair.getPrivate()`
/// or `generator` in `generator.generateKeyPair()`. A plain alias (`var b = a`)
/// counts too.
fn key_source<'t>(mut node: Node<'t>, source: &str) -> Option<Node<'t>> {
    loop {
        node = match node.kind() {
            "parenthesized_expression" => node.named_child(0)?,
            "cast_expression" => node.child_by_field_name("value")?,
            "method_invocation"
                if KEY_METHODS.contains(&text(node.child_by_field_name("name")?, source)) =>
            {
                node.child_by_field_name("object")?
            }
            "identifier" | "field_access" => return Some(node),
            _ => return None,
        };
    }
}
