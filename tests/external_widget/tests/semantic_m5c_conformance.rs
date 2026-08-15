#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    CommandDerivation, CommandOrigin, Element, EventContext, EventPhase, EventSource, IntoEffects,
    NoHostProtocol, SemanticAction, SemanticActionRequest, SemanticActionTarget, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticKey, SemanticNodeContribution,
    SemanticRole, SemanticState, StyleTokens, UiApp, UiEvent, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetInvalidation,
};
use runenui_runtime::{
    AppRuntime, CommandSubmission, LayoutConstraints, MountedNodeId, PumpBudget, RuntimeConfig,
    SemanticNodeId, SubmitSemanticActionError, SubmitSemanticActionErrorKind, SurfaceBuildContext,
    SurfaceId, TraceRecordKind, TraceReplay, WorkSequence,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnerMode {
    EnabledActionable,
    EnabledPassive,
    DisabledActionable,
    DisabledPassive,
}

impl OwnerMode {
    const fn activation(self) -> WidgetActivation {
        match self {
            Self::EnabledActionable => WidgetActivation::actionable(true),
            Self::EnabledPassive => WidgetActivation::NONE,
            Self::DisabledActionable => WidgetActivation::actionable(false),
            Self::DisabledPassive => WidgetActivation::disabled(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeAvailability {
    Available,
    Disabled,
    Inert,
    Hidden,
}

impl NodeAvailability {
    const fn semantic_state(self) -> SemanticState {
        match self {
            Self::Available => SemanticState::ENABLED,
            Self::Disabled => SemanticState::ENABLED.with_disabled(true),
            Self::Inert => SemanticState::ENABLED.with_inert(true),
            Self::Hidden => SemanticState::ENABLED.with_hidden(true),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusMode {
    Automatic,
    Explicit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CallbackMode {
    None,
    InvalidateSemanticDefault,
    PreventActivateDefault,
    DelegateContextMenu,
    InvalidateLayout,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProbeConfig {
    owner: OwnerMode,
    primary: NodeAvailability,
    named: NodeAvailability,
    focus: FocusMode,
    callback: CallbackMode,
}

impl ProbeConfig {
    const fn actionable() -> Self {
        Self {
            owner: OwnerMode::EnabledActionable,
            primary: NodeAvailability::Available,
            named: NodeAvailability::Available,
            focus: FocusMode::Automatic,
            callback: CallbackMode::None,
        }
    }

    const fn passive() -> Self {
        Self {
            owner: OwnerMode::EnabledPassive,
            ..Self::actionable()
        }
    }

    const fn disabled_actionable() -> Self {
        Self {
            owner: OwnerMode::DisabledActionable,
            ..Self::actionable()
        }
    }

    const fn with_owner(mut self, owner: OwnerMode) -> Self {
        self.owner = owner;
        self
    }

    const fn with_primary(mut self, primary: NodeAvailability) -> Self {
        self.primary = primary;
        self
    }

    const fn with_named(mut self, named: NodeAvailability) -> Self {
        self.named = named;
        self
    }

    const fn with_focus(mut self, focus: FocusMode) -> Self {
        self.focus = focus;
        self
    }

    const fn with_callback(mut self, callback: CallbackMode) -> Self {
        self.callback = callback;
        self
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
        self.event_observations.borrow_mut().push(EventObservation {
            command: command.command(),
            source: command.origin().source(),
            derivation: command.origin().derivation(),
            semantic_target: command.semantic_action_target().cloned(),
        });
        apply_callback_mode(self.config.callback, command, context);
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        self.config.owner.activation()
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

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
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
            .with_state(self.config.primary.semantic_state())
            .with_action(SemanticAction::Activate)
            .with_action(SemanticAction::RequestFocus)
            .with_action(SemanticAction::OpenMenu)
            .with_action(SemanticAction::OpenContextMenu)
            .with_child(
                SemanticNodeContribution::new(named, SemanticRole::Button)
                    .with_name("named")
                    .with_state(self.config.named.semantic_state())
                    .with_action(SemanticAction::Activate)
                    .with_action(SemanticAction::OpenMenu)
                    .with_action(SemanticAction::OpenContextMenu),
            );
        SemanticContribution::single(primary)
    }
}

fn apply_callback_mode(
    mode: CallbackMode,
    command: &runenui_core::SemanticCommandEvent,
    context: &mut EventContext<'_, ProbeAction>,
) {
    if context.phase() != EventPhase::Target {
        return;
    }
    match mode {
        CallbackMode::None => {}
        CallbackMode::InvalidateSemanticDefault
            if matches!(
                command.command(),
                SemanticCommand::Activate | SemanticCommand::RequestFocus
            ) =>
        {
            context.invalidate(WidgetInvalidation::INTERACTION);
        }
        CallbackMode::PreventActivateDefault if command.command() == SemanticCommand::Activate => {
            context.prevent_default();
        }
        CallbackMode::DelegateContextMenu
            if command.command() == SemanticCommand::OpenContextMenu
                && command.semantic_action_target().is_some() =>
        {
            context.emit_command(SemanticCommand::OpenMenu);
        }
        CallbackMode::InvalidateLayout if command.command() == SemanticCommand::OpenMenu => {
            context.invalidate(WidgetInvalidation::LAYOUT);
        }
        CallbackMode::InvalidateSemanticDefault
        | CallbackMode::PreventActivateDefault
        | CallbackMode::DelegateContextMenu
        | CallbackMode::InvalidateLayout => {}
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
        match state.config.focus {
            FocusMode::Automatic => element,
            FocusMode::Explicit => element.focusable(true),
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
    named_inert: bool,
}

fn runtime(config: ProbeConfig) -> AppRuntime<ProbeApp> {
    AppRuntime::<ProbeApp>::mount(initial_state(config))
}

fn runtime_with_config(config: ProbeConfig, runtime_config: RuntimeConfig) -> AppRuntime<ProbeApp> {
    AppRuntime::<ProbeApp>::mount_with_config(initial_state(config), runtime_config)
}

fn initial_state(config: ProbeConfig) -> ProbeState {
    ProbeState {
        config,
        present: true,
        semantic_callbacks: Rc::new(Cell::new(0)),
        event_observations: Rc::new(RefCell::new(Vec::new())),
        activation_targets: Rc::new(RefCell::new(Vec::new())),
        activation_calls: Rc::new(Cell::new(0)),
        application_updates: 0,
    }
}

fn publish(runtime: &mut AppRuntime<ProbeApp>) -> PublishedTargets {
    drain_queued_work(runtime);
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
        named_inert: named.state().inert(),
    }
}

fn republish(runtime: &mut AppRuntime<ProbeApp>) {
    drain_queued_work(runtime);
    let tokens = StyleTokens::new();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("semantic republication is admitted"));
}

fn request(
    targets: &PublishedTargets,
    target: &SemanticNodeId,
    action: SemanticAction,
) -> SemanticActionRequest {
    SemanticActionRequest::new(targets.surface.clone(), target.clone(), action)
}

fn drain_queued_work(runtime: &mut AppRuntime<ProbeApp>) {
    let report = runtime.pump(PumpBudget::new(usize::MAX, 0, 0, 0));
    assert_eq!(report.remaining_queued_envelopes(), 0);
}

fn pump_one(runtime: &mut AppRuntime<ProbeApp>) {
    assert_eq!(
        runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .processed_envelopes(),
        1
    );
}

fn expect_rejection(
    result: Result<CommandSubmission, SubmitSemanticActionError>,
) -> SubmitSemanticActionError {
    let Err(error) = result else {
        unreachable!("semantic request was expected to reject")
    };
    error
}

fn assert_exact_rejection(
    runtime: &mut AppRuntime<ProbeApp>,
    request: SemanticActionRequest,
    expected: SubmitSemanticActionErrorKind,
) {
    let expected_request = request.clone();
    let error = expect_rejection(runtime.submit_semantic_action(request));
    assert_eq!(error.kind(), expected);
    assert_eq!(error.into_request(), expected_request);
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

fn assert_semantic_trace_lineage(runtime: &AppRuntime<ProbeApp>, work: WorkSequence) {
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

#[test]
fn activation_support_distinguishes_primary_and_named_owner_actionability() {
    let mut passive = runtime(ProbeConfig::passive());
    let targets = publish(&mut passive);
    assert!(!targets.primary_actions.contains(&SemanticAction::Activate));
    assert!(targets.named_actions.contains(&SemanticAction::Activate));
    assert_exact_rejection(
        &mut passive,
        request(&targets, &targets.primary, SemanticAction::Activate),
        SubmitSemanticActionErrorKind::UnsupportedAction,
    );
    passive
        .submit_semantic_action(request(&targets, &targets.named, SemanticAction::Activate))
        .unwrap_or_else(|_| unreachable!("named activation ignores owner actionable"));
    pump_one(&mut passive);
    assert_eq!(passive.state().activation_calls.get(), 1);
}

#[test]
fn activation_availability_preserves_support_but_rejects_disabled_or_inert_state() {
    let mut primary_disabled = runtime(ProbeConfig::disabled_actionable());
    let primary_targets = publish(&mut primary_disabled);
    assert!(primary_targets.primary_disabled);
    assert!(
        primary_targets
            .primary_actions
            .contains(&SemanticAction::Activate)
    );
    assert_exact_rejection(
        &mut primary_disabled,
        request(
            &primary_targets,
            &primary_targets.primary,
            SemanticAction::Activate,
        ),
        SubmitSemanticActionErrorKind::UnavailableAction,
    );

    assert_named_unavailable(
        ProbeConfig::passive().with_owner(OwnerMode::DisabledPassive),
        false,
    );
    assert_named_unavailable(
        ProbeConfig::passive().with_named(NodeAvailability::Disabled),
        false,
    );
    assert_named_unavailable(
        ProbeConfig::passive().with_named(NodeAvailability::Inert),
        true,
    );
}

fn assert_named_unavailable(config: ProbeConfig, expect_inert: bool) {
    let mut runtime = runtime(config);
    let targets = publish(&mut runtime);
    assert!(targets.named_actions.contains(&SemanticAction::Activate));
    assert!(targets.named_disabled || targets.named_inert);
    assert_eq!(targets.named_inert, expect_inert);
    assert_exact_rejection(
        &mut runtime,
        request(&targets, &targets.named, SemanticAction::Activate),
        SubmitSemanticActionErrorKind::UnavailableAction,
    );
}

#[test]
fn request_focus_uses_primary_only_and_current_m4_focus_eligibility() {
    let mut explicit = runtime(ProbeConfig::passive().with_focus(FocusMode::Explicit));
    let explicit_targets = publish(&mut explicit);
    assert!(
        explicit_targets
            .primary_actions
            .contains(&SemanticAction::RequestFocus)
    );
    explicit
        .submit_semantic_action(request(
            &explicit_targets,
            &explicit_targets.primary,
            SemanticAction::RequestFocus,
        ))
        .unwrap_or_else(|_| unreachable!("explicit focus target is admitted"));
    pump_one(&mut explicit);
    assert_eq!(
        explicit.focus().focused_node(),
        Some(&explicit_targets.owner)
    );

    let mut automatic = runtime(ProbeConfig::actionable());
    let automatic_targets = publish(&mut automatic);
    assert_exact_rejection(
        &mut automatic,
        request(
            &automatic_targets,
            &automatic_targets.named,
            SemanticAction::RequestFocus,
        ),
        SubmitSemanticActionErrorKind::UnsupportedAction,
    );
    automatic
        .submit_semantic_action(request(
            &automatic_targets,
            &automatic_targets.primary,
            SemanticAction::RequestFocus,
        ))
        .unwrap_or_else(|_| unreachable!("automatic actionable focus target is admitted"));
    pump_one(&mut automatic);
    assert_eq!(
        automatic.focus().focused_node(),
        Some(&automatic_targets.owner)
    );

    let mut passive = runtime(ProbeConfig::passive());
    let passive_targets = publish(&mut passive);
    assert!(
        !passive_targets
            .primary_actions
            .contains(&SemanticAction::RequestFocus)
    );
    assert_exact_rejection(
        &mut passive,
        request(
            &passive_targets,
            &passive_targets.primary,
            SemanticAction::RequestFocus,
        ),
        SubmitSemanticActionErrorKind::UnsupportedAction,
    );
}

#[test]
fn menu_actions_route_without_owner_actionable_or_activation_default() {
    let mut runtime = runtime(ProbeConfig::passive());
    let targets = publish(&mut runtime);
    for action in [SemanticAction::OpenMenu, SemanticAction::OpenContextMenu] {
        runtime
            .submit_semantic_action(request(&targets, &targets.named, action))
            .unwrap_or_else(|_| {
                unreachable!("menu actions do not require owner actionable readiness")
            });
        pump_one(&mut runtime);
    }
    let observations = runtime.state().event_observations.borrow();
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].command, SemanticCommand::OpenMenu);
    assert_eq!(observations[1].command, SemanticCommand::OpenContextMenu);
    assert_eq!(runtime.state().activation_calls.get(), 0);
}

#[test]
fn foreign_dirty_and_capacity_rejections_are_atomic_and_recover_exact_requests() {
    let mut local = runtime(ProbeConfig::actionable());
    let local_targets = publish(&mut local);
    let mut foreign = runtime(ProbeConfig::actionable());
    let foreign_targets = publish(&mut foreign);

    assert_exact_rejection(
        &mut local,
        SemanticActionRequest::new(
            foreign_targets.surface.clone(),
            local_targets.named.clone(),
            SemanticAction::Activate,
        ),
        SubmitSemanticActionErrorKind::ForeignSurface,
    );
    assert_exact_rejection(
        &mut local,
        SemanticActionRequest::new(
            local_targets.surface.clone(),
            foreign_targets.named,
            SemanticAction::Activate,
        ),
        SubmitSemanticActionErrorKind::ForeignTarget,
    );

    let semantic_baseline = local.state().semantic_callbacks.get();
    local
        .submit_action(ProbeAction::Reconfigure(ProbeConfig::passive()))
        .unwrap_or_else(|_| unreachable!("reconfiguration action is admitted"));
    pump_one(&mut local);
    assert_exact_rejection(
        &mut local,
        request(
            &local_targets,
            &local_targets.named,
            SemanticAction::Activate,
        ),
        SubmitSemanticActionErrorKind::StaleAuthority,
    );
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
    assert_exact_rejection(
        &mut full,
        request(&full_targets, &full_targets.named, SemanticAction::Activate),
        SubmitSemanticActionErrorKind::Full,
    );
    assert_eq!(full.state().activation_calls.get(), 0);
    assert!(full.state().event_observations.borrow().is_empty());

    let mut closed = runtime(ProbeConfig::actionable());
    let closed_targets = publish(&mut closed);
    closed.shutdown();
    assert_exact_rejection(
        &mut closed,
        request(
            &closed_targets,
            &closed_targets.named,
            SemanticAction::Activate,
        ),
        SubmitSemanticActionErrorKind::Closed,
    );
    assert_eq!(closed.state().activation_calls.get(), 0);
}

#[test]
fn layout_only_dirtiness_does_not_block_semantic_action_admission() {
    let config = ProbeConfig::passive().with_callback(CallbackMode::InvalidateLayout);
    let mut runtime = runtime(config);
    let targets = publish(&mut runtime);
    runtime
        .submit_command(
            targets.owner.clone(),
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("ordinary layout-invalidating command is admitted"));
    pump_one(&mut runtime);

    runtime
        .submit_semantic_action(request(&targets, &targets.named, SemanticAction::OpenMenu))
        .unwrap_or_else(|_| unreachable!("layout-only dirtiness does not stale action authority"));
    pump_one(&mut runtime);
    assert_eq!(runtime.state().event_observations.borrow().len(), 2);
}

#[test]
fn hidden_target_is_not_in_current_surface_and_replaced_generation_becomes_stale() {
    let mut runtime = runtime(ProbeConfig::actionable());
    let targets = publish(&mut runtime);
    runtime
        .submit_action(ProbeAction::Reconfigure(
            ProbeConfig::actionable().with_named(NodeAvailability::Hidden),
        ))
        .unwrap_or_else(|_| unreachable!("hidden-state reconfiguration is admitted"));
    pump_one(&mut runtime);
    republish(&mut runtime);
    assert_exact_rejection(
        &mut runtime,
        request(&targets, &targets.named, SemanticAction::Activate),
        SubmitSemanticActionErrorKind::TargetNotInSurface,
    );

    runtime
        .submit_action(ProbeAction::ReplaceOwner)
        .unwrap_or_else(|_| unreachable!("owner replacement is admitted"));
    pump_one(&mut runtime);
    republish(&mut runtime);
    assert_exact_rejection(
        &mut runtime,
        request(&targets, &targets.named, SemanticAction::Activate),
        SubmitSemanticActionErrorKind::StaleTarget,
    );
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
fn callback_invalidated_activate_and_prevent_default_have_distinct_trace_outcomes() {
    let invalidating = ProbeConfig::actionable().with_callback(CallbackMode::InvalidateSemanticDefault);
    assert_activate_default_suppression(invalidating, true);
    let prevented = ProbeConfig::actionable().with_callback(CallbackMode::PreventActivateDefault);
    assert_activate_default_suppression(prevented, false);
}

fn assert_activate_default_suppression(config: ProbeConfig, expect_invalidated: bool) {
    let mut runtime = runtime(config);
    let targets = publish(&mut runtime);
    let accepted = runtime
        .submit_semantic_action(request(&targets, &targets.named, SemanticAction::Activate))
        .unwrap_or_else(|_| unreachable!("semantic activation is admitted before callback"));
    pump_one(&mut runtime);
    assert_eq!(runtime.state().event_observations.borrow().len(), 1);
    assert_eq!(runtime.state().activation_calls.get(), 0);
    let invalidated = runtime.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultTargetInvalidated {
                    command: SemanticCommand::Activate,
                    ..
                }
            )
    });
    let prevented = runtime.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultSuppressed {
                    command: SemanticCommand::Activate
                }
            )
    });
    assert_eq!(invalidated, expect_invalidated);
    assert_eq!(prevented, !expect_invalidated);
}

#[test]
fn callback_invalidated_request_focus_suppresses_focus_default_without_refresh() {
    let config = ProbeConfig::passive()
        .with_focus(FocusMode::Explicit)
        .with_callback(CallbackMode::InvalidateSemanticDefault);
    let mut runtime = runtime(config);
    let targets = publish(&mut runtime);
    let accepted = runtime
        .submit_semantic_action(request(
            &targets,
            &targets.primary,
            SemanticAction::RequestFocus,
        ))
        .unwrap_or_else(|_| unreachable!("focus request is admitted before callback"));
    pump_one(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), None);
    assert!(runtime.trace().records().any(|record| {
        record.work_sequence() == Some(accepted.sequence())
            && matches!(
                record.kind(),
                TraceRecordKind::SemanticDefaultTargetInvalidated {
                    command: SemanticCommand::RequestFocus,
                    ..
                }
            )
    }));
}

#[test]
fn non_semantic_and_delegated_commands_never_inherit_semantic_target_metadata() {
    let config = ProbeConfig::actionable().with_callback(CallbackMode::DelegateContextMenu);
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
    assert!(
        runtime.state().event_observations.borrow()[0]
            .semantic_target
            .is_none()
    );

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
            && record
                .work_sequence()
                .is_some_and(|work| work.get() == accepted.sequence().get())
    }));
    assert!(replay.records().any(|record| {
        record.kind().as_str() == "routed_event_committed"
            && record
                .work_sequence()
                .is_some_and(|work| work.get() == accepted.sequence().get())
    }));
}
