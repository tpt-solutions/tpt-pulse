# Changelog

All notable changes to `tpt-pulse-backend-leptos` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- `Live` values, which components read and write.
- `derived` and `derived2` memos that recompute a signal graph when its inputs change.
- A Leptos example (`examples/leptos_app.rs`) that renders HTML on the server.
- Bridge tests that run as `wasm32` under Node with `wasm-bindgen-test`.

### Known gaps

- The Leptos app was verified by server-side rendering, not in a browser.
