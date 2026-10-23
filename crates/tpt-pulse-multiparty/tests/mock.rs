//! Simulation with payloads checked against message schemas.

#![cfg(feature = "schema")]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tpt_pulse_multiparty::{simulate_messages, GlobalProtocol, Outcome, Schemas, SimulationError};

#[derive(JsonSchema, Serialize, Deserialize)]
struct Ask {
    topic: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
struct Done {
    tags: Vec<String>,
}

fn protocol() -> GlobalProtocol {
    GlobalProtocol::parse(
        "
        Client -> Server : Ask;
        choice by Server {
            Server -> Client : Done;
        } or {
            Server -> Client : Done;
        }
        ",
    )
    .expect("parses")
}

fn schemas() -> Schemas {
    let mut schemas = Schemas::new();
    schemas.add::<Ask>("Ask").add::<Done>("Done");
    schemas
}

fn valid(_: &str, label: &str) -> Value {
    match label {
        "Ask" => json!({ "topic": "weather" }),
        _ => json!({ "tags": ["a", "b"] }),
    }
}

#[test]
fn messages_carry_their_payloads_in_order() {
    let (outcome, messages) =
        simulate_messages(&protocol(), &schemas(), |_| 0, 10, valid).expect("runs");
    assert!(matches!(outcome, Outcome::Finished(_)));
    let sent: Vec<(&str, &str, &str, Value)> = messages
        .iter()
        .map(|m| {
            (
                m.from.as_str(),
                m.to.as_str(),
                m.label.as_str(),
                m.payload.clone(),
            )
        })
        .collect();
    assert_eq!(
        sent,
        [
            ("Client", "Server", "Ask", json!({ "topic": "weather" })),
            ("Server", "Client", "Done", json!({ "tags": ["a", "b"] })),
        ]
    );
}

#[test]
fn a_payload_that_breaks_its_schema_stops_the_run() {
    let error = simulate_messages(
        &protocol(),
        &schemas(),
        |_| 0,
        10,
        |_, label| {
            if label == "Ask" {
                json!({ "depth": 3 })
            } else {
                valid("", label)
            }
        },
    )
    .expect_err("the Ask payload is missing its topic");
    match error {
        SimulationError::Rejected {
            role,
            label,
            reason,
        } => {
            assert_eq!(role, "Client");
            assert_eq!(label, "Ask");
            assert!(reason.contains("topic"), "{reason}");
        }
        other => panic!("expected a rejected message, got {other}"),
    }
}

#[test]
fn a_label_without_a_schema_is_an_error() {
    let mut partial = Schemas::new();
    partial.add::<Ask>("Ask");
    let error =
        simulate_messages(&protocol(), &partial, |_| 0, 10, valid).expect_err("Done has no schema");
    assert_eq!(
        error,
        SimulationError::Schema {
            label: "Done".into(),
            reason: "no schema is registered for this label".into(),
        }
    );
}
