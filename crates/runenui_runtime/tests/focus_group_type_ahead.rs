#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    CommandOrigin, EditIntent, EditableContribution, EditingSessionPolicy, Element, EventContext,
    EventPhase, FocusGroup, FocusGroupActivationPolicy, FocusGroupBoundaryPolicy,
    FocusGroupTypeAhead, FocusGroupTypeAheadError, FocusReason, FocusScope, Focusability,
    InputModality, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, NoHostProtocol, PhysicalKey, SemanticCommand, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, TextSensitivity,
    UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetTextInput, button, column,
};
use runenui_runtime::{
    AppRuntime, ManualClock, MonotonicInstant, MountedNodeId, PumpBudget, RuntimeConfig,
    RuntimeLimits, RuntimeStatus, TraceConfig, TraceRecordKind, TraceReplay,
    TraceRoutedAdmissionRejection,
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
    const fn manual() -> Self {
        Self {
            activation: FocusGroupActivationPolicy::Manual,
            activations: Vec::new(),
        }
    }

    const fn activating() -> Self {
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
            member("six", "beacon", true, false),
            member("four", "bravo", true, false),
            member("five", "delta", false, true),
            member("disabled", "echo", false, false),
            member("unicode", "Äther", true, false),
            member("composed", "éclair", true, false),
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

#[test]
fn type_ahead_policy_rejects_zero_and_monotonic_domain_overflow() {
    assert_eq!(
        FocusGroupTypeAhead::new(Duration::ZERO),
        Err(FocusGroupTypeAheadError::ZeroTimeout)
    );
    assert_eq!(
        FocusGroupTypeAhead::new(Duration::from_secs(u64::MAX)),
        Err(FocusGroupTypeAheadError::TimeoutOverflow)
    );
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
    keyboard(
        character,
        KeyModifiers::NONE,
        KeyboardCompositionState::Inactive,
    )
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
fn rapid_multi_character_prefix_narrows_only_when_unique_and_miss_retries_current_character() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
    type_character(&mut runtime, "l");
    assert_focus(&mut runtime, "one");
    type_character(&mut runtime, "p");
    assert_focus(&mut runtime, "one");
    type_character(&mut runtime, "i");
    assert_focus(&mut runtime, "two");

    type_character(&mut runtime, "b");
    assert_focus(&mut runtime, "three");
    type_character(&mut runtime, "r");
    assert_focus(&mut runtime, "four");
}

#[test]
fn multi_character_extension_prefers_the_current_matching_member() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "two");

    type_character(&mut runtime, "b");
    assert_focus(&mut runtime, "three");
    type_character(&mut runtime, "e");
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
fn ordinary_disabled_member_is_skipped_while_discoverable_disabled_remains_searchable() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");
    let beta = id(&mut runtime, "three");

    type_character(&mut runtime, "e");
    assert_eq!(runtime.focus().focused_node(), Some(&beta));

    type_character(&mut runtime, "d");
    assert_focus(&mut runtime, "five");
}

#[test]
fn matching_uses_locale_neutral_lowercase_without_canonical_normalization() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "ä");
    assert_focus(&mut runtime, "unicode");

    focus(&mut runtime, "three");
    let beta = id(&mut runtime, "three");
    type_character(&mut runtime, "e\u{301}");
    assert_eq!(runtime.focus().focused_node(), Some(&beta));

    type_character(&mut runtime, "é");
    assert_focus(&mut runtime, "composed");
}

