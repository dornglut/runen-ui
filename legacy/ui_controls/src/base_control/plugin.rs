//! File: domain/ui/ui_controls/src/base_control/plugin.rs
//! Crate: ui_controls

use crate::{ControlCatalogIndex, ControlPackageDescriptor};

use super::{CompiledControlPackage, ControlCompiler, ControlInspection, UiControls};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BaseControlsPlugin;

impl BaseControlsPlugin {
    pub const fn new() -> Self {
        Self
    }

    pub fn contribute(&self, controls: &mut UiControls) {
        controls.add(crate::label::control_contribution());
        controls.add(crate::button::control_contribution());
        controls.add(crate::inspector_field::control_contribution());
        controls.add(crate::color_picker::control_contribution());
        controls.add(crate::action_prompt::control_contribution());
        controls.add(crate::list_view::control_contribution());
        controls.add(crate::tree_view::control_contribution());
        controls.add(crate::table_view::control_contribution());
        controls.add(crate::surface2d::control_contribution());
    }

    pub fn extension(&self) -> UiControls {
        let mut controls = UiControls::new();
        self.contribute(&mut controls);
        controls
    }

    pub fn compile(&self) -> CompiledControlPackage {
        ControlCompiler::new().compile(&self.extension())
    }

    pub fn package(&self) -> ControlPackageDescriptor {
        self.compile().package
    }

    pub fn catalog(&self) -> ControlCatalogIndex {
        self.compile().catalog
    }

    pub fn inspection(&self) -> ControlInspection {
        self.compile().inspection
    }
}
