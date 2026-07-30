//! CardDAV polling as an I/O-free coroutine.
//!
//! WebDAV has no IDLE, so a CardDAV collection is watched by polling: each
//! round runs one RFC 6578 `sync-collection` REPORT. [`CarillonCardDavPoll`]
//! is one such round as an I/O-free coroutine — it wraps io-webdav's
//! `SyncCollection`, forwards its reads and writes, and completes with a
//! content-free result: whether the collection changed and the new opaque
//! sync-token to checkpoint. It requests only `getetag`, never the vCard
//! body, and never exposes a member href.
//!
//! Core owns only the coroutine. The driver (a frontend) owns the TLS
//! connection, the poll interval, the reconnect, the sync-token checkpoint,
//! and turning a [`CarillonCardDavChange`] into a
//! [`CarillonEvent`](crate::event::CarillonEvent). Because a poll dials
//! outward, any frontend can host it (transport class
//! [`Poll`](crate::backend::CarillonTransportClass::Poll)).

use io_http::rfc6750::bearer::HttpAuthBearer;
use io_http::rfc7617::basic::HttpAuthBasic;
use io_webdav::coroutine::{WebdavCoroutine, WebdavCoroutineState, WebdavYield};
use io_webdav::rfc4918::{GETETAG, WebdavAuth};
use io_webdav::rfc6578::sync_collection::{SyncCollection, SyncCollectionError};
use secrecy::ExposeSecret;
use url::Url;

use crate::{
    backend::CarillonCardDavBackend,
    credential::CarillonCredential,
    error::{CarillonWatchError, CarillonWatchResult},
};

/// `User-Agent` sent on every WebDAV request.
const USER_AGENT: &str = "carillon";

/// What the driver must do to make progress on a poll, returned by each
/// [`CarillonCardDavPoll::resume`].
pub enum CarillonCardDavPollProgress {
    /// Read from the socket and pass the bytes to the next `resume` (an
    /// empty slice on EOF, so a close-delimited body finalizes).
    WantsRead,
    /// Write these bytes to the socket, then `resume` again with no input.
    WantsWrite(Vec<u8>),
    /// The poll finished: the content-free outcome, or an error.
    Done(CarillonWatchResult<CarillonCardDavChange>),
}

/// The content-free outcome of one poll. Carries whether the collection
/// changed and the new sync-token — never a member href, etag, or vCard.
pub struct CarillonCardDavChange {
    /// Whether any member changed or vanished since the polled token.
    pub changed: bool,
    /// The new opaque sync-token to checkpoint from, if the server
    /// returned one.
    pub state: Option<String>,
    /// The server rejected the polled sync-token: the driver should
    /// re-baseline (poll again with no token) and not treat this as a
    /// change.
    pub invalid_token: bool,
    /// The server truncated the result set (RFC 6578 §3.6): the driver
    /// should poll again from `state` to drain the rest before sleeping.
    pub truncated: bool,
}

/// An I/O-free CardDAV `sync-collection` poll (one round).
pub struct CarillonCardDavPoll {
    sync: SyncCollection,
}

impl CarillonCardDavPoll {
    /// Builds a poll of `backend`'s collection since `sync_token` (`None`
    /// for a baseline enumeration). `base_url` is the collection's origin
    /// and `path` its absolute path — the driver derives both from the
    /// collection URL. Requests only `getetag`, never the vCard body.
    pub fn new(
        base_url: &Url,
        backend: &CarillonCardDavBackend,
        credential: &CarillonCredential,
        path: &str,
        sync_token: Option<&str>,
    ) -> Self {
        let auth = webdav_auth(backend, credential);
        let sync = SyncCollection::new(base_url, &auth, USER_AGENT, path, sync_token, &[GETETAG]);
        Self { sync }
    }

    /// Advances the poll, returning the next action for the driver. After a
    /// [`WantsRead`](CarillonCardDavPollProgress::WantsRead), pass the bytes
    /// just read (an empty slice on EOF); otherwise pass `None`.
    pub fn resume(&mut self, input: Option<&[u8]>) -> CarillonCardDavPollProgress {
        use CarillonCardDavPollProgress as Progress;
        match self.sync.resume(input) {
            WebdavCoroutineState::Yielded(WebdavYield::WantsWrite(bytes)) => {
                Progress::WantsWrite(bytes)
            }
            WebdavCoroutineState::Yielded(WebdavYield::WantsRead) => Progress::WantsRead,
            WebdavCoroutineState::Complete(Ok(delta)) => {
                let changed = !delta.changed.is_empty() || !delta.vanished.is_empty();
                Progress::Done(Ok(CarillonCardDavChange {
                    changed,
                    state: delta.sync_token,
                    invalid_token: false,
                    truncated: delta.truncated,
                }))
            }
            WebdavCoroutineState::Complete(Err(SyncCollectionError::InvalidSyncToken)) => {
                Progress::Done(Ok(CarillonCardDavChange {
                    changed: false,
                    state: None,
                    invalid_token: true,
                    truncated: false,
                }))
            }
            WebdavCoroutineState::Complete(Err(err)) => Progress::Done(Err(
                CarillonWatchError::CardDav(format!("sync-collection: {err}")),
            )),
        }
    }
}

