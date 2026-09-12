//! Attachment bytes for Mime lite.

/// File attachment (filename + content type + raw bytes).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attachment {
    filename: String,
    content_type: String,
    body: Vec<u8>,
}

impl Attachment {
    /// Builds an attachment from raw bytes.
    #[must_use]
    pub fn from_bytes(
        filename: impl Into<String>,
        content_type: impl Into<String>,
        body: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
            body: body.into(),
        }
    }

    /// Suggested filename.
    #[must_use]
    pub fn filename(&self) -> &str {
        &self.filename
    }

    /// MIME content type (for example `application/pdf`).
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Raw attachment bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }
}
