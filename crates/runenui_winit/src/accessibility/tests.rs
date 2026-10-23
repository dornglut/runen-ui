use super::*;
use runenui_core::{
    __runtime::RuntimeNamespace, EditIntent, EditResolution, EditableContribution,
    EditingSessionPolicy, Element, LogicalSize, NoHostProtocol, SemanticAction, SemanticActionData,
    SemanticAutocomplete, SemanticCheckedState, SemanticCollectionPosition, SemanticContribution,
    SemanticContributionContext, SemanticEditable, SemanticHierarchyLevel, SemanticInvalidState,
    SemanticItem, SemanticKey, SemanticNodeContribution, SemanticNumber, SemanticOrientation,
    SemanticPopupKind, SemanticPressedState, SemanticRange, SemanticReference,
    SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticSelectionMode,
    SemanticState, SemanticText, SemanticValue, StyleEnvironment, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextSelection, TextSensitivity, UiApp,
    UpdateOutput, View, Widget, WidgetActivation, WidgetActivationContext, WidgetActivationOutput,
    WidgetInvalidation, WidgetMeasure, WidgetMeasureInput,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, PumpBudget, SurfaceBuildContext,
};

#[derive(Clone, Copy, Debug)]
struct FixtureAction;

#[derive(Debug)]
struct Fixture {
    phase: u8,
}

fn typed_semantics(known_size: u64) -> SemanticContribution {
    let combo_key = SemanticKey::from_static("combo").unwrap();
    let listbox_key = SemanticKey::from_static("listbox").unwrap();
    let option_key = SemanticKey::from_static("option").unwrap();
    let error_key = SemanticKey::from_static("error").unwrap();
    let slider_key = SemanticKey::from_static("slider").unwrap();
    let toggle_key = SemanticKey::from_static("toggle").unwrap();
    let dialog_key = SemanticKey::from_static("dialog").unwrap();
    let tree_key = SemanticKey::from_static("tree").unwrap();
    let tree_item_key = SemanticKey::from_static("tree-item").unwrap();

    let combo = SemanticNodeContribution::new(combo_key, SemanticRole::ComboBox)
        .with_name("Choice")
        .with_state(
            SemanticState::ENABLED
                .with_expanded(true)
                .with_required(true)
                .with_invalid(SemanticInvalidState::Invalid),
        )
        .with_popup(SemanticPopupKind::ListBox)
        .with_placeholder("Filter choices")
        .with_autocomplete(SemanticAutocomplete::List)
        .with_relationship(SemanticRelationship::new(
            SemanticRelationshipKind::Controls,
            SemanticReference::Local(listbox_key.clone()),
        ))
        .with_relationship(SemanticRelationship::new(
            SemanticRelationshipKind::ActiveDescendant,
            SemanticReference::Local(option_key.clone()),
        ))
        .with_relationship(SemanticRelationship::new(
            SemanticRelationshipKind::ErrorMessage,
            SemanticReference::Local(error_key.clone()),
        ))
        .with_action(SemanticAction::Collapse);
    let option = SemanticNodeContribution::new(option_key, SemanticRole::Option)
        .with_name("One")
        .with_state(SemanticState::ENABLED.with_selected(true))
        .with_collection_position(
            SemanticCollectionPosition::new(0, Some(known_size))
                .unwrap_or_else(|_| unreachable!("controlled collection position is valid")),
        );
    let listbox = SemanticNodeContribution::new(listbox_key, SemanticRole::ListBox)
        .with_name("Choices")
        .with_orientation(SemanticOrientation::Vertical)
        .with_selection_mode(SemanticSelectionMode::Multiple)
        .with_child(option);
    let error = SemanticNodeContribution::new(error_key, SemanticRole::Text)
        .with_name("Choice error node")
        .with_text(SemanticText::plain("Choice error"));

    let minimum = SemanticNumber::new(0.0).unwrap_or_else(|_| unreachable!("finite minimum"));
    let maximum = SemanticNumber::new(10.0).unwrap_or_else(|_| unreachable!("finite maximum"));
    let current = SemanticNumber::new(5.0).unwrap_or_else(|_| unreachable!("finite current"));
    let small_step = SemanticNumber::new(1.0).unwrap_or_else(|_| unreachable!("finite small step"));
    let large_step = SemanticNumber::new(5.0).unwrap_or_else(|_| unreachable!("finite large step"));
    let range = SemanticRange::new(Some(minimum), Some(maximum), Some(current))
        .and_then(|range| range.with_small_step(small_step))
        .and_then(|range| range.with_large_step(large_step))
        .and_then(|range| range.with_value_text("half"))
        .unwrap_or_else(|_| unreachable!("controlled range is valid"));
    let slider = SemanticNodeContribution::new(slider_key, SemanticRole::Slider)
        .with_name("Volume")
        .with_range(range)
        .with_orientation(SemanticOrientation::Horizontal)
        .with_action(SemanticAction::Increment)
        .with_action(SemanticAction::Decrement)
        .with_action(SemanticAction::SetValue);

    let toggle = SemanticNodeContribution::new(toggle_key, SemanticRole::Button)
        .with_name("Toggle")
        .with_state(SemanticState::ENABLED.with_pressed(SemanticPressedState::Pressed));
    let dialog = SemanticNodeContribution::new(dialog_key, SemanticRole::Dialog)
        .with_name("Settings dialog")
        .with_state(SemanticState::ENABLED.with_modal(true));

    let tree_item = SemanticNodeContribution::new(tree_item_key, SemanticRole::TreeItem)
        .with_name("Root item")
        .with_collection_position(
            SemanticCollectionPosition::new(0, Some(1))
                .unwrap_or_else(|_| unreachable!("tree position is valid")),
        )
        .with_hierarchy_level(
            SemanticHierarchyLevel::new(1)
                .unwrap_or_else(|_| unreachable!("tree level is positive")),
        );
    let tree = SemanticNodeContribution::new(tree_key, SemanticRole::Tree)
        .with_name("Navigation tree")
        .with_child(tree_item);

    SemanticContribution::single(
        SemanticNodeContribution::primary(SemanticRole::Group)
            .with_child(combo)
            .with_child(listbox)
            .with_child(error)
            .with_child(slider)
            .with_child(toggle)
            .with_child(dialog)
            .with_child(tree),
    )
}

fn conflicting_collection_semantics() -> SemanticContribution {
    let first = SemanticNodeContribution::new(
        SemanticKey::from_static("conflicting-first").unwrap(),
        SemanticRole::Option,
    )
    .with_name("First conflicting")
    .with_collection_position(
        SemanticCollectionPosition::new(0, Some(1))
            .unwrap_or_else(|_| unreachable!("first collection position is valid")),
    );
    let second = SemanticNodeContribution::new(
        SemanticKey::from_static("conflicting-second").unwrap(),
        SemanticRole::Option,
    )
    .with_name("Second conflicting")
    .with_collection_position(
        SemanticCollectionPosition::new(1, Some(2))
            .unwrap_or_else(|_| unreachable!("second collection position is valid")),
    );
    SemanticContribution::single(
        SemanticNodeContribution::primary(SemanticRole::Group).with_child(
            SemanticNodeContribution::new(
                SemanticKey::from_static("conflicting-listbox").unwrap(),
                SemanticRole::ListBox,
            )
            .with_name("Conflicting choices")
            .with_child(first)
            .with_child(second),
        ),
    )
}

