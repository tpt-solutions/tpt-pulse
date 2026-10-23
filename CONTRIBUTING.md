# Contributing

Contributions are licensed under MIT OR Apache-2.0 unless explicitly stated
otherwise. Keep changes aligned with spec.txt and update todo.md only after
validating the corresponding functionality. Do not describe scaffolds as features.

## Issues only

Contributions are accepted through GitHub issues only. Please open an issue to
report a bug, propose a change, or ask a question. Pull requests are not accepted.

## Local checks

Use stable Rust with rustfmt and Clippy installed. From the workspace root run:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo run --example login`
- `cargo doc --workspace --no-deps`

Core transport has no external runtime dependencies. Compile-fail tests use the
trybuild dev dependency; first builds may need crates.io access. Cargo.lock is
intentionally ignored per the project checklist. No minimum Rust version is
promised yet.

## Tests and API changes

Add runtime tests for valid protocol paths and disconnections. Add trybuild
fixtures for invalid operations, along with reviewed `.stderr` files. For an
intentional diagnostic change, set the `TRYBUILD` environment variable to
`overwrite`, run `cargo test -p tpt-pulse-core --test compile_fail`, review the
snapshots, **unset TRYBUILD**, then rerun tests normally. Compiler upgrades may
change diagnostics; do not accept snapshots without checking the actual error.

Keep compile-time sequencing guarantees separate from runtime abandonment
checks. Do not add unsafe code or silently turn aborts into successful completion.
New dependencies and signal-macro syntax should be justified in architecture docs.

## Releases

Scaffold crates are marked publish=false. Publishing, docs.rs availability, URL
confirmation, and release tagging remain separate release tasks. Do not publish
or tag merely because local tests pass.
