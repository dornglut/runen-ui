#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CommittedTextEvent, CompositionCancelReason,
    CompositionEvent, Element, EventContext, EventPhase, Focusability, InputDeviceId, KeyLocation,
    KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    NoHostProtocol, PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetTextInput, container,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, RuntimeTerminalReason,
    SubmitAutomationErrorKind, SubmitCommandErrorKind, SubmitCompositionErrorKind,
    SubmitKeyboardErrorKind, SubmitTextErrorKind, TraceAutomationRecordRole, TraceConfig,
    TraceDeliveryOutcome, TraceInputRecordRole, TraceRecord, TraceRecordKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum ObservedInput {
    Keyboard(KeyboardPhase),
    Text { bytes: usize, scalars: usize },
    CompositionStart,
    CompositionUpdate { has_range: bool },
    CompositionEnd,
    CompositionCancel(CompositionCancelReason),
    Activated,
    Unmounted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    node: &'static str,
    phase: Option<EventPhase>,
    input: ObservedInput,
    cancelable: Option<bool>,
    prevented: Option<bool>,
}

impl Observation {
    const fn callback(
        node: &'static str,
        phase: EventPhase,
        input: ObservedInput,
        context: &EventContext<'_, InputAction>,
    ) -> Self {
        Self {
            node,
            phase: Some(phase),
            input,
            cancelable: Some(context.default_is_cancelable()),
            prevented: Some(context.default_is_prevented()),
        }
    }

    const fn lifecycle(node: &'static str, input: ObservedInput) -> Self {
        Self {
            node,
            phase: None,
            input,
            cancelable: None,
            prevented: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputAction {
    Activated,
    Remove,
    Replace,
    Disable,
    MakeNotActionable,
    LoseTextCapability,
    LoseCompositionCapability,
    RetireFocusability,
}

#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "the conformance fixture independently selects lifetime, capability, prevention, and ambiguity cases"
)]
struct InputState {
    log: Rc<RefCell<Vec<Observation>>>,
    target_present: bool,
    replacement: bool,
    enabled: bool,
    actionable: bool,
    text_capable: bool,
    composition_capable: bool,
    focus_eligible: bool,
    prevent_keyboard: bool,
    prevent_text: bool,
    duplicate_target_id: bool,
    emit_composition_cancel_action: bool,
    activations: usize,
}

impl InputState {
    const fn standard(log: Rc<RefCell<Vec<Observation>>>) -> Self {
        Self {
            log,
            target_present: true,
            replacement: false,
            enabled: true,
            actionable: true,
            text_capable: true,
            composition_capable: true,
            focus_eligible: true,
            prevent_keyboard: false,
            prevent_text: false,
            duplicate_target_id: false,
            emit_composition_cancel_action: false,
            activations: 0,
        }
    }
}

#[derive(Debug)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "one fixture widget exposes the independent public input capabilities required by the row proofs"
)]
struct InputWidget {
    name: &'static str,
    log: Rc<RefCell<Vec<Observation>>>,
    enabled: bool,
    actionable: bool,
    text_capable: bool,
    composition_capable: bool,
    prevent_keyboard: bool,
    prevent_text: bool,
    emit_composition_cancel_action: bool,
}

impl InputWidget {
    fn observe(&self, event: &UiEvent, context: &mut EventContext<'_, InputAction>) {
        let input = match event {
            UiEvent::Keyboard(event) => {
                if self.prevent_keyboard && context.phase() == EventPhase::Target {
                    context.prevent_default();
                }
                ObservedInput::Keyboard(event.phase())
            }
            UiEvent::CommittedText(event) => {
                if self.prevent_text && context.phase() == EventPhase::Target {
                    context.prevent_default();
                }
                ObservedInput::Text {
                    bytes: event.text().len(),
                    scalars: event.text().chars().count(),
                }
            }
            UiEvent::Composition(CompositionEvent::Start(_)) => ObservedInput::CompositionStart,
            UiEvent::Composition(CompositionEvent::Update(event)) => {
                ObservedInput::CompositionUpdate {
                    has_range: event.range().is_some(),
                }
            }
            UiEvent::Composition(CompositionEvent::End(_)) => ObservedInput::CompositionEnd,
            UiEvent::Composition(CompositionEvent::Cancel(event)) => {
                ObservedInput::CompositionCancel(event.reason())
            }
            _ => return,
        };
        self.log.borrow_mut().push(Observation::callback(
            self.name,
            context.phase(),
            input,
            context,
        ));
    }
}

impl Widget<InputAction> for InputWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, InputAction>,
    ) -> WidgetEventOutput {
        self.observe(event, context);
        if self.emit_composition_cancel_action
            && context.phase() == EventPhase::Target
            && matches!(event, UiEvent::Composition(CompositionEvent::Cancel(_)))
        {
            context.emit(InputAction::Activated);
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        if self.actionable {
            WidgetActivation::actionable(self.enabled)
        } else {
            WidgetActivation::NONE
        }
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(self.text_capable, self.composition_capable)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        _: &mut WidgetActivationContext<InputAction>,
    ) -> WidgetActivationOutput<InputAction> {
        self.log
            .borrow_mut()
            .push(Observation::lifecycle(self.name, ObservedInput::Activated));
        WidgetActivationOutput::action(InputAction::Activated)
    }

    fn unmount(&self, (): &mut Self::State, _: &mut runenui_core::WidgetUnmountContext) {
        self.log
            .borrow_mut()
            .push(Observation::lifecycle(self.name, ObservedInput::Unmounted));
    }
}

impl ChildBearingWidget<InputAction> for InputWidget {}

#[derive(Debug)]
struct ReplacementWidget {
    log: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<InputAction> for ReplacementWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, InputAction>,
    ) -> WidgetEventOutput {
        let input = match event {
            UiEvent::Keyboard(event) => ObservedInput::Keyboard(event.phase()),
            UiEvent::CommittedText(event) => ObservedInput::Text {
                bytes: event.text().len(),
                scalars: event.text().chars().count(),
            },
            UiEvent::Composition(CompositionEvent::Start(_)) => ObservedInput::CompositionStart,
            UiEvent::Composition(CompositionEvent::Update(event)) => {
                ObservedInput::CompositionUpdate {
                    has_range: event.range().is_some(),
                }
            }
            UiEvent::Composition(CompositionEvent::End(_)) => ObservedInput::CompositionEnd,
            UiEvent::Composition(CompositionEvent::Cancel(event)) => {
                ObservedInput::CompositionCancel(event.reason())
            }
            _ => return WidgetEventOutput::none(),
        };
        self.log.borrow_mut().push(Observation::callback(
            "replacement",
            context.phase(),
            input,
            context,
        ));
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        _: &mut WidgetActivationContext<InputAction>,
    ) -> WidgetActivationOutput<InputAction> {
        self.log.borrow_mut().push(Observation::lifecycle(
            "replacement",
            ObservedInput::Activated,
        ));
        WidgetActivationOutput::action(InputAction::Activated)
    }
}

struct InputApp;

impl UiApp for InputApp {
    type State = InputState;
    type Action = InputAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut children = Vec::new();
        if state.target_present {
            let target = if state.replacement {
                Element::new(ReplacementWidget {
                    log: Rc::clone(&state.log),
                })
            } else {
                Element::new(InputWidget {
                    name: "target",
                    log: Rc::clone(&state.log),
                    enabled: state.enabled,
                    actionable: state.actionable,
                    text_capable: state.text_capable,
                    composition_capable: state.composition_capable,
                    prevent_keyboard: state.prevent_keyboard,
                    prevent_text: state.prevent_text,
                    emit_composition_cancel_action: state.emit_composition_cancel_action,
                })
            };
            children.push(target.id("target").key("target").with_focusability(
                if state.focus_eligible {
                    Focusability::Focusable
                } else {
                    Focusability::NotFocusable
                },
            ));
        }
        children.push(
            Element::new(InputWidget {
                name: "other",
                log: Rc::clone(&state.log),
                enabled: true,
                actionable: true,
                text_capable: false,
                composition_capable: false,
                prevent_keyboard: false,
                prevent_text: false,
                emit_composition_cancel_action: false,
            })
            .id("other")
            .key("other")
            .focusable(true),
        );
        if state.duplicate_target_id {
            children.push(
                Element::new(InputWidget {
                    name: "duplicate",
                    log: Rc::clone(&state.log),
                    enabled: true,
                    actionable: true,
                    text_capable: false,
                    composition_capable: false,
                    prevent_keyboard: false,
                    prevent_text: false,
                    emit_composition_cancel_action: false,
                })
                .id("target")
                .key("duplicate"),
            );
        }
        container(
            InputWidget {
                name: "root",
                log: Rc::clone(&state.log),
                enabled: true,
                actionable: false,
                text_capable: false,
                composition_capable: false,
                prevent_keyboard: false,
                prevent_text: false,
                emit_composition_cancel_action: false,
            },
            children,
        )
        .id("root")
        .key("root")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            InputAction::Activated => state.activations += 1,
            InputAction::Remove => state.target_present = false,
            InputAction::Replace => state.replacement = true,
            InputAction::Disable => state.enabled = false,
            InputAction::MakeNotActionable => state.actionable = false,
            InputAction::LoseTextCapability => state.text_capable = false,
            InputAction::LoseCompositionCapability => state.composition_capable = false,
            InputAction::RetireFocusability => state.focus_eligible = false,
        }
    }
}

