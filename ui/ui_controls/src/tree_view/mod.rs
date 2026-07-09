//! File: domain/ui/ui_controls/src/tree_view/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlVisualState,
};

pub const TREE_VIEW_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.tree-view";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "tree-view",
        "TreeView",
        ControlPreset::TreeView,
        RouteCapability::new("runenwerk.ui.controls.tree.navigate"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("roots", UiSchemaShape::list(UiSchemaShape::Object)),
        ControlField::optional("expand_route", UiSchemaShape::RouteRef),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional(
            "expanded_ids",
            UiSchemaShape::list(UiSchemaShape::StableIdRef),
        ),
        ControlField::optional("selected_id", UiSchemaShape::StableIdRef),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("route", UiSchemaShape::RouteRef),
        ControlField::required("node_id", UiSchemaShape::StableIdRef),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("tree-view")
            .with_style(ControlStyleRole::Background, "surface")
            .with_optional_visual_state(ControlVisualState::Focused)
            .with_optional_visual_state(ControlVisualState::Selected)
            .with_optional_visual_state(ControlVisualState::Loading),
    )
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}
