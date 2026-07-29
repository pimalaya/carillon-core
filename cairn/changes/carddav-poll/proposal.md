---
cairn: change
id: carddav-poll
status: landed
created: 2026-07-28
---

# Relocate the CardDAV poll into carillon-core (split layer 2b)

## Why

The IMAP watch moved into core (`CarillonImapWatch`); the server and CLI share
it. The CardDAV poll still lives only in the server (`carddav/session.rs`
`sync_changes` + `carddav/pump.rs`), so it is the last watcher a second frontend
would have to duplicate. This is split layer 2b: bring the poll into core so
both frontends share it and it rings the same content-free event.

## What

A new `carddav` cargo feature adds `CarillonCardDavPoll`: one RFC 6578
`sync-collection` round as an I/O-free coroutine. It wraps io-webdav's
`SyncCollection`, forwards its `WantsRead`/`WantsWrite`, and completes with a
**content-free** `CarillonCardDavChange { changed, state, invalid_token }` —
whether any member changed or vanished, the new opaque sync-token, and whether
the server rejected the token (re-baseline). It requests only `getetag`, never
the vCard body, and never exposes member hrefs.

Like the IMAP move, core owns only the coroutine. The driver owns the TLS
connection, the poll interval, the reconnect, the sync-token checkpoint, and
turns a change into a `CarillonEvent` (source `carddav`). Onboarding
(`probe` / `verify_auth` / `list_addressbooks` / context-root discovery) stays
in the server, exactly as IMAP's `probe`/`list` did.

### Scope

- Core-only here: the `carddav` feature, the coroutine, a `CardDav` error
  variant. The server rewiring to drive it is the companion server change.
- `carddav` is **not** in core's default features (default stays `imap`), so a
  plain core build pulls no io-webdav.
- Poll interval, connection, and reconnect stay the driver's, as the transport
  class (`poll`) already says.
