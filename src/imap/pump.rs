//! The async coroutine pump.
//!
//! io-imap coroutines are I/O-free: they yield `WantsRead` / `WantsWrite`
//! and let the caller own the socket. This drives them over any tokio
//! stream, so core never blocks a thread per held IDLE connection.
//! Transport (TCP, TLS, keepalive, any address policy) is the frontend's;
//! the pump only needs an async byte stream.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result, bail};
use io_imap::{
    codec::fragmentizer::Fragmentizer,
    coroutine::{ImapCoroutine, ImapCoroutineState, ImapYield},
    rfc2177::idle::{ImapIdle, ImapIdleOptions, ImapIdleYield},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::{Duration, timeout},
};

/// Per-read scratch buffer. IDLE never fetches bodies, so a small buffer
/// is plenty; the fragmentizer reassembles across reads.
const READ_BUF: usize = 8 * 1024;

/// Proactively drop and reconnect an idle connection after this long with
/// no server traffic. Bounds the silent-dead-socket window and refreshes
/// server-side IDLE state well under the RFC 2177 section 3 cap.
pub(crate) const IDLE_REFRESH: Duration = Duration::from_secs(15 * 60);

/// A single IDLE round's outcome.
pub(crate) enum IdleOutcome {
    /// The server sent untagged data: something changed, go read the state.
    Data,
    /// The periodic refresh timer fired with no data: reconnect.
    Refresh,
    /// Shutdown was requested.
    Shutdown,
}

/// Drives a request/response coroutine (greeting, login, examine, ...) to
/// completion over an async stream, returning its terminal value.
pub(crate) async fn run<S, C>(
    stream: &mut S,
    fragmentizer: &mut Fragmentizer,
    mut coroutine: C,
) -> Result<C::Return>
where
    S: AsyncRead + AsyncWrite + Unpin,
    C: ImapCoroutine<Yield = ImapYield>,
{
    let mut buf = [0u8; READ_BUF];
    let mut arg: Option<&[u8]> = None;

    loop {
        match coroutine.resume(fragmentizer, arg.take()) {
            ImapCoroutineState::Yielded(ImapYield::WantsWrite(bytes)) => {
                stream.write_all(&bytes).await.context("write failed")?;
            }
            ImapCoroutineState::Yielded(ImapYield::WantsRead) => {
                let n = stream.read(&mut buf).await.context("read failed")?;
                if n == 0 {
                    bail!("connection closed by peer");
                }
                arg = Some(&buf[..n]);
            }
            ImapCoroutineState::Complete(value) => return Ok(value),
        }
    }
}

/// Runs one IDLE round: waits for untagged server data (or the periodic
/// refresh timeout), then ends IDLE cleanly. A set shutdown flag winds
/// IDLE down cleanly (sending DONE) before returning.
pub(crate) async fn idle_once<S>(
    stream: &mut S,
    fragmentizer: &mut Fragmentizer,
    shutdown: &Arc<AtomicBool>,
) -> Result<IdleOutcome>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    if shutdown.load(Ordering::SeqCst) {
        return Ok(IdleOutcome::Shutdown);
    }

    let done = Arc::new(AtomicBool::new(false));
    let mut idle = ImapIdle::new(done.clone(), ImapIdleOptions::default());
    let mut buf = [0u8; READ_BUF];
    let mut arg: Option<&[u8]> = None;

    loop {
        if shutdown.load(Ordering::SeqCst) {
            done.store(true, Ordering::SeqCst);
        }
        match idle.resume(fragmentizer, arg.take()) {
            ImapCoroutineState::Yielded(ImapIdleYield::Event(_)) => {
                done.store(true, Ordering::SeqCst);
                if !shutdown.load(Ordering::SeqCst) {
                    return drain_idle(stream, fragmentizer, &mut idle).await.map(|()| {
                        if shutdown.load(Ordering::SeqCst) {
                            IdleOutcome::Shutdown
                        } else {
                            IdleOutcome::Data
                        }
                    });
                }
            }
            ImapCoroutineState::Yielded(ImapIdleYield::WantsWrite(bytes)) => {
                stream.write_all(&bytes).await.context("write failed")?;
            }
            ImapCoroutineState::Yielded(ImapIdleYield::WantsRead) => {
                match timeout(IDLE_REFRESH, stream.read(&mut buf)).await {
                    Ok(Ok(0)) => bail!("connection closed by peer"),
                    Ok(Ok(n)) => arg = Some(&buf[..n]),
                    Ok(Err(err)) => return Err(err).context("read failed"),
                    Err(_elapsed) => return Ok(IdleOutcome::Refresh),
                }
            }
            ImapCoroutineState::Complete(Ok(())) => {
                return Ok(if shutdown.load(Ordering::SeqCst) {
                    IdleOutcome::Shutdown
                } else {
                    IdleOutcome::Refresh
                });
            }
            ImapCoroutineState::Complete(Err(err)) => return Err(err.into()),
        }
    }
}

/// Drives an IDLE coroutine whose `done` flag is already set to completion
/// (writing DONE, reading the tagged response).
async fn drain_idle<S>(
    stream: &mut S,
    fragmentizer: &mut Fragmentizer,
    idle: &mut ImapIdle,
) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf = [0u8; READ_BUF];
    let mut arg: Option<&[u8]> = None;
    loop {
        match idle.resume(fragmentizer, arg.take()) {
            ImapCoroutineState::Yielded(ImapIdleYield::Event(_)) => {}
            ImapCoroutineState::Yielded(ImapIdleYield::WantsWrite(bytes)) => {
                stream.write_all(&bytes).await.context("write failed")?;
            }
            ImapCoroutineState::Yielded(ImapIdleYield::WantsRead) => {
                let n = stream.read(&mut buf).await.context("read failed")?;
                if n == 0 {
                    bail!("connection closed by peer");
                }
                arg = Some(&buf[..n]);
            }
            ImapCoroutineState::Complete(Ok(())) => return Ok(()),
            ImapCoroutineState::Complete(Err(err)) => return Err(err.into()),
        }
    }
}
