//! Core error type.

use thiserror::Error;

/// Result alias for `carillon-core`.
pub type Result<T> = std::result::Result<T, Error>;

/// Something that went wrong opening or running a watch.
#[derive(Debug, Error)]
pub enum Error {
    /// The selected backend is not wired yet (pre-layer-2 placeholder).
    #[error("watch backend not implemented yet")]
    NotImplemented,
}
