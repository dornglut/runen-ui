//! Deterministic report structures for the app-integration proof.

use serde::{Deserialize, Serialize};

use crate::bridge::UiAppRouteResolutionDiagnostic;
use crate::host::{UiAppHostMutationReport, UiAppHostSnapshot};
use crate::ids::{UiAppActionId, UiAppProofId, UiAppScreenId};
use crate::source::UiAppSourceBuildReport;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAppProofDiagnostic {
    SourceMissing {
        screen: String,
    },
    FormationFailed {
        screen: String,
        diagnostic_count: usize,
    },
    RouteMissing {
        route: String,
    },
    RouteRejected {
        diagnostic: UiAppRouteResolutionDiagnostic,
    },
    MutationMissing {
        action: String,
    },
    NextOutputMissing {
        screen: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppFormationReportSummary {
    pub screen_id: UiAppScreenId,
    pub passed: bool,
    pub diagnostics: usize,
    pub source_map_entries: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppRuntimeReportSummary {
    pub screen_id: UiAppScreenId,
    pub output_contains_text: Vec<String>,
    pub route_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppActionReport {
    pub action_id: UiAppActionId,
    pub route: String,
    pub resolved: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiAppSourceReport {
    pub source: UiAppSourceBuildReport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiAppIntegrationStepReport {
    pub step: String,
    pub before: UiAppHostSnapshot,
    pub source: UiAppSourceReport,
    pub formation: UiAppFormationReportSummary,
    pub runtime: UiAppRuntimeReportSummary,
    pub action: Option<UiAppActionReport>,
    pub mutation: Option<UiAppHostMutationReport>,
    pub after: UiAppHostSnapshot,
    #[serde(default)]
    pub diagnostics: Vec<UiAppProofDiagnostic>,
}

impl UiAppIntegrationStepReport {
    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty()
            && self.formation.passed
            && self.action.as_ref().is_none_or(|action| action.resolved)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiAppIntegrationReport {
    pub proof_id: UiAppProofId,
    pub initial: UiAppHostSnapshot,
    pub final_snapshot: UiAppHostSnapshot,
    pub steps: Vec<UiAppIntegrationStepReport>,
    #[serde(default)]
    pub diagnostics: Vec<UiAppProofDiagnostic>,
}

impl UiAppIntegrationReport {
    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty() && self.steps.iter().all(UiAppIntegrationStepReport::passed)
    }

    pub fn route_ids(&self) -> Vec<&str> {
        self.steps
            .iter()
            .flat_map(|step| step.runtime.route_ids.iter().map(String::as_str))
            .collect()
    }

    pub fn screen_sequence(&self) -> Vec<&str> {
        self.steps
            .iter()
            .map(|step| step.formation.screen_id.as_str())
            .collect()
    }
}