fn settle(runtime: &mut AppRuntime<InputApp>) {
    let report = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn mounted(state: InputState) -> AppRuntime<InputApp> {
    mounted_with_config(state, RuntimeConfig::default())
}

fn mounted_with_config(state: InputState, config: RuntimeConfig) -> AppRuntime<InputApp> {
    let mut runtime = AppRuntime::<InputApp>::mount_with_config(state, config);
    settle(&mut runtime);
    runtime
}

fn target(runtime: &mut AppRuntime<InputApp>, authored_id: &str) -> runenui_runtime::MountedNodeId {
    let authored_id =
        runenui_core::ElementId::new(authored_id).unwrap_or_else(|_| unreachable!("valid id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored_id))
        .unwrap_or_else(|| unreachable!("fixture node is mounted"))
        .id()
        .clone()
}

fn focus(runtime: &mut AppRuntime<InputApp>, authored_id: &str) {
    let target = target(runtime, authored_id);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("fixture target accepts focus"));
    settle(runtime);
}

const fn keyboard(
    phase: KeyboardPhase,
    physical_key: PhysicalKey,
    logical_key: LogicalKey,
    repeat: bool,
    device_id: Option<InputDeviceId>,
) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        physical_key,
        logical_key,
        KeyModifiers::NONE,
        repeat,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        device_id,
    )
}

fn kinds(runtime: &AppRuntime<InputApp>) -> Vec<TraceRecordKind> {
    runtime
        .trace()
        .records()
        .map(TraceRecord::kind)
        .cloned()
        .collect()
}

fn clear_log(runtime: &AppRuntime<InputApp>) {
    runtime.state().log.borrow_mut().clear();
}

#[test]
fn key_01_keyboard_routes_capture_target_bubble_and_enter_is_queued() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    clear_log(&runtime);

    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Enter,
            LogicalKey::Enter,
            false,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("focused keyboard ingress is accepted"));
    assert_eq!(
        runtime.state().activations,
        0,
        "ingress never activates directly"
    );
    settle(&mut runtime);

    assert_eq!(runtime.state().activations, 1);
    let keyboard_callbacks: Vec<_> = log
        .borrow()
        .iter()
        .filter(|fact| matches!(fact.input, ObservedInput::Keyboard(KeyboardPhase::Down)))
        .cloned()
        .collect();
    assert_eq!(keyboard_callbacks.len(), 3);
    assert_eq!(
        keyboard_callbacks
            .iter()
            .map(|fact| (fact.node, fact.phase))
            .collect::<Vec<_>>(),
        [
            ("root", Some(EventPhase::Capture)),
            ("target", Some(EventPhase::Target)),
            ("root", Some(EventPhase::Bubble)),
        ]
    );
    assert!(
        keyboard_callbacks
            .iter()
            .all(|fact| fact.cancelable == Some(true) && fact.prevented == Some(false))
    );
    let trace = kinds(&runtime);
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardSubmissionAccepted))
    );
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardProcessingValidated))
    );
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardEnterActivationDerived))
    );
}

#[test]
fn key_02_keyboard_prevention_and_repeat_do_not_activate() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut prevented_state = InputState::standard(Rc::clone(&log));
    prevented_state.prevent_keyboard = true;
    let mut prevented = mounted(prevented_state);
    focus(&mut prevented, "target");
    prevented
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Enter,
            LogicalKey::Enter,
            false,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("preventable key is accepted"));
    settle(&mut prevented);
    assert_eq!(prevented.state().activations, 0);
    assert!(
        kinds(&prevented)
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardDefaultPrevented))
    );

    let mut repeated = mounted(InputState::standard(log));
    focus(&mut repeated, "target");
    repeated
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Enter,
            LogicalKey::Enter,
            true,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("repeat is still routed"));
    settle(&mut repeated);
    assert_eq!(repeated.state().activations, 0);
    assert!(
        !kinds(&repeated)
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardEnterActivationDerived))
    );
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this one conformance row keeps all exact Space negative cases adjacent to its positive proof"
)]
fn key_03_space_ownership_requires_the_exact_focused_lifetime_and_device() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(log));
    focus(&mut runtime, "target");
    let device_a = InputDeviceId::new(1).unwrap_or_else(|| unreachable!("nonzero device"));
    let device_b = InputDeviceId::new(2).unwrap_or_else(|| unreachable!("nonzero device"));

    for event in [
        keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device_a),
        ),
        keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyX")),
            LogicalKey::Space,
            false,
            Some(device_a),
        ),
        keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Code(String::from("KeyX")),
            LogicalKey::Space,
            false,
            Some(device_a),
        ),
    ] {
        runtime
            .submit_keyboard(event)
            .unwrap_or_else(|_| unreachable!("focused key is admitted"));
        settle(&mut runtime);
    }
    assert_eq!(runtime.state().activations, 0);

    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            true,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device_b),
        ))
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_eq!(
        runtime.state().activations,
        0,
        "wrong-device release is inert"
    );

    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(runtime.state().activations, 0);
    settle(&mut runtime);
    assert_eq!(
        runtime.state().activations,
        1,
        "the original device-A release still consumes the lifetime after device-B misses"
    );

    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!("a fresh Space down is accepted"));
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            true,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!("a repeated release is still routed"));
    settle(&mut runtime);
    assert_eq!(
        runtime.state().activations,
        1,
        "a repeated Space release never activates"
    );

    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!("Space down is accepted"));
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Code(String::from("KeyX")),
            LogicalKey::Space,
            false,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!("a wrong physical release is still routed"));
    settle(&mut runtime);
    assert_eq!(
        runtime.state().activations,
        1,
        "a wrong physical key cannot release the owned Space activation"
    );
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device_a),
        ))
        .unwrap_or_else(|_| unreachable!("the matching physical release is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, 2);
    let trace = kinds(&runtime);
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardSpaceOwnershipEstablished))
    );
    assert!(trace.iter().any(|kind| matches!(
        kind,
        TraceRecordKind::KeyboardSpaceReleaseMatched { matched: false }
    )));
    assert!(trace.iter().any(|kind| matches!(
        kind,
        TraceRecordKind::KeyboardSpaceReleaseMatched { matched: true }
    )));
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardSpaceActivationDerived))
    );
}

#[test]
fn key_04_keyboard_rejections_preserve_the_owned_event_and_runtime_state() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut no_focus = mounted(InputState::standard(Rc::clone(&log)));
    let event = keyboard(
        KeyboardPhase::Down,
        PhysicalKey::Enter,
        LogicalKey::Enter,
        false,
        None,
    );
    let Err(error) = no_focus.submit_keyboard(event.clone()) else {
        unreachable!("no focus rejects keyboard ingress");
    };
    assert_eq!(error.kind(), SubmitKeyboardErrorKind::NoFocusedTarget);
    assert_eq!(error.into_event(), event);
    assert_eq!(no_focus.state().activations, 0);

    let config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            .with_waiting_envelopes(4)
            .with_transaction_outputs(2),
    );
    let mut full = mounted_with_config(InputState::standard(log), config);
    focus(&mut full, "target");
    full.submit_action(InputAction::Activated)
        .unwrap_or_else(|_| unreachable!("first queue slot is available"));
    full.submit_action(InputAction::Activated)
        .unwrap_or_else(|_| unreachable!("second queue slot is available"));
    full.submit_action(InputAction::Activated)
        .unwrap_or_else(|_| unreachable!("third queue slot is available"));
    full.submit_action(InputAction::Activated)
        .unwrap_or_else(|_| unreachable!("fourth queue slot is available"));
    let event = keyboard(
        KeyboardPhase::Down,
        PhysicalKey::Enter,
        LogicalKey::Enter,
        false,
        None,
    );
    let Err(error) = full.submit_keyboard(event.clone()) else {
        unreachable!("full queue rejects keyboard ingress");
    };
    assert_eq!(error.kind(), SubmitKeyboardErrorKind::Full);
    assert_eq!(error.into_event(), event);
    assert_eq!(full.state().activations, 0);
}

