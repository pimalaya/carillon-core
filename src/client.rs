//! Running one watch.
//!
//! [`watch`] connects a source, watches for one session, and rings an
//! event into a channel until the server ends the session or a shutdown is
//! requested. Reconnect and backoff supervision, credential resolution,
//! and consumer fan-out all live upstream. Core just connects, watches,
//! and rings.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use thiserror::Error;
use tokio::sync::mpsc;

use crate::backend::CarillonBackend;
use crate::credential::CarillonCredential;
use crate::event::CarillonEvent;

/// The result of running a watch.
pub type CarillonWatchResult<T> = Result<T, CarillonWatchError>;

/// What can go wrong opening or running a watch.
#[derive(Debug, Error)]
pub enum CarillonWatchError {
    /// The selected backend is not wired yet. Placeholder until the
    /// protocol clients are relocated into core.
    #[error("watch backend not implemented yet")]
    NotImplemented,
}

/// Runs one watch session, emitting knock-knock events into `events`.
///
/// Returns when the server ends the session or `shutdown` is set. This is
/// one session's worth of watching; looping and backoff belong upstream so
/// the frontend can resolve a fresh credential per attempt.
pub async fn watch(
    backend: &CarillonBackend,
    credential: &CarillonCredential,
    events: &mpsc::Sender<CarillonEvent>,
    shutdown: Arc<AtomicBool>,
) -> CarillonWatchResult<()> {
    // TODO: relocate imap/{session,pump} and carddav into core (layer 2)
    // and dispatch on `backend`.
    let _ = (backend, credential, events, shutdown);
    Err(CarillonWatchError::NotImplemented)
}
