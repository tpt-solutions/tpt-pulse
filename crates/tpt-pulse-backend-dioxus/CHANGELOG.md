# Changelog

All notable changes to `tpt-pulse-backend-dioxus` are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Unreleased

### Added

- `Live` values, with the `use_live` hook for component-local state.
- `use_derived2`, a hook that recomputes a signal graph when either input changes.
- A Dioxus example (`examples/dioxus_app.rs`) that renders HTML on the server.
- Bridge tests that run as `wasm32` under Node with `wasm-bindgen-test`.

### Known gaps

- The Dioxus app was verified by server-side rendering and virtual DOM re-rendering, not in
  a browser.
