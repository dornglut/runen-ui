//! File: domain/ui/ui_program_lowering/src/report.rs
//! Crate: ui_program_lowering
//!
//! Formation report contract for semantic authored-UI lowering.

use ui_program::{UiProgram, UiProgramDiagnostic};

use crate::catalog::UiProgramFormationCatalogReport;

#[derive(Clone, Debug, PartialEq)]
pub struct UiProgramFormationReport {
    pub program: UiProgram,
    pub diagnostics: Vec<UiProgramDiagnostic>,
    pub catalog_report: UiProgramFormationCatalogReport,
}

impl UiProgramFormationReport {
    pub fn from_program_and_catalog_report(
        program: UiProgram,
        catalog_report: UiProgramFormationCatalogReport,
    ) -> Self {
        let mut diagnostics = program.diagnostics.clone();
        diagnostics.extend(catalog_report.diagnostics.iter().cloned());

        Self {
            program,
            diagnostics,
            catalog_report,
        }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty() && self.catalog_report.passed()
    }

    pub fn has_diagnostic(&self, code: &str) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code)
    }
}
