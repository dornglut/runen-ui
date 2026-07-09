//! Generic reusable interaction proof for `ui_runtime`.
//!
//! This module resolves normalized `ui_input` facts against mounted controls
//! that carry package-backed `ui_controls` interaction descriptors. It forms
//! replay reports and visible proof models that later adapters can project into
//! static render evidence.
//!
//! It deliberately does not own OS/window input collection, app/editor/game
//! command execution, product mutation, overlay/layering behavior, or full text
//! editing.

mod boundary;
mod fixture;
mod formatting;
mod inspector;
mod replay;
mod report;
mod state_mapping;
mod visual;

pub use boundary::*;
pub use fixture::*;
pub use inspector::*;
pub use replay::*;
pub use report::*;
pub use visual::*;
