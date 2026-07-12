//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.
//!
//! Incompatible configuration is absent from erased [`Element`] values:
//!
//! ```compile_fail
//! use runenui_core::{View, text};
//! let _ = text("label").disabled().into_element();
//! ```
//!
//! Identifier literal macros apply the same Unicode-aware grammar as dynamic
//! constructors and reject invalid literals during compilation:
//!
//! ```compile_fail
//! let _ = runenui_core::element_id!("\u{00A0}");
//! ```
//!
//! ```compile_fail
//! let _ = runenui_core::element_key!("name\u{2003}");
//! ```
//!
//! ```compile_fail
//! let _ = runenui_core::token_id!("name\u{0085}value");
//! ```
//!
//! Widget erasure and opaque state payloads cannot be forged by consumers:
//!
//! ```compile_fail
//! use runenui_core::{WidgetState, WidgetStateTypeId, WidgetTypeId};
//! let _ = WidgetState {
//!     widget_type: WidgetTypeId::of::<()>(),
//!     state_type: WidgetStateTypeId::of::<()>(),
//!     value: Box::new(()),
//! };
//! ```
//!
//! Public built-in view builders cannot bypass their validated conversion path:
//!
//! ```compile_fail
//! use runenui_core::{Element, text};
//! let _ = Element::<()>::new(text("Title"));
//! ```
//!
//! ```compile_fail
//! use runenui_core::{Element, button};
//! let _ = Element::<()>::new(button::<()>("Save"));
//! ```
//!
//! A structurally childless widget cannot use the container authoring path:
//!
//! ```compile_fail
//! use runenui_core::{Widget, children, container, text};
//! #[derive(Debug)]
//! struct Leaf;
//! impl Widget<()> for Leaf {
//!     type State = ();
//!     fn create_state(&self) {}
//! }
//! let _ = container(Leaf, children![text("child")]);
//! ```
//!
//! Gap remains a child-bearing container property, not a generic element setter:
//!
//! ```compile_fail
//! use runenui_core::{View, text};
//! let _ = text("leaf").into_element().gap(4_u16);
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
    AuthoringDiagnostic, Button, ChildLayout, ChildLayoutWidget, Container, Element, Text, View,
    Views, Widget, WidgetActivation, WidgetDiagnostic, WidgetLifecycle, WidgetLifecycleContext,
    WidgetLifecycleRequest, WidgetMeasure, WidgetPaintProof, WidgetSemanticProof, WidgetState,
    WidgetStateMismatch, WidgetStateTypeId, WidgetTextKind, WidgetTypeId, button, column,
    container, row, text,
};
#[doc(hidden)]
pub use identity::is_valid_identifier_literal;
pub use identity::{ElementId, ElementKey, IdentifierError, IntoElementId, IntoElementKey};
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
