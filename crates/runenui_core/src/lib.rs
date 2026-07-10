//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.

#![forbid(unsafe_code)]

mod element_id;
mod element_key;
mod layout;
pub mod prelude;
mod tree;

include!("element_macro.rs");

pub use element_id::ElementId;
pub use element_key::ElementKey;
pub use layout::{Axis, LayoutStyle, Px};
pub use tree::{
    ButtonArgs, ButtonElement, ContainerArgs, ContainerElement, Element, ElementKind, IntoElements,
    TextArgs, TextElement, button, button_with, column, container_with, row, text, text_with,
};
