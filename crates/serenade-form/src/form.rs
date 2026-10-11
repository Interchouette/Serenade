//! Form builder, bind, and validation.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use serenade_http::{Method, Request};
use serenade_security::{CSRF_FIELD_NAME, CsrfToken, CsrfTokenManager, SecurityError};
use serenade_validator::{Constraint, ConstraintViolationList, RecursiveValidator, Validator};

use crate::collection::collection_field_name;
use crate::kind::{ChoiceOptions, FieldConfig, FieldKind};
use crate::parse::parse_urlencoded_multi;
use crate::render::RenderedForm;
use crate::transformer::{DataTransformer, join_multi};
use crate::{FormError, escape_attr, escape_html};

/// Outcome after [`Form::handle_request`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormStatus {
    /// Request was not a form submit for this form.
    NotSubmitted,
    /// Bound successfully (CSRF OK when enabled).
    Bound,
}

/// One named form field with kind, optional constraints, and current string value.
pub struct Field {
    name: String,
    value: String,
    constraints: Vec<Arc<dyn Constraint>>,
    config: FieldConfig,
    transformer: Option<Arc<dyn DataTransformer>>,
}

impl Field {
    fn new(
        name: impl Into<String>,
        config: FieldConfig,
        constraints: Vec<Arc<dyn Constraint>>,
        transformer: Option<Arc<dyn DataTransformer>>,
    ) -> Self {
        Self {
            name: name.into(),
            value: String::new(),
            constraints,
            config,
            transformer,
        }
    }

    /// Field name (HTML `name` attribute).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Current model value (empty before bind / default).
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Field kind.
    #[must_use]
    pub const fn kind(&self) -> FieldKind {
        self.config.kind()
    }

    /// Typed field configuration.
    #[must_use]
    pub const fn config(&self) -> &FieldConfig {
        &self.config
    }
}

/// HTML form with optional CSRF (on by default) and validator bridge.
pub struct Form {
    name: String,
    method: Method,
    action: String,
    fields: Vec<Field>,
    csrf_enabled: bool,
    csrf_token: Option<CsrfToken>,
    submitted: bool,
    violations: ConstraintViolationList,
}

impl Form {
    /// Starts a builder named `name` (also used as CSRF intention id).
    #[must_use]
    pub fn builder(name: impl Into<String>) -> FormBuilder {
        FormBuilder::new(name)
    }

    /// Form name / CSRF intention id.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the last [`Self::handle_request`] bound a submit.
    #[must_use]
    pub const fn is_submitted(&self) -> bool {
        self.submitted
    }

    /// Field values after bind (excludes CSRF field).
    #[must_use]
    pub fn data(&self) -> HashMap<String, String> {
        self.fields
            .iter()
            .map(|f| (f.name.clone(), f.value.clone()))
            .collect()
    }

