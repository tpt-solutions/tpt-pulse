//! Runs the projected roles together with `simulate`, under several branch
//! policies, and checks that none gets stuck.
//!
//! This is a bounded check of the claim behind `GlobalProtocol::project`: with
//! broadcast choices, the projected roles never deadlock. It does not prove the
//! claim, but it catches projections that break it.

use pulse_multiparty::{branch_label, simulate, Action, GlobalProtocol, Outcome};

const POLICIES: usize = 8;

const QUERY_LOOP: &str = "
    loop {
        Client -> Server : Ask;
        choice by Server {
            Server -> Client : Answer;
            continue;
        } or {
            Server -> Client : Done;
        }
    }
";

const WITH_AUDITOR: &str = "
    loop {
        Client -> Server : Ask;
        choice by Server {
            Server -> Auditor : Logged;
            Server -> Client : Answer;
            continue;
        } or {
            Server -> Auditor : Closed;
            Server -> Client : Done;
        }
    }
";

const NESTED: &str = "
    Client -> Server : Hello;
    loop {
        choice by Client {
            loop {
                Client -> Server : Retry;
                choice by Server {
                    Server -> Client : Busy;
                    continue;
                } or {
                    Server -> Auditor : Accepted;
                    Server -> Client : Ok;
                }
            }
        } or {
            Client -> Server : Quit;
            Server -> Auditor : Closed;
        }
    }
";

/// Every policy runs without getting stuck, and at least one run completes.
fn check_every_policy(text: &str) {
    let protocol = GlobalProtocol::parse(text).expect("parses");
    let mut finished = 0;
    for seed in 0..POLICIES {
        match simulate(&protocol, |n| (n * 3 + seed) % POLICIES, 400) {
            Ok(Outcome::Finished(_)) => finished += 1,
            Ok(Outcome::CutOff(_)) => {}
            Err(error) => panic!("policy {seed}: {error}"),
        }
    }
    assert!(finished > 0, "no policy ran the protocol to completion");
}

#[test]
fn query_loop_never_gets_stuck() {
    check_every_policy(QUERY_LOOP);
}

#[test]
fn an_auditor_loop_never_gets_stuck() {
    check_every_policy(WITH_AUDITOR);
}

#[test]
fn nested_loops_and_choices_never_get_stuck() {
    check_every_policy(NESTED);
}

#[test]
fn the_trace_records_each_role_in_order() {
    let protocol = GlobalProtocol::parse(QUERY_LOOP).expect("parses");
    // The decider always takes branch 1, which ends the loop.
    let Ok(Outcome::Finished(trace)) = simulate(&protocol, |_| 1, 100) else {
        panic!("expected the run to finish");
    };
    let client: Vec<Action> = trace
        .iter()
        .filter(|(role, _)| role == "Client")
        .map(|(_, action)| action.clone())
        .collect();
    assert_eq!(
        client,
        [
            Action::send("Server", "Ask"),
            Action::recv("Server", branch_label(1)),
            Action::recv("Server", "Done"),
        ]
    );
}
