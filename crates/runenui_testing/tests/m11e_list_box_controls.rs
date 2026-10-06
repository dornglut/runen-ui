#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;

use runenui_core::{
    Axis, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, ListBoxSelectionMode, LogicalKey, NoHostProtocol, PhysicalKey, SemanticCommand,
    SemanticOrientation, SemanticRole, SemanticSelectionMode, UiApp, View, button, children,
    column, list_box, option_item,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Select(u8),
    RemoveTwo,
    ReplaceTwo,
    Noop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectionFixture {
    SingleManual,
    SingleFollowFocus,
    Multiple,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SecondOptionFixture {
    Standard,
    Disabled,
    DisabledDiscoverable,
    PassiveDisabled,
    PassiveDisabledDiscoverable,
    Removed,
    Replacement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    selected: Vec<u8>,
    selection: SelectionFixture,
    second_option: SecondOptionFixture,
    activations: Vec<u8>,
}

struct ListBoxApp;

impl UiApp for ListBoxApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut options = vec![
            option_item("Alpha", state.selected.contains(&1))
                .id("option.one")
                .on_activate(|| Action::Select(1)),
        ];
        if state.second_option != SecondOptionFixture::Removed {
            let label = if state.second_option == SecondOptionFixture::Replacement {
                "Delta"
            } else {
                "Bravo"
            };
            let key = if state.second_option == SecondOptionFixture::Replacement {
                "option.two.replacement"
            } else {
                "option.two.original"
            };
            let two = option_item(label, state.selected.contains(&2))
                .id("option.two")
                .key(key);
            let two = match state.second_option {
                SecondOptionFixture::Standard | SecondOptionFixture::Replacement => {
                    two.on_activate(|| Action::Select(2))
                }
                SecondOptionFixture::Disabled => two.on_activate(|| Action::Select(2)).disabled(),
                SecondOptionFixture::DisabledDiscoverable => two
                    .on_activate(|| Action::Select(2))
                    .disabled()
                    .discoverable_when_disabled(true),
                SecondOptionFixture::PassiveDisabled => two.disabled(),
                SecondOptionFixture::PassiveDisabledDiscoverable => {
                    two.disabled().discoverable_when_disabled(true)
                }
                SecondOptionFixture::Removed => {
                    unreachable!("removed second option is excluded before construction")
                }
            };
            options.push(two);
        }
        options.push(
            option_item("Charlie", state.selected.contains(&3))
                .id("option.three")
                .on_activate(|| Action::Select(3)),
        );

        let list = match state.selection {
            SelectionFixture::SingleManual => list_box(options).id("list"),
            SelectionFixture::SingleFollowFocus => {
                list_box(options).id("list").selection_follows_focus(true)
            }
            SelectionFixture::Multiple => list_box(options)
                .id("list")
                .selection_mode(ListBoxSelectionMode::Multiple),
        };

        column(children![
            button("Before").id("before").on_activate(|| Action::Noop),
            list,
            button("After").id("after").on_activate(|| Action::Noop),
        ])
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Select(value) if state.selection == SelectionFixture::Multiple => {
                if let Some(index) = state
                    .selected
                    .iter()
                    .position(|selected| *selected == value)
                {
                    state.selected.remove(index);
                } else {
                    state.selected.push(value);
                }
                state.activations.push(value);
            }
            Action::Select(value) => {
                state.selected = vec![value];
                state.activations.push(value);
            }
            Action::RemoveTwo => state.second_option = SecondOptionFixture::Removed,
            Action::ReplaceTwo => state.second_option = SecondOptionFixture::Replacement,
            Action::Noop => {}
        }
    }
}

const fn fixture(selected: Vec<u8>) -> State {
    State {
        selected,
        selection: SelectionFixture::SingleManual,
        second_option: SecondOptionFixture::Standard,
        activations: Vec::new(),
    }
}

fn budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(16).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn settle(harness: &mut TestHarness<ListBoxApp>) {
    assert_eq!(
        harness.run_until_idle(budget()).outcome(),
        SettleOutcome::Idle
    );
}

fn element_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("fixture IDs are valid"))
}

fn command(harness: &mut TestHarness<ListBoxApp>, target: &str, command: SemanticCommand) {
    harness
        .submit_automation_command(element_id(target), command)
        .unwrap_or_else(|error| unreachable!("ListBox command is accepted: {error:?}"));
    settle(harness);
}

