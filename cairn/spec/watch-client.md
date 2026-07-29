---
cairn: spec
capability: watch-client
status: current
---

# The shared I/O-free watch client

carillon-core is the watcher that both Carillon frontends share: the CLI daemon and the server. It watches a source and surfaces a content-free signal on each change. Everything a frontend adds around it (transport, reconnect, config ingestion, credential resolution, consumer fan-out, storage, billing) lives outside core, so the two frontends run the exact same watch logic and a watcher fix lands in both at once.

Core stops at the state. It does not resolve what changed, and it exposes no changes or diff API, because only the consumer knows what it needs from a ring: a new-mail count, the changed senders, or nothing but a re-render. Resolving the state into actual changes is the consumer's job, with the consumer's own credentials and its own protocol client. Building that resolution once, for everyone, would serve no consumer well and would drag core back toward the retired io-email aggregator.

The governing choice is that core is I/O-free: the watch is a coroutine that does no I/O itself. This is not the io-email mistake, which was a sans-io *aggregator* unifying every operation behind one interface. This is a single-purpose composed coroutine, exactly what io-imap already is one level down. Being I/O-free lets each frontend pick its own I/O model: the CLI drives the coroutine with a blocking socket and one thread per watch (it does not want an async runtime), the server with an async socket over many connections (it cannot afford a thread per connection). The carillon-server repository hosts core as a server; its cairn/changes/carillon-core-split change carries the full split rationale.

### Requirement: An I/O-free watch coroutine
carillon-core SHALL be I/O-free: the watch SHALL be a coroutine that composes the protocol coroutines (io-imap now, io-jmap and io-webdav later) and, on each resume, returns what its driver must do next (read, write, surface a changed state, or stop). It SHALL NOT perform I/O, own the transport (TCP, TLS, timeouts, reconnect, any address or SSRF policy), or depend on an async runtime, a TLS stack, a datastore, a keyring, an OAuth token exchange, a notification library, or process spawning. The driver, a frontend, SHALL supply those and SHALL pick its own I/O model (blocking or async).

#### Scenario: A watcher bug is fixed once
- **GIVEN** a defect in the watch coroutine (a mishandled IDLE wake, a wrong state fold)
- **WHEN** it is fixed in carillon-core
- **THEN** both the CLI daemon and the server receive the fix, whether they drive it blocking or async, with no duplicated watcher to fix twice

### Requirement: The event is content-free and self-addressed
The event SHALL be content-free and self-addressed, carrying `(id, ts, account, source, target, state)` and nothing more. It SHALL NOT carry a UID, a resource href, a change kind, or any message content. The watch coroutine SHALL surface only the pure new `state`; the driver SHALL mint the `id` (a random value) and the `ts` (a clock read) and assemble the event, since randomness and a clock read are effects an I/O-free core does not perform. The `state` SHALL be the opaque per-source resync token, or absent when the source exposes none.

#### Scenario: New mail arrives on a watched mailbox
- **GIVEN** a live IMAP watch
- **WHEN** the mailbox changes
- **THEN** core rings one event tagged with the account, the `imap` source, and the mailbox as target, and no content
- **AND** a consumer re-derives what changed by going to look for itself

### Requirement: Core receives a resolved credential
Core SHALL present a credential that is either a password or a pre-minted bearer token. It SHALL NOT unlock a keyring and SHALL NOT mint or refresh OAuth. Credential resolution SHALL happen upstream, so OAuth is, to core, just a token handed in already resolved.

#### Scenario: An OAuth mailbox is watched
- **GIVEN** a mailbox authenticated by OAuth
- **WHEN** the frontend arms the watch
- **THEN** the frontend mints a fresh access token and hands core a bearer credential
- **AND** core presents it on connect without knowing how it was obtained

