---
cairn: delta
id: carddav-poll
---

## ADDED Requirements

### Requirement: The CardDAV poll is an I/O-free coroutine in core
carillon-core SHALL provide, under a `carddav` cargo feature, a
`CarillonCardDavPoll` coroutine that runs one RFC 6578 `sync-collection` round
I/O-free: it SHALL wrap io-webdav's `SyncCollection`, forward its reads and
writes, and complete with a content-free result — whether the collection
changed, the new opaque sync-token, and whether the token was rejected. It
SHALL request only `getetag` and SHALL NOT expose member hrefs, etags, or vCard
content. Core SHALL NOT own the connection, the poll interval, the reconnect,
or the sync-token checkpoint; the driver owns those and mints the
`CarillonEvent`. The `carddav` feature SHALL NOT be a default feature.

#### Scenario: A contact changes in a watched addressbook
- **GIVEN** a driver polling a CardDAV collection with a stored sync-token
- **WHEN** a member's etag changes and the driver runs one `CarillonCardDavPoll`
- **THEN** it completes `changed = true` with the new sync-token as `state`, and
  no href or vCard data, and the driver rings one `carddav` event
