# AGENTS.md

## Overview

Single-crate Rust library (not a workspace). Express.js-inspired web framework. Uses Rust 2021 edition. Built on `hyper 1.x` with `tokio` runtime internally, but exposes a **synchronous** middleware API — handlers run on a blocking thread pool via `tokio::task::spawn_blocking`.

## Commands

```sh
cargo build                       # build the library
cargo test                        # unit + doc + integration tests
cargo test --test example_tests   # integration tests only (spawns example binaries)
cargo run --example <name>        # run a specific example (see Cargo.toml for names)
```

No rustfmt, clippy, or lint CI steps are configured. No task runner.

## Key quirks

- **`autoexamples = false`** — examples must be listed explicitly as `[[example]]` entries in `Cargo.toml`. Adding an example `.rs` file without a corresponding entry will not compile it.
- **Sync-over-async architecture** — `src/server.rs` spawns a tokio runtime in a background thread. Each incoming request is dispatched to `spawn_blocking` where the synchronous middleware chain runs. The response body is buffered in a `Vec<u8>` and sent back to hyper as `Full<Bytes>`.
- **`ListeningServer` lifecycle** — `Drop` joins the server thread (blocks forever). `detach()` leaks the shutdown channel and thread handle so the server keeps running without blocking. This matches the old hyper 0.10 behavior where `listen()` blocked.
- **Integration tests** (`tests/example_tests.rs`) work by building example binaries, spawning them as child processes, and making HTTP requests against them using `reqwest::blocking`. They use a `NICKEL_TEST_HARNESS=1` env var and parse the listening port from stdout. Failures can leak child processes.
- **Cargo parallel build issue** — integration tests use a `BUILD_LOCK` mutex to serialize `cargo build --example` calls to avoid cargo parallel build races.
- **`integration_testing` example** contains `#[cfg(test)]` tests that are included into the test binary via `include!()` macro. These tests run the server in-process and use `Server::new()` + `detach()`.

## Commit conventions

Follow the [Angular commit message convention](https://github.com/conventional-changelog/conventional-changelog-angular/blob/master/convention.md) (e.g. `feat(router): ...`, `fix(*): ...`). Changelogs are generated with `clog` (see `.clog.toml`).

When an AI coding agent generates or substantially contributes to a commit, add an attribution line at the end of the commit body: `Generated with OpenCode using <model>` (e.g. `Generated with OpenCode using anthropic/claude-opus-4.6`).

## Source layout

- `src/lib.rs` — crate root, re-exports public API and `pub use http` for the `http` crate
- `src/nickel.rs` — `Nickel` struct and `Options` (the main server builder)
- `src/server.rs` — tokio-based HTTP server, `ListeningServer`, sync↔async bridge
- `src/router/` — HTTP routing (`Router`, `HttpRouter`, matchers)
- `src/macros/` — `middleware!` and `router!` macro definitions
- `src/middleware.rs` — `Middleware` trait, `Action` enum (`Continue`/`Halt`), `MiddlewareStack`
- `src/request.rs` — `Request` type (owns method, URI string, headers, body bytes)
- `src/response.rs` — `Response` type (buffers body in `Vec<u8>`, converted to `http::Response` at end)
- `src/extensions/` — extension traits for request/response (`Referer`, `Redirect`)
- `tests/util/mod.rs` — test harness utilities (process spawning, reqwest-based HTTP helpers)

## Key dependency choices

- `hyper 1.x` + `hyper-util` + `http` + `http-body-util` — HTTP server
- `tokio` — async runtime (internal only, not exposed to users)
- `reqwest` (dev) — HTTP client for integration tests (replaced old hyper 0.10 client)
- `mime` + `mime_guess` — MIME type handling (replaced hyper 0.10's built-in mime)
- `http` crate — `StatusCode`, `Method`, `HeaderMap` etc. (re-exported as `nickel::http`)