/// Builds the WebDAV authentication for the poll from the resolved
/// credential: a password becomes HTTP Basic, a bearer token HTTP Bearer.
fn webdav_auth(backend: &CarillonCardDavBackend, credential: &CarillonCredential) -> WebdavAuth {
    match credential {
        CarillonCredential::Password(password) => WebdavAuth::Basic(HttpAuthBasic::new(
            backend.login.clone(),
            password.expose_secret().to_string(),
        )),
        CarillonCredential::Bearer(token) => {
            WebdavAuth::Bearer(HttpAuthBearer::new(token.expose_secret().to_string()))
        }
    }
}

#[cfg(test)]
mod adversarial_tests {
    use std::time::Duration;

    use secrecy::SecretString;
    use url::Url;

    use super::{CarillonCardDavPoll, CarillonCardDavPollProgress};
    use crate::backend::CarillonCardDavBackend;
    use crate::credential::CarillonCredential;

    fn poll() -> CarillonCardDavPoll {
        let base = Url::parse("https://dav.example.org").unwrap();
        let backend = CarillonCardDavBackend {
            url: "https://dav.example.org/addressbooks/u/default/".into(),
            login: "user".into(),
            poll: Duration::from_secs(60),
        };
        let credential = CarillonCredential::Password(SecretString::from("password".to_owned()));
        CarillonCardDavPoll::new(
            &base,
            &backend,
            &credential,
            "/addressbooks/u/default/",
            None,
        )
    }

    /// Drives `resume` up to `cap` steps, feeding `hostile` at the first read and
    /// EOF (empty slice) thereafter. A panic or a hang is the failure hunted:
    /// the poll parses an attacker-influenced server's HTTP+XML in-process.
    fn drive(poll: &mut CarillonCardDavPoll, hostile: &[u8], cap: usize) -> bool {
        let mut fed = false;
        let mut next: Option<Vec<u8>> = None;
        for _ in 0..cap {
            match poll.resume(next.take().as_deref()) {
                CarillonCardDavPollProgress::WantsRead => {
                    next = Some(if fed { Vec::new() } else { hostile.to_vec() });
                    fed = true;
                }
                CarillonCardDavPollProgress::WantsWrite(_) => next = None,
                CarillonCardDavPollProgress::Done(_) => return true,
            }
        }
        false
    }

    /// Hostile HTTP responses / `sync-collection` XML bodies.
    const CORPUS: &[&[u8]] = &[
        b"",
        b"garbage not http\r\n\r\n",
        b"HTTP/1.1 207 Multi-Status\r\n\r\n",
        b"HTTP/1.1 207 Multi-Status\r\nContent-Length: 999999999\r\n\r\n<a/>",
        b"HTTP/1.1 207 Multi-Status\r\nContent-Length: 24\r\n\r\n<multistatus>truncated",
        b"HTTP/1.1 207\r\n\r\n<<<<<<<< not xml >>>>>>>>",
        b"HTTP/1.1 207 Multi-Status\r\nContent-Length: 44\r\n\r\n<m><r><r><r><r><r><r><r><r></m>",
        b"HTTP/1.1 500 Boom\r\nContent-Length: 2\r\n\r\nno",
        b"HTTP/1.1 207 \xff\xfe\r\n\r\n\x00\x00\x00",
        b"HTTP/0.9 nonsense response",
        b"HTTP/1.1 207 Multi-Status\r\nTransfer-Encoding: chunked\r\n\r\nZZZ\r\n",
        b"HTTP/1.1 207 OK\r\nContent-Length: 10\r\n\r\n<x>\xc3\x28</x>",
    ];

    #[test]
    fn resume_never_panics_on_hostile_input() {
        for hostile in CORPUS {
            let mut p = poll();
            drive(&mut p, hostile, 256);
        }
    }
}
