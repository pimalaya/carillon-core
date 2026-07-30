# Fuzzing

Coverage-guided fuzzing of the watch parsers with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)
(libFuzzer). Both targets parse an untrusted, user-named server's bytes through the
I/O-free `resume` coroutine — the same surface a malicious server can drive in the
process that holds decrypted credentials (security-model Layer 4). The oracle is that
no input panics, hangs, or allocates without bound (the IMAP 1 MiB fragmentizer bound
guards the last). These are the continuous-fuzzing complement to
`{imap,carddav}::adversarial_tests`, which run the same drive over a fixed corpus on
stable.

- `imap` — drives `CarillonImapWatch::resume`.
- `carddav` — drives `CarillonCardDavPoll::resume`.

cargo-fuzz needs a nightly toolchain. On NixOS, get both from `fuzz/shell.nix`:

```sh
nix-shell fuzz/shell.nix --run "cargo fuzz run imap"
nix-shell fuzz/shell.nix --run "cargo fuzz run carddav"
```

Off NixOS: `cargo install cargo-fuzz` and a nightly toolchain give the same commands.

`fuzz/seeds/<target>/` holds committed seed inputs (hostile responses). CI copies them
into the gitignored `fuzz/corpus/<target>/` and replays with `-runs=0` (no fuzzing),
so a regression on a previously-handled input fails deterministically. libFuzzer saves
new interesting inputs into `fuzz/corpus/<target>/` and any crash into
`fuzz/artifacts/<target>/`.
