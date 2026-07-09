//! File: domain/ui/ui_controls/src/base_control/projection.rs
//! Crate: ui_controls

use crate::{
    ControlAccessibilityDescriptor, ControlInputDescriptor, ControlInspectionDescriptor,
    ControlInteractionDescriptor, ControlModuleDescriptor, ControlPackageDescriptor,
    ControlRenderDescriptor, ControlStateDescriptor, ControlThemeDescriptor,
};

use super::{ControlCatalog, ControlContribution};

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledControlPackage {
    pub package: ControlPackageDescriptor,
    pub controls: Vec<CompiledControl>,
    pub catalog: ControlCatalog,
    pub inspection: ControlInspection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledControl {
    pub contribution: ControlContribution,
    pub module: ControlModuleDescriptor,
    pub layout: crate::ControlLayoutDescriptor,
    pub render: ControlRenderDescriptor,
    pub input: ControlInputDescriptor,
    pub interaction: ControlInteractionDescriptor,
    pub state: ControlStateDescriptor,
    pub theme: ControlThemeDescriptor,
    pub accessibility: ControlAccessibilityDescriptor,
    pub inspection: ControlInspectionDescriptor,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlInspection {
    pub controls: Vec<ControlInspectionDescriptor>,
}

impl ControlInspection {
    pub fn descriptor(&self, control_kind_id: &str) -> Option<&ControlInspectionDescriptor> {
        self.controls
            .iter()
            .find(|descriptor| descriptor.control_kind_id == control_kind_id)
    }

    pub fn len(&self) -> usize {
        self.controls.len()
    }

    pub fn is_empty(&self) -> bool {
        self.controls.is_empty()
    }
}
