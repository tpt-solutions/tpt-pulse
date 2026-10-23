//! Python stubs generated from a protocol and its message schemas.

#![cfg(feature = "schema")]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tpt_pulse_multiparty::{python_stubs, GlobalProtocol, Schemas, StubError};

#[derive(JsonSchema, Serialize, Deserialize)]
struct Account {
    level: Level,
    nickname: Option<String>,
    from: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
enum Level {
    Guest,
    Member,
}

#[derive(JsonSchema, Serialize, Deserialize)]
struct Welcome {
    text: String,
}

fn protocol() -> GlobalProtocol {
    GlobalProtocol::parse(
        "
        Client -> Server : Account;
        loop {
            choice by Server {
                Server -> Client : Welcome;
                continue;
            } or {
                Server -> Client : Welcome;
            }
        }
        ",
    )
    .expect("parses")
}

fn schemas() -> Schemas {
    let mut schemas = Schemas::new();
    schemas.add::<Account>("Account").add::<Welcome>("Welcome");
    schemas
}

#[test]
fn message_types_are_typed_dicts_with_optional_fields() {
    let stubs = python_stubs(&protocol(), &schemas()).expect("stubs");
    assert!(
        stubs.contains("Account = TypedDict(\"Account\", {\"from\": str, \"level\": Account_Level, \"nickname\": NotRequired[Optional[str]]})"),
        "{stubs}"
    );
    assert!(
        stubs.contains("Account_Level = Literal[\"Guest\", \"Member\"]"),
        "{stubs}"
    );
    assert!(
        stubs.contains("Welcome = TypedDict(\"Welcome\", {\"text\": str})"),
        "{stubs}"
    );
    assert!(
        stubs.contains(
            "Messages = TypedDict(\"Messages\", {\"Account\": Account, \"Welcome\": Welcome})"
        ),
        "{stubs}"
    );
}

#[test]
fn each_role_gets_its_projected_program() {
    let stubs = python_stubs(&protocol(), &schemas()).expect("stubs");
    assert!(
        stubs.contains("ClientProgram: List[Op] = [{\"op\": \"send\", \"to\": \"Server\", \"label\": \"Account\"}, "),
        "{stubs}"
    );
    assert!(
        stubs.contains("{\"op\": \"choose\", \"to\": [\"Client\"], \"branches\": "),
        "{stubs}"
    );
    assert!(stubs.contains("ServerProgram: List[Op] = "), "{stubs}");
}

#[test]
fn a_missing_schema_is_an_error() {
    let error = python_stubs(&protocol(), &Schemas::new()).expect_err("no schemas");
    assert_eq!(
        error,
        StubError::MissingSchemas(vec!["Account".into(), "Welcome".into()])
    );
}
