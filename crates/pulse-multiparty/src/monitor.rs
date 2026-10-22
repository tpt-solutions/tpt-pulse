//! Run-time conformance: check a role's observed events against its program.

use std::fmt;

use crate::{branch_label, Action, Program};

/// Checks observed events against one role's program, one event at a time.
///
/// Feed the events a role actually performs, in order. The monitor accepts an
/// event only if the program allows it next. Call [`Monitor::finish`] at the end
/// to require that the whole program was performed. The monitor is deterministic:
/// the decider's branch is fixed by its first announcement, and a loop's next pass
/// or exit is fixed by the program's structure.
#[derive(Debug, Clone)]
pub struct Monitor {
    role: String,
    stack: Vec<Frame>,
}

/// One body being executed. A loop frame is where `continue` returns to.
#[derive(Debug, Clone)]
struct Frame {
    body: Vec<Action>,
    pos: usize,
    looping: bool,
}

/// A trace event that does not follow the program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorError {
    /// The event is not the next action the program allows.
    Unexpected {
        /// The action the program expected, or `None` if the program had finished.
        expected: Option<Action>,
        /// The event that was observed.
        observed: Action,
    },
    /// The trace ended before the program's actions were all performed.
    Incomplete {
        /// How many actions remain in the innermost unfinished body.
        remaining: usize,
    },
}

impl fmt::Display for MonitorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected { expected, observed } => match expected {
                Some(expected) => write!(
                    f,
                    "expected {}, observed {}",
                    describe(expected),
                    describe(observed)
                ),
                None => write!(
                    f,
                    "the program had finished, but observed {}",
                    describe(observed)
                ),
            },
            Self::Incomplete { remaining } => {
                write!(f, "trace ended with {remaining} action(s) still to perform")
            }
        }
    }
}

impl std::error::Error for MonitorError {}

impl Monitor {
    /// Monitor one role's program, as produced by [`crate::GlobalProtocol::project`].
    pub fn new(program: Program) -> Self {
        let mut monitor = Self {
            role: program.role,
            stack: vec![Frame {
                body: program.actions,
                pos: 0,
                looping: false,
            }],
        };
        monitor.settle();
        monitor
    }

    /// The role being monitored.
    pub fn role(&self) -> &str {
        &self.role
    }

    /// The next event the program allows, or `None` once it has finished.
    ///
    /// Where the next step is a branch, this is the announcement of branch 0.
    pub fn expected(&self) -> Option<Action> {
        match self.current()? {
            action @ (Action::Send { .. } | Action::Recv { .. }) => Some(action.clone()),
            Action::Choose { to, .. } => {
                to.first().map(|first| Action::send(first, branch_label(0)))
            }
            Action::Offer { from, .. } => Some(Action::recv(from, branch_label(0))),
            _ => None,
        }
    }

    /// Accept one observed event, or report that it does not follow the program.
    pub fn observe(&mut self, event: &Action) -> Result<(), MonitorError> {
        let Some(head) = self.current().cloned() else {
            return Err(self.unexpected(event));
        };
        match head {
            Action::Send { .. } | Action::Recv { .. } => {
                if &head != event {
                    return Err(self.unexpected(event));
                }
                self.advance();
            }
            Action::Choose { to, branches } => {
                let Some(first) = to.first() else {
                    return Err(self.unexpected(event));
                };
                let Some(index) = (0..branches.len())
                    .find(|&index| *event == Action::send(first, branch_label(index)))
                else {
                    return Err(self.unexpected(event));
                };
                // The first announcement is the event itself. The rest follow, then the branch.
                let mut body: Vec<Action> = to[1..]
                    .iter()
                    .map(|other| Action::send(other, branch_label(index)))
                    .collect();
                body.extend(branches[index].iter().cloned());
                self.advance();
                self.stack.push(Frame {
                    body,
                    pos: 0,
                    looping: false,
                });
            }
            Action::Offer { from, branches } => {
                let Some(index) = (0..branches.len())
                    .find(|&index| *event == Action::recv(&from, branch_label(index)))
                else {
                    return Err(self.unexpected(event));
                };
                self.advance();
                self.stack.push(Frame {
                    body: branches[index].clone(),
                    pos: 0,
                    looping: false,
                });
            }
            Action::Loop(_) | Action::Continue => {
                unreachable!("settle leaves only events at the head")
            }
        }
        self.settle();
        Ok(())
    }

    /// Require that every action of the program has been performed.
    pub fn finish(&self) -> Result<(), MonitorError> {
        match self.stack.last() {
            None => Ok(()),
            Some(frame) => Err(MonitorError::Incomplete {
                remaining: frame.body.len() - frame.pos,
            }),
        }
    }

    fn current(&self) -> Option<&Action> {
        let frame = self.stack.last()?;
        frame.body.get(frame.pos)
    }

    fn advance(&mut self) {
        if let Some(frame) = self.stack.last_mut() {
            frame.pos += 1;
        }
    }

    fn unexpected(&self, event: &Action) -> MonitorError {
        MonitorError::Unexpected {
            expected: self.expected(),
            observed: event.clone(),
        }
    }

    /// Run the silent steps: enter loops, leave finished bodies, and restart loops
    /// on `continue`. Stops at the next event, or when the program is finished.
    fn settle(&mut self) {
        loop {
            let Some(frame) = self.stack.last_mut() else {
                return;
            };
            if frame.pos == frame.body.len() {
                self.stack.pop();
                continue;
            }
            match &frame.body[frame.pos] {
                Action::Loop(body) => {
                    let body = body.clone();
                    frame.pos += 1;
                    self.stack.push(Frame {
                        body,
                        pos: 0,
                        looping: true,
                    });
                }
                Action::Continue => {
                    let index = self
                        .stack
                        .iter()
                        .rposition(|frame| frame.looping)
                        .expect("validated: `continue` is inside a loop");
                    self.stack.truncate(index + 1);
                    self.stack[index].pos = 0;
                }
                _ => return,
            }
        }
    }
}

fn describe(action: &Action) -> String {
    match action {
        Action::Send { to, label } => format!("a send of `{label}` to `{to}`"),
        Action::Recv { from, label } => format!("a receive of `{label}` from `{from}`"),
        Action::Choose { .. } => "a branch announcement".into(),
        Action::Offer { from, .. } => format!("a branch announcement from `{from}`"),
        Action::Loop(_) => "the start of a loop".into(),
        Action::Continue => "the next pass of a loop".into(),
    }
}