#[test]
fn timeout_expires_at_the_exact_deadline_before_the_next_character() {
    let clock = ManualClock::new();
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    runtime.set_monotonic_clock(clock.clone());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
    let deadline = MonotonicInstant::ZERO
        .checked_add(Duration::from_millis(500))
        .unwrap_or_else(|_| unreachable!("fixture timeout deadline is representable"));
    let before = runtime.pump(PumpBudget::new(0, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(before.next_deadline(), Some(deadline));

    clock
        .advance(Duration::from_millis(500))
        .unwrap_or_else(|_| unreachable!("fixture time remains representable"));
    let expired = runtime.pump(PumpBudget::new(0, usize::MAX, usize::MAX, usize::MAX));
    assert_ne!(expired.next_deadline(), Some(deadline));

    type_character(&mut runtime, "l");
    assert_focus(&mut runtime, "one");
}

#[test]
fn unrepresentable_session_deadline_does_not_retain_the_prefix() {
    let clock = ManualClock::new();
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    runtime.set_monotonic_clock(clock.clone());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    clock
        .advance(Duration::from_nanos(u64::MAX - 250_000_000))
        .unwrap_or_else(|_| unreachable!("near-boundary fixture time remains representable"));

    type_character(&mut runtime, "b");
    assert_focus(&mut runtime, "six");
    assert_eq!(
        runtime
            .pump(PumpBudget::new(0, usize::MAX, usize::MAX, usize::MAX))
            .next_deadline(),
        None
    );

    type_character(&mut runtime, "r");
    assert_focus(&mut runtime, "six");
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn timeout_retires_private_buffer_without_requiring_another_key() {
    let clock = ManualClock::new();
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    runtime.set_monotonic_clock(clock.clone());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert!(runtime.__focus_group_type_ahead_active_for_test());

    clock
        .advance(Duration::from_millis(500))
        .unwrap_or_else(|_| unreachable!("fixture time remains representable"));
    runtime.pump(PumpBudget::new(0, usize::MAX, usize::MAX, usize::MAX));

    assert!(!runtime.__focus_group_type_ahead_active_for_test());
}

#[test]
fn queued_predeadline_character_keeps_ingress_time_order_across_timeout_wake() {
    let clock = ManualClock::new();
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    runtime.set_monotonic_clock(clock.clone());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "b");
    assert_focus(&mut runtime, "six");

    clock
        .advance(Duration::from_millis(400))
        .unwrap_or_else(|_| unreachable!("fixture time remains representable"));
    runtime
        .submit_keyboard(key("r"))
        .unwrap_or_else(|_| unreachable!("predeadline key is admitted"));
    clock
        .advance(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("fixture time remains representable"));

    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_focus(&mut runtime, "four");
}

#[test]
fn composition_control_and_meta_suppress_while_shift_and_alt_characters_participate() {
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

    for modifiers in [KeyModifiers::CONTROL, KeyModifiers::META] {
        runtime
            .submit_keyboard(keyboard("a", modifiers, KeyboardCompositionState::Inactive))
            .unwrap_or_else(|_| unreachable!("command-modified key is routed"));
        settle(&mut runtime);
        assert_eq!(runtime.focus().focused_node(), Some(&beta));
    }

    runtime
        .submit_keyboard(keyboard(
            "ä",
            KeyModifiers::ALT,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("Alt-produced character is routed"));
    settle(&mut runtime);
    assert_focus(&mut runtime, "unicode");

    let mut shifted = AppRuntime::<App>::mount(State::manual());
    settle(&mut shifted);
    focus(&mut shifted, "three");
    shifted
        .submit_keyboard(keyboard(
            "A",
            KeyModifiers::SHIFT,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("shifted character is routed"));
    settle(&mut shifted);
    assert_focus(&mut shifted, "one");
}

#[test]
fn capacity_accepts_exact_64_scalar_256_byte_boundary() {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State::manual(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(1024)),
    );
    settle(&mut runtime);
    focus(&mut runtime, "one");
    let alpha = id(&mut runtime, "one");

    let exact_boundary = "😀".repeat(64);
    assert_eq!(exact_boundary.chars().count(), 64);
    assert_eq!(exact_boundary.len(), 256);
    type_character(&mut runtime, &exact_boundary);

    assert_eq!(runtime.focus().focused_node(), Some(&alpha));
    assert!(
        !runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::FocusGroupTypeAheadCapacityRejected))
    );
}

#[test]
fn capacity_rejection_clears_existing_prefix_and_preserves_focus() {
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
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::FocusGroupTypeAheadCapacityRejected))
    );
    assert!(
        runtime
            .trace()
            .export_jsonl()
            .contains("\"focus_group_type_ahead_capacity_rejected\"")
    );

    type_character(&mut runtime, "r");
    assert_eq!(runtime.focus().focused_node(), Some(&beta));
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
fn activate_target_type_ahead_reserves_capacity_before_focus_commit() {
    const QUEUE_CAPACITY: usize = 16;
    const FILLER_ENVELOPES: usize = QUEUE_CAPACITY - 3;
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(QUEUE_CAPACITY)
        .with_transaction_outputs(1);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State::activating(),
        RuntimeConfig::default()
            .with_limits(limits)
            .with_trace_config(TraceConfig::new(1024)),
    );
    settle(&mut runtime);
    focus(&mut runtime, "three");
    let beta = id(&mut runtime, "three");

    runtime
        .submit_keyboard(key("a"))
        .unwrap_or_else(|_| unreachable!("type-ahead input is admitted"));
    for _ in 0..FILLER_ENVELOPES {
        runtime
            .submit_action(Action::Activated("filler"))
            .unwrap_or_else(|_| unreachable!("filler action is admitted"));
    }

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.focus().focused_node(), Some(&beta));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::WaitingEnvelopes
        }
    )));
}

