//! `global_protocol!` with choice and loop, checked at compile time and then
//! against a role's trace.

use tpt_pulse::prelude::*;

#[test]
fn global_protocol_accepts_a_choice_inside_a_loop() {
    let protocol = global_protocol! {
        loop {
            Client -> Server : Ask;
            choice by Server {
                Server -> Client : Answer;
                continue;
            } or {
                Server -> Client : Done;
            }
        }
    };
    assert_protocol!(
        protocol,
        "Client",
        [
            Action::send("Server", "Ask"),
            Action::recv("Server", "choice#0"),
            Action::recv("Server", "Answer"),
            Action::send("Server", "Ask"),
            Action::recv("Server", "choice#1"),
            Action::recv("Server", "Done"),
        ]
    );
}

#[test]
fn global_protocol_still_takes_plain_messages() {
    let protocol = global_protocol! {
        Client -> Server : AuthReq;
        Server -> Client : AuthOk;
    };
    assert_protocol!(
        protocol,
        "Server",
        [
            Action::recv("Client", "AuthReq"),
            Action::send("Client", "AuthOk"),
        ]
    );
}
