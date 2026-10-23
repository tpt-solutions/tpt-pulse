//! TypeScript stubs generated from a protocol and its message schemas.

#![cfg(feature = "schema")]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tpt_pulse_multiparty::{typescript_stubs, GlobalProtocol, Schemas, StubError};

#[derive(JsonSchema, Serialize, Deserialize)]
struct Account {
    level: Level,
    nickname: Option<String>,
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
fn message_types_come_from_the_schemas() {
    let stubs = typescript_stubs(&protocol(), &schemas()).expect("stubs");
    assert!(
        stubs.contains("export type Account = { level: Account_Level; nickname?: string | null };"),
        "{stubs}"
    );
    assert!(
        stubs.contains("export type Account_Level = \"Guest\" | \"Member\";"),
        "{stubs}"
    );
    assert!(
        stubs.contains("export type Welcome = { text: string };"),
        "{stubs}"
    );
    assert!(
        stubs.contains("export type Messages = {\n  Account: Account;\n  Welcome: Welcome;\n};"),
        "{stubs}"
    );
}

#[test]
fn each_role_gets_its_projected_program() {
    let stubs = typescript_stubs(&protocol(), &schemas()).expect("stubs");
    assert!(
        stubs.contains(
            "export const ClientProgram: readonly Op[] = [{ op: \"send\", to: \"Server\", label: \"Account\" }, \
             { op: \"loop\", body: [{ op: \"offer\", from: \"Server\", branches: \
             [[{ op: \"recv\", from: \"Server\", label: \"Welcome\" }, { op: \"continue\" }], \
             [{ op: \"recv\", from: \"Server\", label: \"Welcome\" }]] }] }] as const;"
        ),
        "{stubs}"
    );
    assert!(
        stubs.contains("export const ServerProgram: readonly Op[] = "),
        "{stubs}"
    );
    assert!(
        stubs.contains("{ op: \"choose\", to: [\"Client\"], branches: "),
        "{stubs}"
    );
}

#[test]
fn a_missing_schema_is_an_error_that_names_the_label() {
    let error = typescript_stubs(&protocol(), &Schemas::new()).expect_err("no schemas");
    assert_eq!(
        error,
        StubError::MissingSchemas(vec!["Account".into(), "Welcome".into()])
    );
}

#[test]
fn a_malformed_protocol_is_an_error() {
    let protocol = GlobalProtocol::parse("A -> A : Account;").expect("parses");
    let mut schemas = Schemas::new();
    schemas.add::<Account>("Account");
    assert!(matches!(
        typescript_stubs(&protocol, &schemas),
        Err(StubError::Protocol(_))
    ));
}
