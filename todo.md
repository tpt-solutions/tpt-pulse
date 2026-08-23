# tpt-flow TODO

Session-Typed Channels and Compile-Time Reactive Signals for Rust. Tracks all work for the whole project, phase by phase. See [spec.txt](spec.txt) for the design.

License: dual **MIT OR Apache-2.0**. Author: **TPT Solutions**.

> **Linear-emulation note:** spec.txt §5 Phase 1 calls for "linear type emulation (via `Drop` checks)". Rust has no native linear types, so `Chan` consumption is enforced at compile time via type-state (each op consumes `self` and returns a new type), while a `Drop` impl on any non-`Close`d channel state is a *runtime* safety net (panic/abort on drop) catching cases the type system can't, e.g. an early `return` or `?` that abandons a channel mid-protocol. Track this distinction clearly in docs so it isn't oversold as a compile-time guarantee.
>
> **Multiparty scope note:** spec.txt §5 Phase 3 mentions "multiparty session types" alongside the core two-party `Chan<Send<..>, Recv<..>>` API in §4. Per project decision, Phase 3 below implements full multiparty support (global-protocol definition + per-role projection + `tpt-telos`-verified deadlock-freedom), not a stub.

## Phase 0: Project Scaffolding & Licensing

- [ ] Initialize git repo and `.gitignore` (`/target`, `Cargo.lock`, `.mimocode`)
- [ ] Create Cargo workspace (`resolver = "2"`) with `[workspace.package]` metadata: `license = "MIT OR Apache-2.0"`, `authors = ["TPT Solutions"]`, `edition`, `repository`, `homepage`, `documentation`, `keywords`, `categories`
- [ ] Add `LICENSE-MIT` and `LICENSE-APACHE` files (copyright holder: TPT Solutions)
- [ ] Scaffold crate layout under `crates/`:
  - [ ] `flow-core` — `Chan<Protocol, Role>` type-state channel, protocol algebra (`Send<M, P>`, `Recv<M, P>`, `Either<L, R>`, `Close`), `Dual` trait, `Drop`-based runtime safety net
  - [ ] `flow-ir` — shared IR bridging both session-protocol state machines and signal DAG edges into the `tpt-telos` constraint format
  - [ ] `flow-macros` — proc-macro crate for compile-time reactive DAG generation
  - [ ] `flow-signals` — `Signal<T>` reactive primitive, `map`/`map2`, zero-cost propagation
  - [ ] `flow-multiparty` — global protocol description + per-role projection to `flow-core` local types
  - [ ] `flow-backend-tokio`, `flow-backend-leptos`, `flow-backend-dioxus` — integration adapter crates (implemented in Phase 4, scaffolded as empty crates now)
- [ ] `tpt-flow` umbrella crate re-exporting a `prelude` module matching the spec's `use tpt_flow::prelude::*;`
- [ ] README.md (overview, quickstart, links to spec)
- [ ] ARCHITECTURE.md capturing the diagram in spec.txt §3
- [ ] CONTRIBUTING.md
- [ ] CHANGELOG.md
- [ ] `.github/workflows/ci.yml` (fmt, clippy, test) mirroring `tpt-telos`
- [ ] `.github/ISSUE_TEMPLATE/` + `pull_request_template.md`
- [ ] `examples/` directory with its own README

## Phase 1: Session-Typed Channels (Months 1-3)

