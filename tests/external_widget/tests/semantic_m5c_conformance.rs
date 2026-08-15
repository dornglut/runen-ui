#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    CommandDerivation, CommandOrigin, Element, EventContext, EventPhase, EventSource, IntoEffects,
    NoHostProtocol, SemanticAction, SemanticActionRequest, SemanticActionTarget, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticKey, SemanticNodeContribution,
    SemanticRole, SemanticState, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, RuntimeConfig, SemanticNodeId,
    StyleTokens, SubmitSemanticActionErrorKind, SurfaceBuildContext, SurfaceId, TraceRecordKind,
    TraceReplay,
};

#[derive(Clone, Copy, Debug)]
struct ProbeConfig {
    enabled: bool,
    actionable: bool,
    primary_disabled: bool,
    primary_inert: bool,
    named_disabled: bool,
    named_inert: bool,
    explicit_focus: bool,
    invalidate_default: bool,
    prevent_default: bool,
    emit_delegated_menu: bool,
}

impl ProbeConfig {
    const fn actionable() -> Self {
        Self {
            enabled: true,
            actionable: true,
            primary_disabled: false,
            primary_inert: false,
            named_disabled: false,
            named_inert: false,
            explicit_focus: false,
            invalidate_default: false,
            prevent_default: false,
            emit_delegated_menu: false,
        }
    }

    const fn passive() -> Self {
        Self {
            actionable: false,
            ..Self::actionable()
        }
    }

    const fn disabled_actionable() -> Self {
        Self {
            enabled: false,
            actionable: true,
            ..Self::actionable()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EventObservation {
    command: SemanticCommand,
    source: EventSource,
    derivation: CommandDerivation,
    semantic_target: Option<SemanticActionTarget>,
}

#[derive(Debug)]
struct ProbeWidget {
    config: ProbeConfig,
    semantic_callbacks: Rc<Cell<usize>>,
    event_observations: Rc<RefCell<Vec<EventObservation>>>,
    activation_targets: Rc<RefCell<Vec<Option<SemanticActionTarget>>>>,
    activation_calls: Rc<Cell<usize>>,
}

impl Widget<ProbeAction> for ProbeWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ProbeAction>,
    ) -> WidgetEventOutput {
        let Some(command) = event.as_semantic_command() else {
            return WidgetEventOutput::none();
        };
        self.event_observations
            .borrow_mut()
            .push(EventObservation {
                command: command.command(),
                source: command.origin().source(),
                derivation: command.origin().derivation(),
                semantic_target: command.semantic_action_target().cloned(),
            });
        if context.phase() == EventPhase::Target {
            if self.config.invalidate_default
                && matches!(
                    command.command(),
                    SemanticCommand::Activate | SemanticCommand::RequestFocus
                )
            {
                context.invalidate(WidgetInvalidation::INTERACTION);
            }
            if self.config.prevent_default && command.command() == SemanticCommand::Activate {
                context.prevent_default();
            }
            if self.config.emit_delegated_menu
                && command.command() == SemanticCommand::OpenContextMenu
                && command.semantic_action_target().is_some()
            {
                context.emit_command(SemanticCommand::OpenMenu);
            }
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        match (self.config.enabled, self.config.actionable) {
            (true, true) => WidgetActivation::actionable(true),
            (false, true) => WidgetActivation::actionable(false),
            (true, false) => WidgetActivation::NONE,
            (false, false) => WidgetActivation::disabled(),
        }
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        context: &mut WidgetActivationContext<ProbeAction>,
    ) -> WidgetActivationOutput<ProbeAction> {
        self.activation_calls.set(
            self.activation_calls
                .get()
                .checked_add(1)
                .unwrap_or_else(|| unreachable!("test activation count does not overflow")),
        );
        self.activation_targets
            .borrow_mut()
            .push(context.semantic_action_target().cloned());
        WidgetActivationOutput::action(ProbeAction::Activated)
    }

    fn semantics(
        &self,
        (): &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        self.semantic_callbacks.set(
            self.semantic_callbacks
                .get()
                .checked_add(1)
                .unwrap_or_else(|| unreachable!("test semantic count does not overflow")),
        );
        let named = SemanticKey::from_static("named")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let primary = SemanticNodeContribution::primary(SemanticRole::Button)
            .with_name("primary")
            .with_state(
                SemanticState::ENABLED
                    .with_disabled(self.config.primary_disabled)
                    .with_inert(self.config.primary_inert),
            )
            .with_action(SemanticAction::Activate)
            .with_action(SemanticAction::RequestFocus)
            .with_action(SemanticAction::OpenMenu)
            .with_action(SemanticAction::OpenContextMenu)
            .with_child(
                SemanticNodeContribution::new(named, SemanticRole::Button)
                    .with_name("named")
                    .with_state(
                        SemanticState::ENABLED
                            .with_disabled(self.config.named_disabled)
                            .with_inert(self.config.named_inert),
                    )
                    .with_action(SemanticAction::Activate)
                    .with_action(SemanticAction::OpenMenu)
                    .with_action(SemanticAction::OpenContextMenu),
            );
        SemanticContribution::single(primary)
    }
}

#[derive(Debug)]
struct ReplacementWidget;

impl Widget<ProbeAction> for ReplacementWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}
}

