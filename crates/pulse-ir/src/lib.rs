//! Ordering IR shared by signal graphs and protocol schedules.
//!
//! Both checks reduce to one question: does a schedule exist that respects a
//! set of "must happen before" edges? Each event gets a rank variable and each
//! edge `before -> after` becomes the linear constraint
//! `rank(after) - rank(before) > 0`. Those constraints are satisfiable exactly
//! when the edges form a DAG, and the satisfiability check is delegated to
//! `tpt-telos`'s solver.

use std::fmt;

use tpt_telos_ir::{Constraint, Linear, Relation};
use tpt_telos_verifier::unsat_checked;

/// A directed graph of named events where each edge means "must happen before".
#[derive(Debug, Clone, Default)]
pub struct OrderGraph {
    names: Vec<String>,
    edges: Vec<(usize, usize)>,
}

/// Why an [`OrderGraph`] has no valid schedule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderViolation {
    /// The events form a cycle; the names are the events on that cycle, in order.
    Cycle(Vec<String>),
    /// The solver could not decide satisfiability (for example, on overflow).
    /// This is reported as a failure, never as success.
    Undecided,
}

impl fmt::Display for OrderViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cycle(names) => {
                write!(f, "circular ordering: {}", names.join(" -> "))?;
                if let Some(first) = names.first() {
                    write!(f, " -> {first}")?;
                }
                Ok(())
            }
            Self::Undecided => write!(f, "solver could not decide the ordering"),
        }
    }
}

impl OrderGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an event and return its index.
    pub fn event(&mut self, name: impl Into<String>) -> usize {
        self.names.push(name.into());
        self.names.len() - 1
    }

    /// Require event `before` to happen before event `after`.
    pub fn before(&mut self, before: usize, after: usize) {
        self.edges.push((before, after));
    }

    /// Check that a schedule exists.
    pub fn verify(&self) -> Result<(), OrderViolation> {
        let constraints: Vec<Constraint> = self
            .edges
            .iter()
            .map(|&(before, after)| {
                // A self-edge needs no variables: the constraint `0 > 0` is unsatisfiable.
                // Duplicate variables are avoided because the solver does not merge them.
                let terms = if before == after {
                    Vec::new()
                } else {
                    vec![(rank(after), 1), (rank(before), -1)]
                };
                Constraint(Linear { terms, constant: 0 }, Relation::Gt)
            })
            .collect();
        match unsat_checked(&constraints) {
            Some(false) => Ok(()),
            Some(true) => match self.find_cycle() {
                Some(cycle) => Err(OrderViolation::Cycle(cycle)),
                None => Err(OrderViolation::Undecided),
            },
            None => Err(OrderViolation::Undecided),
        }
    }

    /// Depth-first search for one cycle, used only to explain an unsatisfiable graph.
    fn find_cycle(&self) -> Option<Vec<String>> {
        let mut adjacency = vec![Vec::new(); self.names.len()];
        for &(before, after) in &self.edges {
            adjacency[before].push(after);
        }
        // 0 = unvisited, 1 = on the current path, 2 = finished.
        let mut state = vec![0u8; self.names.len()];
        let mut path = Vec::new();
        for start in 0..self.names.len() {
            if let Some(cycle) = dfs(start, &adjacency, &mut state, &mut path) {
                return Some(cycle.into_iter().map(|i| self.names[i].clone()).collect());
            }
        }
        None
    }
}

fn rank(event: usize) -> String {
    format!("rank{event}")
}

fn dfs(
    node: usize,
    adjacency: &[Vec<usize>],
    state: &mut [u8],
    path: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    if state[node] == 2 {
        return None;
    }
    if state[node] == 1 {
        let start = path.iter().position(|&n| n == node)?;
        return Some(path[start..].to_vec());
    }
    state[node] = 1;
    path.push(node);
    for &next in &adjacency[node] {
        if let Some(cycle) = dfs(next, adjacency, state, path) {
            return Some(cycle);
        }
    }
    path.pop();
    state[node] = 2;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_is_schedulable() {
        let mut graph = OrderGraph::new();
        let a = graph.event("a");
        let b = graph.event("b");
        let c = graph.event("c");
        graph.before(a, b);
        graph.before(b, c);
        graph.before(a, c);
        assert_eq!(graph.verify(), Ok(()));
    }

    #[test]
    fn cycle_is_reported_with_its_events() {
        let mut graph = OrderGraph::new();
        let a = graph.event("a");
        let b = graph.event("b");
        let c = graph.event("c");
        graph.before(a, b);
        graph.before(b, c);
        graph.before(c, a);
        assert_eq!(
            graph.verify(),
            Err(OrderViolation::Cycle(vec![
                "a".into(),
                "b".into(),
                "c".into()
            ]))
        );
    }

    #[test]
    fn self_edge_is_a_cycle() {
        let mut graph = OrderGraph::new();
        let a = graph.event("a");
        graph.before(a, a);
        assert_eq!(graph.verify(), Err(OrderViolation::Cycle(vec!["a".into()])));
    }
}
