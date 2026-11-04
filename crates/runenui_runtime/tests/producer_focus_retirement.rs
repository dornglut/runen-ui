#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    ChildBearingWidget, CommandOrigin, CompositionCancelReason, CompositionEvent, Effects, Element,
    EventContext, EventPhase, FocusEventKind, FocusReason, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, ProducerBudget, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetProducerBudget, WidgetProducerPhase, WidgetTextInput, container,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    SubmitActionErrorKind, SubmitKeyboardErrorKind, TraceConfig, TraceRecordKind,
    TraceSpaceCleanupReason, UiFocusRetirementKind,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Disable,
    Keep,
    Remove,
    Replace,
    Noop,
    Notification(EventPhase),
    CancelOutput(EventPhase),
    AppOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Fact {
    Held(bool),
    Out(EventPhase, FocusReason, bool),
    Unmounted(bool),
    Applied(EventPhase),
    Cancel(EventPhase, CompositionCancelReason),
    AppliedCancel(EventPhase),
    AppOutput,
}

type Log = Rc<RefCell<Vec<Fact>>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputMode {
    Honest,
    Overrun,
    StopAtCapture,
    StopAtTarget,
}

struct State {
    eligible: bool,
    present: bool,
    replacement: bool,
    noops: usize,
    log: Log,
    output_mode: OutputMode,
}

#[derive(Debug)]
struct Parent {
    log: Log,
    notification_outputs: bool,
    output_mode: OutputMode,
}

impl Widget<Action> for Parent {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(usize::from(
            phase == WidgetProducerPhase::Event && self.notification_outputs,
        ))
        .with_subscriptions(0)
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let Some(focus) = event
            .as_focus()
            .filter(|focus| focus.kind() == FocusEventKind::Out)
        {
            self.log
                .borrow_mut()
                .push(Fact::Out(context.phase(), focus.reason(), false));
            if self.output_mode == OutputMode::StopAtCapture
                && context.phase() == EventPhase::Capture
            {
                context.stop_propagation();
            }
            if self.notification_outputs {
                let _ = context.try_emit(Action::Notification(context.phase()));
            }
            context.prevent_default();
        }
        if let UiEvent::Composition(CompositionEvent::Cancel(cancel)) = event {
            self.log
                .borrow_mut()
                .push(Fact::Cancel(context.phase(), cancel.reason()));
            if self.notification_outputs {
                let _ = context.try_emit(Action::CancelOutput(context.phase()));
            }
        }
        WidgetEventOutput::none()
    }
}
impl ChildBearingWidget<Action> for Parent {}

#[derive(Debug)]
struct KeyOwner {
    log: Log,
    output_mode: OutputMode,
    notification_outputs: bool,
}

impl Widget<Action> for KeyOwner {
    type State = bool;
    fn create_state(&self) -> bool {
        false
    }
    fn activation(&self, _: &bool) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn text_input(&self, _: &bool) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
    fn producer_budget(&self, phase: WidgetProducerPhase, _: &bool) -> WidgetProducerBudget {
        WidgetProducerBudget::new(usize::from(
            phase == WidgetProducerPhase::Event && self.notification_outputs,
        ))
        .with_subscriptions(0)
    }
    fn event(
        &mut self,
        held: &mut bool,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let UiEvent::Keyboard(key) = event {
            *held = key.phase() == KeyboardPhase::Down;
            self.log.borrow_mut().push(Fact::Held(*held));
        }
        if let Some(focus) = event
            .as_focus()
            .filter(|focus| focus.kind() == FocusEventKind::Out)
        {
            *held = false;
            self.log
                .borrow_mut()
                .push(Fact::Out(context.phase(), focus.reason(), *held));
            if self.output_mode == OutputMode::StopAtTarget
                && context.phase() == EventPhase::Target
            {
                context.stop_propagation();
            }
            if self.notification_outputs {
                let _ = context.try_emit(Action::Notification(context.phase()));
                if self.output_mode == OutputMode::Overrun {
                    let _ = context.try_emit(Action::Noop);
                }
            }
        }
        if let UiEvent::Composition(CompositionEvent::Cancel(cancel)) = event {
            self.log
                .borrow_mut()
                .push(Fact::Cancel(context.phase(), cancel.reason()));
            if self.notification_outputs {
                let _ = context.try_emit(Action::CancelOutput(context.phase()));
            }
        }
        WidgetEventOutput::none()
    }

