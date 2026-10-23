# Changelog

All notable changes to `tpt-pulse-signals` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- `#[signal_graph]` lowers signal declarations and `set` into plain locals with inlined
  recomputation, checked for cycles at compile time.
- A counting-allocator test that confirms zero heap allocation during propagation.
- Sub-graphs: a `#[signal_graph]` function with `Signal` parameters can be called from
  another graph, and `set` in the caller reaches its signals. Limits are in the crate docs.
- The robotics control loop from the spec as a test, and a propagation benchmark.

### Known gaps

- Signal graphs have no DOT export, because the macro lowers them away at compile time.
- Dependencies are limited to one annotated function. Closures and `Signal::new`
  initializers may not read signals directly, and `set` applies only to sources, as a
  top-level statement.
- Sub-graphs must be declared before their callers in the same module. Cross-crate and
  nested sub-graphs are not supported.