#[test]
#[allow(clippy::assert_is_empty)]
#[allow(
    clippy::too_many_lines,
    reason = "this one conformance row keeps Enter, matched Space, and exact admission-boundary proof together"
)]
fn key_05_keyboard_defaults_reserve_queue_trace_and_command_lineage_before_callbacks() {
    let config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            .with_waiting_envelopes(4)
            .with_transaction_outputs(1),
    );
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut enter = mounted_with_config(InputState::standard(Rc::clone(&log)), config);
    focus(&mut enter, "target");
    clear_log(&enter);
    let key = enter
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Enter,
            LogicalKey::Enter,
            false,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("Enter ingress is accepted"));
    let report = enter
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(report.remaining_queued_envelopes(), 1);
    assert_eq!(enter.state().activations, 0, "activation remains queued");
    let derived = enter
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::KeyboardEnterActivationDerived
            ) && record.work_sequence() == Some(key.sequence())
        })
        .unwrap_or_else(|| unreachable!("Enter derivation is canonical trace work"));
    let accepted = enter
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.command_origin() == Some(CommandOrigin::__runtime_keyboard_default())
        })
        .unwrap_or_else(|| unreachable!("derived command uses canonical command ingress"));
    assert_ne!(accepted.work_sequence(), Some(key.sequence()));
    assert_eq!(accepted.causal_parent(), Some(derived.sequence()));
    settle(&mut enter);
    assert_eq!(enter.state().activations, 1);

    let mut space = mounted_with_config(InputState::standard(log), config);
    focus(&mut space, "target");
    space
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("Space-down ingress is accepted"));
    settle(&mut space);
    let release = space
        .submit_keyboard(keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("Space-up ingress is accepted"));
    let report = space
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.remaining_queued_envelopes(), 1);
    let derived = space
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::KeyboardSpaceActivationDerived
            ) && record.work_sequence() == Some(release.sequence())
        })
        .unwrap_or_else(|| unreachable!("matched Space release derives activation"));
    let accepted = space
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.command_origin() == Some(CommandOrigin::__runtime_keyboard_default())
                && record.causal_parent() == Some(derived.sequence())
        })
        .unwrap_or_else(|| unreachable!("Space derivation keeps its command parent"));
    assert_ne!(accepted.work_sequence(), Some(release.sequence()));
    settle(&mut space);
    assert_eq!(space.state().activations, 1);

    let boundary_log = Rc::new(RefCell::new(Vec::new()));
    let mut boundary = mounted_with_config(InputState::standard(Rc::clone(&boundary_log)), config);
    focus(&mut boundary, "target");
    clear_log(&boundary);
    // With this two-node route and one regular output credit, the old input
    // admission fit exactly at this boundary. Reserving the possible Enter
    // command makes the entire raw route reject before any callback runs.
    boundary.__seed_next_trace_sequence_for_test(u64::MAX - 38);
    boundary
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Enter,
            LogicalKey::Enter,
            false,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("raw ingress still reserves its rejection outcome"));
    let _ = boundary
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        boundary.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert!(boundary_log.borrow().is_empty());
    assert_eq!(boundary.state().activations, 0);
    assert!(kinds(&boundary).iter().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: runenui_runtime::TraceRoutedAdmissionRejection::TraceSequenceExhausted
        }
    )));
    assert!(
        !kinds(&boundary)
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardEnterActivationDerived))
    );
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this one conformance row keeps committed-text routing, rejection, and redaction evidence together"
)]
fn text_01_committed_text_routes_to_the_exact_capable_focus_and_is_redacted_in_trace() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    clear_log(&runtime);
    let text = CommittedTextEvent::new("hé", None)
        .unwrap_or_else(|_| unreachable!("nonempty committed text is valid"));
    runtime
        .submit_text(text)
        .unwrap_or_else(|_| unreachable!("text-capable focus accepts committed text"));
    settle(&mut runtime);

    let callbacks: Vec<_> = log
        .borrow()
        .iter()
        .filter(|fact| matches!(fact.input, ObservedInput::Text { .. }))
        .cloned()
        .collect();
    assert_eq!(callbacks.len(), 3);
    assert_eq!(
        callbacks
            .iter()
            .map(|fact| (fact.node, fact.phase))
            .collect::<Vec<_>>(),
        [
            ("root", Some(EventPhase::Capture)),
            ("target", Some(EventPhase::Target)),
            ("root", Some(EventPhase::Bubble)),
        ]
    );
    assert!(callbacks.iter().all(|fact| {
        matches!(
            fact.input,
            ObservedInput::Text {
                bytes: 3,
                scalars: 2
            }
        ) && fact.cancelable == Some(true)
    }));
    let accepted = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CommittedTextSubmissionAccepted
            )
        })
        .unwrap_or_else(|| unreachable!("committed text acceptance is traced"));
    let input = accepted
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("committed text acceptance owns typed input facts"));
    assert_eq!(input.role(), TraceInputRecordRole::CommittedText);
    let metrics = input
        .text_metrics()
        .unwrap_or_else(|| unreachable!("committed text owns redacted metrics"));
    assert_eq!(metrics.bytes(), 3);
    assert_eq!(metrics.scalars(), 2);
    let validated = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CommittedTextProcessingValidated
            )
        })
        .unwrap_or_else(|| unreachable!("committed text processing is traced"));
    assert!(
        validated.context().input().is_none(),
        "processing classification does not duplicate accepted text metrics"
    );
    assert!(
        !format!("{:?}", runtime.trace().records().collect::<Vec<_>>()).contains("hé"),
        "the canonical trace retains only text length facts"
    );

    let prevented_log = Rc::new(RefCell::new(Vec::new()));
    let mut prevented_state = InputState::standard(prevented_log);
    prevented_state.prevent_text = true;
    let mut prevented = mounted(prevented_state);
    focus(&mut prevented, "target");
    prevented
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("nonempty committed text is valid")),
        )
        .unwrap_or_else(|_| unreachable!("preventable text is accepted"));
    settle(&mut prevented);
    assert!(
        kinds(&prevented)
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::CommittedTextDefaultPrevented))
    );

    let no_capability_log = Rc::new(RefCell::new(Vec::new()));
    let mut no_capability_state = InputState::standard(no_capability_log);
    no_capability_state.text_capable = false;
    let mut no_capability = mounted(no_capability_state);
    focus(&mut no_capability, "target");
    let rejected = CommittedTextEvent::new("x", None)
        .unwrap_or_else(|_| unreachable!("nonempty committed text is valid"));
    let Err(error) = no_capability.submit_text(rejected.clone()) else {
        unreachable!("non-text-capable focus rejects committed text");
    };
    assert_eq!(
        error.kind(),
        SubmitTextErrorKind::FocusedTargetNotTextCapable
    );
    assert_eq!(error.into_event(), rejected);
    assert_eq!(no_capability.state().activations, 0);

    let loss_log = Rc::new(RefCell::new(Vec::new()));
    let mut loss = mounted(InputState::standard(loss_log));
    focus(&mut loss, "target");
    loss.submit_action(InputAction::LoseTextCapability)
        .unwrap_or_else(|_| unreachable!("compatible update is queued"));
    settle(&mut loss);
    assert!(matches!(
        loss.submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("nonempty committed text is valid")),
        ),
        Err(error) if error.kind() == SubmitTextErrorKind::FocusedTargetNotTextCapable
    ));
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this one conformance row keeps composition lifecycle, payload redaction, and causal-parent proof together"
)]
fn ime_01_composition_start_update_end_and_explicit_cancel_are_exact_and_redacted() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    clear_log(&runtime);
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition-capable focus accepts start"));
    let range = runenui_core::CompositionRange::new("preedit-secret", 0, 7)
        .unwrap_or_else(|_| unreachable!("checked preedit range is valid"));
    runtime
        .submit_composition_update(
            start.generation().clone(),
            String::from("preedit-secret"),
            Some(range),
        )
        .unwrap_or_else(|_| unreachable!("pending generation accepts queued update"));
    let end = runtime
        .submit_composition_end(start.generation().clone())
        .unwrap_or_else(|_| unreachable!("pending generation accepts queued end"));
    settle(&mut runtime);

    let events = log.borrow().clone();
    assert!(events.iter().any(|fact| {
        fact.node == "target"
            && fact.phase == Some(EventPhase::Target)
            && fact.input == ObservedInput::CompositionStart
            && fact.cancelable == Some(false)
    }));
    assert!(events.iter().any(|fact| {
        fact.node == "target"
            && fact.phase == Some(EventPhase::Target)
            && fact.input == ObservedInput::CompositionUpdate { has_range: true }
            && fact.cancelable == Some(false)
    }));
    assert!(events.iter().any(|fact| {
        fact.node == "target"
            && fact.phase == Some(EventPhase::Target)
            && fact.input == ObservedInput::CompositionEnd
    }));
    let trace = kinds(&runtime);
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::CompositionGenerationAllocated))
    );
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::CompositionPendingBound))
    );
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::CompositionActiveBound))
    );
    let update = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::CompositionUpdateSubmitted))
        .unwrap_or_else(|| unreachable!("composition update submission is traced"));
    let update_context = update
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("composition update owns typed input facts"));
    assert_eq!(
        update_context.role(),
        TraceInputRecordRole::CompositionUpdate
    );
    let composition = update_context
        .composition()
        .unwrap_or_else(|| unreachable!("composition update owns exact lifetime identity"));
    assert_eq!(composition.generation(), start.generation());
    let metrics = update_context
        .text_metrics()
        .unwrap_or_else(|| unreachable!("composition update owns redacted preedit metrics"));
    assert_eq!(metrics.bytes(), 14);
    assert_eq!(metrics.scalars(), 14);
    let trace_range = update_context
        .composition_range()
        .unwrap_or_else(|| unreachable!("composition update owns its checked range"));
    assert_eq!(trace_range.byte_start(), 0);
    assert_eq!(trace_range.byte_end(), 7);
    assert_eq!(trace_range.scalar_start(), 0);
    assert_eq!(trace_range.scalar_end(), 7);
    let ended = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::CompositionEndSubmitted))
        .unwrap_or_else(|| unreachable!("composition end submission is traced"));
    let end_context = ended
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("composition end owns exact lifetime identity"));
    assert_eq!(
        end_context.role(),
        TraceInputRecordRole::CompositionIdentity
    );
    assert_eq!(
        end_context
            .composition()
            .unwrap_or_else(|| unreachable!("composition end retains identity"))
            .generation(),
        start.generation()
    );
    assert!(
        trace
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::CompositionRetired))
    );
    assert!(
        !format!("{:?}", runtime.trace().records().collect::<Vec<_>>()).contains("preedit-secret"),
        "the trace never retains preedit text"
    );
    let active = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionActiveBound)
                && record.work_sequence() == Some(start.sequence())
        })
        .unwrap_or_else(|| unreachable!("active binding retains the start work sequence"));
    let active_parent = active
        .causal_parent()
        .unwrap_or_else(|| unreachable!("active binding retains routed lineage"));
    assert!(runtime.trace().records().any(|record| {
        record.sequence() == active_parent && record.work_sequence() == Some(start.sequence())
    }));
    let retired = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionRetired)
                && record.work_sequence() == Some(end.sequence())
        })
        .unwrap_or_else(|| unreachable!("end retirement retains the end work sequence"));
    let retired_parent = retired
        .causal_parent()
        .unwrap_or_else(|| unreachable!("end retirement retains routed lineage"));
    assert!(runtime.trace().records().any(|record| {
        record.sequence() == retired_parent && record.work_sequence() == Some(end.sequence())
    }));
    assert!(matches!(
        runtime.submit_composition_end(start.generation().clone()),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));

    clear_log(&runtime);
    let second = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("retired generation permits a fresh start"));
    settle(&mut runtime);
    let cancel = runtime
        .cancel_composition(second.generation().clone())
        .unwrap_or_else(|_| unreachable!("active generation accepts explicit cancellation"));
    settle(&mut runtime);
    assert!(log.borrow().iter().any(|fact| {
        fact.node == "target"
            && fact.phase == Some(EventPhase::Target)
            && fact.input == ObservedInput::CompositionCancel(CompositionCancelReason::Explicit)
            && fact.cancelable == Some(false)
    }));
    let submitted = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionCancelSubmitted)
                && record.work_sequence() == Some(cancel.sequence())
        })
        .unwrap_or_else(|| unreachable!("explicit cancel submission is traced"));
    let submitted_context = submitted
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("explicit cancel submission owns exact identity"));
    assert_eq!(
        submitted_context.role(),
        TraceInputRecordRole::CompositionIdentity
    );
    assert_eq!(
        submitted_context
            .composition()
            .unwrap_or_else(|| unreachable!("cancel submission retains composition identity"))
            .generation(),
        second.generation()
    );
    let committed = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::Explicit
                }
            ) && record.work_sequence() == Some(cancel.sequence())
        })
        .unwrap_or_else(|| unreachable!("explicit cancellation commits after callback success"));
    let committed_context = committed
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("committed cancellation owns cleanup result"));
    assert_eq!(
        committed_context.role(),
        TraceInputRecordRole::CompositionCleanup
    );
    assert_eq!(
        committed_context.delivery(),
        Some(TraceDeliveryOutcome::Delivered)
    );
    assert_eq!(
        committed_context
            .composition()
            .unwrap_or_else(|| unreachable!("committed cancellation retains exact lifetime"))
            .generation(),
        second.generation()
    );
    let retired = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionRetired)
                && record.work_sequence() == Some(cancel.sequence())
        })
        .unwrap_or_else(|| unreachable!("explicit cancellation retires the same work"));
    assert_eq!(retired.causal_parent(), Some(committed.sequence()));
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this one conformance row keeps exact composition rejection ownership and failed-start retirement evidence together"
)]
fn ime_02_composition_rejections_keep_owned_requests_and_authority_unchanged() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut no_focus = mounted(InputState::standard(Rc::clone(&log)));
    let requested_device = InputDeviceId::new(73).unwrap_or_else(|| unreachable!("nonzero"));
    let Err(error) = no_focus.start_composition(Some(requested_device)) else {
        unreachable!("missing focus rejects the caller-owned start request");
    };
    assert_eq!(error.kind(), SubmitCompositionErrorKind::NoFocusedTarget);
    assert_eq!(error.request().device_id(), Some(requested_device));
    assert_eq!(error.into_request().device_id(), Some(requested_device));

    let mut no_capability_state = InputState::standard(Rc::clone(&log));
    no_capability_state.composition_capable = false;
    let mut no_capability = mounted(no_capability_state);
    focus(&mut no_capability, "target");
    assert!(matches!(
        no_capability.start_composition(None),
        Err(error) if error.kind() == SubmitCompositionErrorKind::FocusedTargetNotCompositionCapable
    ));

    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    let missing = runtime.__composition_generation_for_test(97);
    let Err(error) = runtime.submit_composition_end(missing.clone()) else {
        unreachable!("unissued local generation is rejected");
    };
    assert_eq!(error.kind(), SubmitCompositionErrorKind::MissingGeneration);
    assert!(
        matches!(error.into_event(), CompositionEvent::End(end) if end.generation() == &missing)
    );

    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition start is accepted"));
    settle(&mut runtime);
    let invalid_range = runenui_core::CompositionRange::new("wide", 0, 4)
        .unwrap_or_else(|_| unreachable!("fixture range itself is checked"));
    let Err(error) = runtime.submit_composition_update(
        start.generation().clone(),
        String::from("x"),
        Some(invalid_range),
    ) else {
        unreachable!("a range outside the retained preedit is rejected");
    };
    assert_eq!(error.kind(), SubmitCompositionErrorKind::InvalidRange);
    assert!(matches!(
        error.into_event(),
        CompositionEvent::Update(update)
            if update.preedit() == "x" && update.range() == Some(invalid_range)
    ));
    runtime
        .submit_composition_end(start.generation().clone())
        .unwrap_or_else(|_| unreachable!("active generation accepts end"));
    settle(&mut runtime);
    assert!(matches!(
        runtime.submit_composition_update(start.generation().clone(), String::from("late"), None),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));

    let mut foreign = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut foreign, "target");
    let foreign_generation = foreign
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("foreign runtime issues a generation"))
        .generation()
        .clone();
    assert!(matches!(
        runtime.submit_composition_end(foreign_generation),
        Err(error) if error.kind() == SubmitCompositionErrorKind::ForeignGeneration
    ));

    let mut exhausted = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut exhausted, "target");
    exhausted.__seed_next_composition_generation_for_test(None);
    let Err(error) = exhausted.start_composition(Some(requested_device)) else {
        unreachable!("exhausted allocation rejects the request before generating an event");
    };
    assert_eq!(
        error.kind(),
        SubmitCompositionErrorKind::CompositionGenerationExhausted
    );
    assert_eq!(error.into_request().device_id(), Some(requested_device));
    let never_issued_zero = exhausted.__composition_generation_for_test(0);
    assert!(matches!(
        exhausted.submit_composition_end(never_issued_zero),
        Err(error) if error.kind() == SubmitCompositionErrorKind::MissingGeneration
    ));
    let never_issued_max = exhausted.__composition_generation_for_test(u64::MAX);
    assert!(matches!(
        exhausted.submit_composition_end(never_issued_max),
        Err(error) if error.kind() == SubmitCompositionErrorKind::MissingGeneration
    ));

    let mut issued_max = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut issued_max, "target");
    issued_max.__seed_next_composition_generation_for_test(Some(u64::MAX));
    let max_start = issued_max
        .start_composition(Some(requested_device))
        .unwrap_or_else(|_| unreachable!("the final allocatable generation is still issued"));
    settle(&mut issued_max);
    issued_max
        .submit_composition_end(max_start.generation().clone())
        .unwrap_or_else(|_| unreachable!("the actually issued maximum generation remains live"));
    settle(&mut issued_max);
    assert!(matches!(
        issued_max.submit_composition_end(max_start.generation().clone()),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));

    let mut sequence_exhausted = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut sequence_exhausted, "target");
    sequence_exhausted.__seed_next_work_sequence_for_test(0);
    assert!(matches!(
        sequence_exhausted.start_composition(None),
        Err(error) if error.kind() == SubmitCompositionErrorKind::WorkSequenceExhausted
    ));

    let mut trace_exhausted = mounted(InputState::standard(log));
    focus(&mut trace_exhausted, "target");
    trace_exhausted.__seed_next_trace_sequence_for_test(u64::MAX - 1);
    let before = trace_exhausted.__routed_sequence_state_for_test();
    assert!(matches!(
        trace_exhausted.start_composition(None),
        Err(error) if error.kind() == SubmitCompositionErrorKind::TraceSequenceExhausted
    ));
    assert_eq!(trace_exhausted.__routed_sequence_state_for_test(), before);

    let failed_log = Rc::new(RefCell::new(Vec::new()));
    let mut failed_start = mounted(InputState::standard(failed_log));
    focus(&mut failed_start, "target");
    let failed_generation = failed_start
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("the start itself enters the FIFO"))
        .generation()
        .clone();
    failed_start
        .submit_composition_update(failed_generation.clone(), String::from("queued"), None)
        .unwrap_or_else(|_| unreachable!("pending authority accepts later owned work"));
    failed_start.__fail_routed_callback_bridge_for_test();
    let _ = failed_start
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        failed_start.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert!(matches!(
        failed_start.submit_composition_end(failed_generation),
        Err(error)
            if error.kind()
                == SubmitCompositionErrorKind::Terminal(RuntimeTerminalReason::Poisoned)
    ));
    assert!(kinds(&failed_start).iter().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedIntegrityFailed {
            failure: runenui_runtime::TraceRoutedIntegrityFailure::CallbackBridgeFailure
        }
    )));
}