fn multiple_error_message_semantics() -> SemanticContribution {
    let listbox_key = SemanticKey::from_static("error-listbox").unwrap();
    let first_error_key = SemanticKey::from_static("first-error").unwrap();
    let second_error_key = SemanticKey::from_static("second-error").unwrap();
    let combo = SemanticNodeContribution::new(
        SemanticKey::from_static("error-combo").unwrap(),
        SemanticRole::ComboBox,
    )
    .with_name("Invalid choice")
    .with_state(SemanticState::ENABLED.with_invalid(SemanticInvalidState::Invalid))
    .with_popup(SemanticPopupKind::ListBox)
    .with_relationship(SemanticRelationship::new(
        SemanticRelationshipKind::Controls,
        SemanticReference::Local(listbox_key.clone()),
    ))
    .with_relationship(SemanticRelationship::new(
        SemanticRelationshipKind::ErrorMessage,
        SemanticReference::Local(first_error_key.clone()),
    ))
    .with_relationship(SemanticRelationship::new(
        SemanticRelationshipKind::ErrorMessage,
        SemanticReference::Local(second_error_key.clone()),
    ));
    let listbox =
        SemanticNodeContribution::new(listbox_key, SemanticRole::ListBox).with_name("Errors");
    let first_error =
        SemanticNodeContribution::new(first_error_key, SemanticRole::Text).with_name("First error");
    let second_error = SemanticNodeContribution::new(second_error_key, SemanticRole::Text)
        .with_name("Second error");
    SemanticContribution::single(
        SemanticNodeContribution::primary(SemanticRole::Group)
            .with_child(combo)
            .with_child(listbox)
            .with_child(first_error)
            .with_child(second_error),
    )
}

impl Widget<FixtureAction> for Fixture {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn activate(
        &mut self,
        (): &mut Self::State,
        context: &mut WidgetActivationContext<FixtureAction>,
    ) -> WidgetActivationOutput<FixtureAction> {
        context.invalidate(WidgetInvalidation::SEMANTICS);
        WidgetActivationOutput::changed_with_action(FixtureAction)
    }
    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        if self.phase == 5 {
            return conflicting_collection_semantics();
        }
        if self.phase == 6 {
            return multiple_error_message_semantics();
        }
        if self.phase >= 3 {
            return typed_semantics(u64::from(self.phase - 2));
        }
        if self.phase == 2 {
            return SemanticContribution::new(vec![
                SemanticItem::node(SemanticNodeContribution::new(
                    SemanticKey::from_static("first").unwrap(),
                    SemanticRole::Group,
                )),
                SemanticItem::node(SemanticNodeContribution::new(
                    SemanticKey::from_static("second").unwrap(),
                    SemanticRole::Group,
                )),
            ]);
        }
        let text = SemanticNodeContribution::new(
            SemanticKey::from_static("text").unwrap(),
            SemanticRole::Text,
        )
        .with_name("Plain text")
        .with_text(SemanticText::plain("Plain text"));
        let mut button = SemanticNodeContribution::primary(SemanticRole::Button)
            .with_name("Do it")
            .with_description("Activate this control through the native accessibility tree")
            .with_value(SemanticValue::Text("button value".into()))
            .with_action(SemanticAction::Activate)
            .with_action(SemanticAction::RequestFocus)
            .with_action(SemanticAction::OpenMenu)
            .with_action(SemanticAction::OpenContextMenu);
        if self.phase != 1 {
            button = button
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::LabelledBy,
                    SemanticReference::Local(SemanticKey::from_static("text").unwrap()),
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::DescribedBy,
                    SemanticReference::Local(SemanticKey::from_static("text").unwrap()),
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Local(SemanticKey::from_static("text").unwrap()),
                ))
                .with_child(text);
        }
        if self.phase == 0 {
            button = button
                .with_child(
                    SemanticNodeContribution::new(
                        SemanticKey::from_static("check").unwrap(),
                        SemanticRole::Checkbox,
                    )
                    .with_name("Check it")
                    .with_state(SemanticState::ENABLED.with_checked(SemanticCheckedState::Mixed)),
                )
                .with_child(
                    SemanticNodeContribution::new(
                        SemanticKey::from_static("diagnostic").unwrap(),
                        SemanticRole::Group,
                    )
                    .with_value(SemanticValue::Integer(7))
                    .with_state(SemanticState::ENABLED.with_disabled(true).with_inert(true)),
                )
                .with_child(
                    SemanticNodeContribution::new(
                        SemanticKey::from_static("boolean-value").unwrap(),
                        SemanticRole::Generic,
                    )
                    .with_name("Boolean value")
                    .with_value(SemanticValue::Boolean(true)),
                );
        }
        SemanticContribution::single(button)
    }
    fn measure(
        &self,
        _: &Self::State,
        _input: runenui_core::WidgetMeasureInput,
    ) -> runenui_core::WidgetMeasure {
        runenui_core::WidgetMeasure::measured(100_u16.into(), 100_u16.into())
    }
    fn paint(
        &self,
        _: &Self::State,
        _: runenui_core::PaintContributionContext,
    ) -> runenui_core::PaintContribution {
        runenui_core::PaintContribution::empty()
    }
}
struct FixtureApp;
impl UiApp for FixtureApp {
    type State = u8;
    type Action = FixtureAction;
    type HostProtocol = NoHostProtocol;
    fn root(phase: &Self::State) -> impl View<Self::Action> {
        Element::new(Fixture { phase: *phase })
            .id("fixture")
            .key("fixture")
            .focusable(true)
    }
    fn update(
        phase: &mut Self::State,
        FixtureAction: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *phase = phase.saturating_add(1);
    }
}

fn publication(runtime: &mut AppRuntime<FixtureApp>) -> SemanticPublication {
    let style_environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &style_environment,
            LogicalSize::try_new(100.0, 100.0).unwrap(),
        ))
        .unwrap()
        .semantic_publication()
        .clone()
}

