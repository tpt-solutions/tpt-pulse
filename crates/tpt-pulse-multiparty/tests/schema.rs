//! Schemas for message labels, and the labels a protocol is missing.

#![cfg(feature = "schema")]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tpt_pulse_multiparty::{GlobalProtocol, Schemas};

#[derive(JsonSchema, Serialize, Deserialize)]
struct AuthReq {
    user: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
struct AuthOk {
    welcome: String,
}

fn login() -> GlobalProtocol {
    GlobalProtocol::parse(
        "Client -> Server : AuthReq; loop { Server -> Client : AuthOk; continue; }",
    )
    .expect("parses")
}

#[test]
fn a_registered_message_has_its_json_schema() {
    let mut schemas = Schemas::new();
    schemas.add::<AuthReq>("AuthReq");
    let schema = schemas.get("AuthReq").expect("registered");
    assert_eq!(schema["properties"]["user"]["type"], "string");
    assert_eq!(schema["required"][0], "user");
}

#[test]
fn missing_lists_only_unregistered_labels_in_use() {
    let mut schemas = Schemas::new();
    schemas.add::<AuthReq>("AuthReq");
    assert_eq!(schemas.missing(&login()), ["AuthOk"]);

    schemas.add::<AuthOk>("AuthOk");
    assert!(schemas.missing(&login()).is_empty());
    assert_eq!(schemas.labels().collect::<Vec<_>>(), ["AuthOk", "AuthReq"]);
}

#[test]
fn choice_announcements_are_not_messages() {
    let protocol =
        GlobalProtocol::parse("choice by A { A -> B : X; } or { A -> B : Y; }").expect("parses");
    let schemas = Schemas::new();
    assert_eq!(schemas.missing(&protocol), ["X", "Y"]);
}
