#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc, time::Duration};

use runenui_core::{
    ApplicationCommand, ApplicationCommandId, ChildBearingWidget, CommandOrigin, EditIntent,
    EditableContribution, EditingSessionPolicy, Element, EventContext, EventPhase, FocusGroup,
    FocusGroupTypeAhead, Focusability, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol, PhysicalKey, SemanticCommand,
    ShortcutBinding, ShortcutChord, ShortcutRepeatPolicy, StyleEnvironment, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, TextSensitivity,
    UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetTextInput, button, column,
    command_binding, command_scope, container, shortcut_scope,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    SurfaceBuildContext, TraceApplicationCommandOutcome, TraceRecordKind,
    TraceRoutedAdmissionRejection, TraceTargetRejection,
};

const SHORTCUT_DIAGNOSTIC: &str = "runenui.shortcut-scope.duplicate-chord";

fn command_id(value: &'static str) -> ApplicationCommandId {
    ApplicationCommandId::from_static(value)
        .unwrap_or_else(|_| unreachable!("fixture command id is valid"))
}

fn shortcut(
    chord: ShortcutChord,
    repeat_policy: ShortcutRepeatPolicy,
    command: &'static str,
    enabled: bool,
) -> ShortcutBinding {
    ShortcutBinding::new(
        chord,
        repeat_policy,
        ApplicationCommand::new(command_id(command), enabled),
    )
}

fn logical_chord(character: &str, modifiers: KeyModifiers) -> ShortcutChord {
    ShortcutChord::logical(LogicalKey::Character(character.to_owned()), modifiers)
}

fn key(
    logical: &str,
    physical: &str,
    modifiers: KeyModifiers,
    repeated: bool,
    composition: KeyboardCompositionState,
) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(physical.to_owned()),
        LogicalKey::Character(logical.to_owned()),
        modifiers,
        repeated,
        KeyLocation::Standard,
        composition,
        None,
    )
}

fn settle<App: UiApp>(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Unique,
    OuterOnly,
    Ambiguous,
    Disabled,
    AllowRepeat,
    PreventDefault,
}

