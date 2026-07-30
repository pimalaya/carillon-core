#![no_main]

//! Coverage-guided fuzz target for the IMAP watch parser. A malicious,
//! user-named server's bytes flow through `CarillonImapWatch::resume` in the
//! process that (in a deployment) holds every account's decrypted credentials.
//! Oracle: no input may panic, hang, or allocate without bound — the 1 MiB
//! fragmentizer bound guards the last. Mirrors `imap::adversarial_tests`.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use carillon_core::backend::CarillonImapBackend;
use carillon_core::credential::CarillonCredential;
use carillon_core::imap::{CarillonImapWatch, CarillonImapWatchProgress};
use libfuzzer_sys::fuzz_target;
use secrecy::SecretString;

fuzz_target!(|data: &[u8]| {
    let mut watch = CarillonImapWatch::new(
        CarillonImapBackend {
            host: "imap.example.org".to_owned(),
            port: 993,
            login: "user@example.org".to_owned(),
            mailbox: "INBOX".to_owned(),
        },
        CarillonCredential::Password(SecretString::from("password".to_owned())),
        Arc::new(AtomicBool::new(false)),
    );

    // Feed the fuzz input at the first read and EOF (empty slice) after, under a
    // bounded step count so a non-terminating input cannot hang the runner.
    let mut fed = false;
    let mut next: Option<Vec<u8>> = None;
    for _ in 0..512 {
        match watch.resume(next.take().as_deref()) {
            CarillonImapWatchProgress::WantsRead => {
                next = Some(if fed { Vec::new() } else { data.to_vec() });
                fed = true;
            }
            CarillonImapWatchProgress::WantsWrite(_) => next = None,
            CarillonImapWatchProgress::Changed(_) => next = None,
            CarillonImapWatchProgress::Done(_) => break,
        }
    }
});