#[derive(Debug)]
enum ProbeAction {
    Activated,
    Reconfigure(ProbeConfig),
    ReplaceOwner,
}

#[derive(Debug)]
struct ProbeState {
    config: ProbeConfig,
    present: bool,
    semantic_callbacks: Rc<Cell<usize>>,
    event_observations: Rc<RefCell<Vec<EventObservation>>>,
    activation_targets: Rc<RefCell<Vec<Option<SemanticActionTarget>>>>,
    activation_calls: Rc<Cell<usize>>,
    application_updates: usize,
}

struct ProbeApp;

impl UiApp for ProbeApp {
    type State = ProbeState;
    type Action = ProbeAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        if !state.present {
            return Element::new(ReplacementWidget)
                .id("replacement")
                .key("replacement");
        }
        let element = Element::new(ProbeWidget {
            config: state.config,
            semantic_callbacks: Rc::clone(&state.semantic_callbacks),
            event_observations: Rc::clone(&state.event_observations),
            activation_targets: Rc::clone(&state.activation_targets),
            activation_calls: Rc::clone(&state.activation_calls),
        })
        .id("probe")
        .key("probe");
        if state.config.explicit_focus {
            element.focusable(true)
        } else {
            element
        }
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            ProbeAction::Activated => state.application_updates += 1,
            ProbeAction::Reconfigure(config) => state.config = config,
            ProbeAction::ReplaceOwner => state.present = false,
        }
    }
}

#[derive(Clone)]
struct PublishedTargets {
    surface: SurfaceId,
    owner: MountedNodeId,
    primary: SemanticNodeId,
    named: SemanticNodeId,
    primary_actions: Vec<SemanticAction>,
    named_actions: Vec<SemanticAction>,
    primary_disabled: bool,
    named_disabled: bool,
}

fn runtime(config: ProbeConfig) -> AppRuntime<ProbeApp> {
    AppRuntime::<ProbeApp>::mount(ProbeState {
        config,
        present: true,
        semantic_callbacks: Rc::new(Cell::new(0)),
        event_observations: Rc::new(RefCell::new(Vec::new())),
        activation_targets: Rc::new(RefCell::new(Vec::new())),
        activation_calls: Rc::new(Cell::new(0)),
        application_updates: 0,
    })
}

fn runtime_with_config(config: ProbeConfig, runtime_config: RuntimeConfig) -> AppRuntime<ProbeApp> {
    AppRuntime::<ProbeApp>::mount_with_config(
        ProbeState {
            config,
            present: true,
            semantic_callbacks: Rc::new(Cell::new(0)),
            event_observations: Rc::new(RefCell::new(Vec::new())),
            activation_targets: Rc::new(RefCell::new(Vec::new())),
            activation_calls: Rc::new(Cell::new(0)),
            application_updates: 0,
        },
        runtime_config,
    )
}

