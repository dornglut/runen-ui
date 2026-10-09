use runenui_core::{
    Axis, Focusability, HitContributionContext, ListBoxSelectionMode, LogicalSize,
    SemanticCheckedState, SemanticContributionContext, SemanticOrientation, SemanticReference,
    SemanticRelationshipKind, SemanticRole, SemanticSelectionMode, View, WidgetAvailableSpace,
    WidgetMeasure, WidgetMeasureInput, button, checkbox, children, column, list_box, option_item,
    radio_button, radio_group, switch, tab, tab_list, tab_panel, text,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn typed_builders_use_the_open_widget_protocol() {
    let text_element: runenui_core::Element<Action> = text("Title").id("title").into_element();
    let button_element = button("Save")
        .id("save")
        .disabled()
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, text_widget, _) = text_element.into_runtime_parts().into_parts();
    let text_state = text_widget.create_state();
    assert!(matches!(
        text_widget.measure(
            &text_state,
            WidgetMeasureInput::new(
                None,
                None,
                WidgetAvailableSpace::MaxContent,
                WidgetAvailableSpace::MaxContent,
            ),
        ),
        Ok(WidgetMeasure::Text(_))
    ));
    let (_, _, _, _, _, _, _, _, button_widget, _) =
        button_element.into_runtime_parts().into_parts();
    let button_state = button_widget.create_state();
    let semantics = button_widget
        .semantics(&button_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(semantics.roots().len(), 1);
    let button_node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("button contributes one semantic node"));
    assert_eq!(button_node.role(), SemanticRole::Button);
    assert_eq!(button_node.name(), Some("Save"));
    assert!(button_node.state().disabled());

    let text = text("Title").id("title").into_element();
    let button = button("Save")
        .id("save")
        .disabled()
        .on_activate(|| Action::Save)
        .into_element();
    let container = column(children![text, button]).gap(8_u16).into_element();
    assert_eq!(container.children().len(), 2);
    assert!((container.layout().gap().horizontal().get() - 8.0).abs() <= f32::EPSILON);
    assert!((container.layout().gap().vertical().get() - 8.0).abs() <= f32::EPSILON);
    assert!(matches!(
        container.layout().container(),
        runenui_core::LayoutContainer::Flex(_)
    ));
    let (_, _, _, _, _, _, _, _, widget, _) = container.into_runtime_parts().into_parts();
    let state = widget.create_state();
    assert_eq!(
        widget
            .measure(
                &state,
                WidgetMeasureInput::new(
                    None,
                    None,
                    WidgetAvailableSpace::MaxContent,
                    WidgetAvailableSpace::MaxContent,
                ),
            )
            .unwrap_or_else(|_| unreachable!()),
        WidgetMeasure::default()
    );
}

