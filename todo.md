# tpt-pulse TODO

Session-Typed Channels and Compile-Time Reactive Signals for Rust. Tracks all work for the whole project, phase by phase. See [spec.txt](spec.txt) for the design.

License: dual **MIT OR Apache-2.0**. Author: **TPT Solutions**.

> **Linear-emulation note:** spec.txt §5 Phase 1 calls for "linear type emulation (via `Drop` checks)". Rust has no native linear types, so `Chan` consumption is enforced at compile time via type-state (each op consumes `self` and returns a new type), while a `Drop` impl on any non-`Close`d channel state is a *runtime* safety net (panic/abort on drop) catching cases the type system can't, e.g. an early `return` or `?` that abandons a channel mid-protocol. Track this distinction clearly in docs so it isn't oversold as a compile-time guarantee.
>
> **Multiparty scope note:** spec.txt §5 Phase 3 mentions "multiparty session types" alongside the core two-party `Chan<Send<..>, Recv<..>>` API in §4. Per project decision, Phase 3 below implements full multiparty support (global-protocol definition + per-role projection + `tpt-telos`-verified deadlock-freedom), not a stub.
>
> **Publishing:** on hold until the owner approves it. The Phase 5 publishing, docs.rs, and tagging items stay open for that reason.

## Phase 0: Project Scaffolding & Licensing

- [x] Initialize git repo and `.gitignore` (`/target`, `Cargo.lock`, `.mimocode`)
- [x] Create Cargo workspace (`resolver = "2"`) with `[workspace.package]` metadata: `license = "MIT OR Apache-2.0"`, `authors = ["TPT Solutions"]`, `edition`, `repository`, `homepage`, `documentation`, `keywords`, `categories`
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` files (copyright holder: TPT Solutions)
- [x] Scaffold crate layout under `crates/`:
  - [x] `pulse-core` — `Chan<Protocol, Role>` type-state channel, protocol algebra (`Send<M, P>`, `Recv<M, P>`, `Either<L, R>`, `Close`), `Dual` trait, `Drop`-based runtime safety net
  - [x] `pulse-ir` — ordering IR; signal-DAG and protocol-schedule edges are checked through `tpt-telos`'s constraint solver as strict rank constraints
  - [x] `pulse-macros` — proc-macros: `#[signal_graph]`, `global_protocol!`, and `session_types!`
  - [x] `pulse-signals` — `Signal<T>` marker, `map`/`map2`, allocation-free propagation via `#[signal_graph]`
  - [x] `pulse-multiparty` — global protocol description, per-role projection to local programs, deadlock check
  - [x] `pulse-backend-tokio` — async TCP session channels
  - [x] `pulse-backend-leptos` — `Live` values and derived memos for Leptos
  - [x] `pulse-backend-dioxus` — `Live` values and derived hooks for Dioxus
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
- [x] Implement projection: global protocol → per-role local `Chan<P, RoleX>` types consumable by `pulse-core`. `session_types! { Name { A -> B : Msg; ... } }` emits a module with one `pulse-core` protocol type per role, so each endpoint is `Chan<Name::Role, Role>`. Dual roles are checked by the compiler (`tests/session_types.rs`). Sequential protocols only, which is what the global description can express.
- [x] Bridge per-role local protocol state machines into `tpt-telos`'s constraint format to prove whole-session deadlock-freedom. Method: each action is an event, program order and FIFO message order become precedence edges, and `tpt-telos` checks that a schedule exists. Sound for asynchronous FIFO channels.
- [x] Bridge `pulse-signals` DAG edges (via `pulse-ir`) into `tpt-telos`'s constraint format to prove acyclicity (no signal transitively depends on itself)
- [x] Implement a build-time verification pass for both multiparty protocols (`global_protocol!` and `session_types!` project and check at compile time) and signal graphs (`#[signal_graph]` checks at compile time)
- [x] Integration test: a deliberately deadlocking 3-role protocol is caught by `tpt-telos` with a useful diagnostic (`crates/pulse-multiparty` tests)
- [x] Integration test: a deliberately cyclic signal graph is caught by `tpt-telos` before compilation succeeds (`crates/pulse-signals/tests/ui/cycle.rs`)
- [x] **Milestone:** a correct 3-role multiparty protocol verifies deadlock-free (`cargo run --example multiparty`); an intentionally deadlocking session and an intentionally cyclic signal graph are each caught before compiled output.

## Phase 4: `tokio`, `leptos` & `dioxus` Integrations (Months 10-12+)

