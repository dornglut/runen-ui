use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{
    UI_STORY_REGISTRY_DUPLICATE_STORY, UI_STORY_REGISTRY_INVALID_MANIFEST, UiStoryDiagnostic,
    UiStoryDiagnosticOrigin, UiStoryDiagnosticSubject,
};
use crate::identity::UiStoryId;
use crate::manifest_v2::UiStoryManifestV2;

use super::source::UiStoryManifestSourceV2;
use super::validated::ValidatedUiStoryRegistryV2;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryRegistryBuildReportV2 {
    pub diagnostics: Vec<UiStoryDiagnostic>,
    pub parsed_count: usize,
    pub accepted_count: usize,
    pub rejected_count: usize,
    pub duplicate_count: usize,
}

impl UiStoryRegistryBuildReportV2 {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty() && self.rejected_count == 0 && self.duplicate_count == 0
    }

    fn push_diagnostic(&mut self, diagnostic: UiStoryDiagnostic) {
        self.diagnostics.push(diagnostic);
    }

    fn rejected(&mut self) {
        self.rejected_count += 1;
    }

    fn accepted(&mut self) {
        self.accepted_count += 1;
    }

    fn parsed(&mut self) {
        self.parsed_count += 1;
    }

    fn duplicate(&mut self) {
        self.duplicate_count += 1;
        self.rejected();
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryRegistryBuilderV2 {
    sources: Vec<UiStoryManifestSourceV2>,
}

impl UiStoryRegistryBuilderV2 {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source(mut self, source: UiStoryManifestSourceV2) -> Self {
        self.sources.push(source);
        self
    }

    pub fn add_sources(
        mut self,
        sources: impl IntoIterator<Item = UiStoryManifestSourceV2>,
    ) -> Self {
        self.sources.extend(sources);
        self
    }

    pub fn build(self) -> Result<ValidatedUiStoryRegistryV2, UiStoryRegistryBuildReportV2> {
        let mut report = UiStoryRegistryBuildReportV2::default();
        let mut stories = BTreeMap::<UiStoryId, UiStoryManifestV2>::new();

        for source in self.sources {
            let manifest = match UiStoryManifestV2::from_ron_str(&source.contents) {
                Ok(manifest) => {
                    report.parsed();
                    manifest
                }
                Err(error) => {
                    report.rejected();
                    report.push_diagnostic(
                        UiStoryDiagnostic::error(
                            UI_STORY_REGISTRY_INVALID_MANIFEST,
                            UiStoryDiagnosticOrigin::Registry,
                            UiStoryDiagnosticSubject::ManifestSource(source.source_id.clone()),
                            error.message,
                        )
                        .with_context("manifest_source_id", source.source_id.as_str())
                        .with_context("manifest_source_path", source.path)
                        .with_context("manifest_error_code", error.code),
                    );
                    continue;
                }
            };

            let validation_diagnostics = manifest.validate();
            if !validation_diagnostics.is_empty() {
                report.rejected();
                for diagnostic in validation_diagnostics {
                    report.push_diagnostic(with_source_context(diagnostic, &source));
                }
                continue;
            }

            if stories.contains_key(&manifest.story_id) {
                report.duplicate();
                report.push_diagnostic(
                    UiStoryDiagnostic::error(
                        UI_STORY_REGISTRY_DUPLICATE_STORY,
                        UiStoryDiagnosticOrigin::Registry,
                        UiStoryDiagnosticSubject::Story(manifest.story_id.clone()),
                        format!(
                            "duplicate ui story id '{}' in manifest registry sources",
                            manifest.story_id.as_str()
                        ),
                    )
                    .with_context("story_id", manifest.story_id.as_str())
                    .with_context("manifest_source_id", source.source_id.as_str())
                    .with_context("manifest_source_path", source.path),
                );
                continue;
            }

            let story_id = manifest.story_id.clone();
            stories.insert(story_id, manifest);
            report.accepted();
        }

        if report.is_valid() {
            Ok(ValidatedUiStoryRegistryV2::new(stories))
        } else {
            Err(report)
        }
    }
}

fn with_source_context(
    diagnostic: UiStoryDiagnostic,
    source: &UiStoryManifestSourceV2,
) -> UiStoryDiagnostic {
    diagnostic
        .with_context("manifest_source_id", source.source_id.as_str())
        .with_context("manifest_source_path", source.path.as_str())
}