#[test]
#[allow(clippy::assert_is_empty)]
fn binary_control_builders_use_the_open_widget_protocol() {
    assert_eq!(
        SemanticCheckedState::from(false),
        SemanticCheckedState::Unchecked
    );
    assert_eq!(
        SemanticCheckedState::from(true),
        SemanticCheckedState::Checked
    );

    let checkbox_element: runenui_core::Element<Action> =
        checkbox("Tri-state", SemanticCheckedState::Mixed)
            .id("tri-state")
            .disabled()
            .on_activate(|| Action::Save)
            .into_element();
    let (_, _, _, _, _, _, _, _, checkbox_widget, _) =
        checkbox_element.into_runtime_parts().into_parts();
    let checkbox_state = checkbox_widget.create_state();
    let checkbox_semantics = checkbox_widget
        .semantics(&checkbox_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("checkbox semantics are valid"));
    let checkbox_node = checkbox_semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("checkbox contributes one semantic node"));
    assert_eq!(checkbox_node.role(), SemanticRole::Checkbox);
    assert_eq!(checkbox_node.name(), Some("Tri-state"));
    assert_eq!(
        checkbox_node.state().checked(),
        Some(SemanticCheckedState::Mixed)
    );
    assert!(checkbox_node.state().disabled());

    let switch_element: runenui_core::Element<Action> = switch("Power", true)
        .id("power")
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, switch_widget, _) =
        switch_element.into_runtime_parts().into_parts();
    let switch_state = switch_widget.create_state();
    let switch_semantics = switch_widget
        .semantics(&switch_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("switch semantics are valid"));
    let switch_node = switch_semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("switch contributes one semantic node"));
    assert_eq!(switch_node.role(), SemanticRole::Switch);
    assert_eq!(switch_node.name(), Some("Power"));
    assert_eq!(
        switch_node.state().checked(),
        Some(SemanticCheckedState::Checked)
    );

    let passive_element: runenui_core::Element<Action> = checkbox("Passive", false).into_element();
    let (_, _, _, _, _, _, _, _, passive_widget, _) =
        passive_element.into_runtime_parts().into_parts();
    let passive_state = passive_widget.create_state();
    let passive_activation = passive_widget
        .activation(&passive_state)
        .unwrap_or_else(|_| unreachable!("passive checkbox activation is inspectable"));
    assert!(!passive_activation.is_actionable());
    assert!(
        passive_widget
            .hit_test(
                &passive_state,
                HitContributionContext::__runtime_new(LogicalSize::ZERO),
            )
            .unwrap_or_else(|_| unreachable!("passive checkbox hit contribution is inspectable"))
            .is_empty()
    );

    let actionable_element: runenui_core::Element<Action> = switch("Actionable", false)
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, actionable_widget, _) =
        actionable_element.into_runtime_parts().into_parts();
    let actionable_state = actionable_widget.create_state();
    let actionable_activation = actionable_widget
        .activation(&actionable_state)
        .unwrap_or_else(|_| unreachable!("actionable switch activation is inspectable"));
    assert!(actionable_activation.is_actionable());
    assert!(actionable_activation.enabled());
    assert!(
        !actionable_widget
            .hit_test(
                &actionable_state,
                HitContributionContext::__runtime_new(LogicalSize::ZERO),
            )
            .unwrap_or_else(|_| unreachable!("actionable switch hit contribution is inspectable"))
            .is_empty()
    );
}

#[test]
#[allow(clippy::assert_is_empty)]
fn radio_controls_use_public_semantics_and_typed_group_authoring() {
    let radio_element: runenui_core::Element<Action> = radio_button("One", true)
        .id("radio.one")
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, radio_widget, _) = radio_element.into_runtime_parts().into_parts();
    let radio_state = radio_widget.create_state();
    let radio_semantics = radio_widget
        .semantics(&radio_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("radio semantics are valid"));
    let radio_node = radio_semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("radio contributes one semantic node"));
    assert_eq!(radio_node.role(), SemanticRole::RadioButton);
    assert_eq!(
        radio_node.state().checked(),
        Some(SemanticCheckedState::Checked)
    );

    let group: runenui_core::Element<Action> = radio_group([
        radio_button("One", true).id("radio.one"),
        radio_button("Two", false).id("radio.two"),
    ])
    .id("radio.group")
    .gap(6_u16)
    .into_element();
    assert_eq!(group.children().len(), 2);
    assert!(matches!(
        group.layout().container(),
        runenui_core::LayoutContainer::Flex(_)
    ));
    let (_, _, _, _, _, _, _, _, group_widget, _) = group.into_runtime_parts().into_parts();
    let group_state = group_widget.create_state();
    let group_semantics = group_widget
        .semantics(&group_state, SemanticContributionContext::__runtime_new(2))
        .unwrap_or_else(|_| unreachable!("radio group semantics are valid"));
    let group_node = group_semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("radio group contributes one semantic node"));
    assert_eq!(group_node.role(), SemanticRole::RadioGroup);
    assert!(
        group_node
            .children()
            .iter()
            .any(runenui_core::SemanticItem::is_mounted_children)
    );
    assert!(
        group_widget
            .diagnostics(&group_state)
            .unwrap_or_else(|_| unreachable!("radio group diagnostics are inspectable"))
            .is_empty()
    );

    let invalid_group: runenui_core::Element<Action> =
        radio_group([radio_button("One", true), radio_button("Two", true)]).into_element();
    let (_, _, _, _, _, _, _, _, invalid_widget, _) =
        invalid_group.into_runtime_parts().into_parts();
    let invalid_state = invalid_widget.create_state();
    assert!(
        invalid_widget
            .semantics(
                &invalid_state,
                SemanticContributionContext::__runtime_new(2),
            )
            .unwrap_or_else(|_| unreachable!("invalid group semantics are inspectable"))
            .roots()
            .is_empty()
    );
    let diagnostics = invalid_widget
        .diagnostics(&invalid_state)
        .unwrap_or_else(|_| unreachable!("invalid group diagnostics are inspectable"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code(),
        "runenui.control.radio-group.multiple-checked"
    );
}