    fn unmount(&self, held: &mut bool, _: &mut runenui_core::WidgetUnmountContext) {
        self.log.borrow_mut().push(Fact::Unmounted(*held));
    }
}

struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(2, 0)
            .with_reconciliation(2, 0, 2)
            .with_subscriptions(0, 0)
    }
    fn action_producer_budget(_: &State, action: &Action) -> ProducerBudget {
        match action {
            // Two retained owners: two independent three-callback routes can
            // each emit three outputs, plus one initiating application output.
            // Replacement may mount one leaf; no async work can appear.
            Action::Disable | Action::Keep | Action::Remove | Action::Replace => {
                ProducerBudget::new(9, 7)
                    .with_application_outputs(1)
                    .with_reconciliation(1, 2, 1)
            }
            Action::Noop
            | Action::Notification(_)
            | Action::CancelOutput(_)
            | Action::AppOutput => ProducerBudget::new(1, 0).with_reconciliation(0, 2, 0),
        }
        .with_subscriptions(0, 0)
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        // Ordinary C/T/B, four per-owner focus allowances, and one default.
        ProducerBudget::new(15, 12)
            .with_application_outputs(1)
            .with_reconciliation(0, 2, 0)
            .with_subscriptions(0, 0)
    }
    fn root(state: &State) -> impl View<Action> {
        let children = if state.present {
            vec![
                Element::new(KeyOwner {
                    log: Rc::clone(&state.log),
                    output_mode: state.output_mode,
                    notification_outputs: !state.eligible,
                })
                .id("key-owner")
                .key(if state.replacement {
                    "replacement"
                } else {
                    "key-owner"
                })
                .focusable(state.eligible),
            ]
        } else {
            Vec::new()
        };
        container(
            Parent {
                log: Rc::clone(&state.log),
                notification_outputs: !state.eligible,
                output_mode: state.output_mode,
            },
            children,
        )
        .key("parent")
    }
    fn update(state: &mut State, action: Action) -> Effects<Action, NoHostProtocol> {
        match action {
            Action::Disable => {
                state.eligible = false;
                return Effects::action(Action::AppOutput);
            }
            Action::Keep => {}
            Action::Remove => state.present = false,
            Action::Replace => state.replacement = true,
            Action::Noop => state.noops += 1,
            Action::Notification(phase) => {
                state.noops += 1;
                state.log.borrow_mut().push(Fact::Applied(phase));
            }
            Action::CancelOutput(phase) => {
                state.noops += 1;
                state.log.borrow_mut().push(Fact::AppliedCancel(phase));
            }
            Action::AppOutput => state.log.borrow_mut().push(Fact::AppOutput),
        }
        Effects::none()
    }
}

const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);

fn focused_held_runtime(
    capacity: usize,
    trace_capacity: usize,
    output_mode: OutputMode,
) -> AppRuntime<App> {
    focused_held_runtime_with_limits(
        capacity,
        trace_capacity,
        output_mode,
        RuntimeLimits::default(),
    )
}

fn focused_held_runtime_with_limits(
    capacity: usize,
    trace_capacity: usize,
    output_mode: OutputMode,
    limits: RuntimeLimits,
) -> AppRuntime<App> {
    let mut app = AppRuntime::<App>::mount_with_config(
        State {
            eligible: true,
            present: true,
            replacement: false,
            noops: 0,
            log: Rc::new(RefCell::new(Vec::new())),
            output_mode,
        },
        RuntimeConfig::default()
            .with_limits(limits)
            .with_queue_capacity(capacity)
            .with_trace_config(TraceConfig::new(trace_capacity)),
    )
    .unwrap_or_else(|error| unreachable!("initial two-owner frontier fits: {error}"));
    assert!(
        app.pump(FULL)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    let target = app.index().nodes()[1].id().clone();
    app.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|error| unreachable!("interaction frontier fits: {error}"));
    assert!(
        app.pump(FULL)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    let key = KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code("KeyW".into()),
        LogicalKey::Character("w".into()),
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    );
    app.submit_keyboard(key)
        .unwrap_or_else(|_| unreachable!("focused W admitted"));
    assert!(
        app.pump(FULL)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    assert_eq!(&*app.state().log.borrow(), &[Fact::Held(true)]);
    app.state().log.borrow_mut().clear();
    app
}

fn fill_followers(app: &mut AppRuntime<App>, capacity: usize) -> usize {
    let mut followers = 0;
    let refused = loop {
        match app.submit_action(Action::Noop) {
            Ok(_) => {
                followers += 1;
                assert!(followers < capacity);
            }
            Err(error) => break error,
        }
    };
    assert_eq!(refused.kind(), SubmitActionErrorKind::Full);
    assert_eq!(refused.into_action(), Action::Noop);
    let lifetime_credits = if app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!())
        .keyboard()
        .composition_generation()
        .is_some()
    {
        15
    } else {
        0
    };
    let space_credits = if app
        .input_ownership()
        .unwrap_or_else(|_| unreachable!())
        .keyboard()
        .space_activation_owner()
        .is_some()
    {
        15
    } else {
        0
    };
    assert_eq!(followers, capacity - 9 - lifetime_credits - space_credits);
    followers
}

