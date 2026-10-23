//! JSON Schema for message types, keyed by the labels a protocol uses.
//!
//! The wire format is JSON, so a message's schema describes what is sent. Stubs
//! and typed mock peers read their message shapes from here.

use std::collections::BTreeMap;

use schemars::{schema_for, JsonSchema};
use serde_json::Value;

use crate::{GlobalProtocol, Statement};

/// The JSON Schema of each message type, by protocol label.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Schemas {
    by_label: BTreeMap<String, Value>,
}

impl Schemas {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the schema of `T` as the message `label`. A later call replaces an earlier one.
    pub fn add<T: JsonSchema>(&mut self, label: impl Into<String>) -> &mut Self {
        let schema = serde_json::to_value(schema_for!(T)).expect("a schema serializes to JSON");
        self.by_label.insert(label.into(), schema);
        self
    }

    /// The schema recorded for `label`.
    pub fn get(&self, label: &str) -> Option<&Value> {
        self.by_label.get(label)
    }

    /// The labels that have a schema, in order.
    pub fn labels(&self) -> impl Iterator<Item = &str> {
        self.by_label.keys().map(String::as_str)
    }

    /// The labels `protocol` sends that have no schema, in order of first use.
    pub fn missing(&self, protocol: &GlobalProtocol) -> Vec<String> {
        let mut used = Vec::new();
        collect_labels(protocol.statements(), &mut used);
        used.retain(|label| !self.by_label.contains_key(label));
        used
    }
}

pub(crate) fn collect_labels(statements: &[Statement], out: &mut Vec<String>) {
    for statement in statements {
        match statement {
            Statement::Message(step) => {
                if !out.contains(&step.label) {
                    out.push(step.label.clone());
                }
            }
            Statement::Choice { branches, .. } => {
                for branch in branches {
                    collect_labels(branch, out);
                }
            }
            Statement::Loop(body) => collect_labels(body, out),
            Statement::Continue => {}
        }
    }
}
