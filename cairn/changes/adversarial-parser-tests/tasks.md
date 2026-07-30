---
cairn: tasks
change: adversarial-parser-tests
---

## Harness — DONE
- [x] IMAP: `#[cfg(test)]` adversarial harness in `src/imap.rs` driving `CarillonImapWatch::resume` over a hostile corpus under a bounded loop (panic/hang = failure)
- [x] IMAP: targeted test that an over-`MAX_MESSAGE_SIZE` literal announcement terminates the watch (`Done(Err)`), proving the 1 MiB bound
- [x] CardDAV: adversarial harness in `src/carddav.rs` driving `CarillonCardDavPoll::resume` over hostile HTTP/XML (under the `carddav` feature)

## Verify — DONE
- [x] `cargo test --lib --all-features` green (3 adversarial tests)
- [x] `cargo clippy --all-targets --all-features` + `cargo fmt --check` clean

## Spec & log (forcing rule) — DONE
- [x] Fold "parsers are robust against a hostile server" into [[watch-client]]
- [x] Log entry; `status: landed`

## Follow-up (not here)
- [ ] Continuous coverage-guided fuzzing (`cargo-fuzz`) when a nightly toolchain / CI fuzz job exists
