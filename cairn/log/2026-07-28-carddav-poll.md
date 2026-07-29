---
cairn: log
change: carddav-poll
landed: 2026-07-28
---

# Relocate the CardDAV poll into carillon-core (split layer 2b)

The IMAP watch already lived in core; the CardDAV poll was the last watcher
stranded in the server. It now lives in core too, so both frontends share it
and it rings the same content-free event.

## What landed

A `carddav` cargo feature (`io-webdav` + `io-http` + `url`, all
default-features off; **not** in `default`) adds `CarillonCardDavPoll` — one
RFC 6578 `sync-collection` round as an I/O-free coroutine. It wraps io-webdav's
`SyncCollection` (requesting only `getetag`), forwards its `WantsRead` /
`WantsWrite`, and completes with a content-free `CarillonCardDavChange {
changed, state, invalid_token, truncated }`: whether any member changed or
vanished, the new opaque sync-token, whether the token was rejected, and
whether the result was truncated. No member href, etag, or vCard ever leaves
core. `error.rs` gained a `CardDav(String)` variant.

The server now drives it: `carddav/pump.rs` `poll_once` builds a
`CarillonCardDavBackend` + `CarillonCredential`, opens the TLS connection, and
pumps `CarillonCardDavPoll` (baseline suppression, truncation drain,
sync-token checkpoint, and the content-free ring all stay driver-side, exactly
as before). The server's own `sync_changes` / `SyncPollError` were deleted;
`session.rs` keeps `open` / `parts` (now `pub(crate)`) and the onboarding
`probe` / `verify_auth` / `list_addressbooks` / context-root walk.

Capability moved: **watch-client** — a new requirement pins the CardDAV poll
coroutine and its content-free result.

## Verification

Core builds + clippy clean with `--no-default-features --features carddav`
(crates.io io-webdav 0.1). `carillon-server` builds + clippy + fmt clean with
core's `carddav` feature (the server's io-webdav git patch applies to core as a
path dep). Not yet exercised against a live CardDAV server.