#[test]
fn unchanged_live_route_delivers_non_cancelable_focusout_under_full_follower_pressure() {
    for (capacity, trace) in [16, 32, 4096]
        .into_iter()
        .flat_map(|capacity| [0, 512].map(|trace| (capacity, trace)))
    {
        let mut app = focused_held_runtime(capacity, trace, OutputMode::Honest);
        let target = app
            .focus()
            .focused_node()
            .cloned()
            .unwrap_or_else(|| unreachable!());
        app.submit_action(Action::Disable)
            .unwrap_or_else(|_| unreachable!("complete disabling frontier fits"));
        let followers = fill_followers(&mut app, capacity);
        assert_eq!(
            app.pump(PumpBudget::new(0, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!())
                .report()
                .processed_envelopes(),
            0
        );
        assert!(app.state().eligible);
        assert!(app.state().log.borrow().is_empty());
        let committed = app
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        let retired = committed
            .ordered_records()
            .iter()
            .filter_map(|record| match record {
                InputArbitrationRecord::FocusRecipientRetired(value) => Some(value),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            retired.len(),
            1,
            "one committed live FocusOut, not one per callback phase"
        );
        assert_eq!(retired[0].recipient(), &target);
        assert_eq!(retired[0].scope(), committed.final_ownership().scope());
        assert_eq!(retired[0].kind(), UiFocusRetirementKind::DeliveredFocusOut);
        assert!(retired[0].focus_generation() > 0);
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert_eq!(
            app.index().nodes()[1].id(),
            &target,
            "retirement retains the exact mounted widget"
        );
        assert!(app.focus().focused_node().is_none());
        assert_eq!(
            &*app.state().log.borrow(),
            &[
                Fact::Out(EventPhase::Capture, FocusReason::Disablement, false),
                Fact::Out(EventPhase::Target, FocusReason::Disablement, false),
                Fact::Out(EventPhase::Bubble, FocusReason::Disablement, false),
            ]
        );
        assert_eq!(app.state().noops, 0, "notification outputs remain queued");
        let later = app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert!(later.report().is_quiescent());
        assert!(
            !later
                .ordered_records()
                .iter()
                .any(|record| matches!(record, InputArbitrationRecord::FocusRecipientRetired(_))),
            "already delivered proof must not replay"
        );
        assert_eq!(app.state().noops, followers + 3);
        assert_eq!(
            &*app.state().log.borrow(),
            &[
                Fact::Out(EventPhase::Capture, FocusReason::Disablement, false),
                Fact::Out(EventPhase::Target, FocusReason::Disablement, false),
                Fact::Out(EventPhase::Bubble, FocusReason::Disablement, false),
                Fact::Applied(EventPhase::Capture),
                Fact::Applied(EventPhase::Target),
                Fact::Applied(EventPhase::Bubble),
                Fact::AppOutput,
            ],
            "notification outputs precede the initiating application output exactly once"
        );
    }
}

#[test]
fn delivered_old_target_focusout_certifies_even_if_bubble_is_stopped() {
    for trace in [0, 512] {
        let mut app = focused_held_runtime(32, trace, OutputMode::StopAtTarget);
        let old = app
            .focus()
            .focused_node()
            .cloned()
            .unwrap_or_else(|| unreachable!("old focused target"));
        app.submit_action(Action::Disable)
            .unwrap_or_else(|_| unreachable!("disablement admitted"));
        let batch = app.pump(FULL).unwrap_or_else(|_| unreachable!("focus action processed"));
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert!(app.focus().focused_node().is_none());
        assert!(app.state().log.borrow().iter().any(|fact| matches!(
            fact,
            Fact::Out(EventPhase::Target, FocusReason::Disablement, false)
        )));
        assert!(!app.state().log.borrow().iter().any(|fact| matches!(
            fact,
            Fact::Out(EventPhase::Bubble, FocusReason::Disablement, _)
        )));
        assert!(batch.ordered_records().iter().any(|record| matches!(
            record,
            InputArbitrationRecord::FocusRecipientRetired(value)
                if value.recipient() == &old
                    && value.kind() == UiFocusRetirementKind::DeliveredFocusOut
        )));
    }
}

#[test]
fn interrupted_focusout_route_never_certifies_old_target_cleanup() {
    for trace in [0, 512] {
        let mut app = focused_held_runtime(32, trace, OutputMode::StopAtCapture);
        app.submit_action(Action::Disable)
            .unwrap_or_else(|_| unreachable!("disablement admitted"));
        let batch = app
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("focus action processed"));
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert!(app.focus().focused_node().is_none());
        assert!(app.state().log.borrow().iter().any(|fact| matches!(
            fact,
            Fact::Out(EventPhase::Capture, FocusReason::Disablement, false)
        )));
        assert!(!app.state().log.borrow().iter().any(|fact| matches!(
            fact,
            Fact::Out(EventPhase::Target, FocusReason::Disablement, _)
        )));
        assert!(
            !batch
                .ordered_records()
                .iter()
                .any(|record| matches!(record, InputArbitrationRecord::FocusRecipientRetired(_)))
        );
        if trace != 0 {
            assert!(app.trace().records().any(|record| matches!(
                record.kind(),
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            ) && record.context().delivery()
                == Some(runenui_runtime::TraceDeliveryOutcome::Suppressed)));
        }
    }
}

#[test]
fn old_live_focus_retirement_does_not_fabricate_no_recipient_keyboard_up_or_cancel() {
    for trace in [0, 512] {
        let mut app = focused_held_runtime(32, trace, OutputMode::Honest);
        app.submit_action(Action::Disable)
            .unwrap_or_else(|_| unreachable!("retirement admission"));
        let retired = app
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("retirement pump"));
        assert!(retired.ordered_records().iter().any(|record| matches!(
            record,
            InputArbitrationRecord::FocusRecipientRetired(value)
                if value.kind() == UiFocusRetirementKind::DeliveredFocusOut
        )));
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert_eq!(app.focus().focused_node(), None);
        for phase in [KeyboardPhase::Up, KeyboardPhase::Cancel] {
            let release = KeyboardEvent::new(
                phase,
                PhysicalKey::Code("KeyW".into()),
                LogicalKey::Character("w".into()),
                KeyModifiers::NONE,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            );
            let error = app
                .submit_keyboard(release)
                .err()
                .unwrap_or_else(|| unreachable!("no recipient must reject original M13 ingress"));
            assert_eq!(error.kind(), SubmitKeyboardErrorKind::NoFocusedTarget);
        }
        let after = app
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("post-rejection observation"));
        assert!(
            !after.ordered_records().iter().any(|record| matches!(
                record,
                InputArbitrationRecord::InputSettled(_)
                    | InputArbitrationRecord::FocusRecipientRetired(_)
            )),
            "no-focus terminal is host-owned, not a fabricated M13 settlement"
        );
    }
}