### Requirement: Sources carry a transport class
Every source SHALL report a transport class: `standing-connection` (dials out and holds an outbound connection, as IMAP IDLE), `poll` (dials out but re-checks on an interval, as CardDAV sync-collection), or `public-callback` (delivered by an inbound POST through external infrastructure, as Gmail push or Graph subscriptions). A frontend SHALL advertise the classes it can host and SHALL refuse to arm a watch whose class it does not advertise.

#### Scenario: A Gmail push is requested on the CLI
- **GIVEN** the CLI, which advertises `standing-connection` and `poll` but not `public-callback`
- **WHEN** a watch is configured for a `public-callback` source such as Gmail push
- **THEN** the frontend refuses to arm it, and never offers Gmail as a watchable source there

### Requirement: The driver pumps the coroutine and owns the connection lifecycle
The watch coroutine SHALL run one connection's worth of watching: greet, authenticate, then IDLE and re-EXAMINE, surfacing a change and stopping when a shutdown flag is set. The driver SHALL pump it, performing each read and write it requests, applying the idle-refresh timeout, and minting the event on a surfaced change. Reconnect, backoff, transport, and credential resolution SHALL live in the driver, so it opens a fresh connection and resolves a fresh credential per attempt, keeping credential residency minimal.

#### Scenario: The connection drops
- **GIVEN** a running watch coroutine
- **WHEN** a read or write fails because the connection was lost
- **THEN** the driver stops pumping and decides whether and when to reconnect with a fresh coroutine

### Requirement: The IMAP watch is IDLE plus EXAMINE, not a delta watcher
Because the ring is content-free, the IMAP watch SHALL NOT compute structured per-message deltas. It SHALL require only IDLE: greet, authenticate, read the mailbox state by a read-only EXAMINE, then hold IDLE and, on each wake, re-EXAMINE and ring only when the state token advanced. On a CONDSTORE server the token SHALL be `UIDVALIDITY:HIGHESTMODSEQ`, read by an `EXAMINE (CONDSTORE)` and advancing on any change (new mail, flags, deletes); otherwise it SHALL be `UIDVALIDITY:UIDNEXT`, advancing on new mail only. QRESYNC SHALL NOT be required.

#### Scenario: A flag change on a CONDSTORE server
- **GIVEN** an IMAP watch on a CONDSTORE server, using the `UIDVALIDITY:HIGHESTMODSEQ` token
- **WHEN** a message flag changes but no new mail arrives
- **THEN** IDLE wakes, the re-EXAMINE finds HIGHESTMODSEQ advanced, and core rings

#### Scenario: A flag change without CONDSTORE
- **GIVEN** an IMAP watch on a server without CONDSTORE, using the `UIDVALIDITY:UIDNEXT` token
- **WHEN** a message flag changes but no new mail arrives
- **THEN** IDLE wakes, the re-EXAMINE finds the token unchanged, and core does not ring (new mail only, the honest limit of a CONDSTORE-less server)

### Requirement: The CardDAV poll is an I/O-free coroutine in core
carillon-core SHALL provide, under a `carddav` cargo feature, a `CarillonCardDavPoll` coroutine that runs one RFC 6578 `sync-collection` round I/O-free: it SHALL wrap io-webdav's `SyncCollection`, forward its reads and writes, and complete with a content-free result — whether the collection changed, the new opaque sync-token, whether the token was rejected (re-baseline), and whether the result was truncated (poll again to drain). It SHALL request only `getetag` and SHALL NOT expose member hrefs, etags, or vCard content. Core SHALL NOT own the connection, the poll interval, the reconnect, or the sync-token checkpoint; the driver owns those and mints the `carddav` `CarillonEvent`. The `carddav` feature SHALL NOT be a default feature.

#### Scenario: A contact changes in a watched addressbook
- **GIVEN** a driver polling a CardDAV collection with a stored sync-token
- **WHEN** a member's etag changes and the driver runs one `CarillonCardDavPoll`
- **THEN** it completes `changed = true` with the new sync-token as `state`, and no href or vCard data, and the driver rings one `carddav` event