fn publish(runtime: &mut AppRuntime<ProbeApp>) -> PublishedTargets {
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("semantic publication is admitted"));
    let snapshot = publication.semantic_publication().snapshot();
    let primary = snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some("primary"))
        .unwrap_or_else(|| unreachable!("primary semantic node is published"));
    let named = snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some("named"))
        .unwrap_or_else(|| unreachable!("named semantic node is published"));
    PublishedTargets {
        surface: snapshot.surface_id().clone(),
        owner: runtime.index().nodes()[0].id().clone(),
        primary: primary.id().clone(),
        named: named.id().clone(),
        primary_actions: primary.supported_actions().to_vec(),
        named_actions: named.supported_actions().to_vec(),
        primary_disabled: primary.state().disabled(),
        named_disabled: named.state().disabled(),
    }
}

fn request(
    targets: &PublishedTargets,
    target: &SemanticNodeId,
    action: SemanticAction,
) -> SemanticActionRequest {
    SemanticActionRequest::new(targets.surface.clone(), target.clone(), action)
}

fn pump_one(runtime: &mut AppRuntime<ProbeApp>) {
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        1
    );
}

fn assert_target(
    target: &SemanticActionTarget,
    published: &PublishedTargets,
    semantic: &SemanticNodeId,
    key: &SemanticKey,
    action: &SemanticAction,
) {
    assert_eq!(target.surface_id(), &published.surface);
    assert_eq!(target.target(), semantic);
    assert_eq!(target.semantic_key(), key);
    assert_eq!(target.action(), action);
}

#[test]
fn semantic_activate_enters_the_existing_fifo_route_default_and_update_path() {
    let mut runtime = runtime(ProbeConfig::actionable());
    let published = publish(&mut runtime);
    let semantic_baseline = runtime.state().semantic_callbacks.get();
    let submitted = runtime
        .submit_semantic_action(request(
            &published,
            &published.named,
            SemanticAction::Activate,
        ))
        .unwrap_or_else(|_| unreachable!("current named activation is admitted"));

    assert!(runtime.state().event_observations.borrow().is_empty());
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert_eq!(runtime.state().application_updates, 0);
    assert_eq!(runtime.state().semantic_callbacks.get(), semantic_baseline);

    pump_one(&mut runtime);
    let observations = runtime.state().event_observations.borrow();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].command, SemanticCommand::Activate);
    assert_eq!(observations[0].source, EventSource::Accessibility);
    assert_eq!(observations[0].derivation, CommandDerivation::Direct);
    let named = SemanticKey::from_static("named").unwrap_or_else(|_| unreachable!());
    assert_target(
        observations[0]
            .semantic_target
            .as_ref()
            .unwrap_or_else(|| unreachable!("semantic route carries exact metadata")),
        &published,
        &published.named,
        &named,
        &SemanticAction::Activate,
    );
    drop(observations);
    let activation_targets = runtime.state().activation_targets.borrow();
    assert_eq!(activation_targets.len(), 1);
    assert_target(
        activation_targets[0]
            .as_ref()
            .unwrap_or_else(|| unreachable!("semantic activation retains exact metadata")),
        &published,
        &published.named,
        &named,
        &SemanticAction::Activate,
    );
    drop(activation_targets);
    assert_eq!(runtime.state().application_updates, 0);
    assert_semantic_trace_lineage(&runtime, submitted.sequence());

    pump_one(&mut runtime);
    assert_eq!(runtime.state().application_updates, 1);
}

fn assert_semantic_trace_lineage(runtime: &AppRuntime<ProbeApp>, work: runenui_runtime::WorkSequence) {
    let bound = runtime
        .trace()
        .records()
        .find(|record| {
            record.work_sequence() == Some(work)
                && matches!(record.kind(), TraceRecordKind::SemanticActionBound { .. })
        })
        .unwrap_or_else(|| unreachable!("semantic binding is traced"));
    let accepted = runtime
        .trace()
        .records()
        .find(|record| {
            record.work_sequence() == Some(work)
                && matches!(record.kind(), TraceRecordKind::CommandSubmissionAccepted)
        })
        .unwrap_or_else(|| unreachable!("canonical command acceptance is traced"));
    assert_eq!(accepted.causal_parent(), Some(bound.sequence()));
    let routed = runtime
        .trace()
        .records()
        .find(|record| {
            record.work_sequence() == Some(work)
                && matches!(record.kind(), TraceRecordKind::RoutedEventStarted)
        })
        .unwrap_or_else(|| unreachable!("ordinary routed processing is traced"));
    assert_eq!(routed.causal_parent(), Some(accepted.sequence()));
    assert!(runtime.trace().records().any(|record| {
        record.work_sequence() == Some(work)
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultApplied {
                    command: SemanticCommand::Activate
                }
            )
    }));
}

