//! Email message and Mime lite types (Symfony Mailer + Mime shaped).
//!
//! This crate defines [`Email`], [`Address`], [`Body`], and [`Attachment`] only.
//! Transports and DI wiring live in a later slice.

mod address;
mod attachment;
mod body;
mod email;
mod error;

pub use address::Address;
pub use attachment::Attachment;
pub use body::Body;
pub use email::Email;
pub use error::MailerError;

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