- [x] `pulse-backend-tokio`: async `AsyncChan<P, Role>` over tokio TCP sockets, with `async fn send`/`async fn recv` mirroring the sync `pulse-core` API. Frames are length-prefixed JSON, so messages need `serde`.
- [x] Port the `LoginProto` example to an async client/server running over real tokio TCP sockets (`crates/pulse-backend-tokio/tests/login_tcp.rs`, `cargo run --example login_async`)
- [x] `pulse-backend-leptos`: `Live<T>` wraps a Leptos `RwSignal`; `derived` and `derived2` produce memos that recompute a graph when inputs change. Conversions from `RwSignal` are provided. Scope: reactive values and memos, not a full component library.
- [x] `pulse-backend-dioxus`: `Live<T>` wraps a Dioxus `Signal`; `use_live` and `use_derived2` are hooks for components.
- [x] Example: small Leptos app driven by `tpt-pulse` signals (`cargo run --example leptos_app`). It renders HTML on the server; running it in a browser (client-side rendering) was not tested here, because it needs a wasm build.
- [x] Example: small Dioxus app doing the same (`cargo run --example dioxus_app`). It renders HTML on the server.
- [x] Integration tests: async `LoginProto` runs correctly over real tokio TCP sockets; Leptos memos and Dioxus components update after an input changes (`crates/pulse-backend-leptos/tests/live.rs`, `crates/pulse-backend-dioxus/tests/live.rs`)
- [x] **Milestone:** async `LoginProto` runs end-to-end over tokio TCP; the `robotics_control_loop` graph drives UI state in both a Leptos and a Dioxus demo app. Verified by server-side rendering and by re-rendering the Dioxus VirtualDom. Not verified in a browser.

## Phase 5: Documentation, Examples & Release

- [x] Full API docs (`cargo doc`) for all public types, traits, and the reactive-DAG proc-macro (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passes)
- [x] Expand `examples/`: login protocol (sync + async), multiparty protocol demo, robotics control loop, Leptos app, Dioxus app
- [x] Leptos and Dioxus UI demos (`examples/leptos_app.rs`, `examples/dioxus_app.rs`)
- [x] Write a migration/getting-started guide in README.md
- [ ] Publish crates to crates.io — **on hold; not done, pending owner approval.** Crate names are not yet checked for availability. **Blocker:** `tpt-telos-ir` and `tpt-telos-verifier` are git-only dependencies, which crates.io rejects. `tpt-telos` must be published first, then the workspace dependency switched to a version requirement.
- [ ] Set up `docs.rs` documentation — depends on publishing
- [ ] Tag `v0.1.0` release — depends on publishing

## Phase 6: Robustness & Core Expressiveness

Found in the post-Phase-5 review: there are no stubs or code TODOs, but these gaps limit real-world use. Order is by adoption impact.

- [x] Non-panicking API in `pulse-core`: `try_send`, `try_recv`, `try_offer`, `try_choose_left`/`right`, `recv_timeout`, and `SessionError`. The panicking methods stay as the default API, so existing examples are unchanged. Deviation from the original wording: the `try_` forms sit beside the panicking ones instead of replacing them. `recv_timeout` returns the endpoint on timeout so the receive can be retried. `disconnect()` aborts without panicking
- [x] Mirror the `try_` API in `pulse-backend-tokio`, with a generic transport. Tests: `tests/transports.rs` (in-memory, disconnect, bad frames) and `tests/unix_socket.rs` (Unix only; type-checked for Linux here, run in CI)
- [x] Timeouts for `pulse-backend-tokio`: `recv_timeout` awaits only the frame's first byte under the timeout. A one-byte read is cancel safe, so on timeout the endpoint is returned intact. The rest of the frame is read untimed, so framing cannot desynchronize. Not done: `offer` timeouts. Same design applies and can follow
- [x] Protocol recursion in `pulse-core`: `Rec<P>`, `Var`, `Chan::enter`, with `Dual` and substitution-based unfolding. Syntax: `Rec<Body>` where `Var` jumps back to the loop head, and each pass starts with `enter()`. The `Drop` safety net applies to every loop state (`Rec` is never complete). Tests: `recursion.rs` (counted loop, zero-iteration exit)
- [x] Multiparty `loop` and `choice` in `pulse-multiparty`. Design: the decider broadcasts the branch index (`choice#i`) to every other role, so each role projects to a branch-following `Offer` (or `Choose` for the decider). A loop ends when a body falls off its end. `continue` must end a choice branch inside a loop, so every pass communicates. Structure is checked by `GlobalProtocol::project`. Deadlock freedom follows from the broadcast, and `tests/simulation.rs` checks it on FIFO queues under several branch policies. This is a bounded check, not a proof. Text format: `crates/pulse-cli/README.md`. Tests: `tests/protocols.rs`, `tests/simulation.rs`
- [ ] Multiparty loops and choice in `global_protocol!`. Not done: the macro takes plain messages only. The text format is the way to write choices today
- [ ] `session_types!` support for loops and choice. Not done, by design: `session_types!` emits one `pulse-core` type per role. A choice needs an announcement to every other role, which the N-party types cannot express as a single endpoint type. A runtime monitor covers this case instead. Reopen if a two-party `Rec` form is wanted for `session_types!`
- [ ] Two-party shorthand for `session_types!`. **Decision: no new syntax for now.** `session_types!` stays the two-party form and keeps the `Name { A -> B : Msg; }` syntax. Reopen only if users ask for a different form
- [x] `#[diagnostic::on_unimplemented]` on `Protocol` and `Dual`. The existing trybuild fixtures still match; no new fixtures were added for the new messages
- [ ] Cross-function signal graphs in `pulse-signals`: deferred. The macro is 579 lines, and composing graphs needs a design for how sub-graphs share cycle checks
- [x] More transports for `pulse-backend-tokio`: Unix sockets and an in-memory pipe. TLS not done: it needs new dependencies and a certificate story
- [ ] TLS transport for `pulse-backend-tokio`
- [ ] Optional codecs besides JSON (e.g. bincode/postcard) behind a feature flag. **Decision: JSON stays the only codec.** The wire format is part of the protocol contract. Add a codec when a user needs a binary format

