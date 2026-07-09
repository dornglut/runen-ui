use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    AdaptiveProposalExpectationId, CapabilityId, CompositionDefinitionV1,
    CompositionDiagnosticCode, CompositionDiagnosticRecord, CompositionDiagnosticStage,
    CompositionDiagnosticSubject, CompositionFixtureId, CompositionRejection, CompositionState,
    ContentLiveness, HostProfileId, MountedContentRef, MountedUnitId, TargetProfileId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedCompositionValidity {
    Valid,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionFixture {
    pub id: CompositionFixtureId,
    pub host_profile: HostProfileId,
    pub target_profiles: BTreeSet<TargetProfileId>,
    pub definition: CompositionDefinitionV1,
    pub mounted_content: BTreeMap<MountedUnitId, MountedContentRef>,
    pub liveness: BTreeMap<MountedUnitId, ContentLiveness>,
    pub expected_validity: ExpectedCompositionValidity,
    pub expected_capabilities: BTreeSet<CapabilityId>,
    pub expected_diagnostics: BTreeSet<CompositionDiagnosticCode>,
    pub expected_adaptive_proposals: BTreeSet<AdaptiveProposalExpectationId>,
    pub forbidden_imports: BTreeSet<String>,
    pub forbidden_product_behaviors: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionFixtureRun {
    pub fixture_id: CompositionFixtureId,
    pub state: Option<CompositionState>,
    pub rejection: Option<CompositionRejection>,
    pub expectation_failures: Vec<CompositionDiagnosticRecord>,
}

impl CompositionFixtureRun {
    pub fn passed(&self) -> bool {
        self.expectation_failures.is_empty()
    }
}

impl CompositionFixture {
    pub fn run(&self) -> CompositionFixtureRun {
        let result = CompositionState::form(self.definition.clone());
        let (state, rejection) = match result {
            Ok(state) => (Some(state), None),
            Err(rejection) => (None, Some(rejection)),
        };
        let mut failures = Vec::new();
        let actual_valid = state.is_some();
        let expected_valid = matches!(self.expected_validity, ExpectedCompositionValidity::Valid);
        if actual_valid != expected_valid {
            failures.push(self.failure(
                "Fixture validity differs from the declared expectation; update the fixture or the core invariant.",
            ));
        }
        let actual_codes = rejection
            .as_ref()
            .map(|value| {
                value
                    .diagnostics()
                    .iter()
                    .map(|record| record.code())
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        if self.expected_diagnostics != actual_codes {
            failures.push(
                self.failure("Fixture diagnostics differ from the exact declared diagnostic set."),
            );
        }
        let definition_refs = self
            .definition
            .mounted_units()
            .iter()
            .map(|unit| (unit.id, unit.content().clone()))
            .collect::<BTreeMap<_, _>>();
        if definition_refs != self.mounted_content {
            failures.push(
                self.failure("Align mounted-content declarations with the composition definition."),
            );
        }
        if self.liveness.keys().collect::<BTreeSet<_>>()
            != self.mounted_content.keys().collect::<BTreeSet<_>>()
        {
            failures.push(self.failure(
                "Declare exactly one liveness observation for every mounted-content entry.",
            ));
        }
        let actual_capabilities = self
            .definition
            .mounted_units()
            .iter()
            .flat_map(|unit| unit.capabilities().iter().cloned())
            .collect::<BTreeSet<_>>();
        if self.expected_capabilities != actual_capabilities {
            failures.push(self.failure(
                "Fixture capabilities differ from the exact capabilities declared by mounted units.",
            ));
        }
        let actual_target_profiles = self
            .definition
            .targets()
            .iter()
            .map(|target| target.profile.clone())
            .collect::<BTreeSet<_>>();
        if self.target_profiles != actual_target_profiles {
            failures.push(self.failure(
                "Fixture target profiles differ from the exact profiles declared by targets.",
            ));
        }
        if self.forbidden_imports.is_empty() || self.forbidden_product_behaviors.is_empty() {
            failures.push(self.failure(
                "Declare forbidden imports and forbidden product behaviors for this neutral fixture.",
            ));
        }
        CompositionFixtureRun {
            fixture_id: self.id,
            state,
            rejection,
            expectation_failures: failures,
        }
    }

    fn failure(&self, message: &'static str) -> CompositionDiagnosticRecord {
        CompositionDiagnosticRecord::error(
            CompositionDiagnosticCode::FixtureExpectationFailed,
            CompositionDiagnosticStage::Fixture,
            CompositionDiagnosticSubject::Fixture(self.id),
            message,
        )
    }
}