#[test]
fn initial_tree_maps_roles_text_and_custom_action_without_duplicate_label() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(0);
    let publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    let mut activation = adapter.activation_handler();
    assert!(activation.request_initial_tree().is_none());
    let update = adapter.update(&publication);
    assert_eq!(update.mode, UpdateMode::InitialFull);
    assert!(update.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        AdapterDiagnostic::UnsupportedValueType(_) | AdapterDiagnostic::UnsupportedInertState(_)
    )));
    let text = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.value() == Some("Plain text"))
        .unwrap();
    assert_eq!(text.1.role(), Role::Label);
    assert_eq!(text.1.label(), None);
    let button = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Do it"))
        .unwrap();
    assert_eq!(button.1.role(), Role::Button);
    assert_eq!(button.1.value(), Some("button value"));
    assert_eq!(
        button.1.description(),
        Some("Activate this control through the native accessibility tree")
    );
    assert!(button.1.supports_action(Action::Click));
    assert!(button.1.supports_action(Action::Focus));
    assert!(button.1.supports_action(Action::CustomAction));
    assert_eq!(button.1.custom_actions()[0].description, "Open menu".into());
    assert_eq!(button.1.labelled_by(), &[text.0]);
    assert_eq!(button.1.described_by(), &[text.0]);
    assert_eq!(button.1.controls(), &[text.0]);
    let checkbox = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Check it"))
        .unwrap();
    assert_eq!(checkbox.1.role(), Role::CheckBox);
    assert_eq!(checkbox.1.toggled(), Some(Toggled::Mixed));
    let disabled_group = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Group)
        .map(|(_, node)| node)
        .unwrap();
    assert!(disabled_group.is_disabled());
    let boolean_value = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Boolean value"))
        .map(|(_, node)| node)
        .unwrap();
    assert_eq!(boolean_value.role(), Role::GenericContainer);
    assert_eq!(boolean_value.value(), None);
    assert_eq!(boolean_value.toggled(), None);
    assert_eq!(
        update
            .diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic, AdapterDiagnostic::UnsupportedValueType(_)))
            .count(),
        2
    );
    let activated = activation.request_initial_tree().unwrap();
    assert_eq!(activated.tree_id, update.tree_update.tree_id);
    assert!(activated.tree.is_some());
}

#[test]
#[allow(clippy::assert_is_empty)]
fn all_current_roles_map_exactly_and_unsupported_facts_diagnose() {
    let namespace = RuntimeNamespace::__runtime_new();
    let id = namespace.__runtime_semantic_id(0, 1);
    let mut diagnostics = Vec::new();
    assert_eq!(
        map_role(SemanticRole::Generic, &id, &mut diagnostics),
        Role::GenericContainer
    );
    assert_eq!(
        map_role(SemanticRole::Group, &id, &mut diagnostics),
        Role::Group
    );
    assert_eq!(
        map_role(SemanticRole::Text, &id, &mut diagnostics),
        Role::Label
    );
    assert_eq!(
        map_role(SemanticRole::Button, &id, &mut diagnostics),
        Role::Button
    );
    assert_eq!(
        map_role(SemanticRole::EditableText, &id, &mut diagnostics),
        Role::TextInput
    );
    assert_eq!(
        map_role(SemanticRole::Checkbox, &id, &mut diagnostics),
        Role::CheckBox
    );
    assert_eq!(
        map_role(SemanticRole::RadioButton, &id, &mut diagnostics),
        Role::RadioButton
    );
    assert_eq!(
        map_role(SemanticRole::RadioGroup, &id, &mut diagnostics),
        Role::RadioGroup
    );
    assert_eq!(
        map_role(SemanticRole::Switch, &id, &mut diagnostics),
        Role::Switch
    );
    for (semantic, accesskit) in [
        (SemanticRole::Link, Role::Link),
        (SemanticRole::Image, Role::Image),
        (SemanticRole::ComboBox, Role::ComboBox),
        (SemanticRole::Slider, Role::Slider),
        (SemanticRole::ScrollBar, Role::ScrollBar),
        (SemanticRole::Progress, Role::ProgressIndicator),
        (SemanticRole::SpinButton, Role::SpinButton),
        (SemanticRole::ListBox, Role::ListBox),
        (SemanticRole::Option, Role::ListBoxOption),
        (SemanticRole::TabList, Role::TabList),
        (SemanticRole::Tab, Role::Tab),
        (SemanticRole::TabPanel, Role::TabPanel),
        (SemanticRole::Toolbar, Role::Toolbar),
        (SemanticRole::Menu, Role::Menu),
        (SemanticRole::MenuBar, Role::MenuBar),
        (SemanticRole::MenuItem, Role::MenuItem),
        (SemanticRole::MenuItemCheckbox, Role::MenuItemCheckBox),
        (SemanticRole::MenuItemRadio, Role::MenuItemRadio),
        (SemanticRole::Dialog, Role::Dialog),
        (SemanticRole::Tooltip, Role::Tooltip),
        (SemanticRole::Separator, Role::Splitter),
        (SemanticRole::Splitter, Role::Splitter),
        (SemanticRole::Tree, Role::Tree),
        (SemanticRole::TreeItem, Role::TreeItem),
    ] {
        assert_eq!(map_role(semantic, &id, &mut diagnostics), accesskit);
    }
    assert_eq!(
        map_checked_state(SemanticCheckedState::Unchecked, &id, &mut diagnostics),
        Some(Toggled::False)
    );
    assert_eq!(
        map_checked_state(SemanticCheckedState::Checked, &id, &mut diagnostics),
        Some(Toggled::True)
    );
    assert_eq!(
        map_checked_state(SemanticCheckedState::Mixed, &id, &mut diagnostics),
        Some(Toggled::Mixed)
    );
    assert!(diagnostics.is_empty());
    let mut runtime = AppRuntime::<FixtureApp>::mount(0);
    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(&publication(&mut runtime));
    assert!(
        update
            .diagnostics
            .iter()
            .any(|diagnostic| matches!(diagnostic, AdapterDiagnostic::UnsupportedValueType(_)))
    );
}

