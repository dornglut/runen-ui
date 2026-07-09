use ui_layout::{
    UiContainerKind, UiContentState, UiItemIdentityRequirement, UiLargeContentBudget,
    UiLayoutDiagnostic, UiLayoutDiagnosticKind, UiLayoutRole, UiScrollRequirement,
    UiSelectionIdentityRequirement, UiSizeConstraintKind, UiVirtualizationRequirement,
};

#[test]
fn layout_roles_cover_containers_and_large_content() {
    assert_eq!(UiLayoutRole::Panel.as_str(), "panel");
    assert_eq!(UiLayoutRole::Row.as_str(), "row");
    assert_eq!(UiLayoutRole::Column.as_str(), "column");
    assert_eq!(UiLayoutRole::Stack.as_str(), "stack");
    assert_eq!(UiLayoutRole::Split.as_str(), "split");
    assert_eq!(UiLayoutRole::Scroll.as_str(), "scroll");
    assert_eq!(UiLayoutRole::List.as_str(), "list");
    assert_eq!(UiLayoutRole::Table.as_str(), "table");
    assert_eq!(UiLayoutRole::Tree.as_str(), "tree");
    assert_eq!(UiLayoutRole::VirtualList.as_str(), "virtual-list");
    assert_eq!(UiLayoutRole::VirtualTable.as_str(), "virtual-table");
}

#[test]
fn container_size_scroll_and_content_facts_are_named() {
    assert_eq!(UiContainerKind::Panel.as_str(), "panel");
    assert_eq!(UiContainerKind::Viewport.as_str(), "viewport");
    assert_eq!(UiContainerKind::SplitPane.as_str(), "split-pane");
    assert_eq!(UiContainerKind::ScrollRegion.as_str(), "scroll-region");
    assert_eq!(UiSizeConstraintKind::MinSize.as_str(), "min-size");
    assert_eq!(UiSizeConstraintKind::MaxSize.as_str(), "max-size");
    assert_eq!(
        UiSizeConstraintKind::PreferredSize.as_str(),
        "preferred-size"
    );
    assert_eq!(UiSizeConstraintKind::FillWidth.as_str(), "fill-width");
    assert_eq!(UiSizeConstraintKind::FillHeight.as_str(), "fill-height");
    assert_eq!(
        UiSizeConstraintKind::IntrinsicSize.as_str(),
        "intrinsic-size"
    );
    assert_eq!(UiScrollRequirement::Scrollable.as_str(), "scrollable");
    assert_eq!(UiScrollRequirement::ScrollOwner.as_str(), "scroll-owner");
    assert_eq!(UiScrollRequirement::AxisX.as_str(), "scroll-axis-x");
    assert_eq!(UiScrollRequirement::AxisY.as_str(), "scroll-axis-y");
    assert_eq!(UiContentState::Empty.as_str(), "empty");
    assert_eq!(UiContentState::Ready.as_str(), "ready");
}

#[test]
fn identity_budget_and_virtualization_are_metadata_only() {
    let item = UiItemIdentityRequirement::new("row.id");
    let selection = UiSelectionIdentityRequirement::new("row.selection");
    let budget = UiLargeContentBudget::new("table.large-content")
        .with_estimated_item_count(10_000)
        .with_overscan_budget_items(64);
    let diagnostic = UiLayoutDiagnostic::new(
        "table.layout.item-identity",
        UiLayoutDiagnosticKind::MissingItemIdentity,
        "item identity is required",
    );

    assert_eq!(
        UiVirtualizationRequirement::Ready.as_str(),
        "virtualization-ready"
    );
    assert_eq!(
        UiVirtualizationRequirement::EstimatedItemSize.as_str(),
        "estimated-item-size"
    );
    assert_eq!(
        UiVirtualizationRequirement::StableItemIdentity.as_str(),
        "stable-item-identity"
    );
    assert_eq!(
        UiVirtualizationRequirement::WindowedRendering.as_str(),
        "windowed-rendering"
    );
    assert_eq!(
        UiVirtualizationRequirement::OverscanBudget.as_str(),
        "overscan-budget"
    );
    assert_eq!(item.identity_id, "row.id");
    assert!(item.required);
    assert_eq!(selection.identity_id, "row.selection");
    assert!(selection.required);
    assert_eq!(budget.estimated_item_count, Some(10_000));
    assert_eq!(budget.overscan_budget_items, Some(64));
    assert_eq!(diagnostic.kind.as_str(), "missing-item-identity");
}
