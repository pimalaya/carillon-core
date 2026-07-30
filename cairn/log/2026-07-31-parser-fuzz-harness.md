---
cairn: log
date: 2026-07-31
change: parser-fuzz-harness
---

# cargo-fuzz targets + CI regression replay for the watch parsers

Delivered the continuous-fuzzing follow-up from [[adversarial-parser-tests]], modelled
on `pimalaya/vcard-rs`.

## What landed
- **`fuzz/`** detached cargo-fuzz crate: `imap` and `carddav` libFuzzer targets driving
  `CarillonImapWatch::resume` / `CarillonCardDavPoll::resume` over arbitrary bytes with
  the same no-panic/hang/unbounded-alloc oracle as the adversarial tests; a committed
  `fuzz/seeds/<target>/` corpus; `fuzz/shell.nix` (nightly via fenix + cargo-fuzz) and
  a README.
- **`.github/workflows/tests.yml`**: the reusable `pimalaya/nix` test job plus a
  `fuzz-regression` job (nightly, `cargo install cargo-fuzz`, `vm.mmap_rnd_bits=28`,
  seed replay with `-runs=0`).

## Verification
Fuzz manifest validates (`cargo verify-project`); seeds carry the expected bytes; core
`cargo test --lib --all-features` still green (the fuzz crate is a detached
`[workspace]`). The nightly libFuzzer build itself runs only in CI — this environment
is stable with no cargo-fuzz — but the target bodies reuse the exact `resume` API the
adversarial tests already compile and pass.

## Capabilities moved
- [[watch-client]] — robustness requirement now cites the delivered cargo-fuzz targets
  + CI replay, not a follow-up.