#[derive(Debug)]
enum Action {
    Fired(&'static str),
    SetMode(Mode),
    Replace,
    Filler,
}

#[derive(Debug)]
struct State {
    mode: Mode,
    fired: Vec<&'static str>,
    replaced: bool,
    callback_calls: Rc<Cell<usize>>,
}

impl State {
    fn new(mode: Mode) -> Self {
        Self {
            mode,
            fired: Vec::new(),
            replaced: false,
            callback_calls: Rc::new(Cell::new(0)),
        }
    }
}

#[derive(Debug)]
struct KeyboardGate {
    prevent_default: bool,
    calls: Rc<Cell<usize>>,
}

impl Widget<Action> for KeyboardGate {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Bubble && matches!(event, UiEvent::Keyboard(_)) {
            self.calls.set(self.calls.get() + 1);
            if self.prevent_default {
                context.prevent_default();
            }
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for KeyboardGate {}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> Element<Action> {
        let control = KeyModifiers::NONE.with_control();
        let outer = shortcut(
            logical_chord("s", control),
            ShortcutRepeatPolicy::AllowRepeat,
            "shortcut.outer",
            true,
        );
        let mut inner = match state.mode {
            Mode::OuterOnly => Vec::new(),
            Mode::Unique | Mode::PreventDefault => vec![shortcut(
                logical_chord("s", control),
                ShortcutRepeatPolicy::IgnoreRepeat,
                "shortcut.inner",
                true,
            )],
            Mode::Ambiguous => vec![
                shortcut(
                    logical_chord("s", control),
                    ShortcutRepeatPolicy::IgnoreRepeat,
                    "shortcut.inner",
                    true,
                ),
                shortcut(
                    logical_chord("s", control),
                    ShortcutRepeatPolicy::IgnoreRepeat,
                    "shortcut.alternate",
                    true,
                ),
            ],
            Mode::Disabled => vec![shortcut(
                logical_chord("s", control),
                ShortcutRepeatPolicy::AllowRepeat,
                "shortcut.inner",
                false,
            )],
            Mode::AllowRepeat => vec![shortcut(
                logical_chord("s", control),
                ShortcutRepeatPolicy::AllowRepeat,
                "shortcut.inner",
                true,
            )],
        };
        inner.push(shortcut(
            logical_chord("l", control),
            ShortcutRepeatPolicy::IgnoreRepeat,
            "shortcut.logical",
            true,
        ));
        inner.push(shortcut(
            ShortcutChord::physical(PhysicalKey::Code(String::from("KeyP")), control),
            ShortcutRepeatPolicy::IgnoreRepeat,
            "shortcut.physical",
            true,
        ));

        let target = button("target")
            .id("target.a")
            .key(if state.replaced {
                "target.a.replaced"
            } else {
                "target.a.original"
            })
            .on_activate(|| Action::Filler)
            .into_element();
        let gated = container(
            KeyboardGate {
                prevent_default: state.mode == Mode::PreventDefault,
                calls: Rc::clone(&state.callback_calls),
            },
            [target],
        )
        .key("keyboard-gate")
        .into_element();
        let branch_a = command_scope(
            vec![
                command_binding(
                    ApplicationCommand::new(command_id("shortcut.outer"), true),
                    || Action::Fired("outer"),
                ),
                command_binding(
                    ApplicationCommand::new(command_id("shortcut.inner"), true),
                    || Action::Fired("inner"),
                ),
                command_binding(
                    ApplicationCommand::new(command_id("shortcut.alternate"), true),
                    || Action::Fired("alternate"),
                ),
                command_binding(
                    ApplicationCommand::new(command_id("shortcut.logical"), true),
                    || Action::Fired("logical"),
                ),
                command_binding(
                    ApplicationCommand::new(command_id("shortcut.physical"), true),
                    || Action::Fired("physical"),
                ),
            ],
            [shortcut_scope(
                [outer],
                [shortcut_scope(inner, [gated]).key("shortcut.inner")],
            )
            .key("shortcut.outer")],
        )
        .key("command.a")
        .into_element();

        let branch_b = command_scope(
            [command_binding(
                ApplicationCommand::new(command_id("shortcut.inner"), true),
                || Action::Fired("retargeted-b"),
            )],
            [button("other")
                .id("target.b")
                .key("target.b")
                .on_activate(|| Action::Filler)],
        )
        .key("command.b")
        .into_element();

        column(vec![branch_a, branch_b]).key("root").into_element()
    }

