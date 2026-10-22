//! Multiparty session protocols: global descriptions with sequences, choices, and
//! loops, per-role projection, and structural checks.
//!
//! A [`GlobalProtocol`] is a list of [`Statement`]s: messages `from -> to : label`,
//! `choice by D { ... } or { ... }` where the decider `D` picks a branch, and
//! `loop { ... }` whose body ends with `continue` in a branch to repeat, or falls
//! off its end to exit. [`GlobalProtocol::project`] yields each role's local
//! [`Program`]. The decider tells every other role which branch it took, so each
//! role can follow the same path without seeing the rest of the protocol.
//!
//! Sequential protocols are checked with [`Session::verify`], a schedule check that
//! is sound for asynchronous FIFO channels. Protocols with choices or loops are
//! checked structurally by [`GlobalProtocol::project`]. The broadcast of each
//! choice makes every projected path a sequential run, so no role waits forever.
//! A [`Monitor`] checks a role's observed events against its program at run time.

mod monitor;
mod parse;

use std::{collections::BTreeMap, fmt};

use pulse_ir::{OrderGraph, OrderViolation};

pub use monitor::{Monitor, MonitorError};
pub use parse::ParseError;

/// The label the decider sends to announce branch `index` of a choice.
pub fn branch_label(index: usize) -> String {
    format!("choice#{index}")
}

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

/// One statement of a global protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    /// A message from one role to another.
    Message(Step),
    /// The decider picks one branch, and every other role is told which.
    Choice {
        /// The role that picks the branch.
        decider: String,
        /// The branches, in order. Branch `i` is announced as [`branch_label`]`(i)`.
        branches: Vec<Vec<Statement>>,
    },
    /// Run the body, then repeat if a branch ends in [`Statement::Continue`], or
    /// exit if the body falls off its end.
    Loop(Vec<Statement>),
    /// Start the next pass of the innermost enclosing loop. Only valid as the last
    /// statement of a choice branch that is inside a loop.
    Continue,
}

/// A global description of a multiparty conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalProtocol {
    body: Vec<Statement>,
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
    /// Decider only: announce a branch to every role in `to`, then follow it.
    Choose {
        /// Every other role, in announcement order.
        to: Vec<String>,
        /// The branch programs. Branch `i` is announced as [`branch_label`]`(i)`.
        branches: Vec<Vec<Action>>,
    },
    /// Wait for the decider's branch announcement from `from`, then follow that branch.
    Offer {
        /// The decider.
        from: String,
        /// The branch programs, in the same order as the decider's.
        branches: Vec<Vec<Action>>,
    },
    /// Run the body, then repeat if a branch continues, or exit if it falls off.
    Loop(Vec<Action>),
    /// Start the next pass of the innermost enclosing loop.
    Continue,
}

impl Action {
    /// A send of `label` to `to`, for building expected traces.
    pub fn send(to: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Send {
            to: to.into(),
            label: label.into(),
        }
    }

    /// A receive of `label` from `from`, for building expected traces.
    pub fn recv(from: impl Into<String>, label: impl Into<String>) -> Self {
        Self::Recv {
            from: from.into(),
            label: label.into(),
        }
    }

    /// Whether this action, or one nested in it, makes a choice or loops.
    pub fn is_branching(&self) -> bool {
        !matches!(self, Self::Send { .. } | Self::Recv { .. })
    }
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
    /// A `continue` that is not the last statement of a choice branch inside a loop.
    ContinueOutsideChoice,
    /// A choice with no branches.
    EmptyChoice(String),
    /// A choice whose decider is the only role, so no other role hears the branch.
    SoloChoice(String),
    /// [`Session::verify`] was given programs with choices or loops. Check the
    /// global protocol with [`GlobalProtocol::project`] instead.
    BranchingSession,
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
            Self::ContinueOutsideChoice => write!(
                f,
                "`continue` must be the last statement of a choice branch inside a loop"
            ),
            Self::EmptyChoice(decider) => {
                write!(f, "the choice by `{decider}` has no branches")
            }
            Self::SoloChoice(decider) => write!(
                f,
                "the choice by `{decider}` has no other role to hear the branch"
            ),
            Self::BranchingSession => write!(
                f,
                "programs with choices or loops are checked through the global protocol"
            ),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl GlobalProtocol {
    /// Build a protocol of plain messages from `(from, to, label)` triples, in order.
    pub fn from_steps(steps: &[(&str, &str, &str)]) -> Self {
        Self {
            body: steps
                .iter()
                .map(|&(from, to, label)| {
                    Statement::Message(Step {
                        from: from.into(),
                        to: to.into(),
                        label: label.into(),
                    })
                })
                .collect(),
        }
    }

    /// Build a protocol from statements.
    pub fn new(body: Vec<Statement>) -> Self {
        Self { body }
    }

    /// The protocol's statements, in order.
    pub fn statements(&self) -> &[Statement] {
        &self.body
    }

    /// Roles in order of first appearance.
    pub fn roles(&self) -> Vec<String> {
        let mut roles = Vec::new();
        collect_roles(&self.body, &mut roles);
        roles
    }

    /// Parse a protocol from text, in the format used by `cargo pulse`.
    ///
    /// See the grammar in `crates/pulse-cli/README.md`. Comments run from `//` to
    /// the end of the line.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        parse::parse(text).map(Self::new)
    }

