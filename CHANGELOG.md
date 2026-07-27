# Changelog

All notable changes to this project are documented in this file, following [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Scaffolded the crate: the content-free `CarillonEvent` and `CarillonSource`, the `CarillonBackend` source description with its `CarillonImapBackend`, `CarillonCardDavBackend` and `CarillonTransportClass`, the resolved `CarillonCredential`, and the one-session `watch` entry point with `CarillonWatchError`.

  The watch entry point is stubbed pending the relocation of the IMAP and CardDAV clients out of carillon-backend into core.