    /// Declared fields in builder order.
    #[must_use]
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Returns a field value when present.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .map(Field::value)
    }

    /// Sets a field value when the field exists (for example edit prefill).
    ///
    /// Returns `true` when `name` matched a field.
    pub fn set(&mut self, name: &str, value: impl Into<String>) -> bool {
        if let Some(field) = self.fields.iter_mut().find(|f| f.name == name) {
            field.value = value.into();
            true
        } else {
            false
        }
    }

    /// Constraint violations from the last [`Self::validate`] call.
    #[must_use]
    pub const fn violations(&self) -> &ConstraintViolationList {
        &self.violations
    }

    /// Issues a CSRF token when CSRF is enabled (call before render on GET).
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Csrf`] when token generation fails.
    pub fn prepare_csrf(&mut self, manager: &dyn CsrfTokenManager) -> Result<(), FormError> {
        if !self.csrf_enabled {
            return Ok(());
        }
        self.csrf_token = Some(manager.get_token(&self.name)?);
        Ok(())
    }

    /// Binds POST/PUT/PATCH `application/x-www-form-urlencoded` body and checks CSRF.
    ///
    /// # Errors
    ///
    /// Returns parse, transform, or CSRF errors. Wrong method yields
    /// [`FormStatus::NotSubmitted`] without error.
    pub fn handle_request(
        &mut self,
        request: &Request,
        manager: &dyn CsrfTokenManager,
    ) -> Result<FormStatus, FormError> {
        self.submitted = false;
        self.violations = ConstraintViolationList::new();
        if !matches!(request.method(), Method::Post | Method::Put | Method::Patch) {
            return Ok(FormStatus::NotSubmitted);
        }
        let values = parse_urlencoded_multi(request.body())?;
        if self.csrf_enabled {
            let submitted = values
                .get(CSRF_FIELD_NAME)
                .and_then(|v| v.last())
                .map_or("", String::as_str);
            let token = CsrfToken::new(&self.name, submitted);
            if !manager.is_token_valid(&token) {
                return Err(SecurityError::InvalidCsrfToken.into());
            }
            self.csrf_token = Some(token);
        }
        for field in &mut self.fields {
            bind_field(field, &values)?;
        }
        self.submitted = true;
        Ok(FormStatus::Bound)
    }

    /// Runs field constraints through `validator` (defaults to [`RecursiveValidator`] when omitted via [`Self::is_valid`]).
    pub fn validate(&mut self, validator: &dyn Validator) -> bool {
        let mut violations = ConstraintViolationList::new();
        for field in &self.fields {
            let refs: Vec<&dyn Constraint> = field.constraints.iter().map(AsRef::as_ref).collect();
            let field_violations = validator.validate_value(&field.value, &field.name, &refs);
            for v in field_violations.as_slice() {
                violations.add(v.clone());
            }
        }
        let ok = violations.is_empty();
        self.violations = violations;
        ok
    }

    /// Validates with [`RecursiveValidator`].
    #[must_use]
    pub fn is_valid(&mut self) -> bool {
        self.validate(&RecursiveValidator::new())
    }

    /// Renders a full HTML form (escaped values, CSRF hidden field when enabled).
    ///
    /// # Errors
    ///
    /// Returns [`FormError::Csrf`] when CSRF is enabled but no token was prepared or bound,
    /// or [`FormError::Transform`] when a field transformer rejects the model value.
    pub fn render(&self) -> Result<RenderedForm, FormError> {
        let csrf_html = if self.csrf_enabled {
            let token = self
                .csrf_token
                .as_ref()
                .ok_or(SecurityError::InvalidCsrfToken)?;
            format!(
                r#"<input type="hidden" name="{}" value="{}" />"#,
                escape_attr(CSRF_FIELD_NAME),
                escape_attr(token.value())
            )
        } else {
            String::new()
        };
        let mut fields_html = String::new();
        for field in &self.fields {
            fields_html.push_str(&render_field_widget(field)?);
        }
        let html = format!(
            r#"<form name="{name}" method="{method}" action="{action}">{csrf}{fields}</form>"#,
            name = escape_attr(&self.name),
            method = escape_attr(self.method.as_str()),
            action = escape_attr(&self.action),
            csrf = csrf_html,
            fields = fields_html,
        );
        Ok(RenderedForm { html })
    }
}

fn bind_field(field: &mut Field, values: &HashMap<String, Vec<String>>) -> Result<(), FormError> {
    let view = match field.config() {
        FieldConfig::Choice(options) if options.multiple => {
            let submitted = values.get(field.name()).cloned().unwrap_or_default();
            join_multi(&submitted)
        }
        FieldConfig::Checkbox(_) => values
            .get(field.name())
            .and_then(|v| v.last())
            .cloned()
            .unwrap_or_default(),
        _ => values
            .get(field.name())
            .and_then(|v| v.last())
            .cloned()
            .unwrap_or_default(),
    };
    field.value = if let Some(transformer) = &field.transformer {
        transformer.reverse_transform(&view)?
    } else {
        view
    };
    Ok(())
}

fn view_value(field: &Field) -> Result<String, FormError> {
    field
        .transformer
        .as_ref()
        .map_or_else(|| Ok(field.value.clone()), |t| t.transform(&field.value))
}

fn render_field_widget(field: &Field) -> Result<String, FormError> {
    let id = escape_attr(field.name());
    let label = escape_html(field.name());
    let name = escape_attr(field.name());
    let view = view_value(field)?;
    let value = escape_attr(&view);
    let html = match field.config() {
        FieldConfig::Textarea(options) => {
            let rows = options
                .rows
                .map_or(String::new(), |r| format!(r#" rows="{r}""#));
            let placeholder = placeholder_attr(options.placeholder.as_deref());
            format!(
                r#"<label for="{id}">{label}</label><textarea id="{id}" name="{name}"{rows}{placeholder}>{text}</textarea>"#,
                text = escape_html(&view),
            )
        }
        FieldConfig::Hidden(_) => {
            format!(r#"<input type="hidden" id="{id}" name="{name}" value="{value}" />"#)
        }
        FieldConfig::Password(options) => {
            let placeholder = placeholder_attr(options.placeholder.as_deref());
            format!(
                r#"<label for="{id}">{label}</label><input type="password" id="{id}" name="{name}" value="{value}"{placeholder} />"#
            )
        }
        FieldConfig::Checkbox(options) => {
            let checked = if field.value() == options.checked_value
                || matches!(field.value(), "1" | "true")
            {
                " checked"
            } else {
                ""
            };
            let checked_value = escape_attr(&options.checked_value);
            format!(
                r#"<label for="{id}">{label}</label><input type="checkbox" id="{id}" name="{name}" value="{checked_value}"{checked} />"#
            )
        }
        FieldConfig::Choice(options) => render_choice_widget(field, options, &id, &label, &name),
        FieldConfig::Number(options) => {
            let min = options
                .min
                .map_or(String::new(), |m| format!(r#" min="{m}""#));
            let max = options
                .max
                .map_or(String::new(), |m| format!(r#" max="{m}""#));
            let step = options
                .step
                .map_or(String::new(), |s| format!(r#" step="{s}""#));
            format!(
                r#"<label for="{id}">{label}</label><input type="number" id="{id}" name="{name}" value="{value}"{min}{max}{step} />"#
            )
        }
        FieldConfig::Text(options) => {
            let placeholder = placeholder_attr(options.placeholder.as_deref());
            format!(
                r#"<label for="{id}">{label}</label><input type="text" id="{id}" name="{name}" value="{value}"{placeholder} />"#
            )
        }
    };
    Ok(html)
}

fn placeholder_attr(placeholder: Option<&str>) -> String {
    placeholder.map_or(String::new(), |p| {
        format!(r#" placeholder="{}""#, escape_attr(p))
    })
}

fn render_choice_widget(
    field: &Field,
    options: &ChoiceOptions,
    id: &str,
    label: &str,
    name: &str,
) -> String {
    let selected = crate::transformer::split_multi(field.value());
    if options.expanded {
        let mut html = format!(r"<fieldset><legend>{label}</legend>");
        for (i, choice) in options.choices.iter().enumerate() {
            let input_id = format!("{id}_{i}");
            let checked = if selected.contains(&choice.value.as_str()) {
                " checked"
            } else {
                ""
            };
            let input_type = if options.multiple {
                "checkbox"
            } else {
                "radio"
            };
            let _ = write!(
                html,
                r#"<label for="{input_id}"><input type="{input_type}" id="{input_id}" name="{name}" value="{value}"{checked} />{choice_label}</label>"#,
                value = escape_attr(&choice.value),
                choice_label = escape_html(&choice.label),
            );
        }
        html.push_str("</fieldset>");
        return html;
    }
    let multiple = if options.multiple { " multiple" } else { "" };
    let mut html =
        format!(r#"<label for="{id}">{label}</label><select id="{id}" name="{name}"{multiple}>"#);
    for choice in &options.choices {
        let is_selected = selected.contains(&choice.value.as_str());
        let selected_attr = if is_selected { " selected" } else { "" };
        let _ = write!(
            html,
            r#"<option value="{value}"{selected_attr}>{label}</option>"#,
            value = escape_attr(&choice.value),
            label = escape_html(&choice.label),
        );
    }
    html.push_str("</select>");
    html
}

/// Builds a [`Form`].
pub struct FormBuilder {
    name: String,
    method: Method,
    action: String,
    fields: Vec<Field>,
    csrf_enabled: bool,
}

impl FormBuilder {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            method: Method::Post,
            action: String::new(),
            fields: Vec::new(),
            csrf_enabled: true,
        }
    }

    /// Sets the HTML form method (default POST).
    #[must_use]
    pub const fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Sets the form `action` URL (empty = current path).
    #[must_use]
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }

    /// Enables or disables CSRF (default **true**).
    #[must_use]
    pub const fn csrf(mut self, enabled: bool) -> Self {
        self.csrf_enabled = enabled;
        self
    }

    /// Adds a text field with optional constraints (compat path for existing apps).
    #[must_use]
    pub fn field(self, name: impl Into<String>, constraints: Vec<Arc<dyn Constraint>>) -> Self {
        self.field_config(name, FieldConfig::default(), constraints)
    }

    /// Adds a field with typed [`FieldConfig`] and constraints.
    #[must_use]
    pub fn field_config(
        mut self,
        name: impl Into<String>,
        config: FieldConfig,
        constraints: Vec<Arc<dyn Constraint>>,
    ) -> Self {
        self.fields
            .push(Field::new(name, config, constraints, None));
        self
    }

    /// Adds a field with a [`DataTransformer`] applied on bind and render.
    #[must_use]
    pub fn field_transformed(
        mut self,
        name: impl Into<String>,
        config: FieldConfig,
        constraints: Vec<Arc<dyn Constraint>>,
        transformer: Arc<dyn DataTransformer>,
    ) -> Self {
        self.fields
            .push(Field::new(name, config, constraints, Some(transformer)));
        self
    }

    /// Adds nested fields under a dotted prefix (`address.street`).
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_form::Form;
    ///
    /// let form = Form::builder("profile")
    ///     .csrf(false)
    ///     .compound("address", |c| c.field("street", vec![]).field("city", vec![]))
    ///     .build();
    /// assert!(form.get("address.street").is_some());
    /// ```
    #[must_use]
    pub fn compound(
        mut self,
        name: impl Into<String>,
        configure: impl FnOnce(CompoundBuilder) -> CompoundBuilder,
    ) -> Self {
        let nested = configure(CompoundBuilder::new(name));
        self.fields.extend(nested.fields);
        self
    }

    /// Adds `count` indexed collection entries (`items[0][name]`).
    ///
    /// # Examples
    ///
    /// ```
    /// use serenade_form::Form;
    ///
    /// let form = Form::builder("order")
    ///     .csrf(false)
    ///     .collection("items", 2, |entry| entry.field("name", vec![]))
    ///     .build();
    /// assert!(form.get("items[0][name]").is_some());
    /// assert!(form.get("items[1][name]").is_some());
    /// ```
    #[must_use]
    pub fn collection(
        mut self,
        name: impl Into<String>,
        count: usize,
        configure: impl FnOnce(CollectionEntryBuilder) -> CollectionEntryBuilder,
    ) -> Self {
        let collection = name.into();
        let entry = configure(CollectionEntryBuilder::new());
        for index in 0..count {
            for spec in &entry.specs {
                let full_name = collection_field_name(&collection, &index.to_string(), &spec.name);
                self.fields.push(Field::new(
                    full_name,
                    spec.config.clone(),
                    spec.constraints.clone(),
                    spec.transformer.clone(),
                ));
            }
        }
        self
    }

    /// Finishes the builder.
    #[must_use]
    pub fn build(self) -> Form {
        Form {
            name: self.name,
            method: self.method,
            action: self.action,
            fields: self.fields,
            csrf_enabled: self.csrf_enabled,
            csrf_token: None,
            submitted: false,
            violations: ConstraintViolationList::new(),
        }
    }
}

