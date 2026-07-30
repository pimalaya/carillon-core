//! Live smoke test: does the watch ring on a *flag change* (not new mail)?
//!
//! Ignored by default; needs Stalwart on `127.0.0.1:143` in plain mode
//! (io-imap's `tests/stalwart.sh`). This is the "any change" case that a
//! sync-style `cmd` hook depends on: a message already in the mailbox has a
//! flag toggled by another session, changing HIGHESTMODSEQ but *not*
//! UIDNEXT. The watch must still ring — otherwise only new mail is detected.
//!
//! Run: `cargo test --test stalwart_flag_smoke -- --ignored --nocapture`

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
fn watch_rings_on_flag_change() {
    // Pre-seed one message so the watch's initial EXAMINE has stable UIDNEXT.
    append_message();
    // Give the watch time to reach IDLE, then toggle a flag on that message
    // over a second connection: mod-seq advances, UIDNEXT does not.
    let flagger = thread::spawn(|| {
        thread::sleep(Duration::from_secs(4));
        store_seen_flag();
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
        assert!(
            Instant::now() < deadline,
            "timed out before a ring: the watch did NOT detect the flag change \
             (only new mail is detected — HIGHESTMODSEQ is not being tracked)"
        );
        let input = pending.take().map(|n| &buf[..n]);
        match watch.resume(input) {
            Progress::WantsRead => {
                let n = stream.read(&mut buf).expect("read from stalwart");
                assert!(n > 0, "server closed the connection");
                pending = Some(n);
            }
            Progress::WantsWrite(bytes) => stream.write_all(&bytes).expect("write to stalwart"),
            // The seed EXAMINE already saw the pre-appended message, and the
            // only change afterwards is the flag STORE (fired after 4s), so any
            // ring here is the flag change — a mod-seq advance with no new mail.
            Progress::Changed(state) => {
                println!("RANG on flag change: content-free state = {state}");
                shutdown.store(true, Ordering::SeqCst);
                break;
            }
            Progress::Done(result) => panic!("watch ended before ringing: {result:?}"),
        }
    }

    flagger.join().unwrap();
}

/// Appends one tiny message to INBOX over a fresh plain IMAP connection.
fn append_message() {
    let mut s = TcpStream::connect(ADDR).expect("append: connect");
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    read_until(&mut s, "* OK");
    s.write_all(format!("a1 LOGIN {LOGIN} {PASS}\r\n").as_bytes())
        .unwrap();
    read_until(&mut s, "a1 OK");
    let msg = b"From: smoke@pimalaya.org\r\nSubject: carillon flag smoke\r\n\r\nping\r\n";
    s.write_all(format!("a2 APPEND INBOX {{{}}}\r\n", msg.len()).as_bytes())
        .unwrap();
    read_until(&mut s, "+");
    s.write_all(msg).unwrap();
    s.write_all(b"\r\n").unwrap();
    read_until(&mut s, "a2 OK");
    s.write_all(b"a3 LOGOUT\r\n").unwrap();
}

/// Toggles the `\Seen` flag on the newest message over a fresh read-write
/// connection: this advances HIGHESTMODSEQ but leaves UIDNEXT untouched.
fn store_seen_flag() {
    let mut s = TcpStream::connect(ADDR).expect("store: connect");
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    read_until(&mut s, "* OK");
    s.write_all(format!("b1 LOGIN {LOGIN} {PASS}\r\n").as_bytes())
        .unwrap();
    read_until(&mut s, "b1 OK");
    s.write_all(b"b2 SELECT INBOX\r\n").unwrap();
    read_until(&mut s, "b2 OK");
    // Flag the last message in the mailbox.
    s.write_all(b"b3 STORE * +FLAGS (\\Seen)\r\n").unwrap();
    read_until(&mut s, "b3 OK");
    s.write_all(b"b4 LOGOUT\r\n").unwrap();
}

/// Reads from `s`, accumulating, until the response contains `needle`.
fn read_until(s: &mut TcpStream, needle: &str) {
    let mut acc = String::new();
    let mut buf = [0u8; 4096];
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {needle:?}; got:\n{acc}"
        );
        let n = s.read(&mut buf).expect("read");
        assert!(n > 0, "connection closed waiting for {needle:?}");
        acc.push_str(&String::from_utf8_lossy(&buf[..n]));
        if acc.contains(needle) {
            return;
        }
    }
}
