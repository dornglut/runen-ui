//! Headless runtime skeleton for `RunenUI`.
//!
//! This crate will own input dispatch, typed action delivery, update calls,
//! layout orchestration, accessibility extraction, tracing, and surface-frame
//! publication.

#![forbid(unsafe_code)]

pub mod prelude;

/// Marker type for the future runtime.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Runtime;
