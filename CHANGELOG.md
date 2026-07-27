# Changelog

All notable changes to this project are documented in this file, following [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Scaffolded the crate: the content-free `CarillonEvent` and `CarillonSource`, the `CarillonBackend` source description with its `CarillonImapBackend`, `CarillonCardDavBackend` and `CarillonTransportClass`, and the resolved `CarillonCredential`.

- Added the IMAP watch behind the `imap` feature: `imap::watch` greets, authenticates (`LOGIN` or SASL `OAUTHBEARER`), then holds IDLE and rings a content-free event when the mailbox state advances, driven over a caller-owned async stream. Errors surface as `CarillonWatchError`.

  The state token is `UIDVALIDITY:HIGHESTMODSEQ` on a CONDSTORE server (rings on any change: new mail, flags, deletes) and `UIDVALIDITY:UIDNEXT` otherwise (rings on new mail only). Core is generic over the stream and does not own the transport: the frontend opens the connection (TCP, TLS, any address policy) and owns reconnect. The CardDAV poll is not wired yet.
