//! File: domain/ui/ui_program_lowering/src/lib.rs
//! Crate: ui_program_lowering
//!
//! Semantic lowering from authored UI definitions and control package metadata
//! into typed UiProgram graphs.

pub mod catalog;
pub mod entrypoint;
pub(crate) mod lower;
pub mod report;
pub(crate) mod source_map;

pub use catalog::*;
pub use entrypoint::*;
pub use report::*;