#[test]
fn retained_focus_and_removed_or_replaced_owners_do_not_receive_disable_focusout() {
    for (action, trace) in [Action::Keep, Action::Remove, Action::Replace]
        .into_iter()
        .flat_map(|action| [0, 512].map(|trace| (action, trace)))
    {
        let mut app = focused_held_runtime(16, trace, OutputMode::Honest);
        let old = app
            .focus()
            .focused_node()
            .cloned()
            .unwrap_or_else(|| unreachable!());
        app.submit_action(action).unwrap_or_else(|_| unreachable!());
        let _ = fill_followers(&mut app, 16);
        let observed = app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert!(observed.report().is_quiescent());
        assert_eq!(app.status(), RuntimeStatus::Running);
        let retirements = observed
            .ordered_records()
            .iter()
            .filter_map(|record| match record {
                InputArbitrationRecord::FocusRecipientRetired(value) => Some(value),
                _ => None,
            })
            .collect::<Vec<_>>();
        if action == Action::Keep {
            assert!(retirements.is_empty(), "retaining focus is not retirement");
            assert_eq!(app.focus().focused_node(), Some(&old));
            assert!(app.state().log.borrow().is_empty());
        } else {
            assert_eq!(retirements.len(), 1);
            assert_eq!(retirements[0].recipient(), &old);
            assert_eq!(retirements[0].kind(), UiFocusRetirementKind::Unmounted);
            assert!(app.focus().focused_node().is_none());
            assert_eq!(
                &*app.state().log.borrow(),
                &[Fact::Unmounted(true)],
                "retired owners receive no post-unmount callback"
            );
            if action == Action::Replace {
                assert_ne!(app.index().nodes()[1].id(), &old);
            }
        }
    }
}

