//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.
//!
//! Incompatible configuration is absent from erased [`Element`] values:
//!
//! ```compile_fail
//! use runenui_core::{IntoElement, text};
//! let _ = text("label").disabled().into_element();
//! ```

#![forbid(unsafe_code)]

mod computed_style;
mod element;
mod identity;
mod layout;
pub mod prelude;
mod style;
mod style_resolution;
mod style_tokens;
mod value;

include!("element_macros.rs");
include!("identity_macros.rs");
include!("token_macros.rs");

pub use computed_style::ComputedStyle;
pub use element::{
    AuthoringDiagnostic, Button, ButtonElement, Container, ContainerElement, Element, ElementKind,
    IntoElement, IntoElements, Text, TextElement, button, column, row, text,
};
#[doc(hidden)]
pub use identity::is_valid_identifier_literal;
pub use identity::{ElementId, ElementKey, IdentifierError};
pub use layout::{Axis, LayoutStyle};
pub use style::{
    Color, ColorToken, ColorValue, EdgeInsets, Radius, RadiusToken, RadiusValue, SpacingToken,
    SpacingValue, StyleIntent, TokenId,
};
pub use style_resolution::{
    StyleFieldProvenance, StyleProvenance, StyleResolution, UnresolvedStyleToken,
    resolve_literal_style, resolve_style,
};
pub use style_tokens::{DuplicateTokenDefinition, StyleTokens, TokenFamily};
pub use value::{LogicalLength, LogicalLengthError};
