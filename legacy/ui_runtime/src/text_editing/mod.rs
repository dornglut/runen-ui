//! Renderer-neutral text editing runtime proof.
//!
//! Text-editing runtime consumes package-backed editable-text declarations and
//! normalized input facts. It emits caret, selection, composition, edit-intent,
//! suppression, proof-frame, and no-bypass evidence without owning product
//! buffers, editor commands, authored UI edits, clipboard providers, or undo
//! stacks.

mod fixture;
mod proof_frame;
mod replay;
mod report;

pub use fixture::*;
pub use proof_frame::*;
pub use replay::replay_text_editing;
pub use report::*;

pub const BASE_CONTROLS_TEXT_EDITING_PROOF_ID: &str = "base-controls.text-editing.proof";
