//! Mailer errors.

/// Failure while building an email message.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MailerError {
    /// Address is empty or not a plausible mailbox.
    #[error("invalid address: {message}")]
    InvalidAddress {
        /// Reason text.
        message: String,
    },
}
