//! Core typed UI description types for `RunenUI`.
//!
//! This crate owns the public, host-neutral UI description model.
//! It must not depend on runtime, renderer, compiler, host, ECS, or legacy crates.

#![forbid(unsafe_code)]

mod element_id;
mod layout;
pub mod prelude;
mod tree;

pub use element_id::ElementId;
pub use layout::{Axis, LayoutStyle, Px};
pub use tree::{
    ButtonElement, ContainerElement, Element, ElementKind, IntoElements, TextElement, button,
    column, row, text,
};