fn assert_focus_name(harness: &TestHarness<ListBoxApp>, name: &str) {
    let target = harness
        .unique_semantic_target(
            &SemanticQuery::new()
                .with_role(SemanticRole::Option)
                .with_name(name),
        )
        .unwrap_or_else(|error| unreachable!("Option target is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    assert_eq!(snapshot.focused(), Some(target.node_id()));
}

fn assert_button_focus(harness: &TestHarness<ListBoxApp>, name: &str) {
    let target = harness
        .unique_semantic_target(
            &SemanticQuery::new()
                .with_role(SemanticRole::Button)
                .with_name(name),
        )
        .unwrap_or_else(|error| unreachable!("Button target is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    assert_eq!(snapshot.focused(), Some(target.node_id()));
}

const fn key(physical: PhysicalKey, logical: LogicalKey, phase: KeyboardPhase) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        physical,
        logical,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
fn list_box_publishes_exact_selection_orientation_and_collection_metadata() {
    let mut harness = TestHarness::<ListBoxApp>::mount(fixture(vec![2]));
    assert!(harness.publish().is_ok());
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    let list = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ListBox)
        .unwrap_or_else(|| unreachable!("ListBox is published"));
    assert_eq!(list.orientation(), Some(SemanticOrientation::Vertical));
    assert_eq!(list.selection_mode(), Some(SemanticSelectionMode::Single));
    assert_eq!(list.children().len(), 3);
    for (index, child) in list.children().iter().enumerate() {
        let option = snapshot
            .node(child)
            .unwrap_or_else(|| unreachable!("ListBox child is published"));
        assert_eq!(option.role(), SemanticRole::Option);
        let position = option
            .collection_position()
            .unwrap_or_else(|| unreachable!("position is published"));
        assert_eq!(
            position.index(),
            u64::try_from(index).unwrap_or_else(|_| unreachable!())
        );
        assert_eq!(position.known_size(), Some(3));
    }
    let selected = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Option && node.name() == Some("Bravo"))
        .unwrap_or_else(|| unreachable!("selected Option is published"));
    assert_eq!(selected.state().selected(), Some(true));
}

#[test]
#[allow(clippy::assert_is_empty)]
fn manual_list_box_entry_navigation_home_end_and_type_ahead_do_not_mutate_selection() {
    let mut harness = TestHarness::<ListBoxApp>::mount(fixture(vec![2]));
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Bravo");

    command(&mut harness, "option.two", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_button_focus(&harness, "After");
    command(&mut harness, "after", SemanticCommand::FocusPrevious);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Bravo");

    command(&mut harness, "option.two", SemanticCommand::FocusDown);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Charlie");
    assert_eq!(harness.state().selected, vec![2]);
    assert_eq!(harness.state().activations, Vec::<u8>::new());

    harness
        .submit_keyboard(key(
            PhysicalKey::Home,
            LogicalKey::Home,
            KeyboardPhase::Down,
        ))
        .unwrap_or_else(|error| unreachable!("Home is accepted: {error:?}"));
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Alpha");

    harness
        .submit_keyboard(key(PhysicalKey::End, LogicalKey::End, KeyboardPhase::Down))
        .unwrap_or_else(|error| unreachable!("End is accepted: {error:?}"));
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Charlie");

    command(&mut harness, "option.one", SemanticCommand::RequestFocus);
    harness
        .submit_keyboard(key(
            PhysicalKey::Code(String::from("KeyC")),
            LogicalKey::Character(String::from("c")),
            KeyboardPhase::Down,
        ))
        .unwrap_or_else(|error| unreachable!("type-ahead character is accepted: {error:?}"));
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Charlie");
    assert_eq!(harness.state().selected, vec![2]);
    assert_eq!(harness.state().activations, Vec::<u8>::new());
}

#[test]
fn ordinary_space_activation_emits_only_the_application_selection_action() {
    let mut harness = TestHarness::<ListBoxApp>::mount(fixture(vec![1]));
    assert!(harness.publish().is_ok());
    command(&mut harness, "option.three", SemanticCommand::RequestFocus);

    harness
        .submit_keyboard(key(
            PhysicalKey::Space,
            LogicalKey::Space,
            KeyboardPhase::Down,
        ))
        .unwrap_or_else(|error| unreachable!("Space down is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, vec![1]);

    harness
        .submit_keyboard(key(
            PhysicalKey::Space,
            LogicalKey::Space,
            KeyboardPhase::Up,
        ))
        .unwrap_or_else(|error| unreachable!("Space up is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, vec![3]);
    assert_eq!(harness.state().activations, vec![3]);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Charlie");
}

#[test]
fn disabled_discoverability_and_single_follow_focus_use_existing_focus_authority() {
    let mut skipped_state = fixture(vec![1]);
    skipped_state.second_option = SecondOptionFixture::Disabled;
    let mut skipped = TestHarness::<ListBoxApp>::mount(skipped_state);
    assert!(skipped.publish().is_ok());
    command(&mut skipped, "option.one", SemanticCommand::RequestFocus);
    command(&mut skipped, "option.one", SemanticCommand::FocusDown);
    assert!(skipped.publish().is_ok());
    assert_focus_name(&skipped, "Charlie");

    let mut discoverable_state = fixture(vec![1]);
    discoverable_state.second_option = SecondOptionFixture::DisabledDiscoverable;
    let mut discoverable = TestHarness::<ListBoxApp>::mount(discoverable_state);
    assert!(discoverable.publish().is_ok());
    command(
        &mut discoverable,
        "option.one",
        SemanticCommand::RequestFocus,
    );
    command(&mut discoverable, "option.one", SemanticCommand::FocusDown);
    assert!(discoverable.publish().is_ok());
    assert_focus_name(&discoverable, "Bravo");
    assert_eq!(discoverable.state().selected, vec![1]);

    let mut passive_skipped_state = fixture(vec![1]);
    passive_skipped_state.second_option = SecondOptionFixture::PassiveDisabled;
    let mut passive_skipped = TestHarness::<ListBoxApp>::mount(passive_skipped_state);
    assert!(passive_skipped.publish().is_ok());
    command(
        &mut passive_skipped,
        "option.one",
        SemanticCommand::RequestFocus,
    );
    command(
        &mut passive_skipped,
        "option.one",
        SemanticCommand::FocusDown,
    );
    assert!(passive_skipped.publish().is_ok());
    assert_focus_name(&passive_skipped, "Charlie");

    let mut passive_discoverable_state = fixture(vec![1]);
    passive_discoverable_state.second_option = SecondOptionFixture::PassiveDisabledDiscoverable;
    let mut passive_discoverable = TestHarness::<ListBoxApp>::mount(passive_discoverable_state);
    assert!(passive_discoverable.publish().is_ok());
    command(
        &mut passive_discoverable,
        "option.one",
        SemanticCommand::RequestFocus,
    );
    command(
        &mut passive_discoverable,
        "option.one",
        SemanticCommand::FocusDown,
    );
    assert!(passive_discoverable.publish().is_ok());
    assert_focus_name(&passive_discoverable, "Bravo");
    assert_eq!(passive_discoverable.state().selected, vec![1]);

    let mut follow_state = fixture(vec![1]);
    follow_state.selection = SelectionFixture::SingleFollowFocus;
    let mut follow = TestHarness::<ListBoxApp>::mount(follow_state);
    assert!(follow.publish().is_ok());
    command(&mut follow, "option.one", SemanticCommand::RequestFocus);
    command(&mut follow, "option.one", SemanticCommand::FocusDown);
    assert_eq!(follow.state().selected, vec![2]);
    assert_eq!(follow.state().activations, vec![2]);
    assert!(follow.publish().is_ok());
    assert_focus_name(&follow, "Bravo");
}

#[test]
fn multi_selection_keeps_first_selected_entry_and_focus_distinct_from_selection() {
    let mut state = fixture(vec![1, 3]);
    state.selection = SelectionFixture::Multiple;
    let mut harness = TestHarness::<ListBoxApp>::mount(state);
    assert!(harness.publish().is_ok());
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    let list = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ListBox)
        .unwrap_or_else(|| unreachable!("ListBox is published"));
    assert_eq!(list.selection_mode(), Some(SemanticSelectionMode::Multiple));
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .filter(|node| {
                node.role() == SemanticRole::Option && node.state().selected() == Some(true)
            })
            .count(),
        2
    );

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Alpha");

    command(&mut harness, "option.one", SemanticCommand::FocusDown);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Bravo");
    assert_eq!(harness.state().selected, vec![1, 3]);
    assert_eq!(harness.state().activations, Vec::<u8>::new());

    let mut state = fixture(vec![2, 3]);
    state.selection = SelectionFixture::Multiple;
    state.second_option = SecondOptionFixture::Disabled;
    let mut harness = TestHarness::<ListBoxApp>::mount(state);
    assert!(harness.publish().is_ok());
    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Charlie");
    assert_eq!(harness.state().selected, vec![2, 3]);
    assert_eq!(harness.state().activations, Vec::<u8>::new());
}

#[test]
fn removal_and_application_rebuild_recompute_entry_and_collection_metadata_without_rewriting_selection()
 {
    let mut harness = TestHarness::<ListBoxApp>::mount(fixture(vec![2]));
    assert!(harness.publish().is_ok());
    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Bravo");

    harness
        .submit_action(Action::RemoveTwo)
        .unwrap_or_else(|_| unreachable!("removal is admitted"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, vec![2]);
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Alpha");

    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    let options = snapshot
        .nodes()
        .iter()
        .filter(|node| node.role() == SemanticRole::Option)
        .collect::<Vec<_>>();
    assert_eq!(options.len(), 2);
    assert!(options.iter().all(|node| {
        node.collection_position()
            .is_some_and(|position| position.known_size() == Some(2))
    }));

    harness
        .submit_action(Action::Select(3))
        .unwrap_or_else(|_| unreachable!("selection rebuild is admitted"));
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Charlie");
}

#[test]
fn keyed_replacement_rebuilds_selected_preferred_entry_without_realization_identity() {
    let mut harness = TestHarness::<ListBoxApp>::mount(fixture(vec![2]));
    assert!(harness.publish().is_ok());
    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Bravo");

    harness
        .submit_action(Action::ReplaceTwo)
        .unwrap_or_else(|_| unreachable!("replacement action is admitted"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, vec![2]);
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Delta");
    assert_eq!(harness.state().selected, vec![2]);
}

#[test]
fn horizontal_list_box_maps_horizontal_but_not_vertical_directional_commands() {
    struct Horizontal;
    impl UiApp for Horizontal {
        type State = ();
        type Action = ();
        type HostProtocol = NoHostProtocol;
        fn root((): &Self::State) -> impl View<Self::Action> {
            list_box([
                option_item("One", true).id("h.one").on_activate(|| ()),
                option_item("Two", false).id("h.two").on_activate(|| ()),
            ])
            .id("h.list")
            .orientation(Axis::Horizontal)
        }
        fn update((): &mut Self::State, (): Self::Action) {}
    }

    let mut harness = TestHarness::<Horizontal>::mount(());
    assert!(harness.publish().is_ok());
    harness
        .submit_automation_command(element_id("h.one"), SemanticCommand::RequestFocus)
        .unwrap_or_else(|error| unreachable!("focus is accepted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(budget()).outcome(),
        SettleOutcome::Idle
    );

    harness
        .submit_automation_command(element_id("h.one"), SemanticCommand::FocusDown)
        .unwrap_or_else(|error| unreachable!("vertical command is admitted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(budget()).outcome(),
        SettleOutcome::Idle
    );
    assert!(harness.publish().is_ok());
    let one = harness
        .unique_semantic_target(
            &SemanticQuery::new()
                .with_role(SemanticRole::Option)
                .with_name("One"),
        )
        .unwrap_or_else(|error| unreachable!("first Option is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    assert_eq!(snapshot.focused(), Some(one.node_id()));

    harness
        .submit_automation_command(element_id("h.one"), SemanticCommand::FocusRight)
        .unwrap_or_else(|error| unreachable!("horizontal navigation is accepted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(budget()).outcome(),
        SettleOutcome::Idle
    );
    assert!(harness.publish().is_ok());
    let two = harness
        .unique_semantic_target(
            &SemanticQuery::new()
                .with_role(SemanticRole::Option)
                .with_name("Two"),
        )
        .unwrap_or_else(|error| unreachable!("second Option is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    assert_eq!(snapshot.focused(), Some(two.node_id()));
    let list = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ListBox)
        .unwrap_or_else(|| unreachable!("ListBox is published"));
    assert_eq!(list.orientation(), Some(SemanticOrientation::Horizontal));
}