#[test]
fn composition_and_space_retire_before_live_focusout_without_losing_independent_outputs() {
    // Space and composition each retain fifteen credits beside the independent
    // nine-credit disablement action. Their exact-fit boundary is 39.
    for (capacity, trace, outputs_per_route) in [39, 64, 4096]
        .into_iter()
        .flat_map(|capacity| [0, 512].map(|trace| (capacity, trace)))
        .flat_map(|(capacity, trace)| [3, 1024].map(|limit| (capacity, trace, limit)))
    {
        let mut app = focused_held_runtime_with_limits(
            capacity,
            trace,
            OutputMode::Honest,
            RuntimeLimits::default().with_transaction_outputs(outputs_per_route),
        );
        let space = KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Space,
            LogicalKey::Space,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        );
        app.submit_keyboard(space)
            .unwrap_or_else(|_| unreachable!("Space profile fits"));
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        let space_owner = app
            .input_ownership()
            .unwrap_or_else(|_| unreachable!())
            .keyboard()
            .space_activation_owner()
            .cloned();
        assert!(
            space_owner.is_some(),
            "real admitted Space Down binds an owner"
        );
        app.start_composition(None)
            .unwrap_or_else(|_| unreachable!("composition owner is live"));
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        let before = app.input_ownership().unwrap_or_else(|_| unreachable!());
        assert_eq!(
            before.keyboard().space_activation_owner(),
            space_owner.as_ref()
        );
        assert!(before.keyboard().composition_generation().is_some());
        app.state().log.borrow_mut().clear();
        app.submit_action(Action::Disable)
            .unwrap_or_else(|_| unreachable!());
        let followers = fill_followers(&mut app, capacity);
        app.pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert_eq!(
            &*app.state().log.borrow(),
            &[
                Fact::Cancel(EventPhase::Capture, CompositionCancelReason::Disablement),
                Fact::Cancel(EventPhase::Target, CompositionCancelReason::Disablement),
                Fact::Cancel(EventPhase::Bubble, CompositionCancelReason::Disablement),
                Fact::Out(EventPhase::Capture, FocusReason::Disablement, false),
                Fact::Out(EventPhase::Target, FocusReason::Disablement, false),
                Fact::Out(EventPhase::Bubble, FocusReason::Disablement, false),
            ]
        );
        let after = app.input_ownership().unwrap_or_else(|_| unreachable!());
        assert!(after.keyboard().space_activation_owner().is_none());
        assert!(after.keyboard().composition_generation().is_none());
        if trace != 0 {
            assert_cleanup_trace_order(&app);
        }
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(app.state().noops, followers + 6);
        let log = app.state().log.borrow();
        assert_eq!(
            &log[6..],
            &[
                Fact::AppliedCancel(EventPhase::Capture),
                Fact::AppliedCancel(EventPhase::Target),
                Fact::AppliedCancel(EventPhase::Bubble),
                Fact::Applied(EventPhase::Capture),
                Fact::Applied(EventPhase::Target),
                Fact::Applied(EventPhase::Bubble),
                Fact::AppOutput,
            ]
        );
    }
}

fn assert_cleanup_trace_order(app: &AppRuntime<App>) {
    let records = app
        .trace()
        .records()
        .map(runenui_runtime::TraceRecord::kind)
        .collect::<Vec<_>>();
    let composition = records
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::CompositionCancelled {
                    reason: CompositionCancelReason::Disablement
                }
            )
        })
        .unwrap_or_else(|| unreachable!("composition cancellation is recorded"));
    let space = records
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::KeyboardSpaceOwnershipCleared {
                    reason: TraceSpaceCleanupReason::Disablement
                }
            )
        })
        .unwrap_or_else(|| unreachable!("Space retirement is recorded"));
    let focusout = records
        .iter()
        .position(|kind| {
            matches!(
                kind,
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out
                }
            )
        })
        .unwrap_or_else(|| unreachable!("FocusOut is recorded"));
    assert!(
        composition < space && space < focusout,
        "composition and Space retire before FocusOut"
    );
}

