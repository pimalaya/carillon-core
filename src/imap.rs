//! IMAP watching over an async stream.
//!
//! The transport is the frontend's: it opens the stream (TCP, TLS,
//! keepalive, any address or SSRF policy) and owns reconnect. Core owns the
//! IMAP conversation over that stream. Because the ring is content-free,
//! core does not need io-imap's QRESYNC delta watcher: it greets,
//! authenticates, then holds IDLE and, on each wake, re-EXAMINEs to read
//! the mailbox state and rings only when that state advanced. Only IDLE is
//! required; QRESYNC is irrelevant here.

mod pump;

use std::sync::{Arc, atomic::AtomicBool};

use anyhow::{Context, Result, anyhow, bail};
use io_imap::{
    codec::fragmentizer::Fragmentizer,
    rfc3501::{
        examine::{ImapMailboxExamine, ImapMailboxExamineOptions},
        greeting::{ImapGreetingGet, ImapGreetingGetOptions},
        login::{ImapLogin, ImapLoginOptions},
    },
    rfc7628::auth_oauthbearer::{ImapAuthOauthbearer, ImapAuthOauthbearerOptions},
    types::{command::SelectParameter, mailbox::Mailbox, response::Capability},
};
use log::{debug, trace};
use secrecy::ExposeSecret;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};

use crate::{
    backend::CarillonImapBackend,
    credential::CarillonCredential,
    error::{CarillonWatchError, CarillonWatchResult},
    event::{CarillonEvent, CarillonSource},
};

/// Safety bound on the parser buffer. Only mailbox status is read, never a
/// body, so it never needs to grow large; the fragmentizer grows lazily
/// into it.
const MAX_MESSAGE_SIZE: u32 = 1 << 20;

/// Watches an IMAP mailbox over an already-opened `stream`, ringing a
/// content-free [`CarillonEvent`] whenever the mailbox state advances,
/// until the stream drops or `shutdown` is set.
///
/// This is one session. The caller opened the transport and owns
/// reconnect; `account` tags each ring with the watch it belongs to. On a
/// CONDSTORE server the state token is `UIDVALIDITY:HIGHESTMODSEQ`, so a
/// ring fires on any change (new mail, flags, deletes); otherwise it is
/// `UIDVALIDITY:UIDNEXT`, ringing on new mail only.
pub async fn watch<S>(
    account: &str,
    imap: &CarillonImapBackend,
    credential: &CarillonCredential,
    stream: &mut S,
    events: &mpsc::Sender<CarillonEvent>,
    shutdown: Arc<AtomicBool>,
) -> CarillonWatchResult<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    watch_session(account, imap, credential, stream, events, shutdown)
        .await
        .map_err(|err| CarillonWatchError::Imap(format!("{err:#}")))
}

/// The watch session as an `anyhow` chain, mapped to the crate error at the
/// public boundary above.
async fn watch_session<S>(
    account: &str,
    imap: &CarillonImapBackend,
    credential: &CarillonCredential,
    stream: &mut S,
    events: &mpsc::Sender<CarillonEvent>,
    shutdown: Arc<AtomicBool>,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    debug!("begin imap watch");
    trace!(
        "account: {account}, host: {}, mailbox: {}",
        imap.host, imap.mailbox
    );

    let mut fragmentizer = Fragmentizer::new(MAX_MESSAGE_SIZE);

    let greeting_opts = ImapGreetingGetOptions {
        ensure_capabilities: true,
    };
    pump::run(
        stream,
        &mut fragmentizer,
        ImapGreetingGet::new(greeting_opts),
    )
    .await?
    .context("IMAP greeting failed")?;

    let capabilities = authenticate(stream, &mut fragmentizer, imap, credential).await?;
    if !capabilities.contains(&Capability::Idle) {
        bail!("server does not advertise IDLE; cannot watch");
    }
    let condstore = capabilities.contains(&Capability::CondStore)
        || capabilities.contains(&Capability::QResync);
    trace!("condstore state tracking: {condstore}");

    let mailbox = Mailbox::try_from(imap.mailbox.clone())
        .map_err(|_| anyhow!("invalid mailbox name: {}", imap.mailbox))?;

    let mut state = examine_state(stream, &mut fragmentizer, &mailbox, condstore).await?;

    loop {
        match pump::idle_once(stream, &mut fragmentizer, &shutdown).await? {
            pump::IdleOutcome::Shutdown => {
                debug!("end imap watch (shutdown requested)");
                return Ok(());
            }
            pump::IdleOutcome::Refresh => {
                debug!("end imap watch (idle refresh)");
                return Ok(());
            }
            pump::IdleOutcome::Data => {}
        }

        let next = examine_state(stream, &mut fragmentizer, &mailbox, condstore).await?;
        if next == state {
            continue;
        }
        trace!("mailbox state advanced: {state} -> {next}");
        state = next.clone();
        let ring = CarillonEvent::ring(account, CarillonSource::Imap, &imap.mailbox, Some(next));
        if events.send(ring).await.is_err() {
            bail!("event channel closed");
        }
    }
}

/// Authenticates an opened stream (`LOGIN` for a password, SASL
/// `OAUTHBEARER` for a bearer token), returning the post-authentication
/// capabilities (where IDLE surfaces).
async fn authenticate<S>(
    stream: &mut S,
    fragmentizer: &mut Fragmentizer,
    imap: &CarillonImapBackend,
    credential: &CarillonCredential,
) -> Result<Vec<Capability<'static>>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    match credential {
        CarillonCredential::Password(password) => {
            let opts = ImapLoginOptions {
                ensure_capabilities: true,
                auto_id: None,
            };
            let login = ImapLogin::new(&imap.login, password.expose_secret(), opts)
                .context("invalid IMAP credentials")?;
            pump::run(stream, fragmentizer, login)
                .await?
                .context("IMAP login failed")
        }
        CarillonCredential::Bearer(token) => {
            let opts = ImapAuthOauthbearerOptions {
                initial_request: true,
                ensure_capabilities: true,
                auto_id: None,
            };
            let auth = ImapAuthOauthbearer::new(
                &imap.login,
                &imap.host,
                imap.port,
                token.expose_secret(),
                opts,
            );
            pump::run(stream, fragmentizer, auth)
                .await?
                .context("IMAP OAUTHBEARER authentication failed")
        }
    }
}

/// Reads the mailbox state token by a read-only `EXAMINE`. With `condstore`
/// the token is `UIDVALIDITY:HIGHESTMODSEQ`, which advances on any change
/// (new mail, flags, deletes); otherwise `UIDVALIDITY:UIDNEXT`, which
/// advances on new mail only. A `UIDVALIDITY` change (a mailbox reset) also
/// advances it.
async fn examine_state<S>(
    stream: &mut S,
    fragmentizer: &mut Fragmentizer,
    mailbox: &Mailbox<'static>,
    condstore: bool,
) -> Result<String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let parameters = if condstore {
        vec![SelectParameter::CondStore]
    } else {
        Vec::new()
    };
    let examine =
        ImapMailboxExamine::new(mailbox.clone(), ImapMailboxExamineOptions { parameters });
    let data = pump::run(stream, fragmentizer, examine).await??;
    let validity = data.uid_validity.map(|v| v.get()).unwrap_or(0);
    let seq = data
        .highest_mod_seq
        .or_else(|| data.uid_next.map(|u| u64::from(u.get())))
        .unwrap_or(0);
    Ok(format!("{validity}:{seq}"))
}
