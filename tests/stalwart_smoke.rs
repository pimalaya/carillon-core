//! Live smoke test against a local Stalwart IMAP server.
//!
//! Ignored by default; it needs Stalwart running on `127.0.0.1:143` in plain
//! mode (io-imap's `tests/stalwart.sh` bootstraps exactly this). It drives the
//! real [`CarillonImapWatch`] coroutine with a blocking plain-TCP driver — the
//! same coroutine the carillon-server async pump and the CLI blocking pump
//! wrap — and asserts it rings a content-free `Changed` when a message is
//! appended to the watched mailbox.
//!
//! Run: `cargo test --test stalwart_smoke -- --ignored --nocapture`

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use carillon_core::backend::CarillonImapBackend;
use carillon_core::credential::CarillonCredential;
use carillon_core::imap::{CarillonImapWatch, CarillonImapWatchProgress as Progress};
use secrecy::SecretString;

const ADDR: &str = "127.0.0.1:143";
const LOGIN: &str = "test@pimalaya.org";
const PASS: &str = "P!malaya-test-2026";

#[test]
#[ignore = "requires local Stalwart on 127.0.0.1:143 (io-imap tests/stalwart.sh)"]
fn watch_rings_on_append() {
    // Once the watch has had time to reach IDLE, append a message on a second
    // connection to wake it.
    let appender = thread::spawn(|| {
        thread::sleep(Duration::from_secs(3));
        append_message();
    });

    let mut stream = TcpStream::connect(ADDR).expect("connect stalwart");
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();

    let backend = CarillonImapBackend {
        host: "127.0.0.1".to_string(),
        port: 143,
        login: LOGIN.to_string(),
        mailbox: "INBOX".to_string(),
    };
    let credential = CarillonCredential::Password(SecretString::from(PASS));
    let shutdown = Arc::new(AtomicBool::new(false));
    let mut watch = CarillonImapWatch::new(backend, credential, shutdown.clone());

    let mut buf = [0u8; 8 * 1024];
    let mut pending: Option<usize> = None;
    let deadline = Instant::now() + Duration::from_secs(30);

    loop {
        assert!(Instant::now() < deadline, "timed out before a ring");
        let input = pending.take().map(|n| &buf[..n]);
        match watch.resume(input) {
            Progress::WantsRead => {
                let n = stream.read(&mut buf).expect("read from stalwart");
                assert!(n > 0, "server closed the connection");
                pending = Some(n);
            }
            Progress::WantsWrite(bytes) => stream.write_all(&bytes).expect("write to stalwart"),
            Progress::Changed(state) => {
                assert!(!state.is_empty(), "state token should be non-empty");
                println!("RANG: content-free state = {state}");
                shutdown.store(true, Ordering::SeqCst);
                break;
            }
            Progress::Done(result) => panic!("watch ended before ringing: {result:?}"),
        }
    }

    appender.join().unwrap();
}

/// Appends one tiny message to INBOX over a fresh plain IMAP connection,
/// using raw commands (LOGIN + a synchronising-literal APPEND + LOGOUT).
fn append_message() {
    let mut s = TcpStream::connect(ADDR).expect("append: connect");
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();

    read_until(&mut s, "* OK"); // greeting
    s.write_all(format!("a1 LOGIN {LOGIN} {PASS}\r\n").as_bytes())
        .unwrap();
    read_until(&mut s, "a1 OK");

    let msg = b"From: smoke@pimalaya.org\r\nSubject: carillon smoke\r\n\r\nping\r\n";
    s.write_all(format!("a2 APPEND INBOX {{{}}}\r\n", msg.len()).as_bytes())
        .unwrap();
    read_until(&mut s, "+"); // continuation request
    s.write_all(msg).unwrap();
    s.write_all(b"\r\n").unwrap();
    read_until(&mut s, "a2 OK");

    s.write_all(b"a3 LOGOUT\r\n").unwrap();
}

/// Reads from `s`, accumulating, until the response contains `needle`.
fn read_until(s: &mut TcpStream, needle: &str) {
    let mut acc = String::new();
    let mut buf = [0u8; 4096];
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "append: timed out waiting for {needle:?}"
        );
        let n = s.read(&mut buf).expect("append: read");
        assert!(n > 0, "append: connection closed waiting for {needle:?}");
        acc.push_str(&String::from_utf8_lossy(&buf[..n]));
        if acc.contains(needle) {
            return;
        }
    }
}