#[test]
fn named_activation_is_not_gated_by_owner_actionable_but_availability_is_exact() {
    let mut passive = runtime(ProbeConfig::passive());
    let passive_targets = publish(&mut passive);
    assert!(!passive_targets
        .primary_actions
        .contains(&SemanticAction::Activate));
    assert!(passive_targets
        .named_actions
        .contains(&SemanticAction::Activate));
    let primary_error = passive
        .submit_semantic_action(request(
            &passive_targets,
            &passive_targets.primary,
            SemanticAction::Activate,
        ))
        .unwrap_err();
    assert_eq!(
        primary_error.kind(),
        SubmitSemanticActionErrorKind::UnsupportedAction
    );
    passive
        .submit_semantic_action(request(
            &passive_targets,
            &passive_targets.named,
            SemanticAction::Activate,
        ))
        .unwrap_or_else(|_| unreachable!("named activation ignores owner actionable"));
    pump_one(&mut passive);
    assert_eq!(passive.state().activation_calls.get(), 1);

    let mut disabled = runtime(ProbeConfig::disabled_actionable());
    let disabled_targets = publish(&mut disabled);
    assert!(disabled_targets.primary_disabled);
    assert!(disabled_targets
        .primary_actions
        .contains(&SemanticAction::Activate));
    let error = disabled
        .submit_semantic_action(request(
            &disabled_targets,
            &disabled_targets.primary,
            SemanticAction::Activate,
        ))
        .unwrap_err();
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::UnavailableAction);

    let mut named_config = ProbeConfig::passive();
    named_config.named_disabled = true;
    let mut named_disabled = runtime(named_config);
    let named_targets = publish(&mut named_disabled);
    assert!(named_targets.named_disabled);
    assert!(named_targets.named_actions.contains(&SemanticAction::Activate));
    let error = named_disabled
        .submit_semantic_action(request(
            &named_targets,
            &named_targets.named,
            SemanticAction::Activate,
        ))
        .unwrap_err();
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::UnavailableAction);
}

#[test]
fn request_focus_and_menu_actions_follow_existing_command_semantics() {
    let mut explicit_config = ProbeConfig::passive();
    explicit_config.explicit_focus = true;
    let mut explicit = runtime(explicit_config);
    let explicit_targets = publish(&mut explicit);
    assert!(explicit_targets
        .primary_actions
        .contains(&SemanticAction::RequestFocus));
    explicit
        .submit_semantic_action(request(
            &explicit_targets,
            &explicit_targets.primary,
            SemanticAction::RequestFocus,
        ))
        .unwrap_or_else(|_| unreachable!("explicit focus target is admitted"));
    pump_one(&mut explicit);
    assert_eq!(explicit.focus().focused_node(), Some(&explicit_targets.owner));

    let mut automatic = runtime(ProbeConfig::actionable());
    let automatic_targets = publish(&mut automatic);
    automatic
        .submit_semantic_action(request(
            &automatic_targets,
            &automatic_targets.primary,
            SemanticAction::RequestFocus,
        ))
        .unwrap_or_else(|_| unreachable!("automatic actionable focus target is admitted"));
    pump_one(&mut automatic);
    assert_eq!(automatic.focus().focused_node(), Some(&automatic_targets.owner));

    automatic.state().event_observations.borrow_mut().clear();
    for action in [SemanticAction::OpenMenu, SemanticAction::OpenContextMenu] {
        automatic
            .submit_semantic_action(request(
                &automatic_targets,
                &automatic_targets.named,
                action,
            ))
            .unwrap_or_else(|_| unreachable!("supported menu semantic action is admitted"));
        pump_one(&mut automatic);
    }
    let observations = automatic.state().event_observations.borrow();
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].command, SemanticCommand::OpenMenu);
    assert_eq!(observations[1].command, SemanticCommand::OpenContextMenu);
    assert_eq!(automatic.state().activation_calls.get(), 0);
}

