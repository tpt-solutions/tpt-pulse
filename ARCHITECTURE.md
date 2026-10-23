# Architecture

The pipeline from spec section 3 is:

1. Protocol / reactive definitions feed a state-machine / DAG analyzer.
2. The analyzer sends ordering constraints to **tpt-telos** to establish deadlock
   freedom and acyclic dependencies.
3. Verified definitions lower to compiled state machines / direct DAG updates.

Every stage exists. Ordering checks go through `tpt-pulse-ir`, which encodes "event A
happens before event B" as a strict rank constraint and asks tpt-telos's solver for
a schedule. Signal graphs and multiparty sessions both use it.

## Workspace boundaries

| Crate | Responsibility | Status |
| --- | --- | --- |
| tpt-pulse | Public prelude and example entry point | Implemented |
| tpt-pulse-core | Sealed protocol algebra, duality, in-memory typed endpoints | Implemented |
| tpt-pulse-ir | Ordering IR; constraint bridge to tpt-telos | Implemented |
| tpt-pulse-macros | `#[signal_graph]`, `global_protocol!`, `session_types!` | Implemented |
| tpt-pulse-signals | `Signal` marker and allocation-free propagation via `#[signal_graph]` | Implemented |
| tpt-pulse-multiparty | Global definitions, per-role projection, deadlock checks | Implemented |
| tpt-pulse-backend-tokio | Async sessions over TCP | Implemented |
| tpt-pulse-backend-leptos | `Live` values and memos for Leptos | Implemented |
| tpt-pulse-backend-dioxus | `Live` values and hooks for Dioxus | Implemented |

## Signal lowering

`#[signal_graph]` rewrites a function's signal declarations into plain locals.
A derived signal is a call to its closure on clones of its inputs, and `set`
recomputes the downstream signals in declaration order. The expansion has no
runtime graph, no dynamic dispatch, and no heap allocation. Dependencies are
checked through `tpt-pulse-ir` before code is generated, so a cycle is reported with
the tpt-telos diagnostic.

## Multiparty checks

A global protocol projects to one local program per role. A session checks its
programs by treating each action as an event, with program order and FIFO
message order as precedence edges. If a schedule exists, no receive waits
forever. `session_types!` emits one `tpt-pulse-core` protocol type per role, so the
compiler checks that the two endpoints are duals.

## Session implementation

Private transports contain two crossed standard-library MPSC queues. Each
operation takes the transport out of the consumed endpoint, preventing its Drop
check from reporting a false abandonment, and installs it in the continuation.
Private construction and sealed protocols preserve the invariant that boxed
messages match the receiving endpoint's statically known type. No unsafe code
is used. Send and receive states dualize recursively; choice and offer states
exchange a private branch discriminator and dualize each continuation.

Close permits normal destruction. Other states panic on normal abandonment but
release resources quietly during existing panic unwinding. Explicit disconnect
is cancellation, not successful protocol completion. The mechanism is a runtime
safety net, and cannot prevent deliberate destructor bypass or inter-session
blocking cycles. Messages allocate on the synchronous and tokio transports.
Signal propagation does not allocate.

## Dependencies

`tpt-pulse-ir` depends on `tpt-telos-ir` and `tpt-telos-verifier`, pulled from git and
pinned to a commit in the workspace `Cargo.toml`.
