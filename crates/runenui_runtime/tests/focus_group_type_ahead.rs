#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    CommandOrigin, Element, FocusGroup, FocusGroupActivationPolicy, FocusGroupBoundaryPolicy,
    FocusGroupTypeAhead, FocusReason, Focusability, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, UiApp, View, button, column,
};
use runenui_runtime::{
    AppRuntime, ManualClock, MountedNodeId, PumpBudget, RuntimeConfig, TraceConfig,
    TraceRecordKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Activated(&'static str),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    activation: FocusGroupActivationPolicy,
    activations: Vec<&'static str>,
}

impl State {
    fn manual() -> Self {
        Self {
            activation: FocusGroupActivationPolicy::Manual,
            activations: Vec::new(),
        }
    }

    fn activating() -> Self {
        Self {
            activation: FocusGroupActivationPolicy::ActivateTarget,
            activations: Vec::new(),
        }
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> Element<Action> {
        column(vec![
            member("one", "alpha", true, false),
            member("two", "alpine", true, false),
            member("three", "beta", true, false),
            member("four", "bravo", true, false),
            member("five", "delta", false, true),
        ])
        .id("group")
        .key("group")
        .into_element()
        .focus_group(
            FocusGroup::new()
                .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                .with_activation(state.activation)
                .with_type_ahead(type_ahead()),
        )
    }

    fn update(state: &mut State, action: Action) {
        let Action::Activated(name) = action;
        state.activations.push(name);
    }
}

fn type_ahead() -> FocusGroupTypeAhead {
    FocusGroupTypeAhead::new(Duration::from_millis(500))
        .unwrap_or_else(|_| unreachable!("fixture timeout is bounded"))
}

fn member(
    id: &'static str,
    search: &'static str,
    enabled: bool,
    discoverable_when_disabled: bool,
) -> Element<Action> {
    let mut control = button(id)
        .id(id)
        .key(id)
        .on_activate(move || Action::Activated(id));
    if !enabled {
        control = control.disabled();
    }
    let mut element = control.into_element().focus_group_search_text(search);
    if discoverable_when_disabled {
        element = element.with_focusability(Focusability::FocusableWhenDisabled);
    }
    element
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn id(runtime: &mut AppRuntime<App>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("fixture node is mounted"))
        .id()
        .clone()
}

fn focus(runtime: &mut AppRuntime<App>, authored: &str) {
    let target = id(runtime, authored);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn assert_focus(runtime: &mut AppRuntime<App>, authored: &str) {
    let expected = id(runtime, authored);
    assert_eq!(runtime.focus().focused_node(), Some(&expected));
}

fn key(character: &str) -> KeyboardEvent {
    keyboard(character, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
}

fn keyboard(
    character: &str,
    modifiers: KeyModifiers,
    composition: KeyboardCompositionState,
) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("KeyX")),
        LogicalKey::Character(character.to_owned()),
        modifiers,
        false,
        KeyLocation::Standard,
        composition,
        None,
    )
}

fn type_character(runtime: &mut AppRuntime<App>, character: &str) {
    runtime
        .submit_keyboard(key(character))
        .unwrap_or_else(|_| unreachable!("focused keyboard input is admitted"));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

#[test]
fn first_character_prefix_repeated_character_and_wrap_share_group_order() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
    assert_eq!(runtime.focus().reason(), Some(FocusReason::GroupNavigation));

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "two");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
}

#[test]
fn rapid_multi_character_prefix_narrows_and_combined_miss_retries_fresh_character() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
    type_character(&mut runtime, "l");
    assert_focus(&mut runtime, "two");

    type_character(&mut runtime, "b");
    assert_focus(&mut runtime, "three");
}

#[test]
fn disabled_discoverable_member_uses_canonical_focus_eligibility() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "d");
    let disabled = id(&mut runtime, "five");
    assert_eq!(runtime.focus().focused_node(), Some(&disabled));
    assert!(
        runtime
            .index()
            .node(&disabled)
            .is_some_and(runenui_runtime::MountedNodeRef::is_focusable)
    );
    assert!(
        !runtime
            .index()
            .node(&disabled)
            .unwrap_or_else(|| unreachable!("disabled member remains mounted"))
            .activation()
            .enabled()
    );
}

#[test]
fn timeout_resets_prefix_before_the_next_character() {
    let clock = ManualClock::new();
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    runtime.set_monotonic_clock(clock.clone());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
    clock
        .advance(Duration::from_millis(501))
        .unwrap_or_else(|_| unreachable!("fixture time remains representable"));
    type_character(&mut runtime, "l");
    assert_focus(&mut runtime, "one");
}

#[test]
fn composition_and_command_modifiers_suppress_type_ahead_but_shift_is_permitted() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");
    let beta = id(&mut runtime, "three");

    runtime
        .submit_keyboard(keyboard(
            "a",
            KeyModifiers::NONE,
            KeyboardCompositionState::Active,
        ))
        .unwrap_or_else(|_| unreachable!("composition-associated key is routed"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&beta));

    runtime
        .submit_keyboard(keyboard(
            "a",
            KeyModifiers::CONTROL,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("modified key is routed"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&beta));

    runtime
        .submit_keyboard(keyboard(
            "A",
            KeyModifiers::SHIFT,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("shifted character is routed"));
    settle(&mut runtime);
    assert_focus(&mut runtime, "one");
}

#[test]
fn capacity_rejection_preserves_existing_prefix_and_focus() {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State::manual(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(1024)),
    );
    settle(&mut runtime);
    focus(&mut runtime, "one");

    type_character(&mut runtime, "b");
    let beta = id(&mut runtime, "three");
    assert_eq!(runtime.focus().focused_node(), Some(&beta));

    let oversized = "x".repeat(65);
    type_character(&mut runtime, &oversized);
    assert_eq!(runtime.focus().focused_node(), Some(&beta));
    assert!(runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::FocusGroupTypeAheadEvaluated {
                capacity_rejected: true,
                ..
            }
        )
    }));

    type_character(&mut runtime, "r");
    assert_focus(&mut runtime, "four");
}

#[test]
fn activate_target_remains_deferred_until_after_focus_transition() {
    let mut runtime = AppRuntime::<App>::mount(State::activating());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
    assert!(runtime.state().activations.is_empty());

    settle(&mut runtime);
    assert_eq!(runtime.state().activations, vec!["one"]);
}

#[test]
fn trace_exports_only_bounded_type_ahead_observation_not_search_text() {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State::manual(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(1024)),
    );
    settle(&mut runtime);
    focus(&mut runtime, "three");
    type_character(&mut runtime, "a");

    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("focus_group_type_ahead_evaluated"));
    assert!(jsonl.contains("buffer_scalars"));
    for secret in ["alpha", "alpine", "beta", "bravo", "delta"] {
        assert!(!jsonl.contains(secret));
    }
}