#[test]
fn list_box_and_option_item_use_typed_public_authoring_and_exact_semantics() {
    let option: runenui_core::Element<Action> = option_item("One", true)
        .id("option.one")
        .disabled()
        .discoverable_when_disabled(true)
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, option_widget, _) = option.into_runtime_parts().into_parts();
    let option_state = option_widget.create_state();
    let semantics = option_widget
        .semantics(&option_state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("OptionItem semantics are valid"));
    let node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("OptionItem contributes one semantic node"));
    assert_eq!(node.role(), SemanticRole::Option);
    assert_eq!(node.name(), Some("One"));
    assert_eq!(node.state().selected(), Some(true));
    assert!(node.state().disabled());

    let passive_disabled: runenui_core::Element<Action> = option_item("Passive disabled", false)
        .disabled()
        .into_element();
    let (_, _, _, _, _, _, _, _, passive_disabled_widget, _) =
        passive_disabled.into_runtime_parts().into_parts();
    let passive_disabled_state = passive_disabled_widget.create_state();
    let passive_disabled_activation = passive_disabled_widget
        .activation(&passive_disabled_state)
        .unwrap_or_else(|_| unreachable!("passive disabled activation is inspectable"));
    assert!(!passive_disabled_activation.enabled());
    assert!(!passive_disabled_activation.is_actionable());

    let passive_enabled: runenui_core::Element<Action> =
        option_item("Passive enabled", false).into_element();
    let (_, _, _, _, _, _, _, _, passive_enabled_widget, _) =
        passive_enabled.into_runtime_parts().into_parts();
    let passive_enabled_state = passive_enabled_widget.create_state();
    let passive_enabled_activation = passive_enabled_widget
        .activation(&passive_enabled_state)
        .unwrap_or_else(|_| unreachable!("passive enabled activation is inspectable"));
    assert!(passive_enabled_activation.enabled());
    assert!(!passive_enabled_activation.is_actionable());

    let list: runenui_core::Element<Action> = list_box([
        option_item("One", true).id("list.one"),
        option_item("Two", false).id("list.two"),
    ])
    .id("list")
    .orientation(Axis::Horizontal)
    .selection_mode(ListBoxSelectionMode::Multiple)
    .gap(4_u16)
    .into_element();
    assert_eq!(list.children().len(), 2);
    assert!(matches!(
        list.layout().container(),
        runenui_core::LayoutContainer::Flex(_)
    ));
    let (_, _, _, _, _, _, _, _, list_widget, _) = list.into_runtime_parts().into_parts();
    let list_state = list_widget.create_state();
    let semantics = list_widget
        .semantics(&list_state, SemanticContributionContext::__runtime_new(2))
        .unwrap_or_else(|_| unreachable!("ListBox semantics are valid"));
    let node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("ListBox contributes one semantic node"));
    assert_eq!(node.role(), SemanticRole::ListBox);
    assert_eq!(node.orientation(), Some(SemanticOrientation::Horizontal));
    assert_eq!(node.selection_mode(), Some(SemanticSelectionMode::Multiple));
}

