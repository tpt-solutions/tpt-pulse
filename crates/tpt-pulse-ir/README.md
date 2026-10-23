# tpt-pulse-ir

The ordering intermediate representation shared by tpt-pulse's compile-time checks.
Both the signal-graph macro and the multiparty protocol checks reduce to one
question: does some schedule exist that respects a set of "must happen before"
edges?

This crate is internal plumbing. It is not usually used directly. It is
`publish = false` for now.

Part of [tpt-pulse](../../README.md).

## What it does

An `OrderGraph` holds named events and "before -> after" edges between them. Each
event gets a rank variable, and each edge becomes the linear constraint

```text
rank(after) - rank(before) > 0
```

These constraints are satisfiable exactly when the edges form a DAG. The
satisfiability check is delegated to the [tpt-telos](https://github.com/tpt-solutions/tpt-telos)
solver (`tpt-telos-ir` and `tpt-telos-verifier`), pinned to a commit in the
workspace `Cargo.toml`.

When the check fails, `OrderViolation` describes the problem, so the caller can
report the cycle or the ordering conflict.

## Public API

- `OrderGraph::new`: an empty graph (also `Default`).
- `OrderGraph::event(name)`: adds a named event and returns its index.
- `OrderGraph::before(a, b)`: adds the edge "event `a` happens before event `b`".
- `OrderGraph::verify()`: returns `Ok(())` when a schedule exists, or an
  `OrderViolation` otherwise.
- `OrderGraph::to_dot()`: writes the graph in Graphviz DOT format, for inspection.
- `OrderViolation`: the error returned when no schedule exists. The type is `Clone`,
  `Eq`, and `Debug`.

Who uses it:

- [tpt-pulse-macros](../tpt-pulse-macros/README.md) builds an `OrderGraph` for each
  `#[signal_graph]` and rejects cycles at compile time.
- [tpt-pulse-multiparty](../tpt-pulse-multiparty/README.md) uses the same kind of
  check for sequential protocol schedules.

## Guarantees

- A returned schedule respects every edge. The solver decides satisfiability, so the
  result is as strong as the solver.
- The graph does not emit duplicate variable terms, because the solver does not
  merge them. See the limitation below.

## Limitations

- **Solver bug with duplicate terms.** The `tpt-telos` solver reports `x - x > 0` as
  satisfiable, since it does not merge duplicate variable terms. This crate never
  emits such terms, but the solver itself is wrong, and this is recorded in the
  workspace changelog.
- **Pinned solver.** The results depend on the pinned `tpt-telos` revision. Updating
  that revision can change behavior and should be reviewed.

## Testing

`cargo test -p tpt-pulse-ir` runs the unit tests. The cycle and ordering behavior is
also tested through the macro and multiparty crates, which use this graph.

## License

MIT OR Apache-2.0, at your option. See [LICENSE-MIT](../../LICENSE-MIT) and
[LICENSE-APACHE](../../LICENSE-APACHE).
