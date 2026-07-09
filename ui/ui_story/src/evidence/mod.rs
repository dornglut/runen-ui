//! Evidence records for the UI Story V2 proof model.
//!
//! Evidence is supplied by app-owned proof producers and attached to semantic
//! workflow nodes. This module does not execute producers or infer proof from
//! old flat stages.

mod expectation;
mod producer;
mod record;

pub use expectation::UiStoryDiagnosticExpectation;
pub use producer::UiStoryEvidenceProducer;
pub use record::{UiStoryEvidence, UiStoryEvidenceStatus};