#[test]
fn ime_03_focus_transfer_cancels_the_live_owner_before_focus_out() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("focused composition start is accepted"));
    settle(&mut runtime);
    clear_log(&runtime);

    let other = target(&mut runtime, "other");
    let focus_submission = runtime
        .submit_command(
            other,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus transfer is queued"));
    settle(&mut runtime);
    let log = log.borrow();
    let cancellation = log
        .iter()
        .position(|fact| {
            fact.node == "target"
                && fact.phase == Some(EventPhase::Target)
                && fact.input
                    == ObservedInput::CompositionCancel(CompositionCancelReason::FocusTransfer)
        })
        .unwrap_or_else(|| unreachable!("the old focused owner observes cancellation"));
    assert!(
        log[cancellation + 1..]
            .iter()
            .all(|fact| fact.input != ObservedInput::CompositionStart),
        "the old generation cannot restart or retarget after focus leaves"
    );
    drop(log);
    assert!(matches!(
        runtime.submit_composition_end(start.generation().clone()),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));
    let trace = kinds(&runtime);
    let cancellation = trace
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::FocusTransfer
                }
            )
        })
        .unwrap_or_else(|| unreachable!("focus cancellation is traced"));
    let focus_out = runtime
        .trace()
        .records()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: runenui_runtime::FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        })
        .unwrap_or_else(|| unreachable!("focus departure is delivered"));
    assert!(cancellation < focus_out);
    let cancelled = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::FocusTransfer
                }
            ) && record.work_sequence() == Some(focus_submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("focus cleanup cancellation retains command work"));
    let cleanup = cancelled
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("focus cleanup owns typed composition identity"));
    assert_eq!(cleanup.role(), TraceInputRecordRole::CompositionCleanup);
    assert_eq!(cleanup.delivery(), Some(TraceDeliveryOutcome::Delivered));
    assert_eq!(
        cleanup
            .composition()
            .unwrap_or_else(|| unreachable!("focus cleanup retains exact lifetime"))
            .generation(),
        start.generation()
    );
    assert!(cancelled.causal_parent().is_some());
    let retired = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionRetired)
                && record.work_sequence() == Some(focus_submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("focus cleanup retirement retains command work"));
    assert_eq!(retired.causal_parent(), Some(cancelled.sequence()));
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this one conformance row keeps removal, replacement, and compatible-capability-loss ordering evidence together"
)]
fn ime_04_reconciliation_cancels_before_removal_replacement_and_capability_loss() {
    let remove_log = Rc::new(RefCell::new(Vec::new()));
    let mut removal = mounted(InputState::standard(Rc::clone(&remove_log)));
    focus(&mut removal, "target");
    let removal_start = removal
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!());
    let generation = removal_start.generation().clone();
    settle(&mut removal);
    clear_log(&removal);
    removal
        .submit_action(InputAction::Remove)
        .unwrap_or_else(|_| unreachable!("removal action enters the FIFO"));
    settle(&mut removal);
    let removal_observations = remove_log.borrow();
    let cancel = removal_observations
        .iter()
        .position(|fact| {
            fact.node == "target"
                && fact.input == ObservedInput::CompositionCancel(CompositionCancelReason::Removal)
        })
        .unwrap_or_else(|| unreachable!("removal cancellation reaches live target"));
    let unmount = removal_observations
        .iter()
        .position(|fact| fact.node == "target" && fact.input == ObservedInput::Unmounted)
        .unwrap_or_else(|| unreachable!("old target unmounts"));
    assert!(cancel < unmount);
    drop(removal_observations);
    assert!(matches!(
        removal.submit_composition_end(generation),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));
    let removal_trace = kinds(&removal);
    let cancellation = removal_trace
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::Removal
                }
            )
        })
        .unwrap_or_else(|| unreachable!("removal cancellation is traced"));
    let reconciled = removal_trace
        .iter()
        .position(|kind| matches!(kind, TraceRecordKind::TreeReconciled))
        .unwrap_or_else(|| unreachable!("reconciliation is traced"));
    assert!(cancellation < reconciled);
    let cancelled = removal
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::Removal
                }
            ) && record.work_sequence() == Some(removal_start.sequence())
        })
        .unwrap_or_else(|| unreachable!("reconciliation cleanup retains start work"));
    let cleanup = cancelled
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("reconciliation cleanup owns typed input facts"));
    assert_eq!(cleanup.role(), TraceInputRecordRole::CompositionCleanup);
    assert_eq!(cleanup.delivery(), Some(TraceDeliveryOutcome::Delivered));
    assert_eq!(
        cleanup
            .composition()
            .unwrap_or_else(|| unreachable!("reconciliation cleanup retains exact lifetime"))
            .generation(),
        removal_start.generation()
    );
    assert!(cancelled.causal_parent().is_some());
    let retired = removal
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionRetired)
                && record.work_sequence() == Some(removal_start.sequence())
        })
        .unwrap_or_else(|| unreachable!("reconciliation cleanup retires with the same work"));
    assert_eq!(retired.causal_parent(), Some(cancelled.sequence()));

    let replacement_log = Rc::new(RefCell::new(Vec::new()));
    let mut replacement = mounted(InputState::standard(Rc::clone(&replacement_log)));
    focus(&mut replacement, "target");
    replacement
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!());
    settle(&mut replacement);
    clear_log(&replacement);
    replacement
        .submit_action(InputAction::Replace)
        .unwrap_or_else(|_| unreachable!("replacement action enters the FIFO"));
    settle(&mut replacement);
    let replacement_observations = replacement_log.borrow();
    let cancel = replacement_observations
        .iter()
        .position(|fact| {
            fact.node == "target"
                && fact.input
                    == ObservedInput::CompositionCancel(CompositionCancelReason::Replacement)
        })
        .unwrap_or_else(|| unreachable!("replacement cancels the old live owner"));
    let unmount = replacement_observations
        .iter()
        .position(|fact| fact.node == "target" && fact.input == ObservedInput::Unmounted)
        .unwrap_or_else(|| unreachable!("old replacement lifetime unmounts"));
    assert!(cancel < unmount);
    drop(replacement_observations);
    assert!(kinds(&replacement).iter().any(|kind| matches!(
        kind,
        TraceRecordKind::CompositionCancelled {
            reason: CompositionCancelReason::Replacement
        }
    )));

    for action in [
        InputAction::Disable,
        InputAction::LoseTextCapability,
        InputAction::LoseCompositionCapability,
    ] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
        focus(&mut runtime, "target");
        runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!());
        settle(&mut runtime);
        clear_log(&runtime);
        runtime
            .submit_action(action)
            .unwrap_or_else(|_| unreachable!("compatible update enters the FIFO"));
        settle(&mut runtime);
        assert!(log.borrow().iter().any(|fact| {
            fact.node == "target"
                && fact.input
                    == ObservedInput::CompositionCancel(CompositionCancelReason::Disablement)
        }));
        assert!(
            !log.borrow()
                .iter()
                .any(|fact| { fact.node == "target" && fact.input == ObservedInput::Unmounted })
        );
    }
}