#[test]
#[allow(clippy::assert_is_empty)]
fn typed_standard_semantics_project_exact_native_properties_and_collection_delta() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(3);
    let first_publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    let first = adapter.update(&first_publication);
    assert_eq!(first.mode, UpdateMode::InitialFull);
    assert!(first.diagnostics.is_empty());

    let find = |label: &str| {
        first
            .tree_update
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label))
            .unwrap_or_else(|| unreachable!("typed semantic node is projected"))
    };
    let (combo_id, combo) = find("Choice");
    let (listbox_id, listbox) = find("Choices");
    let (option_id, option) = find("One");
    let (error_id, _) = find("Choice error node");
    let (_, slider) = find("Volume");
    let (_, toggle) = find("Toggle");
    let (_, dialog) = find("Settings dialog");
    let (_, tree) = find("Navigation tree");
    let (_, tree_item) = find("Root item");

    assert_eq!(combo.role(), Role::ComboBox);
    assert_eq!(combo.is_expanded(), Some(true));
    assert!(combo.is_required());
    assert_eq!(combo.invalid(), Some(AccessInvalid::True));
    assert_eq!(combo.has_popup(), Some(HasPopup::Listbox));
    assert_eq!(combo.placeholder(), Some("Filter choices"));
    assert_eq!(combo.auto_complete(), Some(AccessAutoComplete::List));
    assert_eq!(combo.controls(), &[*listbox_id]);
    assert_eq!(combo.active_descendant(), Some(*option_id));
    assert_eq!(combo.error_message(), Some(*error_id));

    assert_eq!(listbox.role(), Role::ListBox);
    assert_eq!(listbox.orientation(), Some(AccessOrientation::Vertical));
    assert!(listbox.is_multiselectable());
    assert_eq!(listbox.size_of_set(), Some(1));
    assert_eq!(option.role(), Role::ListBoxOption);
    assert_eq!(option.is_selected(), Some(true));
    assert_eq!(option.position_in_set(), Some(0));

    assert_eq!(slider.role(), Role::Slider);
    assert_eq!(slider.min_numeric_value(), Some(0.0));
    assert_eq!(slider.max_numeric_value(), Some(10.0));
    assert_eq!(slider.numeric_value(), Some(5.0));
    assert_eq!(slider.numeric_value_step(), Some(1.0));
    assert_eq!(slider.numeric_value_jump(), Some(5.0));
    assert_eq!(slider.value(), Some("half"));
    assert_eq!(slider.orientation(), Some(AccessOrientation::Horizontal));
    assert_eq!(toggle.toggled(), Some(Toggled::True));
    assert!(dialog.is_modal());
    assert_eq!(tree.size_of_set(), Some(1));
    assert_eq!(tree_item.level(), Some(0));
    assert_eq!(tree_item.position_in_set(), Some(0));

    runtime
        .submit_action(FixtureAction)
        .unwrap_or_else(|_| unreachable!("fixture phase transition is admitted"));
    let _ = runtime
        .pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let second_publication = publication(&mut runtime);
    let second = adapter.update(&second_publication);
    assert_eq!(second.mode, UpdateMode::Delta);
    assert!(second.diagnostics.is_empty());

    let changed_listbox = second
        .tree_update
        .nodes
        .iter()
        .find(|(id, _)| id == listbox_id)
        .map_or_else(
            || unreachable!("derived collection size change reprojects the native parent"),
            |(_, node)| node,
        );
    assert_eq!(changed_listbox.size_of_set(), Some(2));
    assert_ne!(combo_id, listbox_id);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn range_and_expansion_actions_project_and_round_trip_exact_native_requests() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(3);
    let publication = publication(&mut runtime);
    let surface = publication.snapshot().surface_id().clone();
    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(&publication);
    assert_eq!(update.mode, UpdateMode::InitialFull);
    assert!(update.diagnostics.is_empty());

    let slider_semantic = publication
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Volume"))
        .unwrap_or_else(|| unreachable!("slider semantic node is published"));
    let combo_semantic = publication
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("Choice"))
        .unwrap_or_else(|| unreachable!("combo semantic node is published"));
    let slider = adapter
        .active_id(&surface, slider_semantic.id())
        .unwrap_or_else(|| unreachable!("slider has native identity"));
    let combo = adapter
        .active_id(&surface, combo_semantic.id())
        .unwrap_or_else(|| unreachable!("combo has native identity"));
    let tree_id = update.tree_update.tree_id;
    let projected_slider = &adapter.projection.current_nodes[&slider];
    let projected_combo = &adapter.projection.current_nodes[&combo];

    assert!(projected_slider.supports_action(Action::Increment));
    assert!(projected_slider.supports_action(Action::Decrement));
    assert!(projected_slider.supports_action(Action::SetValue));
    assert!(projected_combo.supports_action(Action::Collapse));

    for (native, expected) in [
        (Action::Increment, SemanticAction::Increment),
        (Action::Decrement, SemanticAction::Decrement),
    ] {
        let request = ActionRequest {
            action: native,
            target_tree: tree_id,
            target_node: slider,
            data: None,
        };
        let translated = adapter
            .action_request(&request)
            .unwrap_or_else(|_| unreachable!("advertised range action translates"));
        assert_eq!(translated.action(), &expected);
        assert!(translated.data().is_none());
    }

    let value = 7.5;
    let set_value = ActionRequest {
        action: Action::SetValue,
        target_tree: tree_id,
        target_node: slider,
        data: Some(ActionData::NumericValue(value)),
    };
    let translated = adapter
        .action_request(&set_value)
        .unwrap_or_else(|_| unreachable!("finite native SetValue translates"));
    assert_eq!(translated.action(), &SemanticAction::SetValue);
    assert_eq!(
        translated.data(),
        Some(&SemanticActionData::NumericValue(
            SemanticNumber::new(value)
                .unwrap_or_else(|_| unreachable!("controlled value is finite"))
        ))
    );

    let collapse = ActionRequest {
        action: Action::Collapse,
        target_tree: tree_id,
        target_node: combo,
        data: None,
    };
    assert_eq!(
        adapter
            .action_request(&collapse)
            .unwrap_or_else(|_| unreachable!("advertised collapse translates"))
            .action(),
        &SemanticAction::Collapse
    );

    let missing_value = ActionRequest {
        action: Action::SetValue,
        target_tree: tree_id,
        target_node: slider,
        data: None,
    };
    assert_eq!(
        adapter.action_request(&missing_value),
        Err(AdapterDiagnostic::UnexpectedActionData(Action::SetValue))
    );
    let non_finite = ActionRequest {
        data: Some(ActionData::NumericValue(f64::NAN)),
        ..set_value
    };
    assert_eq!(
        adapter.action_request(&non_finite),
        Err(AdapterDiagnostic::UnexpectedActionData(Action::SetValue))
    );
}

#[test]
fn conflicting_collection_sizes_diagnose_and_withhold_native_set_size() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(5);
    let publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(&publication);

    assert_eq!(update.mode, UpdateMode::InitialFull);
    let listbox = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Conflicting choices"))
        .map_or_else(
            || unreachable!("conflicting ListBox is projected"),
            |(_, node)| node,
        );
    assert_eq!(listbox.size_of_set(), None);
    assert_eq!(
        update
            .diagnostics
            .iter()
            .filter(|diagnostic| matches!(
                diagnostic,
                AdapterDiagnostic::UnrepresentableCollectionMetadata(_)
            ))
            .count(),
        1
    );
}

#[test]
fn multiple_error_messages_diagnose_and_withhold_native_error_target() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(6);
    let publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(&publication);

    assert_eq!(update.mode, UpdateMode::InitialFull);
    let combo = update
        .tree_update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Invalid choice"))
        .map_or_else(
            || unreachable!("invalid ComboBox is projected"),
            |(_, node)| node,
        );
    assert_eq!(combo.error_message(), None);
    assert_eq!(
        update
            .diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic, AdapterDiagnostic::MultipleErrorMessages(_)))
            .count(),
        1
    );
}

