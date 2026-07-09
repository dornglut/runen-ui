mod frame;
mod proof;
mod report;
mod transform;

pub use frame::{
    Surface2DProofRenderFrame, Surface2DProofRenderSummary, base_controls_surface2d_proof_frame,
    surface2d_report_to_frame,
};
pub use proof::base_controls_surface2d_report;
pub use report::{Surface2DBoundaryCounters, Surface2DProofReport};
pub use transform::Surface2DTransform;

pub const BASE_CONTROLS_SURFACE2D_PROOF_ID: &str = "base-controls.surface2d.proof";
