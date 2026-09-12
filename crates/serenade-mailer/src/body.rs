//! Message body (text and/or HTML).

/// Plain text and/or HTML body parts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Body {
    text: Option<String>,
    html: Option<String>,
}

impl Body {
    /// Empty body.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            text: None,
            html: None,
        }
    }

    /// Text-only body.
    #[must_use]
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            text: Some(body.into()),
            html: None,
        }
    }

    /// HTML-only body.
    #[must_use]
    pub fn html(body: impl Into<String>) -> Self {
        Self {
            text: None,
            html: Some(body.into()),
        }
    }

    /// Both text and HTML parts.
    #[must_use]
    pub fn both(text: impl Into<String>, html: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            html: Some(html.into()),
        }
    }

    /// Plain text part when set.
    #[must_use]
    pub fn text_part(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// HTML part when set.
    #[must_use]
    pub fn html_part(&self) -> Option<&str> {
        self.html.as_deref()
    }

    /// Whether any body part is present.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.text.is_none() && self.html.is_none()
    }
}