## Phase 7: Tooling & Adoption

- [x] Diagram export: `GlobalProtocol::to_mermaid()` and `OrderGraph::to_dot()` for the order graph in `pulse-ir`. Not done: DOT for signal graphs, since the macro lowers them away and exposes no graph at runtime. Would need the macro to emit a const description
- [x] `cargo pulse check <file>` CLI (`crates/pulse-cli`): verifies the file and prints each role's projected type. Also `cargo pulse mermaid`. Deviation: it reports malformed steps, not deadlocks, because a file of global steps cannot deadlock. Deadlock checks for hand-written local programs still need a file format for local programs, which is not built
- [x] Protocol file format for the CLI, with grammar and comments documented in `crates/pulse-cli/README.md`. Example: `examples/protocols/login.pulse`, checked in CI
- [x] Test harness: `assert_protocol!` (in `pulse-multiparty`, re-exported from the prelude) checks a trace of `Action::send`/`Action::recv` events against one role's program. Tests in `tests/assert_protocol.rs`
- [ ] Mock peer generated from a protocol. Not started: it needs typed code generation per protocol, and the protocol format has no message schemas to generate from
- [x] Runtime conformance monitor core: `pulse_multiparty::Monitor` checks an observed event sequence against one role's program, and `finish()` requires the whole program. Not done: the socket wrapper. Frames carry no message labels, so a wrapper cannot map bytes to protocol events until labels are added to the wire format
- [ ] Peer stub generation (TypeScript, Python) from a protocol definition. Recursion now exists for two-party protocols. Multiparty loops and a stable stub format are still needed. Not started: stubs need message schemas, which the protocol format does not have
- [ ] Starter template repo or `cargo generate` template. Deferred until the crates are published, since a template needs versioned dependencies
- [ ] Browser (wasm) test run for the Leptos and Dioxus demos. Partly done: both bridges compile for `wasm32-unknown-unknown`. Not done: running in a browser, which needs a wasm test runner and a browser

## Phase 8: Release Hygiene

- [ ] Publish `tpt-telos` to crates.io and replace the git dependencies in the workspace (prerequisite for Phase 5 publishing)
- [x] Pin and test an MSRV in CI: `rust-version = "1.89"` for all crates, and a CI job that runs `cargo check --workspace --all-targets` on 1.89. 1.89 is the lowest version that builds the whole workspace. Dev-dependencies and `uuid` (transitive) set the floor. The job uses the committed `Cargo.lock`, so commit it before relying on the job
- [x] `cargo deny` CI job with `deny.toml`. Advisories: `RUSTSEC-2024-0436` (`paste`, via leptos, no safe upgrade) is ignored with a reason. Bans: wildcards are a warning, because `tpt-telos` is a pinned git dependency. Not done: `cargo-semver-checks`. It needs a crates.io baseline, so it waits for the first publish
- [x] `Cargo.lock` decision: commit it. The workspace ships the `cargo-pulse` binary, and a committed lock keeps the MSRV job reproducible. `.gitignore` no longer ignores it. Not committed yet
- [x] Benchmarks: `exchange` (sync request/reply, about 650 ns on the dev machine) and `propagation` (signal graph, under 1 ns). Both compile in CI. Not done: tracking results across runs, which needs somewhere to store baselines