#[test]
fn undeclared_focusout_fanout_poison_commits_no_provisional_outputs() {
    for trace in [0, 512] {
        let mut app = focused_held_runtime(16, trace, OutputMode::Overrun);
        app.submit_action(Action::Disable)
            .unwrap_or_else(|_| unreachable!());
        let followers = fill_followers(&mut app, 16);
        let batch = app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            app.status(),
            RuntimeStatus::Terminal(
                runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase: runenui_core::ProducerPhase::Widget(
                            runenui_core::WidgetProducerPhase::Event
                        ),
                        component: runenui_core::BudgetComponent::Outputs,
                        declared: 1,
                        attempted: Some(2)
                    }
                )
            )
        );
        assert_eq!(batch.report().cancelled_by_terminal_transition(), followers);
        assert!(
            !batch
                .ordered_records()
                .iter()
                .any(|record| matches!(record, InputArbitrationRecord::FocusRecipientRetired(_))),
            "partial/failing FocusOut is not a retirement certificate"
        );
        assert_eq!(app.state().noops, 0);
        assert_eq!(
            &*app.state().log.borrow(),
            &[
                Fact::Out(EventPhase::Capture, FocusReason::Disablement, false),
                Fact::Out(EventPhase::Target, FocusReason::Disablement, false),
            ],
            "neither staged notification output nor the initiating app output may execute"
        );
        let repeated = app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(repeated.report().processed_envelopes(), 0);
        assert_eq!(repeated.report().cancelled_by_terminal_transition(), 0);
        assert_eq!(app.state().log.borrow().len(), 2);
    }
}

#[test]
fn queued_predecessor_cannot_use_the_current_zero_callback_budget_as_admission_proof() {
    for trace in [0, 512] {
        let mut app = focused_held_runtime_with_limits(
            32,
            trace,
            OutputMode::Honest,
            RuntimeLimits::default().with_transaction_outputs(3),
        );
        let target = app
            .focus()
            .focused_node()
            .cloned()
            .unwrap_or_else(|| unreachable!());
        app.submit_action(Action::Disable)
            .unwrap_or_else(|_| unreachable!("two independent three-output routes fit"));
        // Both widgets currently declare zero event outputs. The already
        // accepted app update changes both to one before this command can
        // execute. Its ordinary route plus deferred focus allowance then needs
        // more than the configured per-route cap, despite free FIFO capacity.
        let refused = app
            .submit_command(
                target.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::controller(),
            )
            .err().unwrap_or_else(|| unreachable!(
                "current mounted state cannot certify a successor of an arbitrary queued producer"
            ));
        assert_eq!(
            refused.kind(),
            runenui_runtime::SubmitCommandErrorKind::InvalidProducerProfile
        );
        assert_eq!(
            refused.into_unaccepted().into_parts(),
            (
                target.clone(),
                SemanticCommand::RequestFocus,
                CommandOrigin::controller()
            )
        );
        assert!(app.state().eligible);
        assert!(app.state().log.borrow().is_empty());
        assert_eq!(app.focus().focused_node(), Some(&target));
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert_eq!(app.state().noops, 3);
        assert!(app.focus().focused_node().is_none());
        assert_eq!(app.state().log.borrow().last(), Some(&Fact::AppOutput));
    }
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn whole_turn_trace_shortage_precedes_qualified_update() {
    let mut app = focused_held_runtime(32, 512, OutputMode::Honest);
    app.submit_action(Action::Disable)
        .unwrap_or_else(|_| unreachable!());
    // This is a fault model for the counter state left by predecessors, not
    // ordinary trace retention pressure. The base action plan alone still fits.
    app.__seed_next_trace_sequence_for_test(u64::MAX - 12);
    app.pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!());
    assert!(
        app.state().eligible,
        "whole-turn refusal precedes irreversible update"
    );
    assert!(app.state().log.borrow().is_empty());
    assert_eq!(
        app.status(),
        RuntimeStatus::Terminal(runenui_runtime::RuntimeTerminalReason::TraceSequenceExhausted)
    );
}
