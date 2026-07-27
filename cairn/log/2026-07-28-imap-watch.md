---
cairn: log
change: imap-watch
landed: 2026-07-28
---

# Relocate the IMAP watch into core (layer 2a)

Filled the stubbed watch entry point with a real IMAP watcher, behind the `imap` feature (now gating `io-imap` and `anyhow`, so the feature earns its keep per crate-003). Reading the backend's watcher to relocate it surfaced two design decisions, both folded into the watch-client spec.

## Transport is the frontend's; core is generic over the stream

The backend's `pump` was already stream-generic (`S: AsyncRead + AsyncWrite + Unpin`); all the transport lived in its `session.rs`, which pulls in the server's SSRF guard, TLS config, and keepalive. That is per-frontend policy (the server needs SSRF and a handshake semaphore; the CLI trusts the user), the same reason reconnect lives in the frontend. So core does not own the transport: the frontend opens the stream and owns reconnect, and core drives the IMAP conversation over it. Core stays TLS-agnostic, with no rustls, tokio-rustls, or socket2 dependency. This revised the `watch` signature to take a `&mut S` stream rather than a host to dial.

## The content-free watch is IDLE plus EXAMINE, not a delta watcher

The backend uses io-imap's `ImapMailboxWatch` (IDLE + QRESYNC) to compute structured per-UID deltas (new, flags, removed). A content-free ring throws all of that away, so core does not need it. `imap::watch` greets, authenticates, reads the mailbox state by a read-only EXAMINE, then holds IDLE and, on each wake, re-EXAMINEs and rings only when the state advanced. The state token is `UIDVALIDITY:UIDNEXT`, so it rings on new mail. Only IDLE is required; QRESYNC is irrelevant. This is simpler than porting the delta watcher and still yields the resync token the event carries.

## What landed

`imap::watch<S>(account, &CarillonImapBackend, &CarillonCredential, &mut S, &sender, shutdown)`, with `CarillonWatchError`/`CarillonWatchResult` in a new error module. The pump primitives (`run`, `idle_once`, `drain_idle`) live in `imap/pump.rs`, driving io-imap's I/O-free coroutines over the stream; greeting, authentication (LOGIN and SASL OAUTHBEARER), and the EXAMINE state read live in `imap.rs`. `anyhow` is used internally and mapped to the typed crate error at the public boundary. Green on check, clippy (all features), and fmt.

## Deferred

Rings on new mail only: a CONDSTORE `HIGHESTMODSEQ` token (ringing on flag and delete changes too) is the next refinement, since the EXAMINE data already exposes `highest_mod_seq`. The CardDAV poll (layer 2b) and the backend migration onto core (deleting its own session and pump) are separate stones.