#[test]
fn ime_05_pending_shutdown_cleans_the_live_owner_and_trace_is_optional() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("pending composition start is accepted"));
    let _ = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(log.borrow().iter().any(|fact| {
        fact.node == "target"
            && fact.phase == Some(EventPhase::Target)
            && fact.input == ObservedInput::CompositionCancel(CompositionCancelReason::Shutdown)
    }));
    assert!(kinds(&runtime).iter().any(|kind| matches!(
        kind,
        TraceRecordKind::CompositionCancelled {
            reason: CompositionCancelReason::Shutdown
        }
    )));

    let disabled_log = Rc::new(RefCell::new(Vec::new()));
    let mut disabled = mounted_with_config(
        InputState::standard(disabled_log),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    );
    focus(&mut disabled, "target");
    let start = disabled
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("disabled tracing preserves composition ingress"));
    settle(&mut disabled);
    disabled
        .cancel_composition(start.generation().clone())
        .unwrap_or_else(|_| unreachable!("disabled tracing preserves cancellation"));
    settle(&mut disabled);
    assert_eq!(disabled.trace().len(), 0);
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this conformance row proves both bounded-admission and bridge-integrity cleanup ancestry"
)]
fn ime_06_cleanup_admission_or_bridge_failure_terminalizes_before_tree_teardown() {
    let bounded_log = Rc::new(RefCell::new(Vec::new()));
    let bounded_config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            .with_waiting_envelopes(3)
            .with_transaction_outputs(1),
    );
    let mut bounded = mounted_with_config(
        InputState::standard(Rc::clone(&bounded_log)),
        bounded_config,
    );
    focus(&mut bounded, "target");
    let bounded_start = bounded
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("live composition starts before bounded cleanup"));
    settle(&mut bounded);
    clear_log(&bounded);
    bounded
        .submit_action(InputAction::Remove)
        .unwrap_or_else(|_| unreachable!("removal occupies the FIFO head"));
    for _ in 0..2 {
        bounded
            .submit_action(InputAction::Activated)
            .unwrap_or_else(|_| unreachable!("filler occupies cancellation capacity"));
    }
    let _ = bounded
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        bounded.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert!(
        !bounded_log
            .borrow()
            .iter()
            .any(|fact| fact.input == ObservedInput::Unmounted),
        "the reconciliation plan never applies after required cancellation admission fails"
    );
    assert!(bounded.index().nodes().iter().any(|node| {
        node.authored_id()
            == Some(
                &runenui_core::ElementId::new("target")
                    .unwrap_or_else(|_| unreachable!("valid authored id")),
            )
    }));
    let rejection = bounded
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RoutedEventAdmissionRejected {
                    capacity: runenui_runtime::TraceRoutedAdmissionRejection::WaitingEnvelopes
                }
            )
        })
        .unwrap_or_else(|| unreachable!("bounded cleanup records its exact admission rejection"));
    let retired = bounded
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionRetired)
                && record.context().delivery() == Some(TraceDeliveryOutcome::Suppressed)
        })
        .unwrap_or_else(|| unreachable!("failed bounded cleanup explicitly retires composition"));
    let cleanup = retired
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("suppressed retirement owns typed composition facts"));
    assert_eq!(cleanup.role(), TraceInputRecordRole::CompositionCleanup);
    assert_eq!(cleanup.delivery(), Some(TraceDeliveryOutcome::Suppressed));
    assert_eq!(
        cleanup
            .composition()
            .unwrap_or_else(|| unreachable!("suppressed cleanup retains exact lifetime"))
            .generation(),
        bounded_start.generation()
    );
    assert_eq!(retired.causal_parent(), Some(rejection.sequence()));
    let terminal = bounded
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RuntimeTerminal {
                    reason: RuntimeTerminalReason::Poisoned
                }
            )
        })
        .unwrap_or_else(|| unreachable!("bounded cleanup failure terminalizes"));
    assert_eq!(terminal.causal_parent(), Some(retired.sequence()));

    let bridge_log = Rc::new(RefCell::new(Vec::new()));
    let mut bridge = mounted(InputState::standard(Rc::clone(&bridge_log)));
    focus(&mut bridge, "target");
    let bridge_start = bridge
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("live composition starts before bridge failure"));
    settle(&mut bridge);
    clear_log(&bridge);
    bridge.__fail_routed_callback_bridge_for_test();
    bridge
        .submit_action(InputAction::Remove)
        .unwrap_or_else(|_| unreachable!("removal is queued before bridge failure"));
    let _ = bridge
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        bridge.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert!(
        !bridge_log
            .borrow()
            .iter()
            .any(|fact| fact.input == ObservedInput::Unmounted),
        "a bridge failure terminalizes before the invalidated owner can unmount"
    );
    let retired = bridge
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CompositionRetired)
                && record.context().delivery() == Some(TraceDeliveryOutcome::Suppressed)
        })
        .unwrap_or_else(|| unreachable!("bridge failure suppresses composition delivery"));
    let failure_parent = retired
        .causal_parent()
        .unwrap_or_else(|| unreachable!("suppressed retirement retains failure ancestry"));
    let failure = bridge
        .trace()
        .records()
        .find(|record| record.sequence() == failure_parent)
        .unwrap_or_else(|| unreachable!("suppressed retirement parent is retained"));
    assert!(matches!(
        failure.kind(),
        TraceRecordKind::RoutedIntegrityFailed {
            failure: runenui_runtime::TraceRoutedIntegrityFailure::CallbackBridgeFailure
        }
    ));
    let cleanup = retired
        .context()
        .input()
        .unwrap_or_else(|| unreachable!("suppressed retirement owns typed composition facts"));
    assert_eq!(cleanup.role(), TraceInputRecordRole::CompositionCleanup);
    assert_eq!(cleanup.delivery(), Some(TraceDeliveryOutcome::Suppressed));
    assert_eq!(
        cleanup
            .composition()
            .unwrap_or_else(|| unreachable!("bridge cleanup retains exact lifetime"))
            .generation(),
        bridge_start.generation()
    );
    let terminal = bridge
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::RuntimeTerminal {
                    reason: RuntimeTerminalReason::Poisoned
                }
            )
        })
        .unwrap_or_else(|| unreachable!("bridge cleanup failure terminalizes"));
    assert_eq!(terminal.causal_parent(), Some(retired.sequence()));
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this one conformance row keeps every terminal and stale-lifetime Space negative case adjacent"
)]
fn key_03_space_cleanup_rejects_lost_lifetimes_and_terminal_releases() {
    let device = InputDeviceId::new(1).unwrap_or_else(|| unreachable!("nonzero device"));
    let down = || {
        keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device),
        )
    };
    let up = || {
        keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device),
        )
    };

    for (action, reason) in [
        (
            InputAction::Remove,
            runenui_runtime::TraceSpaceCleanupReason::Removal,
        ),
        (
            InputAction::Replace,
            runenui_runtime::TraceSpaceCleanupReason::Replacement,
        ),
        (
            InputAction::Disable,
            runenui_runtime::TraceSpaceCleanupReason::Disablement,
        ),
        (
            InputAction::MakeNotActionable,
            runenui_runtime::TraceSpaceCleanupReason::Disablement,
        ),
    ] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = mounted(InputState::standard(log));
        focus(&mut runtime, "target");
        runtime
            .submit_keyboard(down())
            .unwrap_or_else(|_| unreachable!("space down is accepted"));
        settle(&mut runtime);
        runtime
            .submit_action(action)
            .unwrap_or_else(|_| unreachable!("lifetime transition enters the FIFO"));
        settle(&mut runtime);
        if runtime.focus().focused_node().is_none() {
            focus(&mut runtime, "other");
        }
        runtime
            .submit_keyboard(up())
            .unwrap_or_else(|_| unreachable!("later release is routed or rejected canonically"));
        settle(&mut runtime);
        assert_eq!(runtime.state().activations, 0);
        assert!(kinds(&runtime).iter().any(|kind| matches!(
            kind,
            TraceRecordKind::KeyboardSpaceOwnershipCleared { reason: actual } if *actual == reason
        )));
    }

    let log = Rc::new(RefCell::new(Vec::new()));
    let mut focus_transfer = mounted(InputState::standard(log));
    focus(&mut focus_transfer, "target");
    focus_transfer
        .submit_keyboard(down())
        .unwrap_or_else(|_| unreachable!());
    settle(&mut focus_transfer);
    focus(&mut focus_transfer, "other");
    focus_transfer
        .submit_keyboard(up())
        .unwrap_or_else(|_| unreachable!());
    settle(&mut focus_transfer);
    assert_eq!(focus_transfer.state().activations, 0);
    assert!(kinds(&focus_transfer).iter().any(|kind| matches!(
        kind,
        TraceRecordKind::KeyboardSpaceOwnershipCleared {
            reason: runenui_runtime::TraceSpaceCleanupReason::FocusTransfer
        }
    )));

    let log = Rc::new(RefCell::new(Vec::new()));
    let mut stale_release = mounted(InputState::standard(log));
    focus(&mut stale_release, "target");
    stale_release
        .submit_keyboard(down())
        .unwrap_or_else(|_| unreachable!());
    settle(&mut stale_release);
    stale_release
        .submit_action(InputAction::Remove)
        .unwrap_or_else(|_| unreachable!());
    stale_release
        .submit_keyboard(up())
        .unwrap_or_else(|_| unreachable!("release is accepted against its then-live focus"));
    settle(&mut stale_release);
    assert_eq!(stale_release.state().activations, 0);
    assert!(
        !kinds(&stale_release)
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::KeyboardSpaceActivationDerived))
    );

    let log = Rc::new(RefCell::new(Vec::new()));
    let mut shutdown = mounted(InputState::standard(log));
    focus(&mut shutdown, "target");
    shutdown
        .submit_keyboard(down())
        .unwrap_or_else(|_| unreachable!());
    settle(&mut shutdown);
    let _ = shutdown
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(kinds(&shutdown).iter().any(|kind| matches!(
        kind,
        TraceRecordKind::KeyboardSpaceOwnershipCleared {
            reason: runenui_runtime::TraceSpaceCleanupReason::Shutdown
        }
    )));

    let log = Rc::new(RefCell::new(Vec::new()));
    let mut terminal = mounted(InputState::standard(log));
    focus(&mut terminal, "target");
    terminal
        .submit_keyboard(down())
        .unwrap_or_else(|_| unreachable!());
    settle(&mut terminal);
    terminal.__seed_reconciliation_generation_for_test(u64::MAX);
    terminal
        .submit_action(InputAction::Activated)
        .unwrap_or_else(|_| unreachable!());
    let _ = terminal
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        terminal.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::ReconciliationGenerationExhausted)
    );
    assert!(kinds(&terminal).iter().any(|kind| matches!(
        kind,
        TraceRecordKind::KeyboardSpaceOwnershipCleared {
            reason: runenui_runtime::TraceSpaceCleanupReason::Terminal
        }
    )));
    assert!(matches!(
        terminal.submit_keyboard(up()),
        Err(error)
            if error.kind()
                == SubmitKeyboardErrorKind::Terminal(
                    RuntimeTerminalReason::ReconciliationGenerationExhausted
                )
    ));
}

