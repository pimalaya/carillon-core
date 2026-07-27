//! What core needs to open one watch — and nothing it cannot resolve
//! itself.
//!
//! Core receives a *ready* [`Credential`]: a password or a pre-minted
//! bearer token. It never touches a keyring and never mints or refreshes
//! OAuth; that resolution happens upstream, so core stays a pure watch
//! client that only knows an auth *mechanism* and its secret.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use secrecy::SecretString;
use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::event::Event;

/// How a source is acquired — the axis that decides which frontend can
/// host it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportClass {
    /// The watcher dials out and holds an outbound connection (IMAP IDLE,
    /// and later JMAP EventSource / Maildir notify). Any frontend.
    StandingConnection,
    /// The watcher dials out but re-polls on an interval (CardDAV
    /// `sync-collection`). Any frontend.
    Poll,
    /// Delivered by an inbound POST via external infrastructure (Gmail
    /// push, Graph subscriptions, WebDAV-Push). Backend only.
    PublicCallback,
}

/// A resolved credential. Core never unlocks or refreshes it — it only
/// presents it to the server.
pub enum Credential {
    /// A SASL `LOGIN` / HTTP Basic password.
    Password(SecretString),
    /// A pre-minted OAuth 2.0 access token (`XOAUTH2` / Bearer). Upstream
    /// mints and refreshes it; core just presents it.
    Bearer(SecretString),
}

/// An IMAP mailbox to watch.
pub struct ImapBackend {
    pub host: String,
    pub port: u16,
    pub login: String,
    pub mailbox: String,
}

/// A CardDAV collection to poll.
pub struct CardDavBackend {
    pub url: String,
    pub login: String,
    pub poll: Duration,
}

/// The watchable sources core knows, feature-gated per protocol.
pub enum Backend {
    #[cfg(feature = "imap")]
    Imap(ImapBackend),
    #[cfg(feature = "carddav")]
    CardDav(CardDavBackend),
}

impl Backend {
    /// The transport class of this source, so a frontend can refuse to arm
    /// a watch it cannot host.
    pub fn transport_class(&self) -> TransportClass {
        match self {
            #[cfg(feature = "imap")]
            Backend::Imap(_) => TransportClass::StandingConnection,
            #[cfg(feature = "carddav")]
            Backend::CardDav(_) => TransportClass::Poll,
        }
    }
}

/// Runs one watch, emitting knock-knock [`Event`]s into `events` until the
/// server ends the session or `shutdown` is set.
///
/// This is *one session's* worth of watching. Reconnect/backoff
/// supervision, credential resolution, and consumer fan-out all live
/// upstream — core just connects, watches, and rings.
pub async fn watch(
    backend: &Backend,
    credential: &Credential,
    events: &mpsc::Sender<Event>,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    // Layer 2 relocates `imap/{session,pump}` and `carddav/` here, behind
    // the matching feature, dispatching on `backend`.
    let _ = (backend, credential, events, shutdown);
    Err(Error::NotImplemented)
}