#[test]
#[allow(clippy::assert_is_empty)]
fn exhausted_initial_projection_diagnoses_without_publishing_partial_state() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(0);
    let publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    adapter.projection.next_node_id = None;
    let mut activation = adapter.activation_handler();

    let update = adapter.update(&publication);

    assert_eq!(update.mode, UpdateMode::Unchanged);
    assert_eq!(
        update.diagnostics,
        vec![AdapterDiagnostic::NodeIdSpaceExhausted]
    );
    assert!(update.tree_update.nodes.is_empty());
    assert!(update.tree_update.tree.is_none());
    assert!(adapter.projection.current_surface.is_none());
    assert!(adapter.projection.current_revision.is_none());
    assert!(adapter.projection.current_snapshot.is_none());
    assert!(adapter.projection.semantic_to_accesskit.is_empty());
    assert!(adapter.projection.accesskit_to_semantic.is_empty());
    assert!(adapter.projection.current_nodes.is_empty());
    assert!(activation.request_initial_tree().is_none());
}

#[test]
fn node_id_capacity_preflight_reaches_exact_boundary_without_wrap() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(2);
    let publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    adapter.projection.next_node_id = Some(u64::MAX - 2);

    let update = adapter.update(&publication);

    assert_eq!(update.mode, UpdateMode::InitialFull);
    assert!(
        !update
            .diagnostics
            .contains(&AdapterDiagnostic::NodeIdSpaceExhausted)
    );
    assert!(adapter.projection.next_node_id.is_none());
    assert_eq!(adapter.projection.synthetic_root, Some(NodeId(u64::MAX)));
    let mut semantic_ids = adapter
        .projection
        .semantic_to_accesskit
        .values()
        .map(|id| id.0)
        .collect::<Vec<_>>();
    semantic_ids.sort_unstable();
    assert_eq!(semantic_ids, vec![u64::MAX - 2, u64::MAX - 1]);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn node_id_exhaustion_preserves_the_last_coherent_projection() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(0);
    let first_publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    adapter.update(&first_publication);
    adapter.projection.next_node_id = None;

    let _ = runtime.submit_action(FixtureAction);
    let _ = runtime
        .pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let second_publication = publication(&mut runtime);
    let second = adapter.update(&second_publication);
    assert_eq!(second.mode, UpdateMode::Delta);
    assert!(
        !second
            .diagnostics
            .contains(&AdapterDiagnostic::NodeIdSpaceExhausted)
    );
    assert!(adapter.projection.next_node_id.is_none());

    let surface = second_publication.snapshot().surface_id().clone();
    let button = second_publication.snapshot().roots()[0].clone();
    let button_id = adapter.active_id(&surface, &button).unwrap();
    let before_semantic_to_accesskit = adapter.projection.semantic_to_accesskit.clone();
    let before_accesskit_to_semantic = adapter.projection.accesskit_to_semantic.clone();
    let before_retired_semantic = adapter.projection.retired_semantic.clone();
    let before_retired_accesskit = adapter.projection.retired_accesskit.clone();
    let before_synthetic_root = adapter.projection.synthetic_root;
    let before_revision = adapter.projection.current_revision;
    let before_node_ids = adapter
        .projection
        .current_nodes
        .keys()
        .copied()
        .collect::<Vec<_>>();
    let mut activation = adapter.activation_handler();
    let before_tree = activation.request_initial_tree().unwrap();

    let _ = runtime.submit_action(FixtureAction);
    let _ = runtime
        .pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let third_publication = publication(&mut runtime);
    let rejected = adapter.update(&third_publication);

    assert_eq!(rejected.mode, UpdateMode::Unchanged);
    assert_eq!(
        rejected.diagnostics,
        vec![AdapterDiagnostic::NodeIdSpaceExhausted]
    );
    assert!(adapter.projection.next_node_id.is_none());
    assert_eq!(
        adapter.projection.semantic_to_accesskit,
        before_semantic_to_accesskit
    );
    assert_eq!(
        adapter.projection.accesskit_to_semantic,
        before_accesskit_to_semantic
    );
    assert_eq!(adapter.projection.retired_semantic, before_retired_semantic);
    assert_eq!(
        adapter.projection.retired_accesskit,
        before_retired_accesskit
    );
    assert_eq!(adapter.projection.synthetic_root, before_synthetic_root);
    assert_eq!(adapter.projection.current_revision, before_revision);
    assert_eq!(
        adapter
            .projection
            .current_nodes
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        before_node_ids
    );
    assert_eq!(rejected.tree_update.tree_id, before_tree.tree_id);
    assert_eq!(rejected.tree_update.focus, before_tree.focus);
    assert!(rejected.tree_update.nodes.is_empty());
    assert!(rejected.tree_update.tree.is_none());
    let after_tree = activation.request_initial_tree().unwrap();
    assert_eq!(after_tree.tree_id, before_tree.tree_id);
    assert_eq!(after_tree.focus, before_tree.focus);
    assert_eq!(
        after_tree
            .nodes
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        before_tree
            .nodes
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    );

    let request = ActionRequest {
        action: Action::Click,
        target_tree: rejected.tree_update.tree_id,
        target_node: button_id,
        data: None,
    };
    let translated = adapter.action_request(&request).unwrap();
    assert_eq!(translated.surface_id(), &surface);
    assert_eq!(translated.target(), &button);
    assert_eq!(translated.action(), &SemanticAction::Activate);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn exact_delta_and_skipped_revision_resync_keep_ids_stable_and_retired_ids_unused() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(0);
    let first_publication = publication(&mut runtime);
    let mut adapter = SemanticAdapter::new();
    let first = adapter.update(&first_publication);
    let surface = first_publication.snapshot().surface_id().clone();
    let button = first_publication.snapshot().roots()[0].clone();
    let first_id = adapter.active_id(&surface, &button).unwrap();
    let first_child = first_publication
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Text)
        .map(SemanticNode::id)
        .unwrap()
        .clone();
    let first_child_id = adapter.active_id(&surface, &first_child).unwrap();
    assert_eq!(first.mode, UpdateMode::InitialFull);
    let request = runenui_core::SemanticActionRequest::new(
        surface.clone(),
        button.clone(),
        SemanticAction::Activate,
    );
    runtime.submit_semantic_action(request).unwrap();
    let _ = runtime
        .pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let second_publication = publication(&mut runtime);
    let delta = adapter.update(&second_publication);
    assert_eq!(delta.mode, UpdateMode::Delta);
    assert!(!delta.tree_update.nodes.is_empty());
    let mounted_target = runtime.index().nodes().first().unwrap().id().clone();
    runtime
        .submit_command(
            mounted_target,
            runenui_core::SemanticCommand::RequestFocus,
            runenui_core::CommandOrigin::programmatic(),
        )
        .unwrap();
    let _ = runtime
        .pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let third_publication = publication(&mut runtime);
    let mut skipped_adapter = SemanticAdapter::new();
    skipped_adapter.update(&first_publication);
    let skipped = skipped_adapter.update(&third_publication);
    assert_eq!(skipped.mode, UpdateMode::FullResync);
    let third = adapter.update(&third_publication);
    assert_eq!(third.mode, UpdateMode::Delta);
    assert_eq!(adapter.active_id(&surface, &first_child), None);
    let stale_child_request = ActionRequest {
        action: Action::Click,
        target_tree: third.tree_update.tree_id,
        target_node: first_child_id,
        data: None,
    };
    assert_eq!(
        adapter.action_request(&stale_child_request),
        Err(AdapterDiagnostic::RetiredNodeId)
    );
    assert_eq!(adapter.active_id(&surface, &button), Some(first_id));
}

