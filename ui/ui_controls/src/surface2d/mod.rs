//! Renderer-neutral Surface2D declarations for reusable control packages.
//!
//! This module describes reusable 2D coordinate and navigation surface support.
//! It does not own app-specific canvas semantics, renderer backend resources,
//! product mutation, or host command execution.

mod contribution;
mod descriptor;
mod ids;
mod support;

pub use contribution::{control_contribution, control_module};
pub use descriptor::{
    ControlSurface2DDescriptor, ControlSurface2DInspectionFact, ControlSurface2DSupportSummary,
};
pub use ids::SURFACE2D_CONTROL_KIND_ID;
pub use support::{
    ControlSurface2DAccessibilitySupport, ControlSurface2DBudgetEvidenceKind,
    ControlSurface2DInputMode, ControlSurface2DInteractionSupport, ControlSurface2DLayerKind,
};