#[test]
fn automation_01_unique_resolution_uses_canonical_command_ingress_with_lineage() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(log));
    let authored =
        runenui_core::ElementId::new("target").unwrap_or_else(|_| unreachable!("valid id"));
    let submission = runtime
        .submit_automation_command(authored.clone(), SemanticCommand::Activate)
        .unwrap_or_else(|_| unreachable!("unique target is accepted"));
    assert_eq!(
        runtime.state().activations,
        0,
        "automation only queues work"
    );
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, 1);

    let resolution = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::AutomationResolutionUnique))
        .unwrap_or_else(|| unreachable!("unique resolution is traced"));
    let resolution_context = resolution
        .context()
        .automation()
        .unwrap_or_else(|| unreachable!("unique automation resolution owns authored intent"));
    assert_eq!(resolution_context.role(), TraceAutomationRecordRole::Unique);
    assert_eq!(resolution_context.authored_id(), &authored);
    assert_eq!(resolution_context.command(), SemanticCommand::Activate);
    assert_eq!(resolution_context.candidates(), None);
    let resolution_sequence = resolution.sequence();
    let accepted = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
                && record.work_sequence() == Some(submission.sequence())
        })
        .unwrap_or_else(|| unreachable!("canonical command acceptance is traced"));
    assert_eq!(accepted.causal_parent(), Some(resolution_sequence));
    assert_eq!(accepted.command_origin(), Some(CommandOrigin::automation()));
}

#[test]
fn automation_02_missing_and_ambiguous_rejections_preserve_requests() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut missing = mounted(InputState::standard(Rc::clone(&log)));
    let before = missing.__routed_sequence_state_for_test();
    let authored =
        runenui_core::ElementId::new("missing").unwrap_or_else(|_| unreachable!("valid id"));
    let Err(error) = missing.submit_automation_command(authored.clone(), SemanticCommand::Activate)
    else {
        unreachable!("missing authored id is structurally rejected");
    };
    assert_eq!(error.kind(), &SubmitAutomationErrorKind::MissingAuthoredId);
    assert_eq!(
        error.into_request(),
        (authored.clone(), SemanticCommand::Activate)
    );
    assert_eq!(missing.__routed_sequence_state_for_test().0, before.0);
    let missing_record = missing
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::AutomationResolutionMissing))
        .unwrap_or_else(|| unreachable!("missing resolution is traced"));
    let missing_context = missing_record
        .context()
        .automation()
        .unwrap_or_else(|| unreachable!("missing resolution owns authored intent"));
    assert_eq!(missing_context.role(), TraceAutomationRecordRole::Missing);
    assert_eq!(missing_context.authored_id(), &authored);
    assert_eq!(missing_context.command(), SemanticCommand::Activate);
    assert_eq!(missing_context.candidates(), None);

    let mut ambiguous_state = InputState::standard(Rc::clone(&log));
    ambiguous_state.duplicate_target_id = true;
    let mut ambiguous = mounted(ambiguous_state);
    let before = ambiguous.__routed_sequence_state_for_test();
    let authored =
        runenui_core::ElementId::new("target").unwrap_or_else(|_| unreachable!("valid id"));
    let Err(error) =
        ambiguous.submit_automation_command(authored.clone(), SemanticCommand::Activate)
    else {
        unreachable!("ambiguous authored id is structurally rejected");
    };
    let candidates = match error.kind() {
        SubmitAutomationErrorKind::AmbiguousAuthoredId { candidates } => candidates.clone(),
        SubmitAutomationErrorKind::MissingAuthoredId | SubmitAutomationErrorKind::Command(_) => {
            unreachable!("duplicate authored ID returns deterministic candidates")
        }
        _ => unreachable!("future automation rejection kinds cannot resolve a duplicate"),
    };
    assert_eq!(
        candidates
            .iter()
            .map(runenui_runtime::AutomationMatchDiagnostic::logical_preorder)
            .collect::<Vec<_>>(),
        [1, 3],
        "logical preorder is stable and excludes widget state or content"
    );
    assert_ne!(
        candidates[0].mounted_node_id(),
        candidates[1].mounted_node_id(),
        "diagnostics retain only distinct opaque mounted lifetimes"
    );
    assert_eq!(
        error.into_request(),
        (authored.clone(), SemanticCommand::Activate)
    );
    assert_eq!(ambiguous.__routed_sequence_state_for_test().0, before.0);
    let ambiguous_record = ambiguous
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::AutomationResolutionAmbiguous
            )
        })
        .unwrap_or_else(|| unreachable!("ambiguous resolution is traced"));
    let ambiguous_context = ambiguous_record
        .context()
        .automation()
        .unwrap_or_else(|| unreachable!("ambiguous resolution owns deterministic candidates"));
    assert_eq!(
        ambiguous_context.role(),
        TraceAutomationRecordRole::Ambiguous
    );
    assert_eq!(ambiguous_context.authored_id(), &authored);
    assert_eq!(ambiguous_context.command(), SemanticCommand::Activate);
    assert_eq!(ambiguous_context.candidates(), Some(candidates.as_slice()));
}

