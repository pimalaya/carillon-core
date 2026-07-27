//! The source a watch connects to.
//!
//! A [`CarillonBackend`] describes what to watch as plain connection
//! parameters, one variant per supported protocol. It also reports a
//! [`CarillonTransportClass`], the axis a frontend uses to refuse a source
//! it cannot host. Credentials and consumer fan-out live elsewhere.

use std::time::Duration;

/// How a source is acquired, the axis that decides which frontend can host
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarillonTransportClass {
    /// The watcher dials out and holds an outbound connection, as with
    /// IMAP IDLE. Needs no public endpoint, so any frontend can host it.
    StandingConnection,
    /// The watcher dials out but re-checks on an interval, as with CardDAV
    /// sync-collection. Also outbound, so any frontend can host it.
    Poll,
    /// The source is delivered by an inbound POST through external
    /// infrastructure, as with Gmail push or Microsoft Graph
    /// subscriptions. Only a backend with a public endpoint can host it.
    PublicCallback,
}

/// Connection parameters for an IMAP mailbox watch.
pub struct CarillonImapBackend {
    /// IMAP server host.
    pub host: String,
    /// IMAP server port.
    pub port: u16,
    /// Login, the authentication identity.
    pub login: String,
    /// Mailbox to watch.
    pub mailbox: String,
}

/// Connection parameters for a CardDAV collection poll.
pub struct CarillonCardDavBackend {
    /// Collection URL to poll.
    pub url: String,
    /// Login, the authentication identity.
    pub login: String,
    /// Interval between polls.
    pub poll: Duration,
}

/// A watchable source, one variant per supported protocol.
pub enum CarillonBackend {
    /// An IMAP mailbox, watched over a held IDLE connection.
    Imap(CarillonImapBackend),
    /// A CardDAV collection, watched by polling.
    CardDav(CarillonCardDavBackend),
}

impl CarillonBackend {
    /// Reports this source's transport class, so a frontend can refuse to
    /// arm a watch it cannot host.
    pub fn transport_class(&self) -> CarillonTransportClass {
        match self {
            CarillonBackend::Imap(_) => CarillonTransportClass::StandingConnection,
            CarillonBackend::CardDav(_) => CarillonTransportClass::Poll,
        }
    }
}
