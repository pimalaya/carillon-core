---
cairn: change
id: parser-fuzz-harness
status: landed
created: 2026-07-31
---

# cargo-fuzz targets + CI for the watch parsers

## Why
[[adversarial-parser-tests]] added a stable-Rust corpus guard and noted continuous
coverage-guided fuzzing as a follow-up "when a nightly/CI fuzz job exists". This
delivers it, modelled on `pimalaya/vcard-rs`'s fuzz setup.

## What
- A detached `fuzz/` cargo-fuzz crate with two libFuzzer targets — `imap` and
  `carddav` — that drive `CarillonImapWatch::resume` / `CarillonCardDavPoll::resume`
  over arbitrary bytes with the same bounded-loop oracle (no panic / hang / unbounded
  alloc) as the adversarial tests.
- A committed seed corpus (`fuzz/seeds/<target>/`) of hostile inputs.
- `fuzz/shell.nix` (nightly via fenix + cargo-fuzz) for local NixOS fuzzing.
- `.github/workflows/tests.yml`: the reusable `pimalaya/nix` test job **plus** a
  `fuzz-regression` job that installs cargo-fuzz (nightly), lowers
  `vm.mmap_rnd_bits` for ASan, and replays each seed with `-runs=0` — a deterministic
  regression gate on previously-handled inputs.

## Non-goals / notes
- Long-running / scheduled fuzzing (this is a regression replay, not a soak).
- The nightly libFuzzer build cannot be run in this stable, cargo-fuzz-less
  environment; the target bodies reuse the exact API calls the adversarial tests
  compile and pass, and the manifest validates (`cargo verify-project`). CI is the
  place it is exercised.
