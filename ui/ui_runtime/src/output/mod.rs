//! File: domain/ui/ui_runtime/src/output/mod.rs
//! Purpose: Build renderer-facing UI frame data from retained tree state.

pub mod build_ui_frame;
mod emit;
pub mod evidence;
mod interaction_visual;
mod layer;
mod primitives;
#[cfg(test)]
mod test_support;
mod text;
mod traversal;

pub use build_ui_frame::{InteractionVisualState, build_ui_frame};
pub use evidence::*;