#[test]
fn foreign_dirty_and_capacity_rejections_are_atomic_and_recover_the_exact_request() {
    let mut local = runtime(ProbeConfig::actionable());
    let local_targets = publish(&mut local);
    let mut foreign = runtime(ProbeConfig::actionable());
    let foreign_targets = publish(&mut foreign);

    let foreign_surface_request = SemanticActionRequest::new(
        foreign_targets.surface.clone(),
        local_targets.named.clone(),
        SemanticAction::Activate,
    );
    let error = local
        .submit_semantic_action(foreign_surface_request.clone())
        .unwrap_err();
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::ForeignSurface);
    assert_eq!(error.into_request(), foreign_surface_request);

    let foreign_target_request = SemanticActionRequest::new(
        local_targets.surface.clone(),
        foreign_targets.named.clone(),
        SemanticAction::Activate,
    );
    let error = local
        .submit_semantic_action(foreign_target_request.clone())
        .unwrap_err();
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::ForeignTarget);
    assert_eq!(error.into_request(), foreign_target_request);

    let semantic_baseline = local.state().semantic_callbacks.get();
    local
        .submit_action(ProbeAction::Reconfigure(ProbeConfig::passive()))
        .unwrap_or_else(|_| unreachable!("reconfiguration action is admitted"));
    pump_one(&mut local);
    let dirty_request = request(
        &local_targets,
        &local_targets.named,
        SemanticAction::Activate,
    );
    let error = local
        .submit_semantic_action(dirty_request.clone())
        .unwrap_err();
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::StaleAuthority);
    assert_eq!(error.into_request(), dirty_request);
    assert_eq!(local.state().semantic_callbacks.get(), semantic_baseline);
    assert!(local.state().event_observations.borrow().is_empty());

    assert_full_and_closed_rejections_are_inert();
}

fn assert_full_and_closed_rejections_are_inert() {
    let mut full = runtime_with_config(
        ProbeConfig::actionable(),
        RuntimeConfig::default().with_queue_capacity(1),
    );
    let full_targets = publish(&mut full);
    full.submit_action(ProbeAction::Activated)
        .unwrap_or_else(|_| unreachable!("single queue slot is available"));
    let full_request = request(&full_targets, &full_targets.named, SemanticAction::Activate);
    let error = full
        .submit_semantic_action(full_request.clone())
        .unwrap_err();
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::Full);
    assert_eq!(error.into_request(), full_request);
    assert_eq!(full.state().activation_calls.get(), 0);
    assert!(full.state().event_observations.borrow().is_empty());

    let mut closed = runtime(ProbeConfig::actionable());
    let closed_targets = publish(&mut closed);
    closed.shutdown();
    let closed_request = request(
        &closed_targets,
        &closed_targets.named,
        SemanticAction::Activate,
    );
    let error = closed
        .submit_semantic_action(closed_request.clone())
        .unwrap_err();
    assert_eq!(error.kind(), SubmitSemanticActionErrorKind::Closed);
    assert_eq!(error.into_request(), closed_request);
    assert_eq!(closed.state().activation_calls.get(), 0);
}

#[test]
fn accepted_then_replaced_semantic_work_rejects_without_retargeting() {
    let mut runtime = runtime(ProbeConfig::actionable());
    let published = publish(&mut runtime);
    runtime
        .submit_action(ProbeAction::ReplaceOwner)
        .unwrap_or_else(|_| unreachable!("replacement action is admitted first"));
    let accepted = runtime
        .submit_semantic_action(request(
            &published,
            &published.named,
            SemanticAction::Activate,
        ))
        .unwrap_or_else(|_| unreachable!("semantic request sees current authority at submission"));

    pump_one(&mut runtime);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert!(runtime.state().event_observations.borrow().is_empty());
    pump_one(&mut runtime);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    assert!(runtime.state().event_observations.borrow().is_empty());
    assert!(runtime.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticActionProcessingRejected { .. }
            )
    }));
    assert!(!runtime.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(record.kind(), TraceRecordKind::RoutedEventStarted)
    }));
}

