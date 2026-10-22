//! Runs every role of a protocol together on FIFO queues, and records the trace.
//!
//! The decider's branch at each choice is picked by a caller-supplied policy.
//! The roles are checked by [`Monitor`]s, so a run either follows the protocol
//! exactly or stops with an error. A recorded trace can be replayed against one
//! real endpoint, with the other roles played by the simulation.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use crate::{branch_label, Action, GlobalProtocol, Monitor, MonitorError};

/// Upper bound on branch indices tried for one choice.
const MAX_BRANCHES: usize = 64;

/// Events in the order the simulation performed them, with the role that performed each.
pub type Trace = Vec<(String, Action)>;

/// A finished or cut-off run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every role finished its program.
    Finished(Trace),
    /// The run hit `max_steps` while every role could still move. This happens
    /// with loops that keep repeating.
    CutOff(Trace),
}

/// A run that went wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationError {
    /// The protocol is not well formed, so it cannot be run.
    Protocol(crate::ProtocolError),
    /// No role could move, and some role had not finished.
    Stuck {
        /// The roles that were waiting.
        waiting: Vec<String>,
        /// The trace up to the point where the run got stuck.
        trace: Trace,
    },
    /// A role could not perform an event its program required.
    Mismatch {
        /// The role.
        role: String,
        /// Why the event was rejected.
        error: Box<MonitorError>,
    },
    /// Every role was done, but a role's program was not finished.
    Unfinished {
        /// The role.
        role: String,
        /// Why the program is unfinished.
        error: Box<MonitorError>,
    },
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(error) => write!(f, "{error}"),
            Self::Stuck { waiting, .. } => {
                write!(
                    f,
                    "deadlock: no role can move; waiting: {}",
                    waiting.join(", ")
                )
            }
            Self::Mismatch { role, error } => write!(f, "`{role}`: {error}"),
            Self::Unfinished { role, error } => write!(f, "`{role}` did not finish: {error}"),
        }
    }
}

impl std::error::Error for SimulationError {}

/// Run `protocol` for at most `max_steps` rounds.
///
/// `policy(n)` gives the preferred branch for the `n`th choice made in the run.
/// If that branch is not available, the next index that the decider's
/// announcement accepts is taken.
pub fn simulate(
    protocol: &GlobalProtocol,
    mut policy: impl FnMut(usize) -> usize,
    max_steps: usize,
) -> Result<Outcome, SimulationError> {
    let session = protocol.project().map_err(SimulationError::Protocol)?;
    let mut monitors: BTreeMap<String, Monitor> = session
        .programs()
        .iter()
        .map(|program| (program.role.clone(), Monitor::new(program.clone())))
        .collect();
    let mut queues: BTreeMap<(String, String), VecDeque<String>> = BTreeMap::new();
    let mut trace = Trace::new();
    let mut decisions = 0;

    for _ in 0..max_steps {
        if monitors
            .values()
            .all(|monitor| monitor.expected().is_none())
        {
            for (role, monitor) in &monitors {
                monitor
                    .finish()
                    .map_err(|error| SimulationError::Unfinished {
                        role: role.clone(),
                        error: Box::new(error),
                    })?;
            }
            return Ok(Outcome::Finished(trace));
        }

        let mut progressed = false;
        for (role, monitor) in monitors.iter_mut() {
            let Some(expected) = monitor.expected() else {
                continue;
            };
            let moved = match &expected {
                Action::Send { to, label } if label.starts_with("choice#") => {
                    // A branch announcement. Try the preferred branch first, then the rest.
                    let preferred = policy(decisions) % MAX_BRANCHES;
                    let accepted = (0..MAX_BRANCHES)
                        .map(|offset| (preferred + offset) % MAX_BRANCHES)
                        .find(|&index| {
                            monitor
                                .observe(&Action::send(to, branch_label(index)))
                                .is_ok()
                        });
                    let Some(index) = accepted else {
                        continue;
                    };
                    queues
                        .entry((role.clone(), to.clone()))
                        .or_default()
                        .push_back(branch_label(index));
                    trace.push((role.clone(), Action::send(to, branch_label(index))));
                    decisions += 1;
                    true
                }
                Action::Send { to, label } => {
                    queues
                        .entry((role.clone(), to.clone()))
                        .or_default()
                        .push_back(label.clone());
                    observe(monitor, role, &expected, &mut trace)?;
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
                    if !matches {
                        continue;
                    }
                    let front = front.expect("matched a queued message");
                    queue.pop_front();
                    observe(monitor, role, &Action::recv(from, front), &mut trace)?;
                    true
                }
                _ => unreachable!("expected() only returns sends and receives"),
            };
            progressed |= moved;
        }

        if !progressed {
            let waiting = monitors
                .iter()
                .filter(|(_, monitor)| monitor.expected().is_some())
                .map(|(role, _)| role.clone())
                .collect();
            return Err(SimulationError::Stuck { waiting, trace });
        }
    }
    Ok(Outcome::CutOff(trace))
}

fn observe(
    monitor: &mut Monitor,
    role: &str,
    event: &Action,
    trace: &mut Trace,
) -> Result<(), SimulationError> {
    monitor
        .observe(event)
        .map_err(|error| SimulationError::Mismatch {
            role: role.to_string(),
            error: Box::new(error),
        })?;
    trace.push((role.to_string(), event.clone()));
    Ok(())
}
