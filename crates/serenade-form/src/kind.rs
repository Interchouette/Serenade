//! Field kinds and typed option structs.

/// HTML-oriented field kind used for bind and render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// Single-line text (`input type="text"`).
    Text,
    /// Multi-line text (`textarea`).
    Textarea,
    /// Hidden input.
    Hidden,
    /// Password input (value still stored as a string).
    Password,
    /// Checkbox (`input type="checkbox"`).
    Checkbox,
    /// Single or multi select / radio group (see [`ChoiceOptions::multiple`]).
    Choice,
    /// Numeric input (`input type="number"`).
    Number,
}

/// Options for [`FieldKind::Text`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextOptions {
    /// Optional `placeholder` attribute.
    pub placeholder: Option<String>,
}

/// Options for [`FieldKind::Textarea`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextareaOptions {
    /// Optional `placeholder` attribute.
    pub placeholder: Option<String>,
    /// Optional `rows` attribute.
    pub rows: Option<u32>,
}

/// Options for [`FieldKind::Hidden`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiddenOptions {}

/// Options for [`FieldKind::Password`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PasswordOptions {
    /// Optional `placeholder` attribute.
    pub placeholder: Option<String>,
}

/// Options for [`FieldKind::Checkbox`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxOptions {
    /// Submitted value when the box is checked (default `"1"`).
    pub checked_value: String,
}

impl Default for CheckboxOptions {
    fn default() -> Self {
        Self {
            checked_value: String::from("1"),
        }
    }
}

/// One choice entry: submitted value and display label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceEntry {
    /// Value posted when this choice is selected.
    pub value: String,
    /// Human-readable label.
    pub label: String,
}

impl ChoiceEntry {
    /// Builds a choice with the same string for value and label.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            label: value.clone(),
            value,
        }
    }

    /// Builds a choice with distinct value and label.
    #[must_use]
    pub fn with_label(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Options for [`FieldKind::Choice`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChoiceOptions {
    /// Allowed choices.
    pub choices: Vec<ChoiceEntry>,
    /// When `true`, multiple values may be selected (stored space-separated in [`crate::Field::value`]).
    pub multiple: bool,
    /// When `true`, prefer expanded widgets (radio/checkbox list) over `<select>` in render.
    pub expanded: bool,
}

/// Options for [`FieldKind::Number`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NumberOptions {
    /// Optional HTML `min`.
    pub min: Option<i64>,
    /// Optional HTML `max`.
    pub max: Option<i64>,
    /// Optional HTML `step` (integer step).
    pub step: Option<i64>,
}

/// Typed configuration for a field (kind + options).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldConfig {
    /// [`FieldKind::Text`].
    Text(TextOptions),
    /// [`FieldKind::Textarea`].
    Textarea(TextareaOptions),
    /// [`FieldKind::Hidden`].
    Hidden(HiddenOptions),
    /// [`FieldKind::Password`].
    Password(PasswordOptions),
    /// [`FieldKind::Checkbox`].
    Checkbox(CheckboxOptions),
    /// [`FieldKind::Choice`].
    Choice(ChoiceOptions),
    /// [`FieldKind::Number`].
    Number(NumberOptions),
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self::Text(TextOptions::default())
    }
}

impl FieldConfig {
    /// Returns the [`FieldKind`] for this config.
    #[must_use]
    pub const fn kind(&self) -> FieldKind {
        match self {
            Self::Text(_) => FieldKind::Text,
            Self::Textarea(_) => FieldKind::Textarea,
            Self::Hidden(_) => FieldKind::Hidden,
            Self::Password(_) => FieldKind::Password,
            Self::Checkbox(_) => FieldKind::Checkbox,
            Self::Choice(_) => FieldKind::Choice,
            Self::Number(_) => FieldKind::Number,
        }
    }

    /// Returns choice options when this is a choice field.
    #[must_use]
    pub const fn as_choice(&self) -> Option<&ChoiceOptions> {
        match self {
            Self::Choice(options) => Some(options),
            _ => None,
        }
    }

    /// Returns checkbox options when this is a checkbox field.
    #[must_use]
    pub const fn as_checkbox(&self) -> Option<&CheckboxOptions> {
        match self {
            Self::Checkbox(options) => Some(options),
            _ => None,
        }
    }

    /// Returns number options when this is a number field.
    #[must_use]
    pub const fn as_number(&self) -> Option<&NumberOptions> {
        match self {
            Self::Number(options) => Some(options),
            _ => None,
        }
    }
}