    fn update(state: &mut State, action: Action) {
        match action {
            Action::Fired(value) => state.fired.push(value),
            Action::SetMode(mode) => state.mode = mode,
            Action::Replace => state.replaced = true,
            Action::Filler => {}
        }
    }
}

fn target(runtime: &mut AppRuntime<App>, authored: &'static str) -> runenui_runtime::MountedNodeId {
    let authored = runenui_core::ElementId::from_static(authored)
        .unwrap_or_else(|_| unreachable!("fixture authored id is valid"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("fixture target is mounted"))
        .id()
        .clone()
}

fn focus(runtime: &mut AppRuntime<App>, authored: &'static str) {
    let target = target(runtime, authored);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("fixture focus request is accepted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

fn submit_shortcut(
    runtime: &mut AppRuntime<App>,
    event: KeyboardEvent,
) -> runenui_core::WorkSequence {
    runtime
        .submit_keyboard(event)
        .unwrap_or_else(|_| unreachable!("focused keyboard event is accepted"))
        .sequence()
}

fn has_trace(runtime: &AppRuntime<App>, kind: fn(&TraceRecordKind) -> bool) -> bool {
    runtime.trace().records().any(|record| kind(record.kind()))
}

#[test]
fn nearest_scope_wins_and_missing_inner_declaration_falls_back_to_outer() {
    let control = KeyModifiers::NONE.with_control();

    let mut inner = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut inner);
    focus(&mut inner, "target.a");
    submit_shortcut(
        &mut inner,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut inner);
    assert_eq!(inner.state().fired, ["inner"]);

    let mut outer = AppRuntime::<App>::mount(State::new(Mode::OuterOnly));
    settle(&mut outer);
    focus(&mut outer, "target.a");
    submit_shortcut(
        &mut outer,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut outer);
    assert_eq!(outer.state().fired, ["outer"]);
}

#[test]
fn ambiguous_and_disabled_nearest_declarations_fail_closed_without_outer_fallback() {
    let control = KeyModifiers::NONE.with_control();
    for mode in [Mode::Ambiguous, Mode::Disabled] {
        let mut runtime = AppRuntime::<App>::mount(State::new(mode));
        settle(&mut runtime);
        focus(&mut runtime, "target.a");
        submit_shortcut(
            &mut runtime,
            key(
                "s",
                "KeyS",
                control,
                false,
                KeyboardCompositionState::Inactive,
            ),
        );
        settle(&mut runtime);
        assert!(runtime.state().fired.is_empty());
        assert!(runtime.trace().records().any(|record| match mode {
            Mode::Ambiguous => matches!(record.kind(), TraceRecordKind::KeyboardShortcutAmbiguous),
            Mode::Disabled => matches!(record.kind(), TraceRecordKind::KeyboardShortcutDisabled),
            _ => unreachable!("fixture iterates only fail-closed modes"),
        }));
    }
}

#[test]
fn exact_modifiers_and_logical_physical_identity_are_distinct() {
    let control = KeyModifiers::NONE.with_control();
    let mut runtime = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut runtime);
    focus(&mut runtime, "target.a");

    submit_shortcut(
        &mut runtime,
        key(
            "s",
            "KeyS",
            KeyModifiers::NONE,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut runtime);
    assert!(runtime.state().fired.is_empty());

    submit_shortcut(
        &mut runtime,
        key(
            "s",
            "KeyS",
            control.with_shift(),
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut runtime);
    assert!(runtime.state().fired.is_empty());

    submit_shortcut(
        &mut runtime,
        key(
            "l",
            "KeyX",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut runtime);
    assert_eq!(runtime.state().fired, ["logical"]);

    submit_shortcut(
        &mut runtime,
        key(
            "x",
            "KeyP",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut runtime);
    assert_eq!(runtime.state().fired, ["logical", "physical"]);
}

#[test]
fn repeat_policy_and_composition_suppress_without_outer_fallback() {
    let control = KeyModifiers::NONE.with_control();

    let mut ignored = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut ignored);
    focus(&mut ignored, "target.a");
    submit_shortcut(
        &mut ignored,
        key(
            "s",
            "KeyS",
            control,
            true,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut ignored);
    assert!(ignored.state().fired.is_empty());
    assert!(has_trace(&ignored, |kind| matches!(
        kind,
        TraceRecordKind::KeyboardShortcutRepeatSuppressed
    )));

    let mut allowed = AppRuntime::<App>::mount(State::new(Mode::AllowRepeat));
    settle(&mut allowed);
    focus(&mut allowed, "target.a");
    submit_shortcut(
        &mut allowed,
        key(
            "s",
            "KeyS",
            control,
            true,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut allowed);
    assert_eq!(allowed.state().fired, ["inner"]);

    let mut composing = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut composing);
    focus(&mut composing, "target.a");
    submit_shortcut(
        &mut composing,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Active,
        ),
    );
    settle(&mut composing);
    assert!(composing.state().fired.is_empty());
    assert!(has_trace(&composing, |kind| matches!(
        kind,
        TraceRecordKind::KeyboardShortcutCompositionSuppressed
    )));

    let mut ambiguous_composing = AppRuntime::<App>::mount(State::new(Mode::Ambiguous));
    settle(&mut ambiguous_composing);
    focus(&mut ambiguous_composing, "target.a");
    submit_shortcut(
        &mut ambiguous_composing,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Active,
        ),
    );
    settle(&mut ambiguous_composing);
    assert!(ambiguous_composing.state().fired.is_empty());
    assert!(has_trace(&ambiguous_composing, |kind| matches!(
        kind,
        TraceRecordKind::KeyboardShortcutCompositionSuppressed
    )));
    assert!(!has_trace(&ambiguous_composing, |kind| matches!(
        kind,
        TraceRecordKind::KeyboardShortcutAmbiguous
    )));
}

#[test]
fn routed_prevent_default_suppresses_accelerator_after_callbacks() {
    let control = KeyModifiers::NONE.with_control();
    let mut runtime = AppRuntime::<App>::mount(State::new(Mode::PreventDefault));
    settle(&mut runtime);
    focus(&mut runtime, "target.a");

    submit_shortcut(
        &mut runtime,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    settle(&mut runtime);

    assert_eq!(runtime.state().callback_calls.get(), 1);
    assert!(runtime.state().fired.is_empty());
    assert!(has_trace(&runtime, |kind| matches!(
        kind,
        TraceRecordKind::KeyboardDefaultPrevented
    )));
    assert!(!has_trace(&runtime, |kind| matches!(
        kind,
        TraceRecordKind::KeyboardShortcutMatched
    )));
}

#[test]
fn accelerator_emits_existing_application_command_non_reentrantly() {
    let control = KeyModifiers::NONE.with_control();
    let mut runtime = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut runtime);
    focus(&mut runtime, "target.a");

    submit_shortcut(
        &mut runtime,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert!(
        runtime.state().fired.is_empty(),
        "keyboard default only queues the command"
    );
    assert!(has_trace(&runtime, |kind| matches!(
        kind,
        TraceRecordKind::KeyboardShortcutMatched
    )));

    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert!(
        runtime.state().fired.is_empty(),
        "resolved command action remains queued"
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::ApplicationCommandResolution {
            outcome: TraceApplicationCommandOutcome::Resolved
        }
    )));

    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().fired, ["inner"]);
}

#[test]
fn queued_target_is_not_retargeted_after_focus_transfer() {
    let control = KeyModifiers::NONE.with_control();
    let mut runtime = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut runtime);
    focus(&mut runtime, "target.a");
    let other = target(&mut runtime, "target.b");

    runtime
        .submit_command(
            other.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus transfer is admitted"));
    submit_shortcut(
        &mut runtime,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );

    settle(&mut runtime);

    assert_eq!(runtime.focus().focused_node(), Some(&other));
    assert_eq!(
        runtime.state().fired,
        ["inner"],
        "queued accelerator command must still resolve through target A"
    );
}

#[test]
fn replacement_after_keyboard_processing_makes_queued_command_stale_without_retargeting() {
    let control = KeyModifiers::NONE.with_control();
    let mut runtime = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut runtime);
    focus(&mut runtime, "target.a");

    submit_shortcut(
        &mut runtime,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!("replacement action is admitted"));

    settle(&mut runtime);

    assert!(runtime.state().fired.is_empty());
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::CommandProcessingRejected {
            outcome: TraceTargetRejection::Stale
        }
    )));
}

