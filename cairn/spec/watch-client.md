---
cairn: spec
capability: watch-client
status: current
---

# The shared async watch client

carillon-core is the watcher that both Carillon frontends share: the CLI daemon and the server. It connects to a source, watches for a change, and rings a content-free signal. Everything a frontend adds around it (config ingestion, credential resolution, consumer fan-out, storage, billing) lives outside core, so the two frontends run the exact same loop and a watcher fix lands in both at once.

Core stops at the state. It does not resolve what changed, and it exposes no changes or diff API, because only the consumer knows what it needs from a ring: a new-mail count, the changed senders, or nothing but a re-render. Resolving the state into actual changes is the consumer's job, with the consumer's own credentials and its own protocol client. Building that resolution once, for everyone, would serve no consumer well and would drag core back toward the retired io-email aggregator.

The governing choice is that core is an async client, not a sans-io coroutine layer. A sans-io watch aggregator would repeat the retired io-email pattern, an interface whose upkeep outweighs what it saves. Core owns the async network I/O of watching directly, the way every other Pimalaya consumer rebuilds its own client over io-imap and io-webdav. The carillon-backend repository hosts core as a server; its cairn/changes/carillon-core-split change carries the full split rationale.

### Requirement: An async watch client, not sans-io
carillon-core SHALL be an async watch client that drives the protocol clients (io-imap now, io-jmap and io-webdav later) over a caller-owned async stream. It SHALL be generic over that stream and SHALL NOT own the transport: opening the connection (TCP, TLS, keepalive, any address or SSRF policy) and reconnect are the frontend's. It SHALL NOT be a sans-io coroutine layer, and it SHALL NOT depend on a TLS stack, a datastore, a keyring, an OAuth token exchange, a notification library, process spawning, or the delivery and consumer fan-out. Those effects SHALL be supplied upstream by a frontend.

#### Scenario: A watcher bug is fixed once
- **GIVEN** a defect in the per-session watch (a mishandled IDLE wake, a missed dead socket)
- **WHEN** it is fixed in carillon-core
- **THEN** both the CLI daemon and the server receive the fix, with no duplicated per-session watcher to fix twice

### Requirement: The event is content-free and self-addressed
A watch SHALL, on a detected change, construct exactly one content-free, self-addressed event carrying `(id, ts, account, source, target, state)` and nothing more. It SHALL NOT carry a UID, a resource href, a change kind, or any message content. The `id` and `ts` SHALL be stamped once at fold and stable across retries. The `state` SHALL be the opaque per-source resync token, or absent when the source exposes none.

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

### Requirement: The watch entry point runs one session
The core watch entry point SHALL run one session over a stream the frontend opened: greet, authenticate, watch, and ring into a channel until the stream drops or a shutdown is requested. Reconnect, backoff, transport, and credential resolution SHALL live upstream, so the frontend opens a fresh connection and resolves a fresh credential per attempt, keeping credential residency minimal.

#### Scenario: The session drops
- **GIVEN** a running watch session
- **WHEN** the connection is lost
- **THEN** the core entry point returns, and the frontend decides whether and when to reconnect

### Requirement: The IMAP watch is IDLE plus EXAMINE, not a delta watcher
Because the ring is content-free, the IMAP watch SHALL NOT compute structured per-message deltas. It SHALL require only IDLE: greet, authenticate, read the mailbox state by a read-only EXAMINE, then hold IDLE and, on each wake, re-EXAMINE and ring only when the state token advanced. The state token SHALL be `UIDVALIDITY:UIDNEXT`, advancing on new mail; a CONDSTORE `HIGHESTMODSEQ` token, advancing on flag and delete changes too, is an allowed refinement. QRESYNC SHALL NOT be required.

#### Scenario: A flag change on a CONDSTORE-less server
- **GIVEN** an IMAP watch using the `UIDVALIDITY:UIDNEXT` token
- **WHEN** a message flag changes but no new mail arrives
- **THEN** IDLE wakes, the re-EXAMINE finds the token unchanged, and core does not ring, until the HIGHESTMODSEQ refinement lands
