use ui_controls::{ControlKindId, ControlLayoutDescriptor, LABEL_CONTROL_KIND_ID};
use ui_layout::{
    UiContainerKind, UiContentState, UiItemIdentityRequirement, UiLargeContentBudget,
    UiLayoutDiagnostic, UiLayoutDiagnosticKind, UiLayoutRole, UiScrollRequirement,
    UiSelectionIdentityRequirement, UiSizeConstraintKind, UiVirtualizationRequirement,
};

#[test]
fn control_layout_bridge_references_ui_layout_vocabulary() {
    let summary = label_layout_descriptor().summary();

    assert!(summary.layout_roles.contains(&"panel".to_owned()));
    assert!(summary.layout_roles.contains(&"virtual-list".to_owned()));
    assert!(
        summary
            .container_kinds
            .contains(&"scroll-region".to_owned())
    );
    assert!(
        summary
            .size_constraints
            .contains(&"preferred-size".to_owned())
    );
    assert!(
        summary
            .scroll_requirements
            .contains(&"scroll-owner".to_owned())
    );
    assert!(summary.content_states.contains(&"ready".to_owned()));
}

#[test]
fn control_layout_bridge_summarizes_identity_budget_and_virtualization() {
    let summary = label_layout_descriptor().summary();

    assert!(summary.item_identities.contains(&"row.id".to_owned()));
    assert!(
        summary
            .selection_identities
            .contains(&"row.selection".to_owned())
    );
    assert!(
        summary
            .virtualization_requirements
            .contains(&"windowed-rendering".to_owned())
    );
    assert!(
        summary
            .large_content_budgets
            .contains(&"table.large-content".to_owned())
    );
    assert!(
        summary
            .diagnostics
            .contains(&"missing-scroll-owner".to_owned())
    );
    assert!(
        summary
            .expected_failures
            .contains(&"table.layout.expected".to_owned())
    );
    assert!(!summary.has_runtime_layout_behavior);
}

#[test]
fn control_layout_bridge_exposes_read_only_inspection_facts() {
    let facts = label_layout_descriptor().summary().inspection_facts();

    assert!(
        facts
            .iter()
            .any(|fact| fact.key == "layout_roles" && fact.value.contains("panel"))
    );
    assert!(facts.iter().any(|fact| {
        fact.key == "virtualization_requirements" && fact.value.contains("stable-item-identity")
    }));
    assert!(
        facts
            .iter()
            .any(|fact| { fact.key == "has_runtime_layout_behavior" && fact.value == "false" })
    );
}

fn label_layout_descriptor() -> ControlLayoutDescriptor {
    ControlLayoutDescriptor::new(ControlKindId::new(LABEL_CONTROL_KIND_ID))
        .with_layout_role(UiLayoutRole::Panel)
        .with_layout_role(UiLayoutRole::VirtualList)
        .with_container_kind(UiContainerKind::ScrollRegion)
        .with_size_constraint(UiSizeConstraintKind::PreferredSize)
        .with_scroll_requirement(UiScrollRequirement::ScrollOwner)
        .with_content_state(UiContentState::Ready)
        .with_item_identity(UiItemIdentityRequirement::new("row.id"))
        .with_selection_identity(UiSelectionIdentityRequirement::new("row.selection"))
        .with_virtualization_requirement(UiVirtualizationRequirement::StableItemIdentity)
        .with_virtualization_requirement(UiVirtualizationRequirement::WindowedRendering)
        .with_large_content_budget(
            UiLargeContentBudget::new("table.large-content")
                .with_estimated_item_count(10_000)
                .with_overscan_budget_items(64),
        )
        .with_diagnostic(UiLayoutDiagnostic::new(
            "table.layout.scroll-owner",
            UiLayoutDiagnosticKind::MissingScrollOwner,
            "scroll owner is required",
        ))
        .with_diagnostic(UiLayoutDiagnostic::new(
            "table.layout.expected",
            UiLayoutDiagnosticKind::ExpectedFailure,
            "expected layout contract fixture",
        ))
}