#[test]
fn shortcut_waiting_queue_admission_rejects_before_callbacks_or_partial_output() {
    let control = KeyModifiers::NONE.with_control();
    const QUEUE_CAPACITY: usize = 8;
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(QUEUE_CAPACITY)
        .with_transaction_outputs(1);
    let mut queue_limited = AppRuntime::<App>::mount_with_config(
        State::new(Mode::Unique),
        RuntimeConfig::default().with_limits(limits),
    );
    settle(&mut queue_limited);
    focus(&mut queue_limited, "target.a");
    queue_limited.state().callback_calls.set(0);
    submit_shortcut(
        &mut queue_limited,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    for _ in 0..(QUEUE_CAPACITY - 1) {
        queue_limited
            .submit_action(Action::Filler)
            .unwrap_or_else(|_| unreachable!("filler action is admitted"));
    }
    queue_limited.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(queue_limited.state().callback_calls.get(), 0);
    assert!(queue_limited.state().fired.is_empty());
    assert_eq!(queue_limited.status(), RuntimeStatus::Running);
    assert!(!queue_limited.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::KeyboardShortcutMatched
            | TraceRecordKind::ApplicationCommandSubmissionAccepted { .. }
    )));
    assert!(queue_limited.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::WaitingEnvelopes
        }
    )));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn shortcut_trace_exhaustion_rejects_before_keyboard_callback() {
    let control = KeyModifiers::NONE.with_control();
    let mut runtime = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut runtime);
    focus(&mut runtime, "target.a");
    runtime.state().callback_calls.set(0);
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 2);

    submit_shortcut(
        &mut runtime,
        key(
            "s",
            "KeyS",
            control,
            false,
            KeyboardCompositionState::Inactive,
        ),
    );
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(runtime.state().callback_calls.get(), 0);
    assert!(runtime.state().fired.is_empty());
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::KeyboardShortcutMatched
            | TraceRecordKind::ApplicationCommandSubmissionAccepted { .. }
    )));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(runenui_runtime::RuntimeTerminalReason::TraceSequenceExhausted)
    );
}

