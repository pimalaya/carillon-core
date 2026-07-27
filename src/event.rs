//! The change signal a watch emits.
//!
//! A watch says only that something changed at an address, never what
//! changed. The signal carries no sender, subject, body, UID, resource
//! href, or change kind. Enriching it is a downstream consumer's job,
//! since the consumer holds the credentials and can go look. This mirrors
//! JMAP's StateChange: enough to address and dedup, and nothing more.

use std::time::{SystemTime, UNIX_EPOCH};

use rand::RngExt;
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

/// A content-free, self-addressed change signal.
///
/// Content-free is not anonymous. The event stays self-identifying, so a
/// consumer can route (which target rang) and dedup (whether this state
/// was already handled). A dropped or duplicated event is harmless, since
/// the consumer re-derives truth on the next one.
#[derive(Clone, Debug, Serialize)]
pub struct CarillonEvent {
    /// Unique id, stable across retries so a receiver can dedup and a
    /// downstream signature covers a constant value.
    pub id: String,
    /// Unix timestamp in seconds, stamped once at fold and stable across
    /// retries.
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
    /// Folds a ring for the given account, source, and target, stamping a
    /// fresh id and timestamp.
    pub fn ring(
        account: impl Into<String>,
        source: CarillonSource,
        target: impl Into<String>,
        state: Option<String>,
    ) -> Self {
        Self {
            id: new_id(),
            ts: now_secs(),
            account: account.into(),
            source,
            target: target.into(),
            state,
        }
    }
}

/// Builds a 128-bit random, hex-encoded event id.
fn new_id() -> String {
    format!("{:032x}", rand::rng().random::<u128>())
}

/// Returns the current Unix time in seconds, or 0 before the epoch.
fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}
