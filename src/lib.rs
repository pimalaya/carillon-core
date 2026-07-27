//! `carillon-core`: the shared async watch client behind the Carillon CLI
//! daemon and the Carillon server.
//!
//! It is deliberately **not** sans-io. A sans-io "watch aggregator" would
//! repeat the io-email mistake: an interface layer whose upkeep outweighs
//! what it saves. Instead, like Himalaya or Ortie rebuilding their own
//! client over `io-imap` / `io-webdav`, core owns the async network I/O of
//! watching — it just owns it *once*, so both frontends share a single
//! loop rather than maintaining two.
//!
//! What core does **not** own, on purpose:
//!
//! - **credential resolution** — it receives a ready [`backend::Credential`]
//!   (a password or a pre-minted bearer token); keyring lookup and OAuth
//!   minting/refresh happen upstream;
//! - **storage, billing, delivery, consumers** — those stay at the
//!   frontend edge.
//!
//! Core's whole surface is: the knock-knock [`event::Event`], the
//! [`backend::Backend`] to watch, and [`backend::watch`] to run one.

#[cfg(not(any(feature = "imap", feature = "carddav")))]
compile_error!("enable at least one backend feature (imap, carddav)");

pub mod backend;
pub mod error;
pub mod event;
