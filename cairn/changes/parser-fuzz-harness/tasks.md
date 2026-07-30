---
cairn: tasks
change: parser-fuzz-harness
---

## Fuzz crate — DONE
- [x] `fuzz/Cargo.toml` (detached workspace, cargo-fuzz metadata, imap + carddav bins)
- [x] `fuzz/fuzz_targets/imap.rs` — drives `CarillonImapWatch::resume`
- [x] `fuzz/fuzz_targets/carddav.rs` — drives `CarillonCardDavPoll::resume`
- [x] `fuzz/seeds/{imap,carddav}/` committed hostile seed corpus
- [x] `fuzz/shell.nix` (nightly + cargo-fuzz via fenix) + `fuzz/README.md` + `.gitignore`

## CI — DONE
- [x] `.github/workflows/tests.yml`: reusable `pimalaya/nix` tests + `fuzz-regression` (`-runs=0` seed replay)

## Verify
- [x] `cargo verify-project` on the fuzz manifest OK; seeds have expected bytes
- [x] core `cargo test --lib --all-features` still green (fuzz dir detached)
- [ ] nightly libFuzzer build — CI-only (no nightly/cargo-fuzz in this env)

## Spec & log — DONE
- [x] [[watch-client]] robustness requirement updated (fuzzing delivered, not a follow-up)
- [x] Log entry; `status: landed`
