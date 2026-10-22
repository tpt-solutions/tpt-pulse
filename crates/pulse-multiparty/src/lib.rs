//! Multiparty session protocols: global descriptions, per-role projection, and
//! deadlock-freedom checks through `tpt-telos`.
//!
//! A [`GlobalProtocol`] is an ordered list of messages `from -> to : label`.
//! [`GlobalProtocol::project`] yields each role's local [`Program`]. A [`Session`]
//! of local programs (which may be hand-written, not projected) is checked by
//! [`Session::verify`]. Messages are FIFO per sender/receiver pair, so the k-th
//! send to a role matches the k-th receive from that sender.
//!
//! The check is sound for asynchronous FIFO channels: if it passes, some
//! interleaving runs every receive after its matching send, so no role waits
//! forever. It is a schedule check, not a model checker for arbitrary state.

use std::{collections::BTreeMap, fmt};

use pulse_ir::{OrderGraph, OrderViolation};

/// One message in a global protocol: `from -> to : label`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// Sending role.
    pub from: String,
    /// Receiving role.
    pub to: String,
    /// Message label, normally the message type's name.
    pub label: String,
}

/// An ordered, sequential description of a multiparty conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalProtocol {
    steps: Vec<Step>,
}

/// One local action in a role's program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Send a message to `to`.
    Send {
        /// Receiving role.
        to: String,
        /// Message label.
        label: String,
    },
    /// Wait for a message from `from`.
    Recv {
        /// Sending role.
        from: String,
        /// Message label.
        label: String,
    },
}

/// The ordered local actions of one role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// Role name.
    pub role: String,
    /// Actions, executed in order.
    pub actions: Vec<Action>,
}

/// A set of local programs, one per role, that run together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    programs: Vec<Program>,
}