#[test]
fn automation_02b_underlying_queue_rejection_preserves_request() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let config = RuntimeConfig::default().with_limits(
        RuntimeLimits::default()
            .with_waiting_envelopes(4)
            .with_transaction_outputs(2),
    );
    let mut full = mounted_with_config(InputState::standard(log), config);
    for _ in 0..4 {
        full.submit_action(InputAction::Activated)
            .unwrap_or_else(|_| unreachable!("fixture fills the queue"));
    }
    let authored =
        runenui_core::ElementId::new("target").unwrap_or_else(|_| unreachable!("valid id"));
    let Err(error) = full.submit_automation_command(authored.clone(), SemanticCommand::Activate)
    else {
        unreachable!("underlying exact-target queue rejection propagates");
    };
    assert_eq!(
        error.kind(),
        &SubmitAutomationErrorKind::Command(SubmitCommandErrorKind::Full)
    );
    assert_eq!(error.into_request(), (authored, SemanticCommand::Activate));
}

#[test]
fn automation_03_accepted_target_can_go_stale_without_reresolution() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(log));
    runtime
        .submit_action(InputAction::Replace)
        .unwrap_or_else(|_| unreachable!("replacement enters the FIFO first"));
    runtime
        .submit_automation_command(
            runenui_core::ElementId::new("target").unwrap_or_else(|_| unreachable!("valid id")),
            SemanticCommand::Activate,
        )
        .unwrap_or_else(|_| unreachable!("old exact target is accepted before replacement"));
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, 0);
    assert!(
        kinds(&runtime)
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::AutomationTargetStaleAfterResolution))
    );
    assert_eq!(
        kinds(&runtime)
            .iter()
            .filter(|kind| matches!(kind, TraceRecordKind::AutomationResolutionUnique))
            .count(),
        1,
        "stale work is never resolved again against a replacement"
    );
}