#[test]
fn retained_shortcut_scope_diagnostics_update_unique_duplicate_unique() {
    let mut runtime = AppRuntime::<App>::mount(State::new(Mode::Unique));
    settle(&mut runtime);
    let style = StyleEnvironment::default();

    let duplicate_count = |runtime: &mut AppRuntime<App>| {
        runtime
            .publish_surface(&SurfaceBuildContext::new(
                &style,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("diagnostic publication is admitted"))
            .frame()
            .nodes()
            .iter()
            .flat_map(runenui_runtime::SurfaceNode::diagnostics)
            .filter(|diagnostic| diagnostic.code() == SHORTCUT_DIAGNOSTIC)
            .count()
    };

    assert_eq!(duplicate_count(&mut runtime), 0);

    runtime
        .submit_action(Action::SetMode(Mode::Ambiguous))
        .unwrap_or_else(|_| unreachable!("duplicate mode action is admitted"));
    settle(&mut runtime);
    assert_eq!(duplicate_count(&mut runtime), 1);

    runtime
        .submit_action(Action::SetMode(Mode::Unique))
        .unwrap_or_else(|_| unreachable!("unique mode action is admitted"));
    settle(&mut runtime);
    assert_eq!(duplicate_count(&mut runtime), 0);
}

#[derive(Debug)]
enum TypeAheadAction {
    Shortcut,
}

#[derive(Debug, Default)]
struct TypeAheadState {
    shortcuts: usize,
}

struct TypeAheadApp;

impl UiApp for TypeAheadApp {
    type State = TypeAheadState;
    type Action = TypeAheadAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &TypeAheadState) -> Element<TypeAheadAction> {
        let command = command_id("typeahead.shortcut");
        let type_ahead = FocusGroupTypeAhead::new(Duration::from_millis(500))
            .unwrap_or_else(|_| unreachable!("fixture timeout is bounded"));
        let group = column(vec![
            button("alpha")
                .id("typeahead.alpha")
                .key("typeahead.alpha")
                .into_element()
                .focus_group_search_text("alpha"),
            button("beta")
                .id("typeahead.beta")
                .key("typeahead.beta")
                .into_element()
                .focus_group_search_text("beta"),
        ])
        .key("typeahead.group")
        .into_element()
        .focus_group(FocusGroup::new().with_type_ahead(type_ahead));

        command_scope(
            [command_binding(
                ApplicationCommand::new(command.clone(), true),
                || TypeAheadAction::Shortcut,
            )],
            [shortcut_scope(
                [ShortcutBinding::new(
                    ShortcutChord::logical(
                        LogicalKey::Character(String::from("b")),
                        KeyModifiers::NONE,
                    ),
                    ShortcutRepeatPolicy::IgnoreRepeat,
                    ApplicationCommand::new(command, true),
                )],
                [group],
            )],
        )
        .key("typeahead.command")
        .into_element()
    }

    fn update(state: &mut TypeAheadState, action: TypeAheadAction) {
        match action {
            TypeAheadAction::Shortcut => state.shortcuts += 1,
        }
    }
}

