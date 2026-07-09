//! Control-package and kernel resolution for UiProgram compilation.

use std::collections::BTreeMap;

use ui_artifacts::{UiRuntimeArtifactDiagnostic, UiRuntimeArtifactDiagnosticSeverity};
use ui_program::{ControlKernelRef, UiProgram};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackageResolution {
    pub packages: Vec<ResolvedPackage>,
    pub unresolved_control_kinds: Vec<UnresolvedControlKind>,
    pub unresolved_kernels: Vec<UnresolvedKernel>,
}

impl PackageResolution {
    pub fn resolve(program: &UiProgram) -> Self {
        let mut packages = BTreeMap::<String, ResolvedPackage>::new();

        for node in &program.graphs.control.nodes {
            let package_id = node.package_id.as_str().to_owned();
            let package =
                packages
                    .entry(package_id.to_owned())
                    .or_insert_with(|| ResolvedPackage {
                        package_id,
                        ..ResolvedPackage::default()
                    });
            insert_unique(&mut package.control_node_ids, node.node_id.as_str());
            insert_unique(&mut package.control_kind_ids, node.control_kind.as_str());
        }

        let package_ids = packages.keys().map(String::to_owned).collect::<Vec<_>>();
        let mut unresolved_control_kinds = Vec::new();
        for node in &program.graphs.control.nodes {
            if !belongs_to_package(node.control_kind.as_str(), node.package_id.as_str()) {
                unresolved_control_kinds.push(UnresolvedControlKind {
                    control_node_id: node.node_id.as_str().to_owned(),
                    package_id: node.package_id.as_str().to_owned(),
                    control_kind_id: node.control_kind.as_str().to_owned(),
                });
            }
        }

        let mut unresolved_kernels = Vec::new();
        for constraint in &program.graphs.layout.constraints {
            if let Some(kernel) = constraint.layout_kernel.as_ref() {
                push_kernel_resolution(
                    &mut packages,
                    &mut unresolved_kernels,
                    &package_ids,
                    KernelConsumer::LayoutConstraint {
                        id: constraint.constraint_id.as_str().to_owned(),
                    },
                    kernel,
                );
            }
        }
        for operator in &program.graphs.visual.operators {
            push_kernel_resolution(
                &mut packages,
                &mut unresolved_kernels,
                &package_ids,
                KernelConsumer::VisualOperator {
                    id: operator.operator_id.as_str().to_owned(),
                },
                &operator.visual_kernel,
            );
        }

        Self {
            packages: packages.into_values().collect(),
            unresolved_control_kinds,
            unresolved_kernels,
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.unresolved_control_kinds.is_empty() && self.unresolved_kernels.is_empty()
    }

    pub fn diagnostics(&self) -> Vec<UiRuntimeArtifactDiagnostic> {
        let mut diagnostics = Vec::new();
        for unresolved in &self.unresolved_control_kinds {
            diagnostics.push(UiRuntimeArtifactDiagnostic {
                code: "ui.compiler.package.unresolved_control_kind".to_owned(),
                message: format!(
                    "control kind {} is not provided by package {} for control {}",
                    unresolved.control_kind_id, unresolved.package_id, unresolved.control_node_id
                ),
                severity: UiRuntimeArtifactDiagnosticSeverity::Error,
                source_map_index: None,
            });
        }
        for unresolved in &self.unresolved_kernels {
            diagnostics.push(UiRuntimeArtifactDiagnostic {
                code: "ui.compiler.package.unresolved_kernel".to_owned(),
                message: format!(
                    "kernel {} for {} is not provided by any resolved package",
                    unresolved.kernel_id,
                    unresolved.consumer.display_id()
                ),
                severity: UiRuntimeArtifactDiagnosticSeverity::Error,
                source_map_index: None,
            });
        }
        diagnostics
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolvedPackage {
    pub package_id: String,
    pub control_node_ids: Vec<String>,
    pub control_kind_ids: Vec<String>,
    pub kernel_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedControlKind {
    pub control_node_id: String,
    pub package_id: String,
    pub control_kind_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedKernel {
    pub kernel_id: String,
    pub consumer: KernelConsumer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KernelConsumer {
    LayoutConstraint { id: String },
    VisualOperator { id: String },
}

impl KernelConsumer {
    fn display_id(&self) -> &str {
        match self {
            Self::LayoutConstraint { id } | Self::VisualOperator { id } => id,
        }
    }
}

fn push_kernel_resolution(
    packages: &mut BTreeMap<String, ResolvedPackage>,
    unresolved_kernels: &mut Vec<UnresolvedKernel>,
    package_ids: &[String],
    consumer: KernelConsumer,
    kernel: &ControlKernelRef,
) {
    if let Some(package_id) = owning_package_id(package_ids, kernel.as_str()) {
        if let Some(package) = packages.get_mut(package_id) {
            insert_unique(&mut package.kernel_ids, kernel.as_str());
        }
    } else {
        unresolved_kernels.push(UnresolvedKernel {
            kernel_id: kernel.as_str().to_owned(),
            consumer,
        });
    }
}

fn belongs_to_package(namespaced_id: &str, package_id: &str) -> bool {
    namespaced_id == package_id
        || namespaced_id
            .strip_prefix(package_id)
            .is_some_and(|suffix| suffix.starts_with('.'))
}

fn owning_package_id<'a>(package_ids: &'a [String], namespaced_id: &str) -> Option<&'a str> {
    package_ids
        .iter()
        .map(String::as_str)
        .filter(|package_id| belongs_to_package(namespaced_id, package_id))
        .max_by_key(|package_id| package_id.len())
}

fn insert_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}