#[test]
fn list_box_invalid_selection_authoring_fails_closed_with_exact_diagnostics() {
    let invalid_single: runenui_core::Element<Action> =
        list_box([option_item("One", true), option_item("Two", true)]).into_element();
    assert!(
        invalid_single
            .children()
            .iter()
            .all(|child| child.focusability() == Focusability::Hidden)
    );
    let (_, _, _, _, _, _, _, _, widget, _) = invalid_single.into_runtime_parts().into_parts();
    let state = widget.create_state();
    assert_eq!(
        widget
            .semantics(&state, SemanticContributionContext::__runtime_new(2),)
            .unwrap_or_else(|_| unreachable!("invalid ListBox semantics are inspectable"))
            .roots()
            .len(),
        0
    );
    let diagnostics = widget
        .diagnostics(&state)
        .unwrap_or_else(|_| unreachable!("invalid ListBox diagnostics are inspectable"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code(),
        "runenui.control.list-box.multiple-selected-single"
    );

    let invalid_multi: runenui_core::Element<Action> =
        list_box([option_item("One", true), option_item("Two", false)])
            .selection_mode(ListBoxSelectionMode::Multiple)
            .selection_follows_focus(true)
            .into_element();
    assert!(
        invalid_multi
            .children()
            .iter()
            .all(|child| child.focusability() == Focusability::Hidden)
    );
    let (_, _, _, _, _, _, _, _, widget, _) = invalid_multi.into_runtime_parts().into_parts();
    let state = widget.create_state();
    assert_eq!(
        widget
            .semantics(&state, SemanticContributionContext::__runtime_new(2),)
            .unwrap_or_else(|_| unreachable!("invalid ListBox semantics are inspectable"))
            .roots()
            .len(),
        0
    );
    let diagnostics = widget
        .diagnostics(&state)
        .unwrap_or_else(|_| unreachable!("invalid ListBox diagnostics are inspectable"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code(),
        "runenui.control.list-box.multiple-follow-focus"
    );
}

#[test]
fn tabs_use_typed_public_authoring_and_exact_semantic_relationships() {
    let tab_element: runenui_core::Element<Action> = tab("General", true)
        .id("tab.general")
        .controls("panel.general")
        .on_activate(|| Action::Save)
        .into_element();
    let (_, _, _, _, _, _, _, _, widget, _) = tab_element.into_runtime_parts().into_parts();
    let state = widget.create_state();
    let semantics = widget
        .semantics(&state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("Tab semantics are inspectable"));
    let node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("Tab contributes one semantic node"));
    assert_eq!(node.role(), SemanticRole::Tab);
    assert_eq!(node.name(), Some("General"));
    assert_eq!(node.state().selected(), Some(true));
    let relationship = node
        .relationships()
        .first()
        .unwrap_or_else(|| unreachable!("Tab Controls relationship exists"));
    assert_eq!(relationship.kind(), SemanticRelationshipKind::Controls);
    assert_eq!(
        relationship.target(),
        &SemanticReference::Authored {
            element_id: runenui_core::ElementId::new("panel.general")
                .unwrap_or_else(|_| unreachable!("fixture panel id is valid")),
            semantic_key: None,
        }
    );

    let conditional_tab: runenui_core::Element<Action> = tab("Conditional", false)
        .id("tab.conditional")
        .into_element();
    assert_eq!(conditional_tab.focusability(), Focusability::Focusable);
    let (_, _, _, _, _, _, _, _, widget, _) = conditional_tab.into_runtime_parts().into_parts();
    let state = widget.create_state();
    let semantics = widget
        .semantics(&state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("conditional Tab semantics are inspectable"));
    let node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("conditional Tab remains semantic"));
    assert_eq!(node.role(), SemanticRole::Tab);
    assert_eq!(node.relationships().len(), 0);

    let invalid_controls: runenui_core::Element<Action> =
        tab("Invalid", false).controls(" ").into_element();
    assert_eq!(invalid_controls.authoring_diagnostics().len(), 1);
    assert_eq!(
        invalid_controls.authoring_diagnostics()[0].field(),
        "controls"
    );

    let invalid_panel: runenui_core::Element<Action> =
        tab_panel("", [text("content")]).into_element();
    assert_eq!(invalid_panel.authoring_diagnostics().len(), 1);
    assert_eq!(
        invalid_panel.authoring_diagnostics()[0].field(),
        "labelled_by"
    );

    let list_element: runenui_core::Element<Action> = tab_list([
        tab("General", true).id("tab.general"),
        tab("Input", false).id("tab.input"),
    ])
    .orientation(Axis::Vertical)
    .into_element();
    assert_eq!(list_element.children().len(), 2);
    let (_, _, _, _, _, _, _, _, widget, _) = list_element.into_runtime_parts().into_parts();
    let state = widget.create_state();
    let semantics = widget
        .semantics(&state, SemanticContributionContext::__runtime_new(2))
        .unwrap_or_else(|_| unreachable!("TabList semantics are inspectable"));
    let node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("TabList contributes one semantic node"));
    assert_eq!(node.role(), SemanticRole::TabList);
    assert_eq!(node.orientation(), Some(SemanticOrientation::Vertical));

    let panel_element: runenui_core::Element<Action> =
        tab_panel("tab.general", [text("General content")])
            .id("panel.general")
            .into_element();
    assert_eq!(panel_element.children().len(), 1);
    let (_, _, _, _, _, _, _, _, widget, _) = panel_element.into_runtime_parts().into_parts();
    let state = widget.create_state();
    let semantics = widget
        .semantics(&state, SemanticContributionContext::__runtime_new(1))
        .unwrap_or_else(|_| unreachable!("TabPanel semantics are inspectable"));
    let node = semantics.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("TabPanel contributes one semantic node"));
    assert_eq!(node.role(), SemanticRole::TabPanel);
    assert_eq!(
        node.relationships()[0].kind(),
        SemanticRelationshipKind::LabelledBy
    );
}

