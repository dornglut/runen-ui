//! Durable UiProgram contract.

use serde::{Deserialize, Serialize};

use crate::diagnostics::UiProgramDiagnostic;
use crate::graphs::UiProgramGraphs;
use crate::ids::UiProgramId;
use crate::source_map::{UiProgramSource, UiProgramSourceMapEntry};
use crate::version::UiProgramVersion;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq)]
pub struct UiProgram {
    pub id: UiProgramId,
    pub version: UiProgramVersion,
    #[serde(default)]
    pub sources: Vec<UiProgramSource>,
    pub graphs: UiProgramGraphs,
    pub source_map: Vec<UiProgramSourceMapEntry>,
    pub diagnostics: Vec<UiProgramDiagnostic>,
}

impl UiProgram {
    pub fn new(id: UiProgramId, version: UiProgramVersion) -> Self {
        Self {
            id,
            version,
            sources: Vec::new(),
            graphs: UiProgramGraphs::default(),
            source_map: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: UiProgramSource) -> Self {
        self.sources.push(source);
        self
    }

    pub fn with_source_map_entry(mut self, entry: UiProgramSourceMapEntry) -> Self {
        self.source_map.push(entry);
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: UiProgramDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }
}
