//! The canonical knock-knock event.
//!
//! A watch says only *"something changed at this address"* — never what
//! changed. No sender, subject, or body; no UID, no resource href, no
//! change kind. Enriching the signal is a downstream consumer's job (it
//! holds the credentials and can go look). This is JMAP's `StateChange`:
//! enough to address and dedup, and nothing more.
//!
//! [`id`](Event::id) and [`ts`](Event::ts) are stamped once, at fold time,
//! so every retry of the same ring carries the same id, timestamp, and
//! (downstream) signature.

use rand::RngExt;
use serde::Serialize;

/// Which kind of source rang the doorbell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// An IMAP mailbox (held IDLE, QRESYNC where available).
    Imap,
    /// A CardDAV addressbook collection (polled).
    CardDav,
}

/// A content-free, self-addressed change signal.
///
/// "Content-free" is not "anonymous": the ring stays self-identifying, so
/// a consumer can route (*which* target changed) and dedup (*already
/// handled this state*). A dropped or duplicated ring is harmless — the
/// consumer re-derives truth on the next one.
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    /// Stable across retries so receivers dedup; signed for replay
    /// protection downstream.
    pub id: String,
    /// Unix seconds, stamped once at fold, stable across retries.
    pub ts: i64,
    /// The watch (account) this ring belongs to.
    pub account: String,
    /// Which source rang.
    pub source: Source,
    /// The addressed target: an IMAP mailbox, a CardDAV collection.
    pub target: String,
    /// Opaque source state for dedup / resync (IMAP `UIDNEXT` or `MODSEQ`,
    /// CardDAV sync-token, JMAP state). `None` when the source exposes
    /// none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

impl Event {
    /// Folds a ring for `account` / `source` / `target`, stamping a fresh
    /// id and timestamp.
    pub fn ring(
        account: impl Into<String>,
        source: Source,
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

/// A 128-bit random, hex-encoded event id.
fn new_id() -> String {
    format!("{:032x}", rand::rng().random::<u128>())
}

/// Unix seconds now, or `0` if the clock is before the epoch.
fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}