#[test]
fn surface_transition_full_resyncs_and_retires_old_action_ids() {
    let mut first_runtime = AppRuntime::<FixtureApp>::mount(0);
    let first_publication = publication(&mut first_runtime);
    let first_surface = first_publication.snapshot().surface_id().clone();
    let first_semantic = first_publication.snapshot().roots()[0].clone();
    let mut adapter = SemanticAdapter::new();
    let first = adapter.update(&first_publication);
    let first_node_id = adapter.active_id(&first_surface, &first_semantic).unwrap();
    let stale_request = ActionRequest {
        action: Action::Click,
        target_tree: first.tree_update.tree_id,
        target_node: first_node_id,
        data: None,
    };
    assert_eq!(
        adapter.action_request(&stale_request).unwrap().surface_id(),
        &first_surface
    );

    let mut second_runtime = AppRuntime::<FixtureApp>::mount(0);
    let second_publication = publication(&mut second_runtime);
    let second_surface = second_publication.snapshot().surface_id().clone();
    assert_ne!(first_surface, second_surface);
    let second_semantic = second_publication.snapshot().roots()[0].clone();
    let second = adapter.update(&second_publication);
    assert_eq!(second.mode, UpdateMode::FullResync);
    assert_eq!(second.tree_update.tree_id, TreeId::ROOT);
    assert_eq!(adapter.active_id(&first_surface, &first_semantic), None);
    assert_eq!(
        adapter.action_request(&stale_request),
        Err(AdapterDiagnostic::RetiredNodeId)
    );
    let second_node_id = adapter
        .active_id(&second_surface, &second_semantic)
        .unwrap();
    assert_ne!(first_node_id, second_node_id);
}

#[test]
fn surface_transition_retires_synthetic_root_identity() {
    let mut first_runtime = AppRuntime::<FixtureApp>::mount(2);
    let first_publication = publication(&mut first_runtime);
    let mut adapter = SemanticAdapter::new();
    let first = adapter.update(&first_publication);
    assert_eq!(first.mode, UpdateMode::InitialFull);
    let first_root = adapter.projection.synthetic_root.unwrap();

    let mut second_runtime = AppRuntime::<FixtureApp>::mount(2);
    let second_publication = publication(&mut second_runtime);
    assert_ne!(
        first_publication.snapshot().surface_id(),
        second_publication.snapshot().surface_id()
    );
    let second = adapter.update(&second_publication);
    assert_eq!(second.mode, UpdateMode::FullResync);
    let second_root = adapter.projection.synthetic_root.unwrap();
    assert_ne!(first_root, second_root);
    assert!(adapter.projection.retired_accesskit.contains(&first_root));
}

#[test]
fn synthetic_root_retires_when_same_surface_becomes_single_root() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(2);
    let first_publication = publication(&mut runtime);
    let surface = first_publication.snapshot().surface_id().clone();
    let mut adapter = SemanticAdapter::new();
    let first = adapter.update(&first_publication);
    assert_eq!(first.mode, UpdateMode::InitialFull);
    let first_root = adapter.projection.synthetic_root.unwrap();

    let _ = runtime.submit_action(FixtureAction);
    let _ = runtime
        .pump(runenui_runtime::PumpBudget::new(64, 64, 64, 64))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let second_publication = publication(&mut runtime);
    assert_eq!(second_publication.snapshot().surface_id(), &surface);
    assert_eq!(second_publication.snapshot().roots().len(), 1);
    let second = adapter.update(&second_publication);
    assert_eq!(second.mode, UpdateMode::Delta);
    assert_eq!(adapter.projection.synthetic_root, None);
    assert!(adapter.projection.retired_accesskit.contains(&first_root));
    let current_root = second_publication.snapshot().roots()[0].clone();
    assert_ne!(adapter.active_id(&surface, &current_root), Some(first_root));
}

#[test]
fn action_translation_rejects_wrong_custom_and_foreign_requests() {
    let mut runtime = AppRuntime::<FixtureApp>::mount(0);
    let publication = publication(&mut runtime);
    let surface = publication.snapshot().surface_id().clone();
    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(&publication);
    let button_id = adapter
        .active_id(&surface, &publication.snapshot().roots()[0])
        .unwrap();
    let tree_id = update.tree_update.tree_id;
    for (action, data, expected) in [
        (Action::Click, None, SemanticAction::Activate),
        (Action::Focus, None, SemanticAction::RequestFocus),
        (
            Action::ShowContextMenu,
            None,
            SemanticAction::OpenContextMenu,
        ),
        (
            Action::CustomAction,
            Some(ActionData::CustomAction(OPEN_MENU_CUSTOM_ACTION_ID)),
            SemanticAction::OpenMenu,
        ),
    ] {
        let request = ActionRequest {
            action,
            target_tree: tree_id,
            target_node: button_id,
            data,
        };
        assert_eq!(
            adapter.action_request(&request).unwrap().action(),
            &expected
        );
    }
    let request = ActionRequest {
        action: Action::CustomAction,
        target_tree: tree_id,
        target_node: button_id,
        data: Some(ActionData::CustomAction(99)),
    };
    assert_eq!(
        adapter.action_request(&request),
        Err(AdapterDiagnostic::WrongCustomActionId(99))
    );
    let foreign = ActionRequest {
        target_tree: TreeId(accesskit::Uuid::from_u128(9)),
        ..request
    };
    assert_eq!(
        adapter.action_request(&foreign),
        Err(AdapterDiagnostic::WrongTreeId)
    );
    let unsupported = ActionRequest {
        action: Action::Expand,
        target_tree: tree_id,
        target_node: button_id,
        data: None,
    };
    assert!(matches!(
        adapter.action_request(&unsupported),
        Err(AdapterDiagnostic::UnsupportedSemanticAction {
            action: SemanticAction::Expand,
            ..
        })
    ));
    let missing_custom_data = ActionRequest {
        action: Action::CustomAction,
        target_tree: tree_id,
        target_node: button_id,
        data: None,
    };
    assert_eq!(
        adapter.action_request(&missing_custom_data),
        Err(AdapterDiagnostic::CustomActionDataMissing)
    );
    let unknown_node = ActionRequest {
        target_node: NodeId(u64::MAX),
        ..missing_custom_data
    };
    assert_eq!(
        adapter.action_request(&unknown_node),
        Err(AdapterDiagnostic::UnknownNodeId)
    );
}