fn type_ahead_target(
    runtime: &mut AppRuntime<TypeAheadApp>,
    authored: &'static str,
) -> runenui_runtime::MountedNodeId {
    let authored = runenui_core::ElementId::from_static(authored)
        .unwrap_or_else(|_| unreachable!("fixture authored id is valid"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("type-ahead target is mounted"))
        .id()
        .clone()
}

#[test]
fn accelerator_precedes_type_ahead_when_editor_does_not_own_key() {
    let mut runtime = AppRuntime::<TypeAheadApp>::mount(TypeAheadState::default());
    settle(&mut runtime);
    let alpha = type_ahead_target(&mut runtime, "typeahead.alpha");
    runtime
        .submit_command(
            alpha.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("type-ahead focus request is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    runtime
        .submit_keyboard(key(
            "b",
            "KeyB",
            KeyModifiers::NONE,
            false,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("shortcut/type-ahead input is admitted"));
    settle(&mut runtime);

    assert_eq!(runtime.state().shortcuts, 1);
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&alpha),
        "accelerator claim prevents type-ahead from moving focus"
    );
}

#[derive(Debug)]
enum EditorAction {
    Edit(EditIntent),
    Shortcut,
}

#[derive(Debug, Default)]
struct EditorState {
    shortcut_fired: bool,
}

#[derive(Debug)]
struct EditableProbe;

impl Widget<EditorAction> for EditableProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<EditorAction>> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(17), TextDocumentRevision::new(1));
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
            EditorAction::Edit,
        )
        .ok()
    }
}

struct EditorApp;

impl UiApp for EditorApp {
    type State = EditorState;
    type Action = EditorAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &EditorState) -> Element<EditorAction> {
        let command = command_id("editor.shortcut");
        command_scope(
            [command_binding(
                ApplicationCommand::new(command.clone(), true),
                || EditorAction::Shortcut,
            )],
            [shortcut_scope(
                [ShortcutBinding::new(
                    ShortcutChord::logical(
                        LogicalKey::Command(SemanticCommand::Copy),
                        KeyModifiers::NONE.with_control(),
                    ),
                    ShortcutRepeatPolicy::IgnoreRepeat,
                    ApplicationCommand::new(command, true),
                )],
                [Element::new(EditableProbe)
                    .id("editor.target")
                    .key("editor.target")
                    .with_focusability(Focusability::Focusable)],
            )],
        )
        .key("editor.command")
        .into_element()
    }

    fn update(state: &mut EditorState, action: EditorAction) {
        match action {
            EditorAction::Edit(_) => {}
            EditorAction::Shortcut => state.shortcut_fired = true,
        }
    }
}

#[test]
fn editor_owned_m10_default_precedes_accelerator_matching() {
    let mut runtime = AppRuntime::<EditorApp>::mount(EditorState::default());
    settle(&mut runtime);
    let authored = runenui_core::ElementId::from_static("editor.target")
        .unwrap_or_else(|_| unreachable!("fixture authored id is valid"));
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("editable target is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("editable focus request is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyC")),
            LogicalKey::Command(SemanticCommand::Copy),
            KeyModifiers::NONE.with_control(),
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("editable Copy key is admitted"));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert!(!runtime.state().shortcut_fired);
    assert!(
        !runtime
            .trace()
            .records()
            .any(|record| matches!(record.kind(), TraceRecordKind::KeyboardShortcutMatched))
    );
}
