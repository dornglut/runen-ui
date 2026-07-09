//! Runtime-neutral preview fixtures, scenarios, target matrices, and evidence descriptors.

mod builders;
mod catalog;
mod controls;
mod routes;
mod surfaces;
mod validation;

#[cfg(test)]
mod tests;

use crate::identity::AuthoredId;

pub type UiPreviewFixtureId = AuthoredId;
pub type UiPreviewScenarioId = AuthoredId;
pub type UiPreviewMatrixId = AuthoredId;
pub type UiPreviewEvidenceId = AuthoredId;
pub type UiPreviewDataPackageId = AuthoredId;
pub type UiPreviewCapabilityId = AuthoredId;
pub type UiPreviewTargetProfileId = AuthoredId;
pub type UiPreviewSourcePackageId = AuthoredId;
pub type UiPreviewDiagnosticRef = AuthoredId;
pub type UiPreviewStateRef = AuthoredId;
pub type UiPreviewStepId = AuthoredId;

pub use builders::*;
pub use catalog::*;
pub use controls::*;
pub use routes::*;
pub use surfaces::*;
pub use validation::*;