#[test]
fn callback_invalidated_default_and_explicit_prevention_have_distinct_trace_outcomes() {
    let mut invalidating_config = ProbeConfig::actionable();
    invalidating_config.invalidate_default = true;
    let mut invalidating = runtime(invalidating_config);
    let invalidating_targets = publish(&mut invalidating);
    let accepted = invalidating
        .submit_semantic_action(request(
            &invalidating_targets,
            &invalidating_targets.named,
            SemanticAction::Activate,
        ))
        .unwrap_or_else(|_| unreachable!("semantic activation is admitted before callback"));
    pump_one(&mut invalidating);
    assert_eq!(invalidating.state().event_observations.borrow().len(), 1);
    assert_eq!(invalidating.state().activation_calls.get(), 0);
    assert!(invalidating.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultTargetInvalidated {
                    command: SemanticCommand::Activate,
                    ..
                }
            )
    }));
    assert!(!invalidating.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(record.kind(), TraceRecordKind::SemanticDefaultSuppressed { .. })
    }));

    let mut prevented_config = ProbeConfig::actionable();
    prevented_config.prevent_default = true;
    let mut prevented = runtime(prevented_config);
    let prevented_targets = publish(&mut prevented);
    let accepted = prevented
        .submit_semantic_action(request(
            &prevented_targets,
            &prevented_targets.named,
            SemanticAction::Activate,
        ))
        .unwrap_or_else(|_| unreachable!("semantic activation is admitted"));
    pump_one(&mut prevented);
    assert_eq!(prevented.state().event_observations.borrow().len(), 1);
    assert_eq!(prevented.state().activation_calls.get(), 0);
    assert!(prevented.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultSuppressed {
                    command: SemanticCommand::Activate
                }
            )
    }));
    assert!(!prevented.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultTargetInvalidated { .. }
            )
    }));
}

#[test]
fn non_semantic_and_delegated_commands_never_inherit_semantic_target_metadata() {
    let mut config = ProbeConfig::actionable();
    config.emit_delegated_menu = true;
    let mut runtime = runtime(config);
    let published = publish(&mut runtime);

    runtime
        .submit_command(
            published.owner.clone(),
            SemanticCommand::OpenMenu,
            CommandOrigin::accessibility(),
        )
        .unwrap_or_else(|_| unreachable!("ordinary accessibility command is admitted"));
    pump_one(&mut runtime);
    assert!(runtime.state().event_observations.borrow()[0]
        .semantic_target
        .is_none());

    runtime.state().event_observations.borrow_mut().clear();
    runtime
        .submit_semantic_action(request(
            &published,
            &published.named,
            SemanticAction::OpenContextMenu,
        ))
        .unwrap_or_else(|_| unreachable!("semantic context-menu action is admitted"));
    pump_one(&mut runtime);
    pump_one(&mut runtime);
    let observations = runtime.state().event_observations.borrow();
    assert_eq!(observations.len(), 2);
    assert!(observations[0].semantic_target.is_some());
    assert_eq!(observations[0].derivation, CommandDerivation::Direct);
    assert!(observations[1].semantic_target.is_none());
    assert_eq!(observations[1].derivation, CommandDerivation::Delegated);
    assert_eq!(observations[1].source, EventSource::Accessibility);
}

#[test]
fn semantic_trace_exports_and_replays_as_inert_canonical_observation() {
    let mut runtime = runtime(ProbeConfig::actionable());
    let published = publish(&mut runtime);
    let accepted = runtime
        .submit_semantic_action(request(
            &published,
            &published.named,
            SemanticAction::OpenMenu,
        ))
        .unwrap_or_else(|_| unreachable!("semantic menu action is admitted"));
    pump_one(&mut runtime);

    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("\"name\":\"semantic_action_bound\""));
    assert!(jsonl.contains("\"action\":\"open_menu\""));
    assert!(jsonl.contains("\"kind\":\"named\",\"value\":\"named\""));
    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|_| unreachable!("canonical semantic trace remains replay-compatible"));
    assert!(replay.records().any(|record| {
        record.kind().as_str() == "semantic_action_bound"
            && record.work_sequence().is_some_and(|work| work.get() == accepted.sequence().get())
    }));
    assert!(replay.records().any(|record| {
        record.kind().as_str() == "routed_event_committed"
            && record.work_sequence().is_some_and(|work| work.get() == accepted.sequence().get())
    }));
}
