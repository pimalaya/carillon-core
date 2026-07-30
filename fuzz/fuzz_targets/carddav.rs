#![no_main]

//! Coverage-guided fuzz target for the CardDAV poll parser. Hostile HTTP
//! responses / `sync-collection` XML flow through `CarillonCardDavPoll::resume`.
//! Oracle: no input may panic or hang. Mirrors `carddav::adversarial_tests`.

use std::time::Duration;

use carillon_core::backend::CarillonCardDavBackend;
use carillon_core::carddav::{CarillonCardDavPoll, CarillonCardDavPollProgress};
use carillon_core::credential::CarillonCredential;
use libfuzzer_sys::fuzz_target;
use secrecy::SecretString;
use url::Url;

fuzz_target!(|data: &[u8]| {
    let base = match Url::parse("https://dav.example.org") {
        Ok(base) => base,
        Err(_) => return,
    };
    let backend = CarillonCardDavBackend {
        url: "https://dav.example.org/addressbooks/u/default/".to_owned(),
        login: "user".to_owned(),
        poll: Duration::from_secs(60),
    };
    let credential = CarillonCredential::Password(SecretString::from("password".to_owned()));
    let mut poll = CarillonCardDavPoll::new(
        &base,
        &backend,
        &credential,
        "/addressbooks/u/default/",
        None,
    );

    let mut fed = false;
    let mut next: Option<Vec<u8>> = None;
    for _ in 0..512 {
        match poll.resume(next.take().as_deref()) {
            CarillonCardDavPollProgress::WantsRead => {
                next = Some(if fed { Vec::new() } else { data.to_vec() });
                fed = true;
            }
            CarillonCardDavPollProgress::WantsWrite(_) => next = None,
            CarillonCardDavPollProgress::Done(_) => break,
        }
    }
});