#[test]
fn adapter_tree_ids_are_not_runtime_id_casts() {
    let namespace = RuntimeNamespace::__runtime_new();
    let surface = namespace.__runtime_surface_id(0, 1);
    let mut runtime = AppRuntime::<FixtureApp>::mount(0);
    let publication = publication(&mut runtime);
    assert_ne!(surface, publication.snapshot().surface_id().clone());
    let mut adapter = SemanticAdapter::new();
    let update = adapter.update(&publication);
    assert_eq!(update.tree_update.tree_id, TreeId::ROOT);
}

#[derive(Clone)]
struct EditableState {
    text: String,
    revision: u64,
    sensitivity: TextSensitivity,
    read_only: bool,
    role: SemanticRole,
}

enum EditableAction {
    Edit(EditIntent),
}

#[derive(Debug)]
struct EditableFixture {
    snapshot: TextDocumentSnapshot,
    text: String,
    sensitivity: TextSensitivity,
    read_only: bool,
    role: SemanticRole,
}

impl Widget<EditableAction> for EditableFixture {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::NONE
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<EditableAction>> {
        EditableContribution::new(
            self.snapshot,
            self.text.clone(),
            TextSelection::collapsed(
                TextPosition::new(
                    self.snapshot,
                    &self.text,
                    self.text.len(),
                    TextAffinity::Upstream,
                )
                .ok()?,
            ),
            self.sensitivity,
            self.read_only,
            false,
            EditingSessionPolicy::PreserveExact,
            EditableAction::Edit,
        )
        .ok()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text(runenui_core::TextLeafMeasure::new(self.text.clone()))
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let selection = TextSelection::collapsed(
            TextPosition::new(
                self.snapshot,
                &self.text,
                self.text.len(),
                TextAffinity::Upstream,
            )
            .unwrap(),
        );
        let editable = SemanticEditable::new(
            self.snapshot,
            &self.text,
            selection,
            self.sensitivity,
            self.read_only,
        )
        .unwrap();
        let mut node = SemanticNodeContribution::primary(self.role)
            .with_state(SemanticState::ENABLED.with_read_only(self.read_only))
            .with_editable(editable)
            .with_action(SemanticAction::SetSelection)
            .with_action(SemanticAction::ReplaceSelection);
        if self.role == SemanticRole::ComboBox {
            node = node.with_popup(SemanticPopupKind::ListBox);
        }
        if self.role == SemanticRole::SpinButton {
            node = node.with_range(
                SemanticRange::new(
                    None,
                    None,
                    Some(
                        SemanticNumber::new(0.0)
                            .unwrap_or_else(|_| unreachable!("spin value is finite")),
                    ),
                )
                .unwrap_or_else(|_| unreachable!("spin range is valid")),
            );
        }
        SemanticContribution::single(node)
    }
}

struct EditableApp;

impl UiApp for EditableApp {
    type State = EditableState;
    type Action = EditableAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(EditableFixture {
            snapshot: TextDocumentSnapshot::new(
                TextDocumentId::new(17),
                TextDocumentRevision::new(state.revision),
            ),
            text: state.text.clone(),
            sensitivity: state.sensitivity,
            read_only: state.read_only,
            role: state.role,
        })
        .id("editable")
        .key("editable")
        .focusable(true)
    }

    fn update(
        state: &mut Self::State,
        EditableAction::Edit(intent): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        let range = intent.replacement();
        state
            .text
            .replace_range(range.start()..range.end(), intent.replacement_text());
        state.revision += 1;
        UpdateOutput::edit(EditResolution::accepted(
            intent.request().clone(),
            TextDocumentSnapshot::new(
                TextDocumentId::new(17),
                TextDocumentRevision::new(state.revision),
            ),
        ))
    }
}

fn editable_publication(
    sensitivity: TextSensitivity,
    read_only: bool,
) -> (SemanticPublication, SemanticAdapter) {
    editable_publication_for_role(SemanticRole::EditableText, sensitivity, read_only)
}

fn editable_publication_for_role(
    role: SemanticRole,
    sensitivity: TextSensitivity,
    read_only: bool,
) -> (SemanticPublication, SemanticAdapter) {
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
    ));
    let mut runtime = AppRuntime::<EditableApp>::mount(EditableState {
        text: "ab".to_owned(),
        revision: 0,
        sensitivity,
        read_only,
        role,
    });
    assert!(runtime.register_text_font_bytes(FONT.to_vec()).unwrap() > 0);
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell").unwrap()],
            )
            .unwrap()
    );
    let style = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &style,
            LogicalSize::try_new(100.0, 40.0).unwrap(),
        ))
        .unwrap()
        .semantic_publication()
        .clone();
    let mut adapter = SemanticAdapter::new();
    adapter.update(&publication);
    (publication, adapter)
}

#[test]
fn editable_projection_translates_checked_selection_and_redacted_replacement_actions() {
    let (publication, adapter) = editable_publication(TextSensitivity::Public, false);
    let semantic = &publication.snapshot().nodes()[0];
    let parent = adapter
        .active_id(publication.snapshot().surface_id(), semantic.id())
        .unwrap();
    let text_run = adapter.projection.editable_text_runs[semantic.id()];
    let projected = &adapter.projection.current_nodes[&parent];
    assert_eq!(projected.role(), Role::TextInput);
    assert!(projected.supports_action(Action::SetTextSelection));
    assert!(projected.supports_action(Action::ReplaceSelectedText));

    let selection = adapter
        .action_request(&ActionRequest {
            action: Action::SetTextSelection,
            target_tree: TreeId::ROOT,
            target_node: parent,
            data: Some(ActionData::SetTextSelection(AccessTextSelection {
                anchor: AccessTextPosition {
                    node: text_run,
                    character_index: 1,
                },
                focus: AccessTextPosition {
                    node: text_run,
                    character_index: 1,
                },
            })),
        })
        .unwrap();
    let Some(SemanticActionData::Selection(selection)) = selection.data() else {
        unreachable!("selection action retains checked neutral data");
    };
    assert_eq!(selection.active().byte_offset(), 1);

    let replacement = adapter
        .action_request(&ActionRequest {
            action: Action::ReplaceSelectedText,
            target_tree: TreeId::ROOT,
            target_node: parent,
            data: Some(ActionData::Value("sensitive replacement".into())),
        })
        .unwrap();
    assert_eq!(replacement.action(), &SemanticAction::ReplaceSelection);
    assert!(!format!("{replacement:?}").contains("sensitive replacement"));
}

#[test]
fn composite_editable_roles_project_exact_native_roles() {
    for (semantic_role, native_role) in [
        (SemanticRole::ComboBox, Role::EditableComboBox),
        (SemanticRole::SpinButton, Role::SpinButton),
    ] {
        let (publication, adapter) =
            editable_publication_for_role(semantic_role, TextSensitivity::Public, false);
        let semantic = &publication.snapshot().nodes()[0];
        assert_eq!(semantic.role(), semantic_role);
        let parent = adapter
            .active_id(publication.snapshot().surface_id(), semantic.id())
            .unwrap_or_else(|| unreachable!("editable semantic node is projected"));
        assert_eq!(
            adapter.projection.current_nodes[&parent].role(),
            native_role
        );
    }
}