#[test]
fn trace_export_remains_redacted_and_replay_compatible() {
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State::manual(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(1024)),
    );
    settle(&mut runtime);
    focus(&mut runtime, "three");
    type_character(&mut runtime, "a");

    let jsonl = runtime.trace().export_jsonl();
    for secret in [
        "alpha", "alpine", "beta", "beacon", "bravo", "delta", "echo", "Äther", "éclair",
    ] {
        assert!(!jsonl.contains(secret));
    }

    runtime
        .submit_keyboard(key("QzxTypeAheadSecret"))
        .unwrap_or_else(|_| unreachable!("private unmatched prefix is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    let jsonl = runtime.trace().export_jsonl();
    assert!(!jsonl.contains("QzxTypeAheadSecret"));
    assert!(!jsonl.contains("qzxtypeaheadsecret"));
    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| unreachable!("redacted type-ahead trace replays: {error}"));
    assert!(replay.is_complete());
}

#[test]
fn fresh_prefix_no_match_keeps_focus_stable_until_a_later_character_recovers() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");
    let beta = id(&mut runtime, "three");

    type_character(&mut runtime, "z");
    assert_eq!(runtime.focus().focused_node(), Some(&beta));

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn terminal_and_shutdown_clear_private_type_ahead_state() {
    let mut terminal = AppRuntime::<App>::mount(State::manual());
    settle(&mut terminal);
    focus(&mut terminal, "three");
    type_character(&mut terminal, "a");
    assert!(terminal.__focus_group_type_ahead_active_for_test());

    let target = id(&mut terminal, "one");
    terminal.__seed_next_work_sequence_for_test(0);
    assert!(
        terminal
            .submit_command(
                target,
                SemanticCommand::OpenContextMenu,
                CommandOrigin::programmatic(),
            )
            .is_err()
    );
    assert!(matches!(terminal.status(), RuntimeStatus::Terminal(_)));
    assert!(!terminal.__focus_group_type_ahead_active_for_test());

    let mut shutdown = AppRuntime::<App>::mount(State::manual());
    settle(&mut shutdown);
    focus(&mut shutdown, "three");
    type_character(&mut shutdown, "a");
    assert!(shutdown.__focus_group_type_ahead_active_for_test());
    shutdown.shutdown();
    assert!(!shutdown.__focus_group_type_ahead_active_for_test());
}

#[test]
fn type_ahead_attempt_commits_keyboard_modality_even_without_a_match() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");
    let beta = id(&mut runtime, "three");
    assert_eq!(
        runtime.focus().modality(),
        Some(InputModality::Programmatic)
    );

    type_character(&mut runtime, "z");
    assert_eq!(runtime.focus().focused_node(), Some(&beta));
    assert_eq!(runtime.focus().modality(), Some(InputModality::Keyboard));
}

