//! File: domain/ui/ui_compiler/src/lib.rs
//! Crate: ui_compiler

mod capability;
mod compiler;
mod graph_integrity;
mod package_resolution;
mod report;

pub use capability::{CapabilityCheck, CapabilityCheckStatus, CapabilityCheckSubject};
pub use compiler::UiCompiler;
pub use graph_integrity::{
    GraphIntegrityCheck, GraphIntegrityStatus, GraphIntegritySubject, UiGraphIntegrityReport,
};
pub use package_resolution::{
    KernelConsumer, PackageResolution, ResolvedPackage, UnresolvedControlKind, UnresolvedKernel,
};
pub use report::UiCompilerReport;
pub use ui_artifacts::{ArtifactCacheKey, CompiledSourceMap};

#[cfg(test)]
mod tests;
