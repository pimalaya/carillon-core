---
cairn: change
id: adversarial-parser-tests
status: landed
created: 2026-07-31
---

# Adversarial input tests for the watch parsers

## Why
carillon-server [[security-model]] Layer 4 flags **parsing untrusted mail-server data
in-process** as a first-class 🔴: the daemon connects out to a user-named server and
parses its responses inside the process that holds decrypted credentials for every
account. A user can point a watch at a server they control; a parser bug there is a
crash (DoS of every watch on the box) or worse, code execution next to the crown
jewels. The spec calls for fuzzing the parsers.

`CarillonImapWatch::resume` and `CarillonCardDavPoll::resume` are the I/O-free parse
entrypoints — the exact bytes an attacker-influenced server can send flow through
them. This environment is stable Rust with no `cargo-fuzz`/nightly, so we add a
**stable-Rust adversarial harness**: a corpus of hostile byte inputs driven through
`resume` under a bounded loop, asserting the parser never panics or hangs and that the
1 MiB fragmentizer bound rejects oversized literals rather than allocating. It is not a
substitute for continuous fuzzing (noted as a follow-up), but it locks in a regression
guard and exercises the highest-risk path today, on the stable toolchain and in CI.

## What
- A `#[cfg(test)]` harness that builds a `CarillonImapWatch` over a fixture backend and
  drives `resume` with a corpus of malformed/hostile inputs (garbage, truncated lines,
  embedded NULs, invalid UTF-8, deep nesting, integer-overflow attempts, oversized
  literal announcements), under a bounded iteration cap — a panic or hang fails.
- A targeted test that an **oversized literal** (> `MAX_MESSAGE_SIZE`) is rejected
  (`Done(Err)`), proving the buffer bound holds.
- The same treatment for `CarillonCardDavPoll::resume` (hostile XML/multistatus).

## Non-goals
- Continuous / coverage-guided fuzzing (`cargo-fuzz`) — a follow-up when a nightly
  toolchain / CI fuzz job exists.
- Changing parser behaviour; this only adds tests. Any real panic found becomes its
  own fix.
