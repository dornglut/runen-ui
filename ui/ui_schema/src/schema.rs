//! File: domain/ui/ui_schema/src/schema.rs
//! Crate: ui_schema

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::value::{UiSchemaValue, UiSchemaValueKind};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiSchemaId(String);

impl UiSchemaId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("UI schema IDs must be stable namespaced IDs")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiSchemaContractError> {
        let value = value.into();
        validate_schema_id(&value, "schema")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiSchemaVersion(u32);

impl UiSchemaVersion {
    pub const fn new(value: u32) -> Self {
        assert!(value > 0);
        Self(value)
    }

    pub fn try_new(value: u32) -> Result<Self, UiSchemaContractError> {
        if value == 0 {
            Err(UiSchemaContractError::ZeroSchemaVersion)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiSchemaRef {
    pub id: UiSchemaId,
    pub version: UiSchemaVersion,
}

impl UiSchemaRef {
    pub fn new(id: impl Into<String>, version: u32) -> Self {
        Self {
            id: UiSchemaId::new(id),
            version: UiSchemaVersion::new(version),
        }
    }

    pub fn try_new(id: impl Into<String>, version: u32) -> Result<Self, UiSchemaContractError> {
        Ok(Self {
            id: UiSchemaId::try_new(id)?,
            version: UiSchemaVersion::try_new(version)?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiSchemaContractError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
    ZeroSchemaVersion,
}

impl fmt::Display for UiSchemaContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "empty UI schema {kind} id"),
            Self::UnnamespacedId { kind, value } => {
                write!(formatter, "UI schema {kind} id {value} is not namespaced")
            }
            Self::InvalidIdCharacter { kind, value } => write!(
                formatter,
                "UI schema {kind} id {value} contains an invalid character"
            ),
            Self::ZeroSchemaVersion => write!(formatter, "UI schema version must be non-zero"),
        }
    }
}

impl std::error::Error for UiSchemaContractError {}

fn validate_schema_id(value: &str, kind: &'static str) -> Result<(), UiSchemaContractError> {
    if value.is_empty() {
        return Err(UiSchemaContractError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(UiSchemaContractError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(UiSchemaContractError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiSchema {
    pub schema_ref: UiSchemaRef,
    pub root: UiSchemaShape,
    pub fields: BTreeMap<String, UiSchemaField>,
    pub unknown_fields: UiUnknownFieldPolicy,
}

impl UiSchema {
    pub fn object(id: impl Into<String>, version: u32) -> Self {
        Self {
            schema_ref: UiSchemaRef::new(id, version),
            root: UiSchemaShape::Object,
            fields: BTreeMap::new(),
            unknown_fields: UiUnknownFieldPolicy::Reject,
        }
    }

    pub fn value(id: impl Into<String>, version: u32, root: UiSchemaShape) -> Self {
        Self {
            schema_ref: UiSchemaRef::new(id, version),
            root,
            fields: BTreeMap::new(),
            unknown_fields: UiUnknownFieldPolicy::Reject,
        }
    }

    pub fn with_required_field(mut self, name: impl Into<String>, shape: UiSchemaShape) -> Self {
        self.fields
            .insert(name.into(), UiSchemaField::required(shape));
        self
    }

    pub fn with_optional_field(mut self, name: impl Into<String>, shape: UiSchemaShape) -> Self {
        self.fields
            .insert(name.into(), UiSchemaField::optional(shape));
        self
    }

    pub fn with_preserved_unknown_fields(mut self) -> Self {
        self.unknown_fields = UiUnknownFieldPolicy::PreserveForDebugFixtures;
        self
    }

    pub fn validate(&self, value: &UiSchemaValue) -> UiSchemaValidationReport {
        let mut diagnostics = Vec::new();
        if !self.root.matches_value(value) {
            diagnostics.push(UiSchemaValidationDiagnostic::new(
                self.schema_ref.clone(),
                UiSchemaDiagnosticId::new("ui.schema.root_kind_mismatch"),
                format!(
                    "expected root value kind {:?}, got {:?}",
                    self.root.expected_kind(),
                    value.kind()
                ),
            ));
            return UiSchemaValidationReport {
                schema_ref: self.schema_ref.clone(),
                diagnostics,
            };
        }

        if let UiSchemaValue::Object(values) = value {
            for (name, field) in &self.fields {
                match values.get(name) {
                    Some(field_value) if field.shape.matches_value(field_value) => {}
                    Some(field_value) => {
                        if let (Some(allowed_values), UiSchemaValue::String(actual_value)) =
                            (field.shape.allowed_string_values(), field_value)
                        {
                            diagnostics.push(UiSchemaValidationDiagnostic::for_field(
                                self.schema_ref.clone(),
                                UiSchemaDiagnosticId::new("ui.schema.string_value_not_allowed"),
                                name,
                                format!(
                                    "field {name} value {actual_value:?} is not one of {:?}",
                                    allowed_values
                                ),
                            ));
                        } else {
                            diagnostics.push(UiSchemaValidationDiagnostic::for_field(
                                self.schema_ref.clone(),
                                UiSchemaDiagnosticId::new("ui.schema.field_kind_mismatch"),
                                name,
                                format!(
                                    "expected field {name} kind {:?}, got {:?}",
                                    field.shape.expected_kind(),
                                    field_value.kind()
                                ),
                            ));
                        }
                    }
                    None if field.required => {
                        diagnostics.push(UiSchemaValidationDiagnostic::for_field(
                            self.schema_ref.clone(),
                            UiSchemaDiagnosticId::new("ui.schema.required_field_missing"),
                            name,
                            format!("required field {name} is missing"),
                        ))
                    }
                    None => {}
                }
            }

            if self.unknown_fields == UiUnknownFieldPolicy::Reject {
                for name in values.keys() {
                    if !self.fields.contains_key(name) {
                        diagnostics.push(UiSchemaValidationDiagnostic::for_field(
                            self.schema_ref.clone(),
                            UiSchemaDiagnosticId::new("ui.schema.unknown_field"),
                            name,
                            format!("unknown field {name} is not accepted by this schema"),
                        ));
                    }
                }
            }
        }

        UiSchemaValidationReport {
            schema_ref: self.schema_ref.clone(),
            diagnostics,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiSchemaField {
    pub shape: UiSchemaShape,
    pub required: bool,
    pub default: Option<UiSchemaValue>,
}

impl UiSchemaField {
    pub fn required(shape: UiSchemaShape) -> Self {
        Self {
            shape,
            required: true,
            default: None,
        }
    }

    pub fn optional(shape: UiSchemaShape) -> Self {
        Self {
            shape,
            required: false,
            default: None,
        }
    }

    pub fn with_default(mut self, default: UiSchemaValue) -> Self {
        self.default = Some(default);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiSchemaShape {
    Null,
    Bool,
    Integer,
    UnsignedInteger,
    Number,
    String,
    StringEnum(BTreeSet<String>),
    StableIdRef,
    RouteRef,
    OpaqueHostRef,
    List(Box<UiSchemaShape>),
    Object,
    Nullable(Box<UiSchemaShape>),
}

impl UiSchemaShape {
    pub fn list(element: UiSchemaShape) -> Self {
        Self::List(Box::new(element))
    }

    pub fn nullable(shape: UiSchemaShape) -> Self {
        Self::Nullable(Box::new(shape))
    }

    pub fn string_enum(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::StringEnum(values.into_iter().map(Into::into).collect())
    }

    pub fn allowed_string_values(&self) -> Option<&BTreeSet<String>> {
        match self {
            Self::StringEnum(values) => Some(values),
            Self::Nullable(inner) => inner.allowed_string_values(),
            _ => None,
        }
    }

    pub fn matches_value(&self, value: &UiSchemaValue) -> bool {
        match self {
            Self::Null => matches!(value, UiSchemaValue::Null),
            Self::Bool => matches!(value, UiSchemaValue::Bool(_)),
            Self::Integer => matches!(value, UiSchemaValue::Integer(_)),
            Self::UnsignedInteger => matches!(value, UiSchemaValue::UnsignedInteger(_)),
            Self::Number => matches!(value, UiSchemaValue::Number(number) if number.is_finite()),
            Self::String => matches!(value, UiSchemaValue::String(_)),
            Self::StringEnum(values) => {
                matches!(value, UiSchemaValue::String(value) if values.contains(value))
            }
            Self::StableIdRef => matches!(value, UiSchemaValue::StableIdRef(_)),
            Self::RouteRef => matches!(value, UiSchemaValue::RouteRef(_)),
            Self::OpaqueHostRef => matches!(value, UiSchemaValue::OpaqueHostRef(_)),
            Self::List(element_shape) => value.as_list().is_some_and(|values| {
                values
                    .iter()
                    .all(|entry| element_shape.matches_value(entry))
            }),
            Self::Object => matches!(value, UiSchemaValue::Object(_)),
            Self::Nullable(shape) => {
                matches!(value, UiSchemaValue::Null) || shape.matches_value(value)
            }
        }
    }

    pub fn expected_kind(&self) -> UiSchemaExpectedKind {
        match self {
            Self::Null => UiSchemaExpectedKind::Single(UiSchemaValueKind::Null),
            Self::Bool => UiSchemaExpectedKind::Single(UiSchemaValueKind::Bool),
            Self::Integer => UiSchemaExpectedKind::Single(UiSchemaValueKind::Integer),
            Self::UnsignedInteger => {
                UiSchemaExpectedKind::Single(UiSchemaValueKind::UnsignedInteger)
            }
            Self::Number => UiSchemaExpectedKind::Single(UiSchemaValueKind::Number),
            Self::String => UiSchemaExpectedKind::Single(UiSchemaValueKind::String),
            Self::StringEnum(_) => UiSchemaExpectedKind::Single(UiSchemaValueKind::String),
            Self::StableIdRef => UiSchemaExpectedKind::Single(UiSchemaValueKind::StableIdRef),
            Self::RouteRef => UiSchemaExpectedKind::Single(UiSchemaValueKind::RouteRef),
            Self::OpaqueHostRef => UiSchemaExpectedKind::Single(UiSchemaValueKind::OpaqueHostRef),
            Self::List(_) => UiSchemaExpectedKind::Single(UiSchemaValueKind::List),
            Self::Object => UiSchemaExpectedKind::Single(UiSchemaValueKind::Object),
            Self::Nullable(shape) => {
                UiSchemaExpectedKind::Nullable(Box::new(shape.expected_kind()))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiSchemaExpectedKind {
    Single(UiSchemaValueKind),
    Nullable(Box<UiSchemaExpectedKind>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiUnknownFieldPolicy {
    Reject,
    PreserveForDebugFixtures,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiSchemaDiagnosticId(String);

impl UiSchemaDiagnosticId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSchemaSourceMapRef {
    pub source_id: String,
    pub span: Option<UiSchemaSourceSpan>,
}

impl UiSchemaSourceMapRef {
    pub fn new(source_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: UiSchemaSourceSpan) -> Self {
        self.span = Some(span);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSchemaSourceSpan {
    pub start_byte: u32,
    pub end_byte: u32,
}

impl UiSchemaSourceSpan {
    pub const fn new(start_byte: u32, end_byte: u32) -> Self {
        assert!(start_byte <= end_byte);
        Self {
            start_byte,
            end_byte,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSchemaValidationDiagnostic {
    pub schema_ref: UiSchemaRef,
    pub diagnostic_id: UiSchemaDiagnosticId,
    pub message: String,
    pub field_path: Vec<String>,
    pub source_map: Option<UiSchemaSourceMapRef>,
}

impl UiSchemaValidationDiagnostic {
    pub fn new(
        schema_ref: UiSchemaRef,
        diagnostic_id: UiSchemaDiagnosticId,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_ref,
            diagnostic_id,
            message: message.into(),
            field_path: Vec::new(),
            source_map: None,
        }
    }

    pub fn for_field(
        schema_ref: UiSchemaRef,
        diagnostic_id: UiSchemaDiagnosticId,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_ref,
            diagnostic_id,
            message: message.into(),
            field_path: vec![field.into()],
            source_map: None,
        }
    }

    pub fn with_source_map(mut self, source_map: UiSchemaSourceMapRef) -> Self {
        self.source_map = Some(source_map);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSchemaValidationReport {
    pub schema_ref: UiSchemaRef,
    pub diagnostics: Vec<UiSchemaValidationDiagnostic>,
}

impl UiSchemaValidationReport {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}
