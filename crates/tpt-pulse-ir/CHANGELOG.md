# Changelog

All notable changes to `tpt-pulse-ir` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- Ordering checks: schedules expressed as strict rank constraints (`rank(after) -
  rank(before) > 0`), decided by the `tpt-telos` solver.
- `OrderGraph::to_dot` for Graphviz output.

### Known gaps

- The `tpt-telos` solver reports `x - x > 0` as satisfiable, because it does not merge
  duplicate variable terms. `tpt-pulse-ir` never emits duplicate terms, but the solver
  is wrong.
- Results depend on the `tpt-telos` revision pinned in the workspace `Cargo.toml`.