#[test]
fn repeated_keyboard_events_participate_in_same_character_cycling() {
    let mut runtime = AppRuntime::<App>::mount(State::manual());
    settle(&mut runtime);
    focus(&mut runtime, "three");

    type_character(&mut runtime, "a");
    assert_focus(&mut runtime, "one");

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyA")),
            LogicalKey::Character(String::from("a")),
            KeyModifiers::NONE,
            true,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("repeat character is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_focus(&mut runtime, "two");
}

fn unit_member(id: &'static str, search: &'static str) -> Element<()> {
    button(id)
        .id(id)
        .key(id)
        .into_element()
        .with_focusability(Focusability::Focusable)
        .focus_group_search_text(search)
}

fn unit_group(id: &'static str, prefix: &'static str) -> Element<()> {
    column(vec![
        unit_member(
            match prefix {
                "a" => "a.zulu",
                "b" => "b.zulu",
                _ => unreachable!("fixture prefix is fixed"),
            },
            "zulu",
        ),
        unit_member(
            match prefix {
                "a" => "a.alpha",
                "b" => "b.alpha",
                _ => unreachable!("fixture prefix is fixed"),
            },
            "alpha",
        ),
        unit_member(
            match prefix {
                "a" => "a.alpine",
                "b" => "b.alpine",
                _ => unreachable!("fixture prefix is fixed"),
            },
            "alpine",
        ),
        unit_member(
            match prefix {
                "a" => "a.lima",
                "b" => "b.lima",
                _ => unreachable!("fixture prefix is fixed"),
            },
            "lima",
        ),
    ])
    .id(id)
    .key(id)
    .into_element()
    .focus_group(FocusGroup::new().with_type_ahead(type_ahead()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PolicyResetAction {
    Disable,
    EnableDefault,
    ChangeTimeout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PolicyResetState {
    enabled: bool,
    timeout_ms: u64,
}

struct PolicyResetApp;

impl UiApp for PolicyResetApp {
    type State = PolicyResetState;
    type Action = PolicyResetAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &PolicyResetState) -> Element<PolicyResetAction> {
        let member = |id: &'static str, search: &'static str| {
            button(id)
                .id(id)
                .key(id)
                .into_element()
                .with_focusability(Focusability::Focusable)
                .focus_group_search_text(search)
        };
        let mut group = FocusGroup::new();
        if state.enabled {
            group = group.with_type_ahead(
                FocusGroupTypeAhead::new(Duration::from_millis(state.timeout_ms))
                    .unwrap_or_else(|_| unreachable!("policy fixture timeout is bounded")),
            );
        }
        column(vec![
            member("policy.zulu", "zulu"),
            member("policy.alpha", "alpha"),
            member("policy.alpine", "alpine"),
            member("policy.lima", "lima"),
        ])
        .id("policy.group")
        .key("policy.group")
        .into_element()
        .focus_group(group)
    }

    fn update(state: &mut PolicyResetState, action: PolicyResetAction) {
        match action {
            PolicyResetAction::Disable => state.enabled = false,
            PolicyResetAction::EnableDefault => {
                state.enabled = true;
                state.timeout_ms = 500;
            }
            PolicyResetAction::ChangeTimeout => {
                state.enabled = true;
                state.timeout_ms = 700;
            }
        }
    }
}

fn policy_id(runtime: &mut AppRuntime<PolicyResetApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("policy-reset fixture node is mounted"))
        .id()
        .clone()
}

fn policy_focus(runtime: &mut AppRuntime<PolicyResetApp>, authored: &str) {
    let target = policy_id(runtime, authored);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("policy-reset focus is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

fn policy_character(runtime: &mut AppRuntime<PolicyResetApp>, character: &str) {
    runtime
        .submit_keyboard(key(character))
        .unwrap_or_else(|_| unreachable!("policy-reset keyboard input is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

fn policy_action(runtime: &mut AppRuntime<PolicyResetApp>, action: PolicyResetAction) {
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!("policy-reset action is admitted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[test]
fn disabling_or_reauthoring_type_ahead_policy_clears_the_prefix() {
    let mut runtime = AppRuntime::<PolicyResetApp>::mount(PolicyResetState {
        enabled: true,
        timeout_ms: 500,
    });
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    policy_focus(&mut runtime, "policy.zulu");
    policy_character(&mut runtime, "a");
    let alpha = policy_id(&mut runtime, "policy.alpha");
    assert_eq!(runtime.focus().focused_node(), Some(&alpha));

    policy_action(&mut runtime, PolicyResetAction::Disable);
    policy_action(&mut runtime, PolicyResetAction::EnableDefault);
    policy_character(&mut runtime, "l");
    let lima = policy_id(&mut runtime, "policy.lima");
    assert_eq!(runtime.focus().focused_node(), Some(&lima));

    policy_focus(&mut runtime, "policy.zulu");
    policy_character(&mut runtime, "a");
    policy_action(&mut runtime, PolicyResetAction::ChangeTimeout);
    policy_character(&mut runtime, "l");
    let lima = policy_id(&mut runtime, "policy.lima");
    assert_eq!(runtime.focus().focused_node(), Some(&lima));
}

struct ResetBoundaryApp;

impl UiApp for ResetBoundaryApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<()> {
        column(vec![
            unit_group("group.a", "a"),
            unit_group("group.b", "b"),
            unit_member("outside", "outside"),
        ])
        .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

fn reset_id(runtime: &mut AppRuntime<ResetBoundaryApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("reset-boundary fixture node is mounted"))
        .id()
        .clone()
}

fn reset_focus(runtime: &mut AppRuntime<ResetBoundaryApp>, authored: &str) {
    let target = reset_id(runtime, authored);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("reset-boundary focus is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

fn reset_character(runtime: &mut AppRuntime<ResetBoundaryApp>, character: &str) {
    runtime
        .submit_keyboard(key(character))
        .unwrap_or_else(|_| unreachable!("reset-boundary keyboard input is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

#[test]
fn group_transfer_and_focus_departure_clear_the_exact_group_buffer() {
    let mut runtime = AppRuntime::<ResetBoundaryApp>::mount(());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    reset_focus(&mut runtime, "a.zulu");
    reset_character(&mut runtime, "a");
    let a_alpha = reset_id(&mut runtime, "a.alpha");
    assert_eq!(runtime.focus().focused_node(), Some(&a_alpha));

    reset_focus(&mut runtime, "b.zulu");
    reset_character(&mut runtime, "l");
    let b_lima = reset_id(&mut runtime, "b.lima");
    assert_eq!(runtime.focus().focused_node(), Some(&b_lima));

    reset_focus(&mut runtime, "a.zulu");
    reset_character(&mut runtime, "a");
    reset_focus(&mut runtime, "outside");
    reset_focus(&mut runtime, "a.zulu");
    reset_character(&mut runtime, "l");
    let a_lima = reset_id(&mut runtime, "a.lima");
    assert_eq!(runtime.focus().focused_node(), Some(&a_lima));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReplacementAction {
    Replace,
}

struct ReplacementApp;

impl UiApp for ReplacementApp {
    type State = bool;
    type Action = ReplacementAction;
    type HostProtocol = NoHostProtocol;

    fn root(replaced: &bool) -> Element<ReplacementAction> {
        let member = |id: &'static str, search: &'static str| {
            button(id)
                .id(id)
                .key(id)
                .into_element()
                .with_focusability(Focusability::Focusable)
                .focus_group_search_text(search)
        };
        column(vec![
            member("replace.zulu", "zulu"),
            member("replace.alpha", "alpha"),
            member("replace.alpine", "alpine"),
            member("replace.lima", "lima"),
        ])
        .id("replace.group")
        .key(if *replaced {
            "replace.group.v2"
        } else {
            "replace.group.v1"
        })
        .into_element()
        .focus_group(FocusGroup::new().with_type_ahead(type_ahead()))
    }

    fn update(state: &mut bool, ReplacementAction::Replace: ReplacementAction) {
        *state = true;
    }
}

fn replacement_id(runtime: &mut AppRuntime<ReplacementApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("replacement fixture node is mounted"))
        .id()
        .clone()
}

fn replacement_focus(runtime: &mut AppRuntime<ReplacementApp>, authored: &str) {
    let target = replacement_id(runtime, authored);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("replacement focus is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

fn replacement_character(runtime: &mut AppRuntime<ReplacementApp>, character: &str) {
    runtime
        .submit_keyboard(key(character))
        .unwrap_or_else(|_| unreachable!("replacement keyboard input is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

#[test]
fn owner_replacement_retires_buffer_with_the_old_exact_group_lifetime() {
    let mut runtime = AppRuntime::<ReplacementApp>::mount(false);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    replacement_focus(&mut runtime, "replace.zulu");
    replacement_character(&mut runtime, "a");

    runtime
        .submit_action(ReplacementAction::Replace)
        .unwrap_or_else(|_| unreachable!("replacement action is admitted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    replacement_focus(&mut runtime, "replace.zulu");
    replacement_character(&mut runtime, "l");
    let replacement_lima = replacement_id(&mut runtime, "replace.lima");
    assert_eq!(runtime.focus().focused_node(), Some(&replacement_lima));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RemovalAction {
    Remove,
    Restore,
}

struct RemovalApp;

impl UiApp for RemovalApp {
    type State = bool;
    type Action = RemovalAction;
    type HostProtocol = NoHostProtocol;

    fn root(present: &bool) -> Element<RemovalAction> {
        if !*present {
            return button("outside")
                .id("removal.outside")
                .key("removal.outside")
                .into_element()
                .with_focusability(Focusability::Focusable);
        }
        let member = |id: &'static str, search: &'static str| {
            button(id)
                .id(id)
                .key(id)
                .into_element()
                .with_focusability(Focusability::Focusable)
                .focus_group_search_text(search)
        };
        column(vec![
            member("removal.zulu", "zulu"),
            member("removal.alpha", "alpha"),
            member("removal.alpine", "alpine"),
            member("removal.lima", "lima"),
        ])
        .id("removal.group")
        .key("removal.group")
        .into_element()
        .focus_group(FocusGroup::new().with_type_ahead(type_ahead()))
    }

    fn update(state: &mut bool, action: RemovalAction) {
        *state = matches!(action, RemovalAction::Restore);
    }
}

fn removal_id(runtime: &mut AppRuntime<RemovalApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("removal fixture node is mounted"))
        .id()
        .clone()
}

fn removal_focus(runtime: &mut AppRuntime<RemovalApp>, authored: &str) {
    let target = removal_id(runtime, authored);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("removal focus is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

fn removal_character(runtime: &mut AppRuntime<RemovalApp>, character: &str) {
    runtime
        .submit_keyboard(key(character))
        .unwrap_or_else(|_| unreachable!("removal keyboard input is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

#[test]
fn owner_removal_and_recreation_cannot_revive_the_old_group_prefix() {
    let mut runtime = AppRuntime::<RemovalApp>::mount(true);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    removal_focus(&mut runtime, "removal.zulu");
    removal_character(&mut runtime, "a");
    let alpha = removal_id(&mut runtime, "removal.alpha");
    assert_eq!(runtime.focus().focused_node(), Some(&alpha));

    runtime
        .submit_action(RemovalAction::Remove)
        .unwrap_or_else(|_| unreachable!("removal action is admitted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(runtime.focus().focused_node(), None);

    runtime
        .submit_action(RemovalAction::Restore)
        .unwrap_or_else(|_| unreachable!("restore action is admitted"));
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    removal_focus(&mut runtime, "removal.zulu");
    removal_character(&mut runtime, "l");
    let lima = removal_id(&mut runtime, "removal.lima");
    assert_eq!(runtime.focus().focused_node(), Some(&lima));
}

struct NestedBoundaryApp;

impl UiApp for NestedBoundaryApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<()> {
        let nested = column(vec![
            unit_member("nested.first", "inner"),
            unit_member("nested.second", "second"),
        ])
        .id("nested.group")
        .key("nested.group")
        .into_element()
        .focus_group_search_text("nested")
        .focus_group(FocusGroup::new());

        let scoped = column(vec![unit_member("scoped.member", "scope")])
            .id("nested.scope")
            .key("nested.scope")
            .into_element()
            .focus_scope(FocusScope::new());

        column(vec![
            unit_member("outer.current", "zulu"),
            nested,
            scoped,
            unit_member("outer.after", "after"),
        ])
        .id("outer.group")
        .key("outer.group")
        .into_element()
        .focus_group(FocusGroup::new().with_type_ahead(type_ahead()))
    }

    fn update((): &mut (), (): ()) {}
}

fn nested_id(runtime: &mut AppRuntime<NestedBoundaryApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("nested fixture node is mounted"))
        .id()
        .clone()
}

fn nested_focus(runtime: &mut AppRuntime<NestedBoundaryApp>, authored: &str) {
    let target = nested_id(runtime, authored);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("nested fixture focus is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

#[test]
fn nested_group_search_enters_existing_target_and_nested_scope_is_not_searchable() {
    let mut nested = AppRuntime::<NestedBoundaryApp>::mount(());
    nested.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    nested_focus(&mut nested, "outer.current");
    nested
        .submit_keyboard(key("n"))
        .unwrap_or_else(|_| unreachable!("nested-group search is admitted"));
    nested.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    let nested_first = nested_id(&mut nested, "nested.first");
    assert_eq!(nested.focus().focused_node(), Some(&nested_first));

    let mut scoped = AppRuntime::<NestedBoundaryApp>::mount(());
    scoped.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    nested_focus(&mut scoped, "outer.current");
    let current = nested_id(&mut scoped, "outer.current");
    scoped
        .submit_keyboard(key("s"))
        .unwrap_or_else(|_| unreachable!("scope-boundary search is admitted"));
    scoped.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(scoped.focus().focused_node(), Some(&current));
}

#[derive(Debug)]
struct EditableTypeAheadProbe;

impl Widget<()> for EditableTypeAheadProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<()>> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(1), TextDocumentRevision::new(1));
        let position =
            TextPosition::new(snapshot, "ab", 2, runenui_core::TextAffinity::Downstream).ok()?;
        EditableContribution::new(
            snapshot,
            "ab",
            TextSelection::collapsed(position),
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |_: EditIntent| (),
        )
        .ok()
    }
}

struct EditableTypeAheadApp;

impl UiApp for EditableTypeAheadApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<()> {
        column(vec![
            Element::new(EditableTypeAheadProbe)
                .id("editable.current")
                .key("editable.current")
                .with_focusability(Focusability::Focusable)
                .focus_group_search_text("alpha"),
            unit_member("editable.other", "beta"),
        ])
        .id("editable.group")
        .key("editable.group")
        .into_element()
        .focus_group(FocusGroup::new().with_type_ahead(type_ahead()))
    }

    fn update((): &mut (), (): ()) {}
}

fn editable_id(runtime: &mut AppRuntime<EditableTypeAheadApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("editable type-ahead fixture node is mounted"))
        .id()
        .clone()
}

#[test]
fn editable_owner_keeps_printable_keyboard_precedence_over_type_ahead() {
    let mut runtime = AppRuntime::<EditableTypeAheadApp>::mount(());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let current = editable_id(&mut runtime, "editable.current");
    let other = editable_id(&mut runtime, "editable.other");
    runtime
        .submit_command(
            current.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("editable owner focus is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    runtime
        .submit_keyboard(key("b"))
        .unwrap_or_else(|_| unreachable!("editable keyboard input is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(runtime.focus().focused_node(), Some(&current));
    assert_ne!(runtime.focus().focused_node(), Some(&other));
}

#[derive(Debug)]
struct PreventingMember {
    prevent: bool,
}

impl Widget<()> for PreventingMember {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if self.prevent && context.phase() == EventPhase::Target && event.as_keyboard().is_some() {
            context.prevent_default();
        }
        WidgetEventOutput::none()
    }
}

struct PreventApp;

impl UiApp for PreventApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<()> {
        column(vec![
            Element::new(PreventingMember { prevent: true })
                .id("prevent.a")
                .with_focusability(Focusability::FocusableWhenDisabled)
                .focus_group_search_text("alpha"),
            Element::new(PreventingMember { prevent: false })
                .id("prevent.b")
                .with_focusability(Focusability::FocusableWhenDisabled)
                .focus_group_search_text("beta"),
        ])
        .id("prevent.group")
        .into_element()
        .focus_group(
            FocusGroup::new().with_type_ahead(
                FocusGroupTypeAhead::new(Duration::from_millis(500))
                    .unwrap_or_else(|_| unreachable!("fixture timeout is bounded")),
            ),
        )
    }

    fn update((): &mut (), (): ()) {}
}

fn prevent_id(runtime: &mut AppRuntime<PreventApp>, authored: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("prevent-default fixture node is mounted"))
        .id()
        .clone()
}

#[test]
fn routed_prevent_default_suppresses_type_ahead_before_buffer_or_focus_change() {
    let mut runtime = AppRuntime::<PreventApp>::mount(());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let a = prevent_id(&mut runtime, "prevent.a");
    let b = prevent_id(&mut runtime, "prevent.b");
    runtime
        .submit_command(
            a.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("fixture focus is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    runtime
        .submit_keyboard(key("b"))
        .unwrap_or_else(|_| unreachable!("keyboard input is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(runtime.focus().focused_node(), Some(&a));
    assert_ne!(runtime.focus().focused_node(), Some(&b));
    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("keyboard_default_prevented"));
}