#[test]
fn tab_list_multiple_selected_authoring_fails_closed() {
    let list: runenui_core::Element<Action> = tab_list([
        tab("One", true).id("tab.one"),
        tab("Two", true).id("tab.two"),
    ])
    .into_element();
    assert!(
        list.children()
            .iter()
            .all(|child| child.focusability() == Focusability::Hidden)
    );
    let (_, _, _, _, _, _, _, _, widget, _) = list.into_runtime_parts().into_parts();
    let state = widget.create_state();
    assert_eq!(
        widget
            .semantics(&state, SemanticContributionContext::__runtime_new(2))
            .unwrap_or_else(|_| unreachable!("invalid TabList semantics are inspectable"))
            .roots()
            .len(),
        0
    );
    let diagnostics = widget
        .diagnostics(&state)
        .unwrap_or_else(|_| unreachable!("invalid TabList diagnostics are inspectable"));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code(),
        "runenui.control.tab-list.multiple-selected"
    );
}

#[test]
fn text_measure_descriptor_defaults_and_button_center_are_public() {
    use runenui_core::{TextAlignment, TextBlockPlacement, TextLeafMeasure};
    let input = WidgetMeasureInput::new(
        None,
        None,
        WidgetAvailableSpace::MaxContent,
        WidgetAvailableSpace::MaxContent,
    );
    let plain: runenui_core::Element<Action> = text("Title").into_element();
    let (_, _, _, _, _, _, _, _, widget, _) = plain.into_runtime_parts().into_parts();
    let plain_state = widget.create_state();
    let WidgetMeasure::Text(plain) = widget
        .measure(&plain_state, input)
        .unwrap_or_else(|_| unreachable!("text measures"))
    else {
        unreachable!("Text remains a text leaf");
    };
    assert_eq!(plain.content(), "Title");
    assert_eq!(plain.inline_alignment(), TextAlignment::Start);
    assert_eq!(plain.block_placement(), TextBlockPlacement::Start);

    let button: runenui_core::Element<Action> =
        button("Save").on_activate(|| Action::Save).into_element();
    let (_, _, _, _, _, _, _, _, widget, _) = button.into_runtime_parts().into_parts();
    let state = widget.create_state();
    let WidgetMeasure::Text(button) = widget
        .measure(&state, input)
        .unwrap_or_else(|_| unreachable!("button measures"))
    else {
        unreachable!("Button stays a text leaf");
    };
    assert_eq!(button.content(), "Save");
    assert_eq!(button.inline_alignment(), TextAlignment::Center);
    assert_eq!(button.block_placement(), TextBlockPlacement::Center);

    let downstream = TextLeafMeasure::new("Downstream")
        .with_inline_alignment(TextAlignment::End)
        .with_block_placement(TextBlockPlacement::End);
    assert_eq!(downstream.inline_alignment(), TextAlignment::End);
    assert_eq!(downstream.block_placement(), TextBlockPlacement::End);
}

