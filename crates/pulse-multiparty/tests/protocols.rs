//! Parsing, projection, monitoring, and structural checks for protocols with
//! sequences, choices, and loops.

use pulse_multiparty::{branch_label, Action, GlobalProtocol, Monitor, ProtocolError, Statement};

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
            Server -> Client : Done;
        }
    }
";

fn program(protocol: &GlobalProtocol, role: &str) -> pulse_multiparty::Program {
    protocol
        .project()
        .expect("well formed")
        .programs()
        .iter()
        .find(|program| program.role == role)
        .cloned()
        .expect("role exists")
}

#[test]
fn a_loop_with_a_choice_projects_to_offer_and_choose() {
    let protocol = GlobalProtocol::parse(QUERY_LOOP).expect("parses");
    assert_eq!(
        program(&protocol, "Client").actions,
        [Action::Loop(vec![
            Action::send("Server", "Ask"),
            Action::Offer {
                from: "Server".into(),
                branches: vec![
                    vec![Action::recv("Server", "Answer"), Action::Continue],
                    vec![Action::recv("Server", "Done")],
                ],
            },
        ])]
    );
    assert_eq!(
        program(&protocol, "Server").actions,
        [Action::Loop(vec![
            Action::recv("Client", "Ask"),
            Action::Choose {
                to: vec!["Client".into()],
                branches: vec![
                    vec![Action::send("Client", "Answer"), Action::Continue],
                    vec![Action::send("Client", "Done")],
                ],
            },
        ])]
    );
}

#[test]
fn the_monitor_follows_each_branch_and_the_loop() {
    let protocol = GlobalProtocol::parse(QUERY_LOOP).expect("parses");
    let mut monitor = Monitor::new(program(&protocol, "Client"));
    monitor.observe(&Action::send("Server", "Ask")).unwrap();
    monitor
        .observe(&Action::recv("Server", branch_label(0)))
        .unwrap();
    monitor.observe(&Action::recv("Server", "Answer")).unwrap();
    // The branch ended with `continue`, so the next pass starts here.
    monitor.observe(&Action::send("Server", "Ask")).unwrap();
    monitor
        .observe(&Action::recv("Server", branch_label(1)))
        .unwrap();
    monitor.observe(&Action::recv("Server", "Done")).unwrap();
    // The second branch falls off the loop body, which ends the loop.
    assert_eq!(monitor.finish(), Ok(()));
}

#[test]
fn the_monitor_rejects_the_wrong_branch() {
    let protocol = GlobalProtocol::parse(QUERY_LOOP).expect("parses");
    let mut monitor = Monitor::new(program(&protocol, "Client"));
    monitor.observe(&Action::send("Server", "Ask")).unwrap();
    // Branch 0 expects `Answer`, so a `Done` after the announcement is wrong.
    monitor
        .observe(&Action::recv("Server", branch_label(0)))
        .unwrap();
    assert!(monitor.observe(&Action::recv("Server", "Done")).is_err());
}

#[test]
fn an_uninvolved_role_hears_every_announcement() {
    let protocol = GlobalProtocol::parse(WITH_AUDITOR).expect("parses");
    let mut monitor = Monitor::new(program(&protocol, "Auditor"));
    monitor
        .observe(&Action::recv("Server", branch_label(0)))
        .unwrap();
    monitor.observe(&Action::recv("Server", "Logged")).unwrap();
    monitor
        .observe(&Action::recv("Server", branch_label(1)))
        .unwrap();
    assert_eq!(monitor.finish(), Ok(()));
}

#[test]
fn a_monitor_reports_an_incomplete_trace_inside_a_branch() {
    let protocol = GlobalProtocol::parse(QUERY_LOOP).expect("parses");
    let mut monitor = Monitor::new(program(&protocol, "Client"));
    monitor.observe(&Action::send("Server", "Ask")).unwrap();
    monitor
        .observe(&Action::recv("Server", branch_label(0)))
        .unwrap();
    assert!(monitor.finish().is_err());
}

