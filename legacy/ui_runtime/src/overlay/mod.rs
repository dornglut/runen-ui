//! Renderer-neutral overlay / popup / layering runtime proof.
//!
//! Overlay runtime consumes normalized input facts. It is not part of input
//! ownership. Runtime owns overlay intent, stack, placement, focus, dismissal,
//! report, proof-frame projection, and no-bypass evidence.

mod fixture;
mod layering;
mod placement;
mod proof_frame;
mod report;
mod stack;

pub use fixture::*;
pub use layering::replay_overlay_layering;
pub use proof_frame::*;
pub use report::*;
pub use stack::*;

pub const BASE_CONTROLS_OVERLAY_LAYERING_PROOF_ID: &str = "base-controls.overlay-layering.proof";
pub const BASE_CONTROLS_EXECUTABLE_OVERLAY_LAYERING_STORY_ID: &str =
    "base-controls.overlay-layering.story";