#[test]
fn automation_04_disabled_tracing_preserves_resolution_and_command_behavior() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted_with_config(
        InputState::standard(log),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    );
    runtime
        .submit_automation_command(
            runenui_core::ElementId::new("target").unwrap_or_else(|_| unreachable!("valid id")),
            SemanticCommand::Activate,
        )
        .unwrap_or_else(|_| unreachable!("trace-disabled automation remains admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, 1);
    assert_eq!(runtime.trace().len(), 0);
}

#[test]
fn focusability_only_retirement_preflights_pending_ime_callbacks_with_focusout() {
    // Three-node root/target/other tree: the old FocusOut and a still-live
    // composition cancellation each invoke the old C/T/B route. A preflight
    // that counts only FocusOut undercharges the mandatory callback trace.
    // Work and trace admission remain independent, so set R=1 and make the
    // non-wrapping trace authority the controlled tight resource.
    for active_composition in [false, true] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut runtime = mounted_with_config(
            InputState::standard(Rc::clone(&log)),
            RuntimeConfig::default()
                .with_limits(RuntimeLimits::default().with_transaction_outputs(1)),
        );
        focus(&mut runtime, "target");
        let old = target(&mut runtime, "target");
        if active_composition {
            runtime
                .start_composition(None)
                .unwrap_or_else(|_| unreachable!("focused live owner accepts composition"));
            settle(&mut runtime);
        }
        clear_log(&runtime);
        runtime
            .submit_action(InputAction::RetireFocusability)
            .unwrap_or_else(|_| unreachable!("original action admitted"));
        // Fits the ordinary single FocusOut preflight but not the additional
        // mandatory composition C/T/B invocations and retirement facts.
        runtime.__seed_next_trace_sequence_for_test(u64::MAX - 76);
        let report = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("one source action processed"))
            .report()
            .to_owned();
        assert_eq!(report.processed_envelopes(), 1);
        assert!(!runtime.state().focus_eligible);
        if active_composition {
            assert_eq!(
                runtime.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted),
                "combined mandatory callback and trace plan must reject before mutation of routed state"
            );
            assert_eq!(runtime.focus().focused_node(), Some(&old));
            let observed = log.borrow().clone();
            assert!(
                observed.iter().all(|entry| {
                    entry.input
                        == ObservedInput::CompositionCancel(CompositionCancelReason::Shutdown)
                }),
                "rejected disablement may only route terminal Shutdown cleanup, not the unadmitted Disablement/FocusOut; observed: {observed:?}"
            );
            assert!(runtime.trace().records().any(|record| matches!(
                record.kind(),
                TraceRecordKind::RoutedEventAdmissionRejected {
                    capacity:
                        runenui_runtime::TraceRoutedAdmissionRejection::TraceSequenceExhausted
                }
            )));
        } else {
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            assert_eq!(runtime.focus().focused_node(), None);
            assert!(
                runtime.trace().records().any(|record| matches!(
                    record.kind(),
                    TraceRecordKind::FocusNotificationResolved {
                        kind: runenui_runtime::FocusEventKind::Out
                    }
                ) && record.context().delivery()
                    == Some(TraceDeliveryOutcome::Delivered)),
                "ordinary FocusOut without composition keeps its prior admitted budget"
            );
        }
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "focusability-only revocation must preserve the exact old live route and distinct cleanup reasons"
)]
fn focusability_only_retirement_labels_live_ime_and_space_cleanup_as_disablement() {
    // The widget remains enabled, actionable and composition-capable; only
    // authored focus eligibility changes on the compatible mounted lifetime.
    // Capability-loss cleanup therefore does not pre-cancel composition or
    // revoke Space, and commit_focus_transition must do both correctly.
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    let original_target = target(&mut runtime, "target");
    let device = InputDeviceId::new(31).unwrap_or_else(|| unreachable!("nonzero device"));
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device),
        ))
        .unwrap_or_else(|_| unreachable!("focused owner accepts Space down"));
    settle(&mut runtime);
    runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("focus owner accepts real composition"));
    settle(&mut runtime);
    clear_log(&runtime);
    let trace_start = runtime.trace().len();

    runtime
        .submit_action(InputAction::RetireFocusability)
        .unwrap_or_else(|_| unreachable!("compatible authoring update accepted"));
    let report = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("one source action processed"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 1);
    assert!(!runtime.state().focus_eligible);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(
        runtime
            .index()
            .node(&original_target)
            .map(runenui_runtime::MountedNodeRef::id),
        Some(&original_target),
        "compatibly updated text owner must keep its mounted lifetime"
    );

    let observations = log.borrow();
    assert!(
        observations.iter().any(|entry| {
            entry.node == "target"
                && entry.phase == Some(EventPhase::Target)
                && entry.input
                    == ObservedInput::CompositionCancel(CompositionCancelReason::Disablement)
        }),
        "loss of focusability must cancel live composition with Disablement, not FocusTransfer"
    );
    drop(observations);
    let records = runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let cancelled = records.iter().position(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::CompositionCancelled {
                reason: CompositionCancelReason::Disablement
            }
        ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
    });
    let space = records.iter().position(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::KeyboardSpaceOwnershipCleared {
                reason: runenui_runtime::TraceSpaceCleanupReason::Disablement
            }
        )
    });
    let focus_out = records.iter().position(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::FocusNotificationResolved {
                kind: runenui_runtime::FocusEventKind::Out
            }
        ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
    });
    assert!(
        matches!((cancelled, space, focus_out), (Some(a), Some(b), Some(c)) if a < b && b < c),
        "real cancellation, Space retirement and old-route FocusOut must be ordered and named for disablement"
    );
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one end-to-end reconciliation proof connects composition, Space, FocusOut and stale-release chronology"
)]
fn reconciliation_disablement_retires_active_composition_and_space_before_live_focus_out() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = mounted(InputState::standard(Rc::clone(&log)));
    focus(&mut runtime, "target");
    let original_target = target(&mut runtime, "target");
    let device = InputDeviceId::new(23).unwrap_or_else(|| unreachable!("nonzero device"));
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device),
        ))
        .unwrap_or_else(|_| unreachable!("Space down accepted on focused target"));
    settle(&mut runtime);

    let composition = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("composition-capable owner accepts start"));
    settle(&mut runtime);
    clear_log(&runtime);
    let trace_start = runtime.trace().len();

    runtime
        .submit_action(InputAction::Disable)
        .unwrap_or_else(|_| unreachable!("compatible disablement admitted"));
    let receipt = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("bounded action pump"));
    assert_eq!(receipt.report().processed_envelopes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(
        runtime
            .index()
            .node(&original_target)
            .map(runenui_runtime::MountedNodeRef::id),
        Some(&original_target),
        "the disabled focus owner retains its exact mounted lifetime"
    );
    assert_eq!(runtime.state().activations, 0);
    let observations = log.borrow();
    assert!(
        observations.iter().any(|fact| {
            fact.node == "target"
                && fact.phase == Some(EventPhase::Target)
                && fact.input
                    == ObservedInput::CompositionCancel(CompositionCancelReason::Disablement)
                && fact.cancelable == Some(false)
        }),
        "composition cancellation reaches the still-live old target"
    );
    assert!(
        !observations
            .iter()
            .any(|fact| fact.input == ObservedInput::Unmounted),
        "compatible disablement must not unmount the old widget"
    );
    drop(observations);

    let records = runtime
        .trace()
        .records()
        .skip(trace_start)
        .collect::<Vec<_>>();
    let composition_cancel = records
        .iter()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::Disablement
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        })
        .unwrap_or_else(|| unreachable!("composition cancellation is delivered"));
    let space_cleanup = records
        .iter()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::KeyboardSpaceOwnershipCleared {
                    reason: runenui_runtime::TraceSpaceCleanupReason::Disablement
                }
            )
        })
        .unwrap_or_else(|| unreachable!("Space ownership is revoked on disablement"));
    let focus_out = records
        .iter()
        .position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: runenui_runtime::FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        })
        .unwrap_or_else(|| unreachable!("the live old focus route observes FocusOut"));
    assert!(
        composition_cancel < space_cleanup && space_cleanup < focus_out,
        "IME and Space retirement precede the delivered live-route FocusOut"
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| {
                matches!(
                    record.kind(),
                    TraceRecordKind::FocusNotificationResolved {
                        kind: runenui_runtime::FocusEventKind::Out
                    }
                )
            })
            .count(),
        1,
        "the reconciled focus lifetime retires exactly once"
    );
    assert!(matches!(
        runtime.submit_composition_end(composition.generation().clone()),
        Err(error) if error.kind() == SubmitCompositionErrorKind::StaleGeneration
    ));

    focus(&mut runtime, "other");
    runtime
        .submit_keyboard(keyboard(
            KeyboardPhase::Up,
            PhysicalKey::Space,
            LogicalKey::Space,
            false,
            Some(device),
        ))
        .unwrap_or_else(|_| unreachable!("stale key release enters the canonical input path"));
    settle(&mut runtime);
    assert_eq!(
        runtime.state().activations,
        0,
        "stale Space must not activate another owner"
    );
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one bounded comparison proves both sides of consecutive routed input cleanup admission"
)]
fn composition_output_can_exhaust_the_following_required_focus_out_reservation() {
    // The two routed notifications each conservatively require two free
    // queue positions at a one-output allowance. Capacity for the first
    // notification alone does not guarantee that the entire cleanup chain
    // completes after irreversible application reconciliation.
    for emit_cancel_action in [false, true] {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut state = InputState::standard(Rc::clone(&log));
        state.emit_composition_cancel_action = emit_cancel_action;
        let mut runtime = mounted_with_config(
            state,
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(16)
                    .with_transaction_outputs(1),
            ),
        );
        focus(&mut runtime, "target");
        let owner = target(&mut runtime, "target");
        runtime
            .start_composition(None)
            .unwrap_or_else(|_| unreachable!("live owner accepts composition"));
        settle(&mut runtime);
        clear_log(&runtime);

        // After the first Disable envelope is popped, 14 remain waiting:
        // two slots suffice for composition cancellation, but if its routed
        // callback queues one action only one slot remains for FocusOut.
        runtime
            .submit_action(InputAction::Disable)
            .unwrap_or_else(|_| unreachable!("disabling action is admitted"));
        for _ in 0..14 {
            runtime
                .submit_action(InputAction::Activated)
                .unwrap_or_else(|_| unreachable!("filler action is admitted"));
        }
        // Exclude the external admissions: any later accepted action belongs to
        // the composition callback and can be proven to precede the failure.
        let trace_start = runtime.trace().len();
        let batch = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("bounded pump is observable"));
        assert_eq!(batch.report().processed_envelopes(), 1);
        assert!(!runtime.state().enabled);
        assert_eq!(runtime.state().activations, 0);
        assert!(log.borrow().iter().any(|fact| {
            fact.node == "target"
                && fact.phase == Some(EventPhase::Target)
                && fact.input
                    == ObservedInput::CompositionCancel(CompositionCancelReason::Disablement)
        }));
        let records = runtime
            .trace()
            .records()
            .skip(trace_start)
            .collect::<Vec<_>>();
        let cancelled = records.iter().position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::Disablement
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        });
        assert!(cancelled.is_some());
        let focus_out = records.iter().position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: runenui_runtime::FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        });
        let emitted = records
            .iter()
            .position(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted));
        if emit_cancel_action {
            let emitted_at =
                emitted.unwrap_or_else(|| unreachable!("composition callback output was queued"));
            let rejected_at = records
                .iter()
                .position(|record| {
                    matches!(
                        record.kind(),
                        TraceRecordKind::RoutedEventAdmissionRejected {
                            capacity:
                                runenui_runtime::TraceRoutedAdmissionRejection::WaitingEnvelopes
                        }
                    )
                })
                .unwrap_or_else(|| unreachable!("FocusOut reservation was rejected"));
            assert!(
                cancelled.is_some_and(|at| at < emitted_at) && emitted_at < rejected_at,
                "the first callback output commits before the second notification rejects"
            );
            assert_eq!(runtime.focus().focused_node(), Some(&owner));
            assert_eq!(
                runtime.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
            );
            assert_eq!(
                batch.report().outcome(),
                runenui_runtime::PumpOutcome::Terminal(RuntimeTerminalReason::Poisoned)
            );
            assert!(
                focus_out.is_none(),
                "the second callback cannot claim delivery"
            );
            assert!(records.iter().any(|record| matches!(
                record.kind(),
                TraceRecordKind::RoutedEventAdmissionRejected {
                    capacity: runenui_runtime::TraceRoutedAdmissionRejection::WaitingEnvelopes
                }
            )));
        } else {
            assert!(
                emitted.is_none(),
                "an output-free composition callback creates no extra action"
            );
            assert_eq!(runtime.status(), RuntimeStatus::Running);
            assert_eq!(runtime.focus().focused_node(), None);
            assert!(
                focus_out.is_some_and(|position| position > cancelled.unwrap_or_default()),
                "composition cleanup precedes the delivered FocusOut"
            );
        }
    }
}

#[test]
fn focusability_only_retirement_keeps_ime_and_focus_out_in_one_output_transaction() {
    // Unlike capability-loss cleanup, focusability-only retirement preserves
    // composition eligibility until the live old-route focus transition.
    // Its cancellation and FocusOut must share one conservative routed
    // admission and output allowance, including at the exact FIFO boundary.
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut state = InputState::standard(Rc::clone(&log));
    state.emit_composition_cancel_action = true;
    let mut runtime = mounted_with_config(
        state,
        RuntimeConfig::default().with_limits(
            RuntimeLimits::default()
                .with_waiting_envelopes(16)
                .with_transaction_outputs(1),
        ),
    );
    focus(&mut runtime, "target");
    let original = target(&mut runtime, "target");
    runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("live focus owner accepts composition"));
    settle(&mut runtime);
    clear_log(&runtime);

    runtime
        .submit_action(InputAction::RetireFocusability)
        .unwrap_or_else(|_| unreachable!("retirement action accepted"));
    for _ in 0..14 {
        runtime
            .submit_action(InputAction::Activated)
            .unwrap_or_else(|_| unreachable!("filler action accepted"));
    }
    // After the first pop, q=14 and C=16: exactly 2*R free slots.
    // The composition callback emits one action; no second reservation may
    // be demanded for the FocusOut within this SAME routed transaction.
    let trace_start = runtime.trace().len();
    let first = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("one action processed"))
        .report()
        .to_owned();
    assert_eq!(first.processed_envelopes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(!runtime.state().focus_eligible);
    assert_eq!(runtime.focus().focused_node(), None);
    assert_eq!(
        runtime
            .index()
            .node(&original)
            .map(runenui_runtime::MountedNodeRef::id),
        Some(&original),
        "the original mounted widget remains live"
    );
    assert!(log.borrow().iter().any(|entry| {
        entry.node == "target"
            && entry.phase == Some(EventPhase::Target)
            && entry.input
                == ObservedInput::CompositionCancel(CompositionCancelReason::Disablement)
    }));

    {
        let records = runtime
            .trace()
            .records()
            .skip(trace_start)
            .collect::<Vec<_>>();
        let cancelled = records.iter().position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::Disablement
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        });
        let focus_out = records.iter().position(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: runenui_runtime::FocusEventKind::Out
                }
            ) && record.context().delivery() == Some(TraceDeliveryOutcome::Delivered)
        });
        let accepted = records
            .iter()
            .position(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted));
        assert!(
            matches!((cancelled, focus_out, accepted), (Some(a), Some(b), Some(c)) if a < b && b < c),
            "composition cancellation and FocusOut must finish before their shared callback output is committed"
        );
        assert_eq!(
            records
                .iter()
                .filter(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted))
                .count(),
            1,
            "one callback action is committed once"
        );
    }

    // All accepted older actions precede the callback-produced tail action.
    let followers = runtime
        .pump(PumpBudget::new(14, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("older admitted actions process"))
        .report()
        .to_owned();
    assert_eq!(followers.processed_envelopes(), 14);
    assert_eq!(runtime.state().activations, 14);
    let callback_tail = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("callback action processes at FIFO tail"))
        .report()
        .to_owned();
    assert_eq!(callback_tail.processed_envelopes(), 1);
    assert_eq!(runtime.state().activations, 15);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}
