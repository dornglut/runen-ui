//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.

#![forbid(unsafe_code)]

mod element;
mod identity;
mod layout;
pub mod prelude;
mod style;

include!("element_macros.rs");

pub use element::{
    ButtonArgs, ButtonElement, ContainerArgs, ContainerElement, Element, ElementKind, IntoElements,
    TextArgs, TextElement, button, button_with, column, container_with, row, text, text_with,
};
pub use identity::{ElementId, ElementKey};
pub use layout::{Axis, LayoutStyle, Px};
pub use style::{Color, EdgeInsets, Length, Radius, Spacing};