    /// Render the protocol as a Mermaid sequence diagram.
    ///
    /// Choices become `alt` blocks and loops become `loop` blocks. The decider's
    /// announcements are implicit in the diagram, so they are not drawn.
    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("sequenceDiagram\n");
        for role in self.roles() {
            out.push_str(&format!("    participant {role}\n"));
        }
        mermaid_body(&self.body, 1, &mut out);
        out
    }

    /// Check the protocol and project it onto each role.
    ///
    /// Fails on malformed structure, such as a message to oneself, a `continue`
    /// outside a loop's choice, or a choice that no other role hears. A protocol
    /// without choices or loops is then verified as a session, so matching and
    /// deadlock freedom are checked too.
    pub fn project(&self) -> Result<Session, ProtocolError> {
        let roles = self.roles();
        validate(&self.body, roles.len(), false, false)?;
        let programs = roles
            .iter()
            .map(|role| Program {
                role: role.clone(),
                actions: project_body(role, &roles, &self.body),
            })
            .collect();
        let session = Session { programs };
        if !session.is_branching() {
            session.verify()?;
        }
        Ok(session)
    }
}

fn collect_roles(statements: &[Statement], roles: &mut Vec<String>) {
    fn push(roles: &mut Vec<String>, role: &String) {
        if !roles.contains(role) {
            roles.push(role.clone());
        }
    }
    for statement in statements {
        match statement {
            Statement::Message(step) => {
                push(roles, &step.from);
                push(roles, &step.to);
            }
            Statement::Choice { decider, branches } => {
                push(roles, decider);
                for branch in branches {
                    collect_roles(branch, roles);
                }
            }
            Statement::Loop(body) => collect_roles(body, roles),
            Statement::Continue => {}
        }
    }
}

/// Reject structure the projection cannot follow.
///
/// `in_loop` says whether a loop encloses this list. `tail_of_branch` says
/// whether this list is a choice branch, so that a `continue` may end it.
fn validate(
    statements: &[Statement],
    role_count: usize,
    in_loop: bool,
    tail_of_branch: bool,
) -> Result<(), ProtocolError> {
    for (index, statement) in statements.iter().enumerate() {
        match statement {
            Statement::Message(step) => {
                if step.from == step.to {
                    return Err(ProtocolError::SelfMessage {
                        role: step.from.clone(),
                        label: step.label.clone(),
                    });
                }
            }
            Statement::Choice { decider, branches } => {
                if branches.is_empty() {
                    return Err(ProtocolError::EmptyChoice(decider.clone()));
                }
                if role_count < 2 {
                    return Err(ProtocolError::SoloChoice(decider.clone()));
                }
                for branch in branches {
                    validate(branch, role_count, in_loop, true)?;
                }
            }
            Statement::Loop(body) => validate(body, role_count, true, false)?,
            Statement::Continue => {
                let last = index + 1 == statements.len();
                if !(in_loop && tail_of_branch && last) {
                    return Err(ProtocolError::ContinueOutsideChoice);
                }
            }
        }
    }
    Ok(())
}

fn project_body(role: &str, roles: &[String], statements: &[Statement]) -> Vec<Action> {
    let mut actions = Vec::new();
    for statement in statements {
        match statement {
            Statement::Message(step) => {
                if step.from == role {
                    actions.push(Action::send(&step.to, &step.label));
                } else if step.to == role {
                    actions.push(Action::recv(&step.from, &step.label));
                }
            }
            Statement::Choice { decider, branches } => {
                let branches = branches
                    .iter()
                    .map(|branch| project_body(role, roles, branch))
                    .collect();
                if decider == role {
                    let to = roles
                        .iter()
                        .filter(|other| other.as_str() != role)
                        .cloned()
                        .collect();
                    actions.push(Action::Choose { to, branches });
                } else {
                    actions.push(Action::Offer {
                        from: decider.clone(),
                        branches,
                    });
                }
            }
            Statement::Loop(body) => actions.push(Action::Loop(project_body(role, roles, body))),
            Statement::Continue => actions.push(Action::Continue),
        }
    }
    actions
}

