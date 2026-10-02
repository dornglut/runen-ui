#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{
    ApplicationCommand, ApplicationCommandDisposition, ApplicationCommandId, ChildBearingWidget,
    CommandOrigin, Element, EventContext, EventPhase, NoHostProtocol, SemanticCommand,
    SemanticCommandEvent, StyleEnvironment, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    command_binding, command_scope, container, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, RuntimeConfig, RuntimeLimits, SurfaceBuildContext,
    TraceApplicationCommandOutcome, TraceRecordKind, TraceRoutedAdmissionRejection,
    TraceTargetRejection,
};

const COMMAND: &str = "document.save";

fn command_id() -> ApplicationCommandId {
    ApplicationCommandId::from_static(COMMAND)
        .unwrap_or_else(|_| unreachable!("test command id is valid"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Enabled,
    Disabled,
    Ambiguous,
    Unbound,
    CustomScope,
}

#[derive(Debug)]
enum Action {
    Inner,
    Outer,
    Replace,
    SetMode(Mode),
}

#[derive(Debug)]
struct State {
    mode: Mode,
    replaced: bool,
    updates: Vec<&'static str>,
    emitter_calls: Rc<Cell<usize>>,
}

#[derive(Debug)]
struct CommandEmitter {
    command: ApplicationCommandId,
    calls: Rc<Cell<usize>>,
}

impl Widget<Action> for CommandEmitter {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Bubble
            && matches!(
                event
                    .as_semantic_command()
                    .map(SemanticCommandEvent::command),
                Some(SemanticCommand::OpenMenu)
            )
        {
            self.calls.set(self.calls.get() + 1);
            context.emit_application_command(self.command.clone());
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for CommandEmitter {}

#[derive(Debug)]
struct DownstreamCommandScope {
    command: ApplicationCommandId,
}

impl Widget<Action> for DownstreamCommandScope {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Capture
            && event
                .as_application_command()
                .is_some_and(|event| event.command() == &self.command)
        {
            context.consume_application_command(ApplicationCommandDisposition::Resolved);
            context.emit(Action::Inner);
        }
        WidgetEventOutput::none()
    }
}

impl ChildBearingWidget<Action> for DownstreamCommandScope {}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let id = command_id();
        let leaf = text("leaf").id("leaf").key(if state.replaced {
            "leaf-new"
        } else {
            "leaf-old"
        });

        let inner: Element<Action> = match state.mode {
            Mode::CustomScope => container(
                DownstreamCommandScope {
                    command: id.clone(),
                },
                [leaf],
            )
            .key("inner-custom")
            .into_element(),
            mode => {
                let bindings = match mode {
                    Mode::Enabled => vec![command_binding(
                        ApplicationCommand::new(id.clone(), true),
                        || Action::Inner,
                    )],
                    Mode::Disabled => vec![command_binding(
                        ApplicationCommand::new(id.clone(), false),
                        || Action::Inner,
                    )],
                    Mode::Ambiguous => vec![
                        command_binding(ApplicationCommand::new(id.clone(), true), || {
                            Action::Inner
                        }),
                        command_binding(ApplicationCommand::new(id.clone(), true), || {
                            Action::Inner
                        }),
                    ],
                    Mode::Unbound => Vec::new(),
                    Mode::CustomScope => unreachable!(),
                };
                command_scope(bindings, [leaf]).key("inner").into_element()
            }
        };

        // This emitter is deliberately outside the inner scope. If delegated command
        // emission incorrectly targets the current callback node instead of the original
        // interaction target, later lookup bypasses the inner scope and reaches the outer one.
        let emitted = container(
            CommandEmitter {
                command: id.clone(),
                calls: Rc::clone(&state.emitter_calls),
            },
            [inner],
        )
        .key("emitter");

        let outer_bindings = if state.mode == Mode::Unbound {
            Vec::new()
        } else {
            vec![command_binding(ApplicationCommand::new(id, true), || {
                Action::Outer
            })]
        };
        command_scope(outer_bindings, [emitted])
            .key("outer")
            .into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Inner => state.updates.push("inner"),
            Action::Outer => state.updates.push("outer"),
            Action::Replace => state.replaced = true,
            Action::SetMode(mode) => state.mode = mode,
        }
    }
}