#[test]
fn selectable_text_reuses_public_m10_selection_and_copy_without_mutation() {
    use runenui_core::{
        EditableContributionError, SemanticAction, SemanticEditableMode, TextAffinity,
        TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection,
        selectable_text,
    };

    let snapshot =
        TextDocumentSnapshot::new(TextDocumentId::new(250), TextDocumentRevision::new(2));
    let source = "read-only public documentation";
    let position = TextPosition::new(snapshot, source, source.len(), TextAffinity::Downstream)
        .unwrap_or_else(|_| unreachable!("selection is valid"));
    let selection = TextSelection::collapsed(position);
    let element: runenui_core::Element<Action> = selectable_text(snapshot, source, selection)
        .unwrap_or_else(|_| unreachable!("read-only contract is valid"))
        .id("selectable.copy")
        .into_element();

    let (_, _, _, _, _, _, _, _, widget, _) = element.into_runtime_parts().into_parts();
    let state = widget.create_state();
    let editable = widget
        .editable(&state)
        .unwrap_or_else(|_| unreachable!("widget state matches"))
        .unwrap_or_else(|| unreachable!("read-only text contributes M10 selection"));
    assert!(editable.read_only());
    assert_eq!(editable.text(), source);
    assert_eq!(editable.snapshot(), snapshot);

    let contribution = widget
        .semantics(&state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("widget semantics are sound"));
    let node = contribution.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("read-only text has one semantic node"));
    assert_eq!(node.role(), SemanticRole::EditableText);
    assert!(node.state().read_only());
    assert_eq!(node.editable_mode(), Some(SemanticEditableMode::Multiline));
    assert_eq!(
        node.editable().and_then(|editable| editable.value()),
        Some(source)
    );
    // M10D exposes clipboard Copy through routed commands and framework
    // services, not through a semantically advertised clipboard action.
    assert!(!node.actions().contains(&SemanticAction::Copy));
    assert!(node.actions().contains(&SemanticAction::SelectAll));
    assert!(!node.actions().contains(&SemanticAction::ReplaceSelection));
    assert!(!node.actions().contains(&SemanticAction::Paste));
    assert!(!node.actions().contains(&SemanticAction::Cut));

    let foreign = TextDocumentSnapshot::new(TextDocumentId::new(250), TextDocumentRevision::new(3));
    let other_position = TextPosition::new(foreign, source, 0, TextAffinity::Downstream)
        .unwrap_or_else(|_| unreachable!("foreign position is valid"));
    assert!(matches!(
        selectable_text(snapshot, source, TextSelection::collapsed(other_position)),
        Err(EditableContributionError::SelectionSnapshotMismatch),
    ));
}

