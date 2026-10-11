//! HTML form component: bind request, CSRF by default, XSS-safe render helpers.
//!
//! Apps declare fields, kinds, and constraints; Serenade owns CSRF validation and
//! XSS-safe escaping. See `docs-dev/FORMS.md` and `docs-dev/SECURITY.md`.
//!
//! Multipart uploads and row/theme helpers are out of scope for this crate surface
//! until those APIs land separately.

mod collection;
mod error;
mod escape;
mod form;
mod kind;
mod parse;
mod render;
mod transformer;

pub use collection::{
    COLLECTION_PROTOTYPE_INDEX, CollectionPrototypeMeta, NestedData, collection_field_name,
    collection_indices, collection_prototype_field_name, unflatten_form_data,
};
pub use error::FormError;
pub use escape::{escape_attr, escape_html};
pub use form::{CollectionEntryBuilder, CompoundBuilder, Field, Form, FormBuilder, FormStatus};
pub use kind::{
    CheckboxOptions, ChoiceEntry, ChoiceOptions, FieldConfig, FieldKind, HiddenOptions,
    NumberOptions, PasswordOptions, TextOptions, TextareaOptions,
};
pub use parse::{parse_urlencoded, parse_urlencoded_multi};
pub use render::RenderedForm;
pub use serenade_security::CSRF_FIELD_NAME;
pub use transformer::{
    BoolToStringTransformer, ChoiceToValueTransformer, DataTransformer, I64ToStringTransformer,
    join_multi, split_multi,
};

/// Compile-time crate version for diagnostics.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests;
