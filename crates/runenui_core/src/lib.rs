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
//! Mounted widget state cannot be forged by consumers:
//!
//! ```compile_fail
//! use runenui_core::__runtime::MountedWidgetState;
//! let _ = MountedWidgetState {
//!     value: Box::new(()),
//! };
//! ```
//!
//! Transient elements cannot execute mounted lifecycle or capabilities:
//!
//! ```compile_fail
//! use runenui_core::{View, text};
//! let element = text("temporary").into_element();
//! let _ = element.create_widget_state();
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

mod builtins;
mod computed_style;
mod element;
mod identity;
mod layout;
pub mod prelude;
mod style;
mod style_resolution;
mod style_tokens;
mod value;
mod widget_context;
mod widget_erasure;
mod widget_mapping;

include!("element_macros.rs");
include!("identity_macros.rs");
include!("token_macros.rs");

pub use builtins::{Button, Container, Text, button, column, container, row, text};
pub use computed_style::ComputedStyle;
pub use element::{
    AuthoringDiagnostic, ChildLayout, ChildLayoutWidget, Element, View, Views, Widget,
    WidgetActivation, WidgetDiagnostic, WidgetMeasure, WidgetPaintProof, WidgetSemanticProof,
    WidgetStateTypeId, WidgetTextKind, WidgetTypeId,
};
/// Unstable safe bridge from transient core elements to the mounted runtime.
///
/// This namespace is public only because core and runtime are separate Rust
/// crates. It is doc-hidden, outside the prelude, unsupported for application
/// use, semver-exempt before 1.0, and may change without compatibility support.
#[doc(hidden)]
pub mod __runtime {
    pub use crate::widget_erasure::{
        ElementParts, ElementRuntimeParts, MountedWidget, MountedWidgetState, WidgetBridgeError,
    };
}
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
pub use widget_context::{
    WidgetActivationContext, WidgetInvalidation, WidgetMountContext, WidgetUnmountContext,
    WidgetUnmountReason, WidgetUpdateContext,
};