#[test]
fn a_flat_protocol_is_monitored_in_order() {
    let protocol = GlobalProtocol::parse(
        "Client -> Server : AuthReq; Server -> Client : AuthOk; Client -> Server : Logout;",
    )
    .expect("parses");
    let mut monitor = Monitor::new(program(&protocol, "Client"));
    assert_eq!(monitor.role(), "Client");
    assert_eq!(monitor.expected(), Some(Action::send("Server", "AuthReq")));
    monitor.observe(&Action::send("Server", "AuthReq")).unwrap();
    assert!(monitor.observe(&Action::send("Server", "Logout")).is_err());
    monitor.observe(&Action::recv("Server", "AuthOk")).unwrap();
    monitor.observe(&Action::send("Server", "Logout")).unwrap();
    assert_eq!(monitor.finish(), Ok(()));
    assert!(monitor.observe(&Action::send("Server", "Again")).is_err());
}

#[test]
fn continue_must_end_a_branch_inside_a_loop() {
    // A `continue` directly in the loop body, with no choice.
    let direct = GlobalProtocol::parse("loop { A -> B : X; continue; }").expect("parses");
    assert_eq!(direct.project(), Err(ProtocolError::ContinueOutsideChoice));

    // A `continue` in a branch, but not as its last statement.
    let not_last =
        GlobalProtocol::parse("loop { choice by A { continue; B -> A : Y; } or { A -> B : Z; } }")
            .expect("parses");
    assert_eq!(
        not_last.project(),
        Err(ProtocolError::ContinueOutsideChoice)
    );

    // A `continue` outside any loop.
    let outside = GlobalProtocol::parse("choice by A { A -> B : X; continue; } or { A -> B : Y; }")
        .expect("parses");
    assert_eq!(outside.project(), Err(ProtocolError::ContinueOutsideChoice));
}

#[test]
fn a_choice_needs_another_role_to_hear_it() {
    let protocol = GlobalProtocol::new(vec![Statement::Choice {
        decider: "A".into(),
        branches: vec![vec![]],
    }]);
    assert_eq!(
        protocol.project(),
        Err(ProtocolError::SoloChoice("A".into()))
    );
}

#[test]
fn a_choice_needs_a_branch() {
    let protocol = GlobalProtocol::new(vec![
        Statement::Message(pulse_multiparty::Step {
            from: "A".into(),
            to: "B".into(),
            label: "X".into(),
        }),
        Statement::Choice {
            decider: "A".into(),
            branches: vec![],
        },
    ]);
    assert_eq!(
        protocol.project(),
        Err(ProtocolError::EmptyChoice("A".into()))
    );
}

#[test]
fn parse_errors_report_the_line() {
    let error = GlobalProtocol::parse("A -> B : Hi;\nB A Oops;").expect_err("malformed");
    assert_eq!(error.line, 2);
    assert_eq!(error.reason, "expected `->` after the sender");
    assert_eq!(error.to_string(), "line 2: expected `->` after the sender");
}

#[test]
fn parse_rejects_keywords_as_roles_and_unclosed_blocks() {
    let keyword = GlobalProtocol::parse("A -> loop : X;").expect_err("keyword");
    assert!(keyword.reason.contains("keyword"), "{keyword}");
    let unclosed = GlobalProtocol::parse("loop { A -> B : X;").expect_err("unclosed");
    assert_eq!(unclosed.reason, "unclosed `{`");
    let stray = GlobalProtocol::parse("A -> B : X; }").expect_err("stray");
    assert_eq!(stray.reason, "unexpected `}`");
}

#[test]
fn parse_rejects_an_empty_protocol_and_bad_characters() {
    assert!(GlobalProtocol::parse("// nothing here").is_err());
    let bad = GlobalProtocol::parse("A -> B : X?;").expect_err("bad character");
    assert_eq!(bad.reason, "unexpected character `?`");
}

#[test]
fn mermaid_draws_the_loop_and_the_branches() {
    let diagram = GlobalProtocol::parse(QUERY_LOOP)
        .expect("parses")
        .to_mermaid();
    assert!(
        diagram.contains("    loop until a branch ends it\n"),
        "{diagram}"
    );
    assert!(
        diagram.contains("        Client->>Server: Ask\n"),
        "{diagram}"
    );
    assert!(
        diagram.contains("    alt Server picks branch 0\n"),
        "{diagram}"
    );
    assert!(
        diagram.contains("    else Server picks branch 1\n"),
        "{diagram}"
    );
}
