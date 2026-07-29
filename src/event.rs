//! The change signal a watch surfaces.
//!
//! A watch says only that something changed at an address, never what
//! changed. The signal carries no sender, subject, body, UID, resource
//! href, or change kind. Enriching it is a downstream consumer's job, since
//! the consumer holds the credentials and can go look. This mirrors JMAP's
//! StateChange: enough to address and dedup, and nothing more.
//!
//! Core is I/O-free, so it never mints the [`id`](CarillonEvent::id) or
//! [`ts`](CarillonEvent::ts): a random id and a clock read are effects a
//! driver performs. The watch coroutine surfaces the pure new state, and
//! the driver assembles the event with [`CarillonEvent::new`].

use serde::Serialize;

/// Which kind of source rang the doorbell.
///
/// Tags a [`CarillonEvent`] with its origin protocol so a consumer can
/// route without parsing the target string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CarillonSource {
    /// An IMAP mailbox watched over a held IDLE connection.
    Imap,
    /// A CardDAV addressbook collection watched by polling.
    CardDav,
}

impl CarillonSource {
    /// The lowercase wire string, matching the serialized form.
    pub fn as_str(&self) -> &'static str {
        match self {
            CarillonSource::Imap => "imap",
            CarillonSource::CardDav => "carddav",
        }
    }
}

/// A content-free, self-addressed change signal.
///
/// Content-free is not anonymous. The event stays self-identifying, so a
/// consumer can route (which target rang) and dedup (whether this state was
/// already handled). A dropped or duplicated event is harmless, since the
/// consumer re-derives truth on the next one.
#[derive(Clone, Debug, Serialize)]
pub struct CarillonEvent {
    /// Unique id, stable across retries so a receiver can dedup and a
    /// downstream signature covers a constant value. Minted by the driver.
    pub id: String,
    /// Unix timestamp in seconds, stable across retries. Minted by the
    /// driver.
    pub ts: i64,
    /// The watch (account) this signal belongs to.
    pub account: String,
    /// Which source rang.
    pub source: CarillonSource,
    /// The addressed target: an IMAP mailbox name or a CardDAV collection.
    pub target: String,
    /// Opaque per-source state used to resync, such as an IMAP UIDNEXT or
    /// MODSEQ, or a CardDAV sync-token. None when the source exposes none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl CarillonEvent {
    /// Assembles an event from its parts. `id` (dedup) and `ts` (replay)
    /// are the driver's to mint, since a random id and a clock read are
    /// effects an I/O-free core does not perform.
    pub fn new(
        id: impl Into<String>,
        ts: i64,
        account: impl Into<String>,
        source: CarillonSource,
        target: impl Into<String>,
        state: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            ts,
            account: account.into(),
            source,
            target: target.into(),
            state,
        }
    }
}