- [ ] Define protocol algebra types in `flow-core`: `Send<M, Cont>`, `Recv<M, Cont>`, `Either<L, R>` (offer/choice), `Close`
- [ ] Define `Chan<P, Role>` type-state wrapper; each op consumes `self` by value and returns the continuation type (`P`'s tail)
- [ ] Implement `Chan<Send<M, P>, R>::send(self, msg: M) -> Chan<P, R>`
- [ ] Implement `Chan<Recv<M, P>, R>::recv(self) -> (M, Chan<P, R>)`
- [ ] Implement choice/offer combinators for `Either<L, R>` matching spec.txt §4's `Either::Left(ok_chan)` / `Either::Right(err_chan)` pattern
- [ ] Implement `Dual` trait proving one endpoint's protocol is the exact dual (`Send`↔`Recv`) of the other endpoint's, checked at channel-pair construction
- [ ] Implement `Drop` on any non-`Close` `Chan` state that panics/aborts if a channel is abandoned mid-protocol (the runtime safety net described above)
- [ ] In-memory channel pair constructor (`session_channel::<P>()`) producing dual-typed endpoints wired via `std::sync::mpsc` or `crossbeam`
- [ ] Unit tests: spec.txt §4's `LoginProto` (`login_client`) compiles and runs correctly for both the `AuthOk` and `AuthErr` branches against a matching server implementation
- [ ] `trybuild` compile-fail tests: sending when the protocol expects a recv fails to compile; performing ops out of sequence fails to compile; recv'ing the wrong message type fails to compile
- [ ] **Milestone:** spec.txt §4's `LoginProto` example compiles, runs end-to-end over the in-memory channel for both branches, and a reordered send/recv sequence fails to compile.

## Phase 2: Compile-Time Reactive Signals (Months 4-6)

- [ ] Define `Signal<T>` primitive in `flow-signals` (current value + statically-known dependents)
- [ ] Implement `Signal::new(initial)`, `.get()`
- [ ] Implement `Signal::map(&self, f)` producing a derived `Signal<U>`
- [ ] Implement `Signal::map2(&a, &b, f)` producing a derived `Signal<U>` combining two signals (spec.txt §4 example)
- [ ] Design the compile-time DAG representation and pick the macro ergonomics for `flow-macros` (e.g. an attribute macro on functions like `robotics_control_loop` vs. a `signal_graph! { ... }` block) — evaluate both, commit to one
- [ ] Implement the proc-macro pass: analyze `Signal::new`/`map`/`map2` call sites in the annotated scope, build the dependency graph, and codegen `.set()` as a direct, inlined call chain to every downstream dependent (no dynamic dispatch, no runtime graph walk)
- [ ] Unit tests: spec.txt §4's `robotics_control_loop` example produces the correct `motor_out` value after `sensor_a.set(10.0)`
- [ ] Codegen/benchmark check confirming zero heap allocation and no vtable dispatch in generated propagation code
- [ ] **Milestone:** spec.txt §4's `robotics_control_loop` example compiles, and codegen inspection confirms `.set()` lowers to direct function calls with no runtime graph and no allocation.

## Phase 3: `tpt-telos` Integration — Multiparty Protocols & Acyclic Graphs (Months 7-9)

- [ ] Add `tpt-telos` (`C:\Programming\2 WIP\tpt-telos`) as a workspace path dependency (revisit as a versioned/git dependency once tpt-flow is ready to publish)
- [ ] Design a global protocol description type in `flow-multiparty` (N roles, ordered message exchanges) following standard multiparty session type theory
- [ ] Implement projection: global protocol → per-role local `Chan<P, RoleX>` types consumable by `flow-core`
- [ ] Bridge per-role local protocol state machines into `tpt-telos`'s constraint/state-transition format to prove whole-session deadlock-freedom (no role can end up waiting on a message no other role will ever send)
- [ ] Bridge `flow-signals` DAG edges (via `flow-ir`) into `tpt-telos`'s constraint format to prove acyclicity (no signal transitively depends on itself)
- [ ] Implement a build-time verification pass run alongside/before macro codegen for both multiparty protocols and signal graphs
- [ ] Integration test: a deliberately deadlocking 3-role protocol (e.g. two roles each waiting to receive from the other) is caught by `tpt-telos` with a useful diagnostic
- [ ] Integration test: a deliberately cyclic signal graph (e.g. `combined` feeding back into `sensor_a`) is caught by `tpt-telos` before compilation succeeds
- [ ] **Milestone:** a correct 3-role multiparty protocol compiles and verifies deadlock-free via `tpt-telos`; an intentionally deadlocking protocol and an intentionally cyclic signal graph are each caught before compiled output.

## Phase 4: `tokio`, `leptos` & `dioxus` Integrations (Months 10-12+)

- [ ] `flow-backend-tokio`: async `AsyncChan<P, Role>` wrapping tokio channels/sockets, with `async fn send`/`async fn recv` mirroring the sync `flow-core` API
- [ ] Port the `LoginProto` example to an async client/server running over real tokio TCP sockets
- [ ] `flow-backend-leptos`: bridge `Signal<T>` into leptos's reactive system (conversions to/from leptos `Signal`/`RwSignal`, or direct usability inside `view!`)
- [ ] `flow-backend-dioxus`: bridge `Signal<T>` into dioxus's `Signal`/hook system equivalently
- [ ] Example: small leptos app driven by `tpt-flow` signals for UI state
- [ ] Example: small dioxus app doing the same
- [ ] Integration tests: async `LoginProto` runs correctly over real tokio TCP sockets; leptos/dioxus example signal updates correctly re-render components
- [ ] **Milestone:** async `LoginProto` example runs end-to-end over tokio TCP; the `robotics_control_loop` signals drive live UI state updates in both a leptos and a dioxus demo app.

## Phase 5: Documentation, Examples & Release

- [ ] Full API docs (`cargo doc`) for all public types, traits, and the reactive-DAG proc-macro
- [ ] Expand `examples/`: login protocol (sync + async), multiparty protocol demo, robotics control loop, leptos UI demo, dioxus UI demo
- [ ] Write a migration/getting-started guide in README.md
- [ ] Publish crates to crates.io (`flow-core`, `flow-ir`, `flow-macros`, `flow-signals`, `flow-multiparty`, backend adapters, `tpt-flow` umbrella crate)
- [ ] Set up `docs.rs` documentation
- [ ] Tag `v0.1.0` release
</content>