/// Why a protocol or session is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// A message is sent by a role to itself.
    SelfMessage {
        /// The role.
        role: String,
        /// The message label.
        label: String,
    },
    /// The same role appears in two programs.
    DuplicateRole(String),
    /// The sends and receives between two roles do not pair up one-to-one.
    Unmatched {
        /// Sending role.
        from: String,
        /// Receiving role.
        to: String,
        /// Messages sent.
        sent: usize,
        /// Messages expected.
        received: usize,
    },
    /// The k-th send and k-th receive on a pair of roles carry different labels.
    LabelMismatch {
        /// Sending role.
        from: String,
        /// Receiving role.
        to: String,
        /// Label the sender uses.
        sent: String,
        /// Label the receiver expects.
        received: String,
    },
    /// The local programs can never all finish. The events are on the circular wait.
    Deadlock(Vec<String>),
    /// The solver could not decide whether the session can progress.
    Undecided,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelfMessage { role, label } => {
                write!(f, "role `{role}` sends `{label}` to itself")
            }
            Self::DuplicateRole(role) => write!(f, "role `{role}` has more than one program"),
            Self::Unmatched {
                from,
                to,
                sent,
                received,
            } => write!(
                f,
                "`{from}` sends {sent} message(s) to `{to}`, but `{to}` receives {received} from `{from}`"
            ),
            Self::LabelMismatch {
                from,
                to,
                sent,
                received,
            } => write!(
                f,
                "`{from}` sends `{sent}` to `{to}`, but `{to}` expects `{received}` from `{from}`"
            ),
            Self::Deadlock(events) => {
                write!(f, "deadlock, circular wait: {}", events.join(" -> "))?;
                if let Some(first) = events.first() {
                    write!(f, " -> {first}")?;
                }
                Ok(())
            }
            Self::Undecided => write!(f, "solver could not decide deadlock freedom"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl GlobalProtocol {
    /// Build a protocol from `(from, to, label)` triples, in conversation order.
    pub fn from_steps(steps: &[(&str, &str, &str)]) -> Self {
        Self {
            steps: steps
                .iter()
                .map(|&(from, to, label)| Step {
                    from: from.into(),
                    to: to.into(),
                    label: label.into(),
                })
                .collect(),
        }
    }

    /// The protocol's steps, in conversation order.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Roles in order of first appearance.
    pub fn roles(&self) -> Vec<String> {
        let mut roles: Vec<String> = Vec::new();
        for step in &self.steps {
            for role in [&step.from, &step.to] {
                if !roles.contains(role) {
                    roles.push(role.clone());
                }
            }
        }
        roles
    }

    /// Project onto each role, producing a verified [`Session`].
    ///
    /// A sequential global protocol projects to programs that cannot deadlock,
    /// so this fails only on malformed steps, such as a message to oneself.
    pub fn project(&self) -> Result<Session, ProtocolError> {
        for step in &self.steps {
            if step.from == step.to {
                return Err(ProtocolError::SelfMessage {
                    role: step.from.clone(),
                    label: step.label.clone(),
                });
            }
        }
        let programs = self
            .roles()
            .into_iter()
            .map(|role| {
                let actions = self
                    .steps
                    .iter()
                    .filter_map(|step| {
                        if step.from == role {
                            Some(Action::Send {
                                to: step.to.clone(),
                                label: step.label.clone(),
                            })
                        } else if step.to == role {
                            Some(Action::Recv {
                                from: step.from.clone(),
                                label: step.label.clone(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                Program { role, actions }
            })
            .collect();
        let session = Session { programs };
        session.verify()?;
        Ok(session)
    }
}

impl Session {
    /// Build a session from local programs. Call [`Session::verify`] before running it.
    pub fn new(programs: Vec<Program>) -> Self {
        Self { programs }
    }

    /// The local programs, one per role.
    pub fn programs(&self) -> &[Program] {
        &self.programs
    }

    /// Check that every message pairs up and that the programs cannot deadlock.
    pub fn verify(&self) -> Result<(), ProtocolError> {
        let mut seen = BTreeMap::new();
        for program in &self.programs {
            if seen.insert(program.role.as_str(), ()).is_some() {
                return Err(ProtocolError::DuplicateRole(program.role.clone()));
            }
            for action in &program.actions {
                let peer = match action {
                    Action::Send { to, .. } => to,
                    Action::Recv { from, .. } => from,
                };
                if peer == &program.role {
                    return Err(ProtocolError::SelfMessage {
                        role: program.role.clone(),
                        label: action_label(action).into(),
                    });
                }
            }
        }

        let mut graph = OrderGraph::new();
        // Event ids for each action, indexed by program then action position.
        let mut events: Vec<Vec<usize>> = Vec::new();
        for program in &self.programs {
            let ids: Vec<usize> = program
                .actions
                .iter()
                .map(|action| graph.event(event_name(&program.role, action)))
                .collect();
            for pair in ids.windows(2) {
                graph.before(pair[0], pair[1]);
            }
            events.push(ids);
        }

        for sender in &self.programs {
            for receiver in &self.programs {
                if sender.role == receiver.role {
                    continue;
                }
                let sends = self.events_where(&sender.role, &events, |action| match action {
                    Action::Send { to, label } if to == &receiver.role => Some(label),
                    _ => None,
                });
                let recvs = self.events_where(&receiver.role, &events, |action| match action {
                    Action::Recv { from, label } if from == &sender.role => Some(label),
                    _ => None,
                });
                if sends.len() != recvs.len() {
                    return Err(ProtocolError::Unmatched {
                        from: sender.role.clone(),
                        to: receiver.role.clone(),
                        sent: sends.len(),
                        received: recvs.len(),
                    });
                }
                for ((send_event, sent), (recv_event, received)) in sends.iter().zip(&recvs) {
                    if sent != received {
                        return Err(ProtocolError::LabelMismatch {
                            from: sender.role.clone(),
                            to: receiver.role.clone(),
                            sent: sent.clone(),
                            received: received.clone(),
                        });
                    }
                    graph.before(*send_event, *recv_event);
                }
            }
        }

        match graph.verify() {
            Ok(()) => Ok(()),
            Err(OrderViolation::Cycle(events)) => Err(ProtocolError::Deadlock(events)),
            Err(OrderViolation::Undecided) => Err(ProtocolError::Undecided),
        }
    }

    /// Events of `role` whose action matches `select`, paired with the label, in program order.
    fn events_where<'a>(
        &'a self,
        role: &str,
        events: &[Vec<usize>],
        select: impl Fn(&'a Action) -> Option<&'a String>,
    ) -> Vec<(usize, String)> {
        let index = self
            .programs
            .iter()
            .position(|program| program.role == role)
            .expect("role was taken from the session");
        self.programs[index]
            .actions
            .iter()
            .zip(&events[index])
            .filter_map(|(action, &event)| select(action).map(|label| (event, label.clone())))
            .collect()
    }
}

fn action_label(action: &Action) -> &str {
    match action {
        Action::Send { label, .. } | Action::Recv { label, .. } => label,
    }
}

fn event_name(role: &str, action: &Action) -> String {
    match action {
        Action::Send { to, label } => format!("{role} sends {label} to {to}"),
        Action::Recv { from, label } => format!("{role} waits for {label} from {from}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send(to: &str, label: &str) -> Action {
        Action::Send {
            to: to.into(),
            label: label.into(),
        }
    }

    fn recv(from: &str, label: &str) -> Action {
        Action::Recv {
            from: from.into(),
            label: label.into(),
        }
    }

    fn program(role: &str, actions: Vec<Action>) -> Program {
        Program {
            role: role.into(),
            actions,
        }
    }

    #[test]
    fn three_role_protocol_projects_and_verifies() {
        let protocol = GlobalProtocol::from_steps(&[
            ("Client", "Gateway", "Request"),
            ("Gateway", "Backend", "Forward"),
            ("Backend", "Gateway", "Reply"),
            ("Gateway", "Client", "Answer"),
        ]);
        let session = protocol.project().expect("well-formed protocol");
        assert_eq!(protocol.roles(), ["Client", "Gateway", "Backend"]);
        assert_eq!(
            session.programs()[0].actions,
            [send("Gateway", "Request"), recv("Gateway", "Answer")]
        );
        assert_eq!(session.verify(), Ok(()));
    }

    #[test]
    fn mutual_wait_is_a_deadlock() {
        let session = Session::new(vec![
            program("A", vec![recv("B", "Ping"), send("B", "Pong")]),
            program("B", vec![recv("A", "Pong"), send("A", "Ping")]),
        ]);
        let Err(ProtocolError::Deadlock(events)) = session.verify() else {
            panic!("expected a deadlock");
        };
        assert!(events.iter().any(|event| event.starts_with("A waits")));
        assert!(events.iter().any(|event| event.starts_with("B waits")));
    }

    #[test]
    fn three_role_circular_wait_is_a_deadlock() {
        let session = Session::new(vec![
            program("A", vec![recv("C", "X"), send("B", "Y")]),
            program("B", vec![recv("A", "Y"), send("C", "Z")]),
            program("C", vec![recv("B", "Z"), send("A", "X")]),
        ]);
        assert!(matches!(session.verify(), Err(ProtocolError::Deadlock(_))));
    }

    #[test]
    fn unmatched_receive_is_rejected() {
        let session = Session::new(vec![
            program("A", vec![send("B", "Hi")]),
            program("B", vec![recv("A", "Hi"), recv("A", "Hi")]),
        ]);
        assert_eq!(
            session.verify(),
            Err(ProtocolError::Unmatched {
                from: "A".into(),
                to: "B".into(),
                sent: 1,
                received: 2,
            })
        );
    }

    #[test]
    fn label_mismatch_is_rejected() {
        let session = Session::new(vec![
            program("A", vec![send("B", "Hi")]),
            program("B", vec![recv("A", "Bye")]),
        ]);
        assert!(matches!(
            session.verify(),
            Err(ProtocolError::LabelMismatch { .. })
        ));
    }

    #[test]
    fn self_message_is_rejected() {
        let protocol = GlobalProtocol::from_steps(&[("A", "A", "Loop")]);
        assert_eq!(
            protocol.project(),
            Err(ProtocolError::SelfMessage {
                role: "A".into(),
                label: "Loop".into(),
            })
        );
    }
}
