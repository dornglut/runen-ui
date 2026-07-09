//! File: domain/ui/ui_controls/src/base_control/lowering/layout.rs
//! Crate: ui_controls

use ui_layout::{
    UiContainerKind, UiContentState, UiItemIdentityRequirement, UiLargeContentBudget, UiLayoutRole,
    UiScrollRequirement, UiSelectionIdentityRequirement, UiSizeConstraintKind,
    UiVirtualizationRequirement,
};

use crate::ControlKindId;

use super::super::{ControlDef, ControlPreset};

pub(crate) fn lower_layout(
    def: &ControlDef,
    kind_id: ControlKindId,
) -> crate::ControlLayoutDescriptor {
    match def.preset() {
        ControlPreset::Label => add_content_state(
            add_size_constraints(
                add_layout_roles(
                    crate::ControlLayoutDescriptor::new(kind_id),
                    &[UiLayoutRole::Row],
                ),
                &[UiSizeConstraintKind::IntrinsicSize],
            ),
            &[UiContentState::Ready],
        ),
        ControlPreset::Button => base_surface_layout(
            kind_id,
            &[UiLayoutRole::Row, UiLayoutRole::Stack],
            &[
                UiSizeConstraintKind::MinSize,
                UiSizeConstraintKind::IntrinsicSize,
            ],
        ),
        ControlPreset::InspectorField => base_surface_layout(
            kind_id,
            &[UiLayoutRole::Row],
            &[
                UiSizeConstraintKind::FillWidth,
                UiSizeConstraintKind::IntrinsicSize,
            ],
        ),
        ControlPreset::ColorPicker => base_surface_layout(
            kind_id,
            &[UiLayoutRole::Panel, UiLayoutRole::Stack],
            &[
                UiSizeConstraintKind::MinSize,
                UiSizeConstraintKind::PreferredSize,
            ],
        ),
        ControlPreset::ActionPrompt => base_surface_layout(
            kind_id,
            &[UiLayoutRole::Panel, UiLayoutRole::Column],
            &[UiSizeConstraintKind::PreferredSize],
        ),
        ControlPreset::ListView => collection_layout(
            kind_id,
            &[UiLayoutRole::List, UiLayoutRole::VirtualList],
            "list-item-id",
            "selected-list-item-id",
            "list-view-large-content-budget",
            1_000,
            24,
        ),
        ControlPreset::TreeView => collection_layout(
            kind_id,
            &[UiLayoutRole::Tree],
            "tree-node-id",
            "selected-tree-node-id",
            "tree-view-large-content-budget",
            1_000,
            24,
        ),
        ControlPreset::TableView => add_scroll_requirements(
            collection_layout(
                kind_id,
                &[UiLayoutRole::Table, UiLayoutRole::VirtualTable],
                "table-row-id",
                "selected-table-row-id",
                "table-view-large-content-budget",
                10_000,
                48,
            ),
            &[UiScrollRequirement::AxisX],
        ),
        ControlPreset::Surface2D => surface2d_layout(kind_id),
    }
}

fn base_surface_layout(
    kind_id: ControlKindId,
    roles: &[UiLayoutRole],
    constraints: &[UiSizeConstraintKind],
) -> crate::ControlLayoutDescriptor {
    add_content_state(
        add_size_constraints(
            add_container_kinds(
                add_layout_roles(crate::ControlLayoutDescriptor::new(kind_id), roles),
                &[UiContainerKind::Group],
            ),
            constraints,
        ),
        &[UiContentState::Ready],
    )
}