fn mermaid_body(statements: &[Statement], depth: usize, out: &mut String) {
    let indent = "    ".repeat(depth);
    for statement in statements {
        match statement {
            Statement::Message(step) => {
                // Mermaid reads `;` as the end of a message, so escape it.
                let label = step.label.replace(';', "#59;");
                out.push_str(&format!("{indent}{}->>{}: {label}\n", step.from, step.to));
            }
            Statement::Choice { decider, branches } => {
                for (index, branch) in branches.iter().enumerate() {
                    let keyword = if index == 0 { "alt" } else { "else" };
                    out.push_str(&format!(
                        "{indent}{keyword} {decider} picks branch {index}\n"
                    ));
                    mermaid_body(branch, depth + 1, out);
                }
                out.push_str(&format!("{indent}end\n"));
            }
            Statement::Loop(body) => {
                out.push_str(&format!("{indent}loop until a branch ends it\n"));
                mermaid_body(body, depth + 1, out);
                out.push_str(&format!("{indent}end\n"));
            }
            Statement::Continue => {}
        }
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

    fn is_branching(&self) -> bool {
        self.programs
            .iter()
            .any(|program| program.actions.iter().any(Action::is_branching))
    }

    /// Check that every message pairs up and that the programs cannot deadlock.
    ///
    /// Only straight-line programs are supported. Programs with choices or loops
    /// fail with [`ProtocolError::BranchingSession`].
    pub fn verify(&self) -> Result<(), ProtocolError> {
        if self.is_branching() {
            return Err(ProtocolError::BranchingSession);
        }
        let mut seen = BTreeMap::new();
        for program in &self.programs {
            if seen.insert(program.role.as_str(), ()).is_some() {
                return Err(ProtocolError::DuplicateRole(program.role.clone()));
            }
            for action in &program.actions {
                if peer_of(action) == Some(program.role.as_str()) {
                    return Err(ProtocolError::SelfMessage {
                        role: program.role.clone(),
                        label: label_of(action).into(),
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

/// The other role a straight-line action talks to.
fn peer_of(action: &Action) -> Option<&str> {
    match action {
        Action::Send { to, .. } => Some(to),
        Action::Recv { from, .. } => Some(from),
        _ => None,
    }
}

/// The label of a straight-line action, or empty for anything else.
fn label_of(action: &Action) -> &str {
    match action {
        Action::Send { label, .. } | Action::Recv { label, .. } => label,
        _ => "",
    }
}

fn event_name(role: &str, action: &Action) -> String {
    match action {
        Action::Send { to, label } => format!("{role} sends {label} to {to}"),
        Action::Recv { from, label } => format!("{role} waits for {label} from {from}"),
        _ => unreachable!("branching actions are rejected before verification"),
    }
}

/// Assert that a trace of events is a complete run of one role's program.
///
/// `protocol` is a [`GlobalProtocol`], and `role` names the role whose events
/// are listed. Each event is an [`Action`] built with [`Action::send`] or
/// [`Action::recv`], in the order the role performs them. The macro projects the
/// protocol, feeds each event to a [`Monitor`], and panics at the first event
/// that does not follow the program, or if the trace ends early.
///
/// ```
/// use pulse_multiparty::{assert_protocol, Action, GlobalProtocol};
///
/// let protocol = GlobalProtocol::parse("Client -> Server : Req; Server -> Client : Ok;").unwrap();
/// assert_protocol!(protocol, "Client", [Action::send("Server", "Req"), Action::recv("Server", "Ok")]);
/// ```
#[macro_export]
macro_rules! assert_protocol {
    ($protocol:expr, $role:expr, [$($event:expr),* $(,)?] $(,)?) => {{
        let session = $protocol
            .project()
            .unwrap_or_else(|error| panic!("protocol is not well formed: {error}"));
        let role: &str = $role;
        let program = session
            .programs()
            .iter()
            .find(|program| program.role == role)
            .cloned()
            .unwrap_or_else(|| panic!("the protocol has no role `{role}`"));
        let mut monitor = $crate::Monitor::new(program);
        $(
            if let Err(error) = monitor.observe(&$event) {
                panic!("trace does not follow the protocol for `{role}`: {error}");
            }
        )*
        if let Err(error) = monitor.finish() {
            panic!("trace does not follow the protocol for `{role}`: {error}");
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send(to: &str, label: &str) -> Action {
        Action::send(to, label)
    }

    fn recv(from: &str, label: &str) -> Action {
        Action::recv(from, label)
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
    fn branching_programs_are_not_verified_as_sessions() {
        let session = Session::new(vec![program(
            "A",
            vec![Action::Loop(vec![Action::Continue])],
        )]);
        assert_eq!(session.verify(), Err(ProtocolError::BranchingSession));
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
