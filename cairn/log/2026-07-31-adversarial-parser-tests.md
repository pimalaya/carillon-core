---
cairn: log
date: 2026-07-31
change: adversarial-parser-tests
---

# Adversarial input tests for the watch parsers

Locks in a regression guard on carillon-server [[security-model]] Layer 4 — parsing
untrusted mail-server data in-process — from the core side. A user can point a watch
at a server they control; a parser panic/hang/OOM there crashes or wedges the process
that holds every account's credentials.

## What landed
- **IMAP** (`src/imap.rs`, `#[cfg(test)] mod adversarial_tests`): a hostile corpus
  (garbage, truncated lines, embedded NULs, invalid UTF-8, deep nesting, integer
  overflow, unterminated literals) driven through `CarillonImapWatch::resume` under a
  256-step bounded loop — a panic or hang fails. Plus a targeted test proving an
  over-`MAX_MESSAGE_SIZE` (1 MiB) literal announcement terminates the watch rather
  than buffering.
- **CardDAV** (`src/carddav.rs`, under the `carddav` feature): hostile HTTP responses
  / `sync-collection` XML bodies driven through `CarillonCardDavPoll::resume`.
- No parser change was needed — every input terminated cleanly (no panic, no hang).

## Environment note
Stable Rust 1.95, no `cargo-fuzz`/nightly here, so this is a stable-Rust harness, not
coverage-guided fuzzing (kept as a follow-up). It runs in the normal `cargo test`,
so it gates in CI on the stable toolchain.

## Capabilities moved
- [[watch-client]] — new "parsers are robust against a hostile server" requirement
  (terminating error, never panic/hang/unbounded-alloc; 1 MiB fragmentizer bound).

## Verification
`cargo test --lib --all-features` (3 adversarial tests) + `cargo clippy --all-targets
--all-features` + `cargo fmt --check` all green.