#[test]
fn public_text_field_binds_checked_m10_editor_with_typed_line_policy_and_semantics() {
    use runenui_core::{
        EditableContributionError, SemanticAction, SemanticEditableMode, SemanticInvalidState,
        TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextFieldError,
        TextNewlinePolicy, TextPosition, TextSelection, text_field,
    };

    let source = "hello";
    let snapshot = TextDocumentSnapshot::new(TextDocumentId::new(81), TextDocumentRevision::new(4));
    let position = TextPosition::new(snapshot, source, source.len(), TextAffinity::Upstream)
        .unwrap_or_else(|_| unreachable!("fixture caret is valid"));
    let selection = TextSelection::collapsed(position);

    let editor = text_field(
        snapshot,
        source,
        selection,
        SemanticEditableMode::SingleLine,
        |_| Action::Save,
    )
    .unwrap_or_else(|_| unreachable!("app's single-line document is valid"))
    .id("input.name")
    .placeholder("Name")
    .labelled_by("label.name")
    .described_by("hint.name")
    .error_message("error.name")
    .required(true)
    .invalid(SemanticInvalidState::Invalid);
    let element: runenui_core::Element<Action> = editor.into_element();
    let (_, _, _, _, _, _, _, _, widget, _) = element.into_runtime_parts().into_parts();
    let state = widget.create_state();
    let editable = widget
        .editable(&state)
        .unwrap_or_else(|_| unreachable!("widget state matches"))
        .unwrap_or_else(|| unreachable!("text field uses M10"));
    assert_eq!(editable.text(), source);
    assert_eq!(editable.snapshot(), snapshot);
    assert_eq!(
        editable.newline_policy(),
        TextNewlinePolicy::ReplaceWithSpace
    );
    assert!(!editable.read_only());
    let declaration = widget
        .semantics(&state, SemanticContributionContext::default())
        .unwrap_or_else(|_| unreachable!("field semantics are valid"));
    let node = declaration.roots()[0]
        .as_node()
        .unwrap_or_else(|| unreachable!("input contributes one semantic node"));
    assert_eq!(node.role(), SemanticRole::EditableText);
    assert_eq!(node.editable_mode(), Some(SemanticEditableMode::SingleLine));
    assert_eq!(node.placeholder(), Some("Name"));
    assert_eq!(node.state().required(), Some(true));
    assert_eq!(node.state().invalid(), Some(SemanticInvalidState::Invalid));
    assert_eq!(node.relationships().len(), 3);
    assert!(node.actions().contains(&SemanticAction::ReplaceSelection));
    assert!(!node.actions().contains(&SemanticAction::Copy));
    assert!(!node.actions().contains(&SemanticAction::Paste));

    let multiline = text_field(
        snapshot,
        source,
        selection,
        SemanticEditableMode::Multiline,
        |_| Action::Save,
    )
    .unwrap_or_else(|_| unreachable!("multiline source is valid"))
    .into_element();
    let (_, _, _, _, _, _, _, _, widget, _) = multiline.into_runtime_parts().into_parts();
    let state = widget.create_state();
    assert_eq!(
        widget
            .editable(&state)
            .unwrap_or_else(|_| unreachable!("widget state matches"))
            .unwrap_or_else(|| unreachable!("field supplies M10 edit contribution"))
            .newline_policy(),
        TextNewlinePolicy::Preserve
    );

    assert!(matches!(
        text_field(
            snapshot,
            "not\nallowed",
            selection,
            SemanticEditableMode::SingleLine,
            |_| Action::Save,
        ),
        Err(TextFieldError::SingleLineSourceContainsNewline),
    ));
    let foreign = TextDocumentSnapshot::new(TextDocumentId::new(81), TextDocumentRevision::new(5));
    let other_position = TextPosition::new(foreign, source, 0, TextAffinity::Downstream)
        .unwrap_or_else(|_| unreachable!("foreign caret is valid"));
    assert!(matches!(
        text_field(
            snapshot,
            source,
            TextSelection::collapsed(other_position),
            SemanticEditableMode::Multiline,
            |_| Action::Save,
        ),
        Err(TextFieldError::InvalidSelection(
            EditableContributionError::SelectionSnapshotMismatch
        )),
    ));
}
