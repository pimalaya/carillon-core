---
cairn: tasks
id: carddav-poll
---

# Tasks

- [x] Add the `carddav` cargo feature (`dep:io-webdav` + `dep:io-http` + `dep:url`, all default-features off); keep it out of `default`
- [x] `error.rs`: add a `CardDav(String)` variant
- [x] `carddav.rs`: `CarillonCardDavPoll` (wraps io-webdav `SyncCollection`, `getetag` only) + `CarillonCardDavPollProgress` + content-free `CarillonCardDavChange` (changed/state/invalid_token/truncated)
- [x] `lib.rs`: `#[cfg(feature = "carddav")] pub mod carddav`
- [x] Fold delta into `spec/watch-client.md`; write log; set status landed
- [x] Companion: rewire `carillon-server` to drive it (green build + clippy + fmt)
- [ ] Live CardDAV smoke test (deferred; no local CardDAV server)
