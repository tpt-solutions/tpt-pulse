//! Tests for the `assert_protocol!` trace harness.

use tpt_pulse_multiparty::{assert_protocol, Action, GlobalProtocol};

fn login() -> GlobalProtocol {
    GlobalProtocol::parse(
        "Client -> Server : AuthReq; Server -> Client : AuthOk; Client -> Server : Logout;",
    )
    .expect("valid protocol")
}

#[test]
fn a_complete_trace_passes_for_each_role() {
    assert_protocol!(
        login(),
        "Client",
        [
            Action::send("Server", "AuthReq"),
            Action::recv("Server", "AuthOk"),
            Action::send("Server", "Logout"),
        ]
    );
    assert_protocol!(
        login(),
        "Server",
        [
            Action::recv("Client", "AuthReq"),
            Action::send("Client", "AuthOk"),
            Action::recv("Client", "Logout"),
        ]
    );
}

#[test]
#[should_panic(
    expected = "expected a send of `AuthReq` to `Server`, observed a receive of `AuthOk` from `Server`"
)]
fn an_out_of_order_event_fails() {
    assert_protocol!(login(), "Client", [Action::recv("Server", "AuthOk")]);
}

#[test]
#[should_panic(expected = "trace ended with 1 action(s) still to perform")]
fn a_short_trace_fails() {
    assert_protocol!(
        login(),
        "Client",
        [
            Action::send("Server", "AuthReq"),
            Action::recv("Server", "AuthOk"),
        ]
    );
}

#[test]
#[should_panic(expected = "the protocol has no role `Auditor`")]
fn an_unknown_role_fails() {
    assert_protocol!(login(), "Auditor", []);
}
