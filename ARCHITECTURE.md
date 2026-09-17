# Architecture

The target pipeline from spec section 3 is:

1. Protocol / reactive definitions feed a state-machine / DAG analyzer.
2. The analyzer sends state transitions and constraints to **tpt-telos SMT**
   to establish deadlock freedom and acyclic dependencies.
3. Verified definitions lower to compiled state machines / direct DAG updates.

Only the synchronous two-party channel portion currently exists. No SMT checks
or generated propagation run in this workspace yet.

## Workspace boundaries

| Crate | Responsibility | Status |
| --- | --- | --- |
| tpt-pulse | Public prelude and example entry point | Session API available |
| pulse-core | Sealed protocol algebra, duality, in-memory typed endpoints | Implemented |
| pulse-ir | Shared state transitions / DAG edges and constraint bridge | Scaffold |
| pulse-macros | Compile-time DAG analysis and direct propagation codegen | Proc-macro scaffold |
| pulse-signals | Signal storage and generated propagation interface | Scaffold |
| pulse-multiparty | Global definitions, per-role projection, verification | Scaffold |
| pulse-backend-tokio | Async sessions and TCP | Scaffold |
| pulse-backend-leptos | Reactive UI bridge | Scaffold |
| pulse-backend-dioxus | Reactive UI bridge | Scaffold |

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
blocking cycles. Messages allocate; the planned no-allocation property concerns
future generated signal propagation, not this transport.

## Future verification

Phase 3 must implement full multiparty global-to-local projection and actual
tpt-telos verification, rather than treating this scaffold as a proof. The local
tpt-telos checkout is not yet a dependency, so ordinary builds remain portable.
