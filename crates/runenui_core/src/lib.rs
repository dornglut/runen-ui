//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.

#![forbid(unsafe_code)]

mod computed_style;
mod element;
mod identity;
mod layout;
pub mod prelude;
mod style;
mod style_resolution;

include!("element_macros.rs");

pub use computed_style::ComputedStyle;
pub use element::{
    ButtonArgs, ButtonElement, ContainerArgs, ContainerElement, Element, ElementKind, IntoElements,
    TextArgs, TextElement, button, button_with, column, container_with, row, text, text_with,
};
pub use identity::{ElementId, ElementKey};
pub use layout::{Axis, LayoutStyle, Px};
pub use style::{
    Color, ColorToken, ColorValue, EdgeInsets, Length, LengthToken, LengthValue, Radius,
    RadiusToken, RadiusValue, Spacing, SpacingToken, SpacingValue, StyleIntent, TokenId,
};
pub use style_resolution::{StyleResolution, UnresolvedStyleToken, resolve_literal_style};
