---
cairn: delta
change: parser-fuzz-harness
---

## MODIFIED Requirements

### The parsers are robust against a hostile server ([[watch-client]])
Coverage-guided fuzzing is now delivered, not a follow-up: a `cargo-fuzz` target per
coroutine (`fuzz/`, nightly, `imap` + `carddav`) drives `resume` over arbitrary bytes
with the bounded-loop oracle, and its committed seed corpus is replayed in CI
(`-runs=0`) as a deterministic regression gate alongside the stable adversarial tests.
