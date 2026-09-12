//! Email address (mailbox) for Mime lite.

use crate::MailerError;

/// A mailbox: bare address plus optional display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Address {
    email: String,
    name: Option<String>,
}

impl Address {
    /// Creates an address with no display name.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError::InvalidAddress`] when `email` is empty or lacks `@`.
    pub fn new(email: impl Into<String>) -> Result<Self, MailerError> {
        Self::with_name(email, None::<String>)
    }

    /// Creates an address with an optional display name.
    ///
    /// # Errors
    ///
    /// Returns [`MailerError::InvalidAddress`] when `email` is empty or lacks `@`.
    pub fn with_name(
        email: impl Into<String>,
        name: Option<impl Into<String>>,
    ) -> Result<Self, MailerError> {
        let email = email.into().trim().to_owned();
        validate_email(&email)?;
        Ok(Self {
            email,
            name: name.map(Into::into).filter(|value| !value.is_empty()),
        })
    }

    /// Email address string.
    #[must_use]
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Optional display name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl TryFrom<&str> for Address {
    type Error = MailerError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

fn validate_email(email: &str) -> Result<(), MailerError> {
    if email.is_empty() || !email.contains('@') || email.starts_with('@') || email.ends_with('@') {
        return Err(MailerError::InvalidAddress {
            message: format!("invalid email address: {email}"),
        });
    }
    Ok(())
}
