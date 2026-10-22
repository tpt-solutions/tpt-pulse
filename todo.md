# tpt-pulse TODO

Session-Typed Channels and Compile-Time Reactive Signals for Rust. Tracks all work for the whole project, phase by phase. See [spec.txt](spec.txt) for the design.

License: dual **MIT OR Apache-2.0**. Author: **TPT Solutions**.

> **Linear-emulation note:** spec.txt §5 Phase 1 calls for "linear type emulation (via `Drop` checks)". Rust has no native linear types, so `Chan` consumption is enforced at compile time via type-state (each op consumes `self` and returns a new type), while a `Drop` impl on any non-`Close`d channel state is a *runtime* safety net (panic/abort on drop) catching cases the type system can't, e.g. an early `return` or `?` that abandons a channel mid-protocol. Track this distinction clearly in docs so it isn't oversold as a compile-time guarantee.
>
> **Multiparty scope note:** spec.txt §5 Phase 3 mentions "multiparty session types" alongside the core two-party `Chan<Send<..>, Recv<..>>` API in §4. Per project decision, Phase 3 below implements full multiparty support (global-protocol definition + per-role projection + `tpt-telos`-verified deadlock-freedom), not a stub.
>
> **Deferred:** the Leptos and Dioxus integrations (Phase 4 and parts of Phase 5) are deferred to a follow-up release, per the project decision to ship Tokio first. Publishing (Phase 5) is on hold until the owner approves it.

## Phase 0: Project Scaffolding & Licensing

- [x] Initialize git repo and `.gitignore` (`/target`, `Cargo.lock`, `.mimocode`)
- [x] Create Cargo workspace (`resolver = "2"`) with `[workspace.package]` metadata: `license = "MIT OR Apache-2.0"`, `authors = ["TPT Solutions"]`, `edition`, `repository`, `homepage`, `documentation`, `keywords`, `categories`
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` files (copyright holder: TPT Solutions)
- [x] Scaffold crate layout under `crates/`:
  - [x] `pulse-core` — `Chan<Protocol, Role>` type-state channel, protocol algebra (`Send<M, P>`, `Recv<M, P>`, `Either<L, R>`, `Close`), `Dual` trait, `Drop`-based runtime safety net
  - [x] `pulse-ir` — ordering IR; signal-DAG and protocol-schedule edges are checked through `tpt-telos`'s constraint solver as strict rank constraints
  - [x] `pulse-macros` — proc-macros: `#[signal_graph]` and `global_protocol!`
  - [x] `pulse-signals` — `Signal<T>` marker, `map`/`map2`, allocation-free propagation via `#[signal_graph]`
  - [x] `pulse-multiparty` — global protocol description, per-role projection to local programs, deadlock check
  - [x] `pulse-backend-tokio` — async TCP session channels
  - [ ] `pulse-backend-leptos`, `pulse-backend-dioxus` — deferred; remain empty scaffolds
- [x] `tpt-pulse` umbrella crate re-exporting a `prelude` module matching the spec's `use tpt_pulse::prelude::*;`
- [x] README.md (overview, quickstart, links to spec)
- [x] ARCHITECTURE.md capturing the diagram in spec.txt §3
- [x] CONTRIBUTING.md
- [x] CHANGELOG.md
- [x] `.github/workflows/ci.yml` (fmt, clippy, test) mirroring `tpt-telos`; push trigger on `master`
- [x] `.github/ISSUE_TEMPLATE/` + `pull_request_template.md`
- [x] `examples/` directory with its own README

## Phase 1: Session-Typed Channels (Months 1-3)