fn state(mode: Mode) -> State {
    State {
        mode,
        replaced: false,
        updates: Vec::new(),
        emitter_calls: Rc::new(Cell::new(0)),
    }
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn pump_one(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
}

fn leaf(runtime: &mut AppRuntime<App>) -> runenui_runtime::MountedNodeId {
    let id = runenui_core::ElementId::from_static("leaf")
        .unwrap_or_else(|_| unreachable!("test element id is valid"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("leaf is mounted"))
        .id()
        .clone()
}

fn submit_trigger(runtime: &mut AppRuntime<App>) {
    let target = leaf(runtime);
    runtime
        .submit_command(
            target,
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live trigger target is accepted"));
}

fn has_outcome(runtime: &AppRuntime<App>, expected: TraceApplicationCommandOutcome) -> bool {
    runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::ApplicationCommandResolution { outcome } if *outcome == expected
        )
    })
}

#[test]
fn command_identity_reuses_validated_authored_identifier_grammar() {
    assert!(ApplicationCommandId::new("document.save").is_ok());
    assert!(ApplicationCommandId::new("").is_err());
    assert!(ApplicationCommandId::new(" leading").is_err());
    assert!(ApplicationCommandId::new("line\nbreak").is_err());
}

#[test]
fn ancestor_emission_retains_original_target_and_nearest_scope_resolves_non_reentrantly() {
    let mut runtime = AppRuntime::<App>::mount(state(Mode::Enabled));
    settle(&mut runtime);
    submit_trigger(&mut runtime);

    pump_one(&mut runtime);
    assert_eq!(runtime.state().emitter_calls.get(), 1);
    assert_eq!(runtime.state().updates, []);

    pump_one(&mut runtime);
    assert_eq!(runtime.state().updates, [], "scope action remains queued");
    assert!(has_outcome(
        &runtime,
        TraceApplicationCommandOutcome::Resolved
    ));

    pump_one(&mut runtime);
    assert_eq!(runtime.state().updates, ["inner"]);
}

#[test]
fn disabled_and_ambiguous_inner_scopes_shadow_outer_bindings() {
    for (mode, expected) in [
        (Mode::Disabled, TraceApplicationCommandOutcome::Disabled),
        (Mode::Ambiguous, TraceApplicationCommandOutcome::Ambiguous),
    ] {
        let mut runtime = AppRuntime::<App>::mount(state(mode));
        settle(&mut runtime);
        submit_trigger(&mut runtime);
        pump_one(&mut runtime);
        pump_one(&mut runtime);
        pump_one(&mut runtime);
        assert_eq!(runtime.state().updates, []);
        assert!(has_outcome(&runtime, expected));
    }
}

#[test]
fn duplicate_scope_diagnostic_is_published_and_observable() {
    let mut runtime = AppRuntime::<App>::mount(state(Mode::Ambiguous));
    settle(&mut runtime);
    let style = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("diagnostic surface publication is admitted"));

    assert!(
        publication
            .frame()
            .nodes()
            .iter()
            .flat_map(runenui_runtime::SurfaceNode::diagnostics)
            .any(|diagnostic| diagnostic.code() == "runenui.command-scope.duplicate-command")
    );
}

