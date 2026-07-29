---
cairn: log
change: io-free-pivot
landed: 2026-07-29
---

# Make core I/O-free: the watch is a coroutine, not an async client

Reversed the earlier "async client" decision. The realisation: the CLI does not want async (a few watches, one blocking thread each is simpler), while the server does (many IDLE connections, no thread per connection). Forcing core to be async forced the CLI to be async too. Making core I/O-free instead lets each frontend pick its own I/O model, and the watch logic is still shared once.

This is not the io-email mistake. That was a sans-io *aggregator* unifying every operation behind one interface. This is a single-purpose composed coroutine, exactly what io-imap already is one level down.

## What changed

`imap.rs` is now `CarillonImapWatch`, a state machine that composes io-imap's own coroutines (greeting, LOGIN or SASL OAUTHBEARER, EXAMINE, IDLE), all of which are `'static` and so can be held across resumes. Each `resume(input)` returns a `CarillonImapWatchProgress`: `WantsRead`, `WantsWrite(bytes)`, `Changed(state)`, or `Done(result)`. The driver performs the read or write, applies the idle-refresh timeout, mints a `CarillonEvent` from a `Changed`, and reconnects on a `Done`. A shutdown `Arc<AtomicBool>` (set by a signal handler) winds IDLE down cleanly. The CONDSTORE HIGHESTMODSEQ state token carried over unchanged.

The async pump, the `mpsc` channel, and the stream generics are gone. Core dropped `tokio`, `anyhow`, `log`, and `rand`. `CarillonEvent::ring` (which minted `id` from `rand` and `ts` from `SystemTime`, both effects) is replaced by a pure `CarillonEvent::new`, and the driver mints `id` and `ts`. The `Changed` yield carries only the pure state string, per the rule that a content-free event's identity fields are the driver's to stamp.

Core is now genuinely I/O-free: no I/O, no clock, no randomness, no runtime. It keeps only io-imap (feature-gated), secrecy, serde, and thiserror, and is a step from no_std. Green on check (default and no-default), clippy all-features, and fmt.

## Follow-on

The `cli/` crate built against the old async `imap::watch` and no longer compiles; it is superseded by the mirador-based CLI (mirador is already a blocking, sans-io-driven watch CLI, the right host). The server will grow its own async driver around the same coroutine. The delivery layer mints `id`/`ts` there.
