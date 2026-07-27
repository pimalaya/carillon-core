#![cfg_attr(docsrs, feature(doc_cfg))]

//! # carillon-core
//!
//! The shared async watch client behind the Carillon CLI daemon and the
//! Carillon server. Both frontends depend on it, and neither wraps the
//! other.
//!
//! ## Not sans-io, on purpose
//!
//! A sans-io watch aggregator would repeat the io-email mistake, an
//! interface layer whose upkeep outweighs what it saves. Core instead owns
//! the async network I/O of watching directly, the way Himalaya and Ortie
//! each rebuild their own client over io-imap and io-webdav. The one thing
//! shared is a single watch loop rather than two.
//!
//! ## What core owns, and what it does not
//!
//! Core owns the change signal ([`event::CarillonEvent`]), the source to
//! watch ([`backend::CarillonBackend`]), the resolved credential it
//! presents ([`credential::CarillonCredential`]), and the per-protocol
//! watch conversation ([`imap::watch`]). Core does not own the transport:
//! the frontend opens the stream (TCP, TLS, keepalive, any address or SSRF
//! policy) and owns reconnect, so core stays TLS-agnostic and generic over
//! the async stream. Core does not own credential resolution (keyring
//! lookup and OAuth minting happen upstream), storage, billing, delivery,
//! or consumer fan-out either. Those stay at the frontend edge.
//!
//! ## Transport classes
//!
//! Every source reports a [`backend::CarillonTransportClass`]. A standing
//! connection or a poll dials outward, so any frontend can host it. A
//! public callback needs an inbound endpoint, so only the server can host
//! it. A frontend advertises the classes it can host and refuses to arm a
//! watch outside them, which is why the CLI cannot watch a Gmail push.
//!
//! ## Layout
//!
//! The modules are flat. The event module holds the signal, the backend
//! module the source description and its transport class, the credential
//! module the presented secret, the error module the watch result and
//! error, and the imap module the IMAP watch conversation driven over a
//! caller-owned stream. The design history lives under cairn/.

pub mod backend;
pub mod credential;
pub mod error;
pub mod event;
#[cfg(feature = "imap")]
pub mod imap;
