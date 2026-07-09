//! Compiler report contract and convenience accessors.

use ui_artifacts::{ArtifactCacheKey, CompiledSourceMap, UiRuntimeArtifact};

use crate::{CapabilityCheck, PackageResolution, UiGraphIntegrityReport};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiCompilerReport {
    pub artifact: UiRuntimeArtifact,
    pub package_resolution: PackageResolution,
    pub capability_checks: Vec<CapabilityCheck>,
    pub graph_integrity: UiGraphIntegrityReport,
}

impl UiCompilerReport {
    pub fn cache_key(&self) -> &ArtifactCacheKey {
        &self.artifact.manifest.cache_key
    }

    pub fn compiled_source_map(&self) -> &CompiledSourceMap {
        &self.artifact.manifest.source_map
    }

    pub fn passed(&self) -> bool {
        self.package_resolution.is_resolved()
            && self
                .capability_checks
                .iter()
                .all(CapabilityCheck::is_satisfied)
            && self.graph_integrity.passed()
    }
}