#[test]
fn secret_and_read_only_editable_projection_cannot_gain_native_disclosure_or_mutation() {
    let (secret_publication, secret_adapter) = editable_publication(TextSensitivity::Secret, false);
    let secret = &secret_publication.snapshot().nodes()[0];
    let secret_parent = secret_adapter
        .active_id(secret_publication.snapshot().surface_id(), secret.id())
        .unwrap();
    let projected_secret = &secret_adapter.projection.current_nodes[&secret_parent];
    assert_eq!(projected_secret.role(), Role::PasswordInput);
    assert_eq!(projected_secret.value(), None);
    assert!(!projected_secret.supports_action(Action::SetTextSelection));
    assert!(projected_secret.supports_action(Action::ReplaceSelectedText));
    assert!(
        !secret_adapter
            .projection
            .editable_text_runs
            .contains_key(secret.id())
    );

    let (read_only_publication, read_only_adapter) =
        editable_publication(TextSensitivity::Public, true);
    let read_only = &read_only_publication.snapshot().nodes()[0];
    let read_only_parent = read_only_adapter
        .active_id(
            read_only_publication.snapshot().surface_id(),
            read_only.id(),
        )
        .unwrap();
    let projected_read_only = &read_only_adapter.projection.current_nodes[&read_only_parent];
    assert!(projected_read_only.is_read_only());
    assert!(projected_read_only.supports_action(Action::SetTextSelection));
    assert!(!projected_read_only.supports_action(Action::ReplaceSelectedText));
}

#[derive(Clone)]
struct StandardPasswordState {
    content: String,
    sensitivity: TextSensitivity,
}

enum StandardPasswordAction {
    Edit(Box<EditIntent>),
    Toggle,
}

struct StandardPasswordApp;

impl UiApp for StandardPasswordApp {
    type State = StandardPasswordState;
    type Action = StandardPasswordAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(190), TextDocumentRevision::new(1));
        let selection = TextSelection::collapsed(
            TextPosition::new(
                snapshot,
                &state.content,
                state.content.len(),
                TextAffinity::Upstream,
            )
            .unwrap_or_else(|_| unreachable!("checked application selection")),
        );
        let field = runenui_core::text_field(
            snapshot,
            state.content.clone(),
            selection,
            SemanticEditableMode::SingleLine,
            |edit| StandardPasswordAction::Edit(Box::new(edit)),
        )
        .unwrap_or_else(|_| unreachable!("single-line source is valid"));
        let field = if state.sensitivity == TextSensitivity::Secret {
            field
                .password()
                .unwrap_or_else(|_| unreachable!("single-line password is valid"))
        } else {
            field
        };
        field.id("form.password").placeholder("Password")
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            StandardPasswordAction::Toggle => {
                state.sensitivity = if state.sensitivity == TextSensitivity::Secret {
                    TextSensitivity::Public
                } else {
                    TextSensitivity::Secret
                };
                UpdateOutput::effects(runenui_core::Effects::none())
            }
            StandardPasswordAction::Edit(intent) => UpdateOutput::edit(EditResolution::rejected(
                intent.request().clone(),
                TextDocumentSnapshot::new(TextDocumentId::new(190), TextDocumentRevision::new(1)),
            )),
        }
    }
}

fn publish_standard_password(
    runtime: &mut AppRuntime<StandardPasswordApp>,
    adapter: &mut SemanticAdapter,
) -> SemanticPublication {
    let style = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &style,
            LogicalSize::try_new(280.0, 50.0).unwrap(),
        ))
        .unwrap()
        .semantic_publication()
        .clone();
    adapter.update(&publication);
    publication
}

fn assert_standard_password_secret(
    publication: &SemanticPublication,
    adapter: &SemanticAdapter,
) -> NodeId {
    let semantic = &publication.snapshot().nodes()[0];
    assert_eq!(
        semantic.editable().and_then(|editable| editable.value()),
        None
    );
    assert!(
        semantic
            .supported_actions()
            .contains(&SemanticAction::SetSelection)
    );
    let parent = adapter
        .active_id(publication.snapshot().surface_id(), semantic.id())
        .unwrap();
    let native = &adapter.projection.current_nodes[&parent];
    assert_eq!(native.role(), Role::PasswordInput);
    assert_eq!(native.value(), None);
    assert!(!native.supports_action(Action::SetTextSelection));
    assert!(native.supports_action(Action::ReplaceSelectedText));
    assert!(
        !adapter
            .projection
            .editable_text_runs
            .contains_key(semantic.id())
    );
    parent
}

#[test]
fn standard_password_native_projection_retires_public_text_resources_on_reclassification() {
    const SOURCE: &str = "vault-é漢🔒";
    const FONT: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
    ));
    let mut runtime = AppRuntime::<StandardPasswordApp>::mount(StandardPasswordState {
        content: SOURCE.to_owned(),
        sensitivity: TextSensitivity::Secret,
    });
    assert!(runtime.register_text_font_bytes(FONT.to_vec()).is_ok());
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell").unwrap()],
            )
            .is_ok()
    );
    let mut adapter = SemanticAdapter::new();
    let first = publish_standard_password(&mut runtime, &mut adapter);
    let parent = assert_standard_password_secret(&first, &adapter);
    assert!(
        adapter
            .action_request(&ActionRequest {
                action: Action::SetTextSelection,
                target_tree: TreeId::ROOT,
                target_node: parent,
                data: Some(ActionData::SetTextSelection(AccessTextSelection {
                    anchor: AccessTextPosition {
                        node: parent,
                        character_index: 0,
                    },
                    focus: AccessTextPosition {
                        node: parent,
                        character_index: 0,
                    },
                })),
            })
            .is_err()
    );

    runtime
        .submit_action(StandardPasswordAction::Toggle)
        .unwrap();
    let _ = runtime
        .pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let public = publish_standard_password(&mut runtime, &mut adapter);
    let public_semantic = &public.snapshot().nodes()[0];
    let public_parent = adapter
        .active_id(public.snapshot().surface_id(), public_semantic.id())
        .unwrap();
    let public_native = &adapter.projection.current_nodes[&public_parent];
    assert_eq!(public_native.role(), Role::TextInput);
    assert_eq!(public_native.value(), Some(SOURCE));
    assert!(public_native.supports_action(Action::SetTextSelection));
    assert!(
        adapter
            .projection
            .editable_text_runs
            .contains_key(public_semantic.id())
    );

    runtime
        .submit_action(StandardPasswordAction::Toggle)
        .unwrap();
    let _ = runtime
        .pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"));
    let again = publish_standard_password(&mut runtime, &mut adapter);
    assert_standard_password_secret(&again, &adapter);
    assert!(
        adapter
            .projection
            .current_nodes
            .values()
            .all(|node| { node.value() != Some(SOURCE) })
    );
}