fn collection_layout(
    kind_id: ControlKindId,
    roles: &[UiLayoutRole],
    item_identity: &str,
    selection_identity: &str,
    budget_id: &str,
    estimated_item_count: u32,
    overscan_budget_items: u32,
) -> crate::ControlLayoutDescriptor {
    add_virtualization_requirements(
        add_content_state(
            add_scroll_requirements(
                add_size_constraints(
                    add_container_kinds(
                        add_layout_roles(crate::ControlLayoutDescriptor::new(kind_id), roles)
                            .with_layout_role(UiLayoutRole::Scroll),
                        &[UiContainerKind::Collection, UiContainerKind::ScrollRegion],
                    ),
                    &[
                        UiSizeConstraintKind::FillWidth,
                        UiSizeConstraintKind::FillHeight,
                    ],
                ),
                &[
                    UiScrollRequirement::Scrollable,
                    UiScrollRequirement::ScrollOwner,
                    UiScrollRequirement::AxisY,
                    UiScrollRequirement::PositionHostOwned,
                ],
            ),
            &[
                UiContentState::Empty,
                UiContentState::Loading,
                UiContentState::Error,
                UiContentState::Overflow,
                UiContentState::Ready,
            ],
        )
        .with_item_identity(UiItemIdentityRequirement::new(item_identity))
        .with_selection_identity(UiSelectionIdentityRequirement::new(selection_identity))
        .with_large_content_budget(
            UiLargeContentBudget::new(budget_id)
                .with_estimated_item_count(estimated_item_count)
                .with_overscan_budget_items(overscan_budget_items),
        ),
        &[
            UiVirtualizationRequirement::Ready,
            UiVirtualizationRequirement::EstimatedItemSize,
            UiVirtualizationRequirement::StableItemIdentity,
            UiVirtualizationRequirement::WindowedRendering,
            UiVirtualizationRequirement::OverscanBudget,
        ],
    )
}

fn surface2d_layout(kind_id: ControlKindId) -> crate::ControlLayoutDescriptor {
    add_content_state(
        add_scroll_requirements(
            add_size_constraints(
                add_container_kinds(
                    add_layout_roles(
                        crate::ControlLayoutDescriptor::new(kind_id),
                        &[UiLayoutRole::Panel, UiLayoutRole::Scroll],
                    ),
                    &[UiContainerKind::Viewport, UiContainerKind::ScrollRegion],
                ),
                &[
                    UiSizeConstraintKind::FillWidth,
                    UiSizeConstraintKind::FillHeight,
                ],
            ),
            &[
                UiScrollRequirement::Scrollable,
                UiScrollRequirement::ScrollOwner,
                UiScrollRequirement::AxisX,
                UiScrollRequirement::AxisY,
                UiScrollRequirement::PositionHostOwned,
            ],
        )
        .with_large_content_budget(
            UiLargeContentBudget::new("surface2d-large-content-budget")
                .with_estimated_item_count(10_000)
                .with_overscan_budget_items(64),
        ),
        &[UiContentState::Ready, UiContentState::Overflow],
    )
}

fn add_layout_roles(
    mut descriptor: crate::ControlLayoutDescriptor,
    values: &[UiLayoutRole],
) -> crate::ControlLayoutDescriptor {
    for value in values {
        descriptor = descriptor.with_layout_role(*value);
    }
    descriptor
}

fn add_container_kinds(
    mut descriptor: crate::ControlLayoutDescriptor,
    values: &[UiContainerKind],
) -> crate::ControlLayoutDescriptor {
    for value in values {
        descriptor = descriptor.with_container_kind(*value);
    }
    descriptor
}

fn add_size_constraints(
    mut descriptor: crate::ControlLayoutDescriptor,
    values: &[UiSizeConstraintKind],
) -> crate::ControlLayoutDescriptor {
    for value in values {
        descriptor = descriptor.with_size_constraint(*value);
    }
    descriptor
}

fn add_scroll_requirements(
    mut descriptor: crate::ControlLayoutDescriptor,
    values: &[UiScrollRequirement],
) -> crate::ControlLayoutDescriptor {
    for value in values {
        descriptor = descriptor.with_scroll_requirement(*value);
    }
    descriptor
}

fn add_content_state(
    mut descriptor: crate::ControlLayoutDescriptor,
    values: &[UiContentState],
) -> crate::ControlLayoutDescriptor {
    for value in values {
        descriptor = descriptor.with_content_state(*value);
    }
    descriptor
}

fn add_virtualization_requirements(
    mut descriptor: crate::ControlLayoutDescriptor,
    values: &[UiVirtualizationRequirement],
) -> crate::ControlLayoutDescriptor {
    for value in values {
        descriptor = descriptor.with_virtualization_requirement(*value);
    }
    descriptor
}
