#![cfg_attr(docsrs, feature(doc_cfg))]

//! # carillon-core
//!
//! The shared I/O-free watch client behind the Carillon CLI daemon and the
//! Carillon server. Both frontends depend on it, and neither wraps the
//! other.
//!
//! ## I/O-free, driven by the frontend
//!
//! Core does no I/O. The watch is a coroutine ([`imap::CarillonImapWatch`])
//! that composes io-imap's own coroutines and, on each `resume`, returns
//! what the driver must do next: read, write, mint a ring for a changed
//! state, or stop. A frontend brings the driver and picks its own I/O
//! model: the CLI drives it with a blocking socket and one thread per
//! watch, the server with an async socket over many connections. The watch
//! logic is shared once; the I/O is not.
//!
//! This is not the io-email mistake (a sans-io *aggregator* unifying every
//! operation). It is a single-purpose composed coroutine, exactly what
//! io-imap already is one level down.
//!
//! ## What core owns, and what it does not
//!
//! Core owns the change signal ([`event::CarillonEvent`]), the source to
//! watch ([`backend::CarillonBackend`]), the resolved credential it presents
//! ([`credential::CarillonCredential`]), and the watch coroutine. Core does
//! not own the transport (TCP, TLS, timeouts), reconnect, credential
//! resolution (keyring, OAuth minting), storage, billing, delivery, or the
//! event's `id` and `ts` (a random id and a clock read are effects a driver
//! performs). Those stay at the frontend edge.
//!
//! ## Transport classes
//!
//! Every source reports a [`backend::CarillonTransportClass`]. A standing
//! connection or a poll dials outward, so any frontend can host it. A public
//! callback needs an inbound endpoint, so only the server can host it. A
//! frontend advertises the classes it can host and refuses to arm a watch
//! outside them, which is why the CLI cannot watch a Gmail push.
//!
//! ## Layout
//!
//! The modules are flat. The event module holds the signal, the backend
//! module the source description and its transport class, the credential
//! module the presented secret, the error module the watch result and
//! error, and the imap module the I/O-free watch coroutine. The design
//! history lives under cairn/.

pub mod backend;
#[cfg(feature = "carddav")]
pub mod carddav;
pub mod credential;
pub mod error;
pub mod event;
#[cfg(feature = "imap")]
pub mod imap;
