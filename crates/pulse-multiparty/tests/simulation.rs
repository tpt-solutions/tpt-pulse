//! Executes the projected roles together on FIFO queues, with the decider's
//! branch chosen by a policy, and checks that no run gets stuck.
//!
//! This is a bounded check of the claim behind `GlobalProtocol::project`: with
//! broadcast choices, the projected roles never deadlock. It does not prove the
//! claim, but it catches projections that break it.

use std::collections::{BTreeMap, VecDeque};

use pulse_multiparty::{branch_label, Action, GlobalProtocol, Monitor};

/// Upper bound on branch indices the simulator tries for one choice.
const MAX_BRANCHES: usize = 8;

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// Every role finished its program.
    Finished { steps: usize },
    /// The run was cut off after `max_steps` while still able to move.
    CutOff,
}

/// Run the protocol. `seed` shifts which branch the decider prefers at each choice.
fn run(protocol: &GlobalProtocol, seed: usize, max_steps: usize) -> Result<Outcome, String> {
    let session = protocol.project().expect("well formed");
    let mut monitors: BTreeMap<String, Monitor> = session
        .programs()
        .iter()
        .map(|program| (program.role.clone(), Monitor::new(program.clone())))
        .collect();
    let mut queues: BTreeMap<(String, String), VecDeque<String>> = BTreeMap::new();
    let mut decisions = 0;

    for step in 0..max_steps {
        if monitors
            .values()
            .all(|monitor| monitor.expected().is_none())
        {
            for (role, monitor) in &monitors {
                monitor
                    .finish()
                    .map_err(|error| format!("`{role}` did not finish: {error}"))?;
            }
            return Ok(Outcome::Finished { steps: step });
        }

        let mut progressed = false;
        for (role, monitor) in monitors.iter_mut() {
            let Some(expected) = monitor.expected() else {
                continue;
            };
            let moved = match &expected {
                Action::Send { to, label } if label.starts_with("choice#") => {
                    // The decider's choice: try the preferred branch first, then the rest.
                    let preferred = (decisions * 3 + seed) % MAX_BRANCHES;
                    let pick = (0..MAX_BRANCHES).map(|offset| (preferred + offset) % MAX_BRANCHES);
                    let accepted = pick.into_iter().find(|&index| {
                        monitor
                            .observe(&Action::send(to, branch_label(index)))
                            .is_ok()
                    });
                    match accepted {
                        Some(index) => {
                            // `observe` above already advanced the monitor past the announcement.
                            queues
                                .entry((role.clone(), to.clone()))
                                .or_default()
                                .push_back(branch_label(index));
                            decisions += 1;
                            true
                        }
                        None => false,
                    }
                }
                Action::Send { to, label } => {
                    queues
                        .entry((role.clone(), to.clone()))
                        .or_default()
                        .push_back(label.clone());
                    monitor
                        .observe(&expected)
                        .map_err(|error| format!("`{role}`: {error}"))?;
                    true
                }
                Action::Recv { from, label } => {
                    let queue = queues.entry((from.clone(), role.clone())).or_default();
                    let front = queue.front().cloned();
                    // A branch announcement may carry any branch index.
                    let matches = match &front {
                        Some(front) if label.starts_with("choice#") => front.starts_with("choice#"),
                        Some(front) => front == label,
                        None => false,
                    };
                    if matches {
                        let front = front.expect("matched a queued message");
                        queue.pop_front();
                        monitor
                            .observe(&Action::recv(from, front))
                            .map_err(|error| format!("`{role}`: {error}"))?;
                    }
                    matches
                }
                _ => unreachable!("expected() only returns sends and receives"),
            };
            progressed |= moved;
        }

        if !progressed {
            let stuck: Vec<&str> = monitors
                .iter()
                .filter(|(_, monitor)| monitor.expected().is_some())
                .map(|(role, _)| role.as_str())
                .collect();
            return Err(format!("deadlock: no role can move; waiting: {stuck:?}"));
        }
    }
    Ok(Outcome::CutOff)
}

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

fn check_every_policy(text: &str) {
    let protocol = GlobalProtocol::parse(text).expect("parses");
    let mut finished = 0;
    for seed in 0..MAX_BRANCHES {
        match run(&protocol, seed, 400) {
            Ok(Outcome::Finished { .. }) => finished += 1,
            Ok(Outcome::CutOff) => {}
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
