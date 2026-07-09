//! Compiler entry point for UiProgram artifact lowering.

use ui_artifacts::UiRuntimeArtifact;
use ui_program::UiProgram;

use crate::{CapabilityCheck, PackageResolution, UiCompilerReport, UiGraphIntegrityReport};

#[derive(Clone, Debug, Default)]
pub struct UiCompiler;

impl UiCompiler {
    pub fn compile(&self, program: &UiProgram) -> UiRuntimeArtifact {
        self.compile_report(program).artifact
    }

    pub fn compile_report(&self, program: &UiProgram) -> UiCompilerReport {
        let package_resolution = PackageResolution::resolve(program);
        let capability_checks = CapabilityCheck::from_program(program);
        let graph_integrity = UiGraphIntegrityReport::from_program(program);
        let mut artifact = UiRuntimeArtifact::from_program(program);

        for diagnostic in package_resolution.diagnostics() {
            artifact.manifest.push_diagnostic(diagnostic);
        }
        for diagnostic in capability_checks
            .iter()
            .filter_map(CapabilityCheck::diagnostic)
        {
            artifact.manifest.push_diagnostic(diagnostic);
        }
        for diagnostic in graph_integrity.diagnostics() {
            artifact.manifest.push_diagnostic(diagnostic);
        }

        UiCompilerReport {
            artifact,
            package_resolution,
            capability_checks,
            graph_integrity,
        }
    }
}
