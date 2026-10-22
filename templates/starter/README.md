# Starter session

A minimal tpt-pulse project: one two-party protocol, a client and a server on two
threads, and a test that checks the client's trace against the protocol.

Copy this directory out of the repository, then:

```sh
cargo run
cargo test
```

The dependency points at the git repository, because the crates are not published
yet. For a branching protocol, use `global_protocol!` with `choice` and `loop`, or
write the protocol as a text file and check it with `cargo pulse check`.
