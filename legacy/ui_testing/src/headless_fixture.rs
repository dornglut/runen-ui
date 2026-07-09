//! Headless host fixture execution.

use serde::{Deserialize, Serialize};
use ui_accessibility::AccessibilityTree;
use ui_artifacts::{RuntimeTableKind, UiRuntimeArtifact};
use ui_binding::HostDataSnapshot;
use ui_compiler::{UiCompiler, UiCompilerReport};
use ui_evaluator::{UiEvaluationContext, UiEvaluator, UiOutput};
use ui_geometry::{GeometryPlan, GeometryViewport};
use ui_hosts::{
    DomainCommand, HeadlessHost, HostCommand, HostKind, HostRouteMapVersion, HostRouteMapping,
    HostSurfaceFacts,
};
use ui_program::{RouteCapability, RouteId, RouteSchemaVersion, UiProgram};
use ui_schema::UiSchemaValue;
use ui_state::UiStateModel;

use crate::program_fixture::headless_program;
use crate::{DiagnosticAssertion, ReproducibilityAssertion, SourceMapAssertion};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessFixture {
    pub fixture_id: String,
    pub program: UiProgram,
    pub host: HeadlessHost,
    pub surface_facts: HostSurfaceFacts,
    #[serde(default)]
    pub host_data: Vec<HostDataSnapshot>,
}

impl HeadlessFixture {
    pub fn label_inspector(fixture_id: impl Into<String>) -> Self {
        let fixture_id = fixture_id.into();
        let route_map_version = HostRouteMapVersion::new(1);
        let preview_capability = RouteCapability::new("headless.fixture.preview");
        let host = HeadlessHost::new(route_map_version).with_mapping(
            HostRouteMapping::new(
                RouteId::new("headless.fixture.preview"),
                RouteSchemaVersion::new(1),
                route_map_version,
                HostCommand::new(HostKind::Headless, "headless.fixture.preview"),
            )
            .with_capability(preview_capability.clone())
            .with_domain_command(DomainCommand::new("domain.ui", "domain.ui.fixture.preview")),
        );

        Self {
            fixture_id,
            program: headless_program(preview_capability),
            host,
            surface_facts: HostSurfaceFacts::headless("surface.fixture.headless"),
            host_data: vec![HostDataSnapshot::new(
                "host.fixture.title",
                UiSchemaValue::string("Inspector"),
                1,
            )],
        }
    }

    pub fn compile_report(&self) -> UiCompilerReport {
        UiCompiler.compile_report(&self.program)
    }

    pub fn compile(&self) -> UiRuntimeArtifact {
        self.compile_report().artifact
    }

    pub fn run(&self) -> HeadlessFixtureRun {
        let artifact = self.compile();
        let mut state = UiStateModel::default();
        let mut context = UiEvaluationContext::default();
        for host_data in self.host_data.iter().cloned() {
            context = context.with_host_data(host_data);
        }
        let output = UiEvaluator.evaluate_with_context(&artifact, &mut state, context);
        let accessibility = AccessibilityTree::from_artifact(&artifact);
        let geometry =
            GeometryPlan::from_artifact(&artifact, GeometryViewport::headless(320.0, 160.0));
        let source_map_assertion = SourceMapAssertion::target_in_table(
            "definition.fixture.title",
            "program.fixture.control.title",
            RuntimeTableKind::Control,
        );
        let diagnostic_assertion =
            DiagnosticAssertion::code_absent("ui.compiler.capability.missing_control_declaration");
        let reproducibility_assertion = ReproducibilityAssertion::from_fixture(self);

        HeadlessFixtureRun {
            artifact,
            output,
            state,
            accessibility,
            geometry,
            source_map_assertion,
            diagnostic_assertion,
            reproducibility_assertion,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessFixtureRun {
    pub artifact: UiRuntimeArtifact,
    pub output: UiOutput,
    pub state: UiStateModel,
    pub accessibility: AccessibilityTree,
    pub geometry: GeometryPlan,
    pub source_map_assertion: SourceMapAssertion,
    pub diagnostic_assertion: DiagnosticAssertion,
    pub reproducibility_assertion: ReproducibilityAssertion,
}

impl HeadlessFixtureRun {
    pub fn passed(&self) -> bool {
        self.source_map_assertion
            .assert_artifact(&self.artifact)
            .is_ok()
            && self
                .diagnostic_assertion
                .assert_artifact(&self.artifact)
                .is_ok()
            && self.reproducibility_assertion.passed()
            && self.accessibility.passed()
            && self.geometry.passed()
            && self.output.diagnostics.is_empty()
    }
}