/// Builds nested fields under a dotted name prefix.
pub struct CompoundBuilder {
    prefix: String,
    fields: Vec<Field>,
}

impl CompoundBuilder {
    fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            fields: Vec::new(),
        }
    }

    fn child_name(&self, name: impl Into<String>) -> String {
        format!("{}.{}", self.prefix, name.into())
    }

    /// Adds a text field under this compound prefix.
    #[must_use]
    pub fn field(self, name: impl Into<String>, constraints: Vec<Arc<dyn Constraint>>) -> Self {
        self.field_config(name, FieldConfig::default(), constraints)
    }

    /// Adds a typed field under this compound prefix.
    #[must_use]
    pub fn field_config(
        mut self,
        name: impl Into<String>,
        config: FieldConfig,
        constraints: Vec<Arc<dyn Constraint>>,
    ) -> Self {
        let full = self.child_name(name);
        self.fields
            .push(Field::new(full, config, constraints, None));
        self
    }

    /// Adds a transformed field under this compound prefix.
    #[must_use]
    pub fn field_transformed(
        mut self,
        name: impl Into<String>,
        config: FieldConfig,
        constraints: Vec<Arc<dyn Constraint>>,
        transformer: Arc<dyn DataTransformer>,
    ) -> Self {
        let full = self.child_name(name);
        self.fields
            .push(Field::new(full, config, constraints, Some(transformer)));
        self
    }

    /// Nests another compound group (`address.geo.lat`).
    #[must_use]
    pub fn compound(
        mut self,
        name: impl Into<String>,
        configure: impl FnOnce(Self) -> Self,
    ) -> Self {
        let nested = configure(Self::new(self.child_name(name)));
        self.fields.extend(nested.fields);
        self
    }
}

