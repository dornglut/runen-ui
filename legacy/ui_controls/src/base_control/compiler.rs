//! File: domain/ui/ui_controls/src/base_control/compiler.rs
//! Crate: ui_controls

use crate::{
    ControlAccessibilityDescriptor, ControlCatalogIndex, ControlCatalogMetadata,
    ControlInputDescriptor, ControlModuleDescriptor, ControlPackageAuthoringBuilder,
    ControlPackageVersion, ControlRenderDescriptor, ControlStateDescriptor,
    ControlTargetProfileRef, ControlThemeDescriptor, RUNENWERK_CONTROL_PACKAGE_ID,
    RUNENWERK_CONTROL_TARGET_EDITOR,
};

use super::lowering::{
    accessibility, generic_text_support, input, inspection, interaction, layering_support, layout,
    module, render, state, surface2d_support, text_editing_support, theme,
};
use super::{
    CompiledControl, CompiledControlPackage, ControlContribution, ControlInspection, UiControls,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlCompiler;

impl ControlCompiler {
    pub const fn new() -> Self {
        Self
    }

    pub fn compile(&self, controls: &UiControls) -> CompiledControlPackage {
        let lowered = controls
            .contributions()
            .iter()
            .map(|contribution| self.lower_contribution(contribution))
            .collect::<Vec<_>>();

        let mut package_builder =
            ControlPackageAuthoringBuilder::new(RUNENWERK_CONTROL_PACKAGE_ID, ControlPackageVersion::new(1))
                .with_display_name("Runenwerk base UI controls")
                .with_description("Reusable descriptor package for Runenwerk base controls. Runtime mount eligibility remains disabled until story, render, and budget evidence are attached.")
                .with_category("base-controls")
                .with_tag("control-package")
                .with_target_profile(ControlTargetProfileRef::new(RUNENWERK_CONTROL_TARGET_EDITOR))
                .with_catalog_metadata(ControlCatalogMetadata::new(RUNENWERK_CONTROL_PACKAGE_ID, "Base Controls"));

        for control in &lowered {
            package_builder = package_builder
                .with_module(control.module.clone())
                .with_interaction_descriptor(control.interaction.clone());
        }

        let mut package = package_builder.build();
        for control in &lowered {
            package = package.with_overlay_descriptor(layering_support::lower_layering_support(
                control.contribution.def(),
                control.module.kind.control_kind_id.clone(),
            ));
            if let Some(descriptor) = text_editing_support::lower_text_editing_support(
                control.contribution.def(),
                control.module.kind.control_kind_id.clone(),
            ) {
                package = package.with_editable_text_descriptor(descriptor);
            }
            if let Some(descriptor) = generic_text_support::lower_generic_text_support(
                control.contribution.def(),
                control.module.kind.control_kind_id.clone(),
            ) {
                package = package.with_generic_text_descriptor(descriptor);
            }
            if let Some(descriptor) = surface2d_support::lower_surface2d_descriptor(
                control.contribution.def(),
                control.module.kind.control_kind_id.clone(),
            ) {
                package = package.with_surface2d_descriptor(descriptor);
            }
        }
        let controls = lowered
            .into_iter()
            .map(|control| {
                let inspection = inspection::lower_inspection(&package, &control);
                let interaction = package
                    .interaction_descriptor(&control.module.kind.control_kind_id)
                    .cloned()
                    .expect("compiled base controls must carry package interaction descriptors");
                CompiledControl {
                    contribution: control.contribution,
                    module: control.module,
                    layout: control.layout,
                    render: control.render,
                    input: control.input,
                    interaction,
                    state: control.state,
                    theme: control.theme,
                    accessibility: control.accessibility,
                    inspection,
                }
            })
            .collect::<Vec<_>>();
        let catalog = ControlCatalogIndex::from_packages([&package]);
        let inspection = ControlInspection {
            controls: controls
                .iter()
                .map(|control| control.inspection.clone())
                .collect(),
        };

        CompiledControlPackage {
            package,
            controls,
            catalog,
            inspection,
        }
    }

    pub fn compile_module(&self, contribution: &ControlContribution) -> ControlModuleDescriptor {
        module::lower_module(contribution.def())
    }

    fn lower_contribution(&self, contribution: &ControlContribution) -> LoweredControl {
        let module = module::lower_module(contribution.def());
        let kind_id = module.kind.control_kind_id.clone();
        LoweredControl {
            contribution: contribution.clone(),
            module,
            layout: layout::lower_layout(contribution.def(), kind_id.clone()),
            render: render::lower_render(contribution.def(), kind_id.clone()),
            input: input::lower_input(contribution.def(), kind_id.clone()),
            interaction: interaction::lower_interaction(contribution.def(), kind_id.clone()),
            state: state::lower_state(contribution.def(), kind_id.clone()),
            theme: theme::lower_theme(contribution.def(), kind_id.clone()),
            accessibility: accessibility::lower_accessibility(contribution.def(), kind_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LoweredControl {
    pub(crate) contribution: ControlContribution,
    pub(crate) module: ControlModuleDescriptor,
    pub(crate) layout: crate::ControlLayoutDescriptor,
    pub(crate) render: ControlRenderDescriptor,
    pub(crate) input: ControlInputDescriptor,
    pub(crate) interaction: crate::ControlInteractionDescriptor,
    pub(crate) state: ControlStateDescriptor,
    pub(crate) theme: ControlThemeDescriptor,
    pub(crate) accessibility: ControlAccessibilityDescriptor,
}