- [x] Define protocol algebra types in `pulse-core`: `Send<M, Cont>`, `Recv<M, Cont>`, `Either<L, R>` (offer/choice), `Close`
- [x] Define `Chan<P, Role>` type-state wrapper; each op consumes `self` by value and returns the continuation type (`P`'s tail)
- [x] Implement `Chan<Send<M, P>, R>::send(self, msg: M) -> Chan<P, R>`
- [x] Implement `Chan<Recv<M, P>, R>::recv(self) -> (M, Chan<P, R>)`
- [x] Implement choice/offer combinators for `Either<L, R>` matching spec.txt §4's `Either::Left(ok_chan)` / `Either::Right(err_chan)` pattern
- [x] Implement `Dual` trait proving one endpoint's protocol is the exact dual (`Send`↔`Recv`) of the other endpoint's, checked at channel-pair construction
- [x] Implement `Drop` on any non-`Close` `Chan` state that panics/aborts if a channel is abandoned mid-protocol (the runtime safety net described above)
- [x] In-memory channel pair constructor (`session_channel::<P>()`) producing dual-typed endpoints wired via `std::sync::mpsc` or `crossbeam`
- [x] Unit tests: spec.txt §4's `LoginProto` (`login_client`) compiles and runs correctly for both the `AuthOk` and `AuthErr` branches against a matching server implementation
- [x] `trybuild` compile-fail tests: sending when the protocol expects a recv fails to compile; performing ops out of sequence fails to compile; recv'ing the wrong message type fails to compile
- [x] **Milestone:** spec.txt §4's `LoginProto` example compiles, runs end-to-end over the in-memory channel for both branches, and a reordered send/recv sequence fails to compile.

## Phase 2: Compile-Time Reactive Signals (Months 4-6)

- [x] Define `Signal<T>` primitive in `pulse-signals`. Note: `Signal` is a compile-time marker; its methods panic if called outside `#[signal_graph]`, and the macro lowers every use. Dependencies are a compile-time graph, not a runtime one.
- [x] Implement `Signal::new(initial)`, `.get()`
- [x] Implement `Signal::map(&self, f)` producing a derived `Signal<U>`
- [x] Implement `Signal::map2(&a, &b, f)` producing a derived `Signal<U>` combining two signals (spec.txt §4 example)
- [x] Design the compile-time DAG representation and pick the macro ergonomics for `pulse-macros` — **decided: attribute macro `#[signal_graph]` on the function**, which keeps spec §4's code unchanged. A `signal_graph! { ... }` block was rejected because it adds syntax the spec does not show.
- [x] Implement the proc-macro pass: analyze `Signal::new`/`map`/`map2` call sites, build the dependency graph, check it with `pulse-ir`/`tpt-telos`, and lower `.set()` to a direct, inlined recomputation of every downstream dependent (no dynamic dispatch, no runtime graph walk). Limitations: signals are local to the annotated function; closures and `Signal::new` initializers may not read signals directly; `set` is only allowed on sources as a top-level statement.
- [x] Unit tests: spec.txt §4's `robotics_control_loop` example produces the correct `motor_out` value after `sensor_a.set(10.0)` (`crates/pulse-signals/tests/robotics.rs`)
- [x] Allocation check: a counting global allocator confirms zero heap allocation during propagation (`crates/pulse-signals/tests/no_alloc.rs`). Assembly inspection (`cargo asm`) was not done; "no vtable dispatch" holds by construction, since the lowering emits only direct calls.
- [x] **Milestone:** spec.txt §4's `robotics_control_loop` example compiles and produces the documented output (`cargo run --example robotics`), and propagation performs no allocation.

## Phase 3: `tpt-telos` Integration — Multiparty Protocols & Acyclic Graphs (Months 7-9)

- [x] Add `tpt-telos` as a git dependency (pinned to commit `7d2f62c`) through `pulse-ir`
- [x] Design a global protocol description type in `pulse-multiparty` (N roles, ordered message exchanges) following standard multiparty session type theory
- [ ] Implement projection: global protocol → per-role local `Chan<P, RoleX>` types consumable by `pulse-core`. **Partial:** projection produces per-role `Program`s of send/receive actions and verifies them. Generating type-level `Chan<P, RoleX>` aliases from those programs is not implemented.
- [x] Bridge per-role local protocol state machines into `tpt-telos`'s constraint format to prove whole-session deadlock-freedom. Method: each action is an event, program order and FIFO message order become precedence edges, and `tpt-telos` checks that a schedule exists. Sound for asynchronous FIFO channels.
- [x] Bridge `pulse-signals` DAG edges (via `pulse-ir`) into `tpt-telos`'s constraint format to prove acyclicity (no signal transitively depends on itself)
- [x] Implement a build-time verification pass for both multiparty protocols (`global_protocol!` projects and checks at compile time) and signal graphs (`#[signal_graph]` checks at compile time)
- [x] Integration test: a deliberately deadlocking 3-role protocol is caught by `tpt-telos` with a useful diagnostic (`crates/pulse-multiparty` tests)
- [x] Integration test: a deliberately cyclic signal graph is caught by `tpt-telos` before compilation succeeds (`crates/pulse-signals/tests/ui/cycle.rs`)
- [x] **Milestone:** a correct 3-role multiparty protocol verifies deadlock-free (`cargo run --example multiparty`); an intentionally deadlocking session and an intentionally cyclic signal graph are each caught before compiled output.

## Phase 4: `tokio`, `leptos` & `dioxus` Integrations (Months 10-12+)

- [x] `pulse-backend-tokio`: async `AsyncChan<P, Role>` over tokio TCP sockets, with `async fn send`/`async fn recv` mirroring the sync `pulse-core` API. Frames are length-prefixed JSON, so messages need `serde`.
- [x] Port the `LoginProto` example to an async client/server running over real tokio TCP sockets (`crates/pulse-backend-tokio/tests/login_tcp.rs`, `cargo run --example login_async`)
- [ ] `pulse-backend-leptos`: bridge `Signal<T>` into leptos's reactive system — **deferred**
- [ ] `pulse-backend-dioxus`: bridge `Signal<T>` into dioxus's `Signal`/hook system — **deferred**
- [ ] Example: small leptos app driven by `tpt-pulse` signals for UI state — **deferred**
- [ ] Example: small dioxus app doing the same — **deferred**
- [x] Integration tests: async `LoginProto` runs correctly over real tokio TCP sockets
- [ ] **Milestone:** async `LoginProto` runs end-to-end over tokio TCP (done); the signal-driven leptos and dioxus demo apps are deferred.

## Phase 5: Documentation, Examples & Release

- [x] Full API docs (`cargo doc`) for all public types, traits, and the reactive-DAG proc-macro (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passes)
- [x] Expand `examples/`: login protocol (sync + async), multiparty protocol demo, robotics control loop
- [ ] Leptos and Dioxus UI demos — **deferred**
- [x] Write a migration/getting-started guide in README.md
- [ ] Publish crates to crates.io — **on hold; not done, pending owner approval.** Crate names are not yet checked for availability.
- [ ] Set up `docs.rs` documentation — depends on publishing
- [ ] Tag `v0.1.0` release — depends on publishing
</content>
