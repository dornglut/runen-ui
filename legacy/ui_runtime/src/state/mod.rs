//! File: domain/ui/ui_runtime/src/state/mod.rs
//! Purpose: Persistent runtime interaction state.

pub mod runtime_state;

pub use runtime_state::{
    ScrollOffset, ScrollbarAxisActivityFrames, ScrollbarAxisOpacities, ScrollbarAxisTarget,
    ScrollbarThumbDragState, UiRuntimeState,
};