#[test]
fn retained_scope_rebuild_updates_duplicate_diagnostic_publication() {
    let mut runtime = AppRuntime::<App>::mount(state(Mode::Enabled));
    settle(&mut runtime);
    let style = StyleEnvironment::default();

    let duplicate_count = |runtime: &mut AppRuntime<App>| {
        runtime
            .publish_surface(&SurfaceBuildContext::new(
                &style,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("diagnostic surface publication is admitted"))
            .frame()
            .nodes()
            .iter()
            .flat_map(runenui_runtime::SurfaceNode::diagnostics)
            .filter(|diagnostic| diagnostic.code() == "runenui.command-scope.duplicate-command")
            .count()
    };

    assert_eq!(duplicate_count(&mut runtime), 0);

    runtime
        .submit_action(Action::SetMode(Mode::Ambiguous))
        .unwrap_or_else(|_| unreachable!("mode change is accepted"));
    settle(&mut runtime);
    assert_eq!(duplicate_count(&mut runtime), 1);

    runtime
        .submit_action(Action::SetMode(Mode::Enabled))
        .unwrap_or_else(|_| unreachable!("mode restoration is accepted"));
    settle(&mut runtime);
    assert_eq!(duplicate_count(&mut runtime), 0);
}

#[test]
fn unbound_command_is_explicit_and_inert() {
    let mut runtime = AppRuntime::<App>::mount(state(Mode::Unbound));
    settle(&mut runtime);
    submit_trigger(&mut runtime);
    pump_one(&mut runtime);
    pump_one(&mut runtime);
    pump_one(&mut runtime);
    assert_eq!(runtime.state().updates, []);
    assert!(has_outcome(
        &runtime,
        TraceApplicationCommandOutcome::Unbound
    ));
}

#[test]
fn downstream_scope_uses_the_same_public_disposition_and_action_path() {
    let mut runtime = AppRuntime::<App>::mount(state(Mode::CustomScope));
    settle(&mut runtime);
    submit_trigger(&mut runtime);
    pump_one(&mut runtime);
    pump_one(&mut runtime);
    pump_one(&mut runtime);
    assert_eq!(runtime.state().updates, ["inner"]);
    assert!(has_outcome(
        &runtime,
        TraceApplicationCommandOutcome::Resolved
    ));
}

#[test]
fn replacement_before_later_command_processing_rejects_exact_stale_target() {
    let mut runtime = AppRuntime::<App>::mount(state(Mode::Enabled));
    settle(&mut runtime);
    submit_trigger(&mut runtime);
    runtime
        .submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!("replacement action is accepted"));

    pump_one(&mut runtime);
    pump_one(&mut runtime);
    pump_one(&mut runtime);
    pump_one(&mut runtime);

    assert_eq!(runtime.state().updates, []);
    assert!(!has_outcome(
        &runtime,
        TraceApplicationCommandOutcome::Resolved
    ));
    let accepted = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationCommandSubmissionAccepted { .. }
            )
        })
        .unwrap_or_else(|| unreachable!("application command acceptance is traced"));
    let rejected = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::CommandProcessingRejected {
                    outcome: TraceTargetRejection::Stale
                }
            )
        })
        .unwrap_or_else(|| unreachable!("stale application command target is rejected"));
    assert_eq!(rejected.causal_parent(), Some(accepted.sequence()));
}

#[test]
fn routed_output_admission_rejects_before_emitter_callback_or_partial_command_commit() {
    let limits = RuntimeLimits::default().with_transaction_outputs(0);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state(Mode::Enabled),
        RuntimeConfig::default().with_limits(limits),
    );
    settle(&mut runtime);
    submit_trigger(&mut runtime);

    pump_one(&mut runtime);
    assert_eq!(runtime.state().emitter_calls.get(), 0);
    assert_eq!(runtime.state().updates, []);
    assert!(runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::RoutedEventAdmissionRejected {
                capacity: TraceRoutedAdmissionRejection::TransactionOutputs
            }
        )
    }));
    assert!(!runtime.trace().records().any(|record| {
        matches!(
            record.kind(),
            TraceRecordKind::DelegatedApplicationCommandCollected { .. }
                | TraceRecordKind::ApplicationCommandResolution { .. }
        )
    }));
}

#[test]
fn application_command_waiting_queue_saturation_rejects_before_emitter_callback() {
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(2)
        .with_transaction_outputs(2);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state(Mode::Enabled),
        RuntimeConfig::default().with_limits(limits),
    );
    settle(&mut runtime);
    submit_trigger(&mut runtime);
    runtime
        .submit_action(Action::Outer)
        .unwrap_or_else(|_| unreachable!("filler action is admitted"));

    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
    assert_eq!(runtime.state().emitter_calls.get(), 0);
    assert_eq!(runtime.state().updates, []);
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::WaitingEnvelopes
        }
    )));
    assert!(!runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::DelegatedApplicationCommandCollected { .. }
            | TraceRecordKind::ApplicationCommandResolution { .. }
    )));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn application_command_trace_exhaustion_rejects_before_emitter_callback() {
    let limits = RuntimeLimits::default()
        .with_waiting_envelopes(2)
        .with_transaction_outputs(1);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state(Mode::Enabled),
        RuntimeConfig::default().with_limits(limits),
    );
    settle(&mut runtime);
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 2);
    submit_trigger(&mut runtime);

    pump_one(&mut runtime);

    assert_eq!(runtime.state().emitter_calls.get(), 0);
    assert_eq!(runtime.state().updates, []);
    assert_eq!(
        runtime.status(),
        runenui_runtime::RuntimeStatus::Terminal(
            runenui_runtime::RuntimeTerminalReason::TraceSequenceExhausted
        )
    );
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::RoutedEventAdmissionRejected {
            capacity: TraceRoutedAdmissionRejection::TraceSequenceExhausted
        }
    )));
    assert!(!runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::DelegatedApplicationCommandCollected { .. }
            | TraceRecordKind::ApplicationCommandResolution { .. }
    )));
}