struct EntryFieldSpec {
    name: String,
    config: FieldConfig,
    constraints: Vec<Arc<dyn Constraint>>,
    transformer: Option<Arc<dyn DataTransformer>>,
}

/// Describes fields repeated for each collection index.
pub struct CollectionEntryBuilder {
    specs: Vec<EntryFieldSpec>,
}

impl CollectionEntryBuilder {
    const fn new() -> Self {
        Self { specs: Vec::new() }
    }

    /// Adds a text child field template for each collection entry.
    #[must_use]
    pub fn field(self, name: impl Into<String>, constraints: Vec<Arc<dyn Constraint>>) -> Self {
        self.field_config(name, FieldConfig::default(), constraints)
    }

    /// Adds a typed child field template for each collection entry.
    #[must_use]
    pub fn field_config(
        mut self,
        name: impl Into<String>,
        config: FieldConfig,
        constraints: Vec<Arc<dyn Constraint>>,
    ) -> Self {
        self.specs.push(EntryFieldSpec {
            name: name.into(),
            config,
            constraints,
            transformer: None,
        });
        self
    }

    /// Adds a transformed child field template for each collection entry.
    #[must_use]
    pub fn field_transformed(
        mut self,
        name: impl Into<String>,
        config: FieldConfig,
        constraints: Vec<Arc<dyn Constraint>>,
        transformer: Arc<dyn DataTransformer>,
    ) -> Self {
        self.specs.push(EntryFieldSpec {
            name: name.into(),
            config,
            constraints,
            transformer: Some(transformer),
        });
        self
    }
}
