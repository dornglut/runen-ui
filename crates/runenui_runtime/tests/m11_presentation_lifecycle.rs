#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, Element, EventContext, EventPhase, FocusReason, HitContribution,
    HitContributionContext, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LayoutDimension, LayoutStyle, LogicalKey, LogicalLength, LogicalPoint,
    LogicalRect, LogicalSize, NoHostProtocol, PhysicalKey, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, PresentationDismissReason,
    PresentationFocusPolicy, PresentationOrigin, PresentationOutsidePointerPolicy,
    PresentationRotation, PresentationScale, PresentationTransform, PresentationTranslation,
    SemanticCommand, StyleEnvironment, SurfacePresentation, SurfacePresentationAnchor,
    SurfacePresentationPlacement, SurfacePresentationSide, UiApp, UiEvent, UnitInterval, View,
    Widget, WidgetActivation, WidgetEventOutput, WidgetMeasure, WidgetMeasureInput,
    WidgetTextInput, button, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, ReconciliationDiagnostic,
    RuntimeConfig, SurfaceBuildContext, SurfaceInputContext, TraceConfig,
};

fn fixed(width: u16, height: u16) -> LayoutStyle {
    LayoutStyle::default()
        .with_width(LayoutDimension::length(LogicalLength::from(width)))
        .with_height(LayoutDimension::length(LogicalLength::from(height)))
}

fn context<'a>(
    environment: &'a StyleEnvironment,
    width: u16,
    height: u16,
) -> SurfaceBuildContext<'a> {
    SurfaceBuildContext::new(
        environment,
        LayoutConstraints::tight(LogicalSize::new(
            LogicalLength::from(width),
            LogicalLength::from(height),
        )),
    )
}

fn settle<App: UiApp>(runtime: &mut AppRuntime<App>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
}

fn node_id(publication: &runenui_runtime::SurfacePublication, authored: &str) -> MountedNodeId {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("fixture authored node is published"))
        .id()
        .clone()
}

fn node_center(publication: &runenui_runtime::SurfacePublication, authored: &str) -> LogicalPoint {
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("fixture authored node is published"));
    let bounds = node.bounds();
    LogicalPoint::new(
        bounds.x() + bounds.width() * 0.5,
        bounds.y() + bounds.height() * 0.5,
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"))
}

fn pointer(input: &SurfaceInputContext, point: LogicalPoint, phase: PointerPhase) -> PointerEvent {
    let id = PointerId::new(342).unwrap_or_else(|| unreachable!("fixture pointer id is non-zero"));
    let event = PointerEvent::new(id, PointerDeviceKind::Mouse, phase, point, input.clone());
    match phase {
        PointerPhase::Down => event
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        PointerPhase::Move => event.with_buttons(PointerButtons::new([PointerButton::Primary])),
        PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
        _ => event,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InteractionAction {
    OutsidePressed,
    Dismissed {
        name: &'static str,
        reason: PresentationDismissReason,
    },
}

#[derive(Clone, Debug)]
struct InteractionProbe {
    name: &'static str,
    outside: bool,
    prevent_cancel: bool,
    capture_on_down: bool,
    moves: Rc<RefCell<Vec<(MountedNodeId, Option<MountedNodeId>)>>>,
}

impl Widget<InteractionAction> for InteractionProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        event_context: &mut EventContext<'_, InteractionAction>,
    ) -> WidgetEventOutput {
        if event_context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }
        if let Some(command) = event.as_semantic_command() {
            match command.command() {
                SemanticCommand::PresentationDismiss(reason) => {
                    event_context.emit(InteractionAction::Dismissed {
                        name: self.name,
                        reason,
                    });
                }
                SemanticCommand::CancelOrBack if self.prevent_cancel => {
                    event_context.prevent_default();
                }
                _ => {}
            }
        }
        if let UiEvent::Pointer(pointer) = event {
            if self.capture_on_down && pointer.phase() == PointerPhase::Down {
                event_context.capture_pointer();
            }
            if self.capture_on_down && pointer.phase() == PointerPhase::Move {
                self.moves.borrow_mut().push((
                    event_context.original_target().clone(),
                    event_context.physical_target().cloned(),
                ));
            }
            if self.outside && pointer.phase() == PointerPhase::Down {
                event_context.emit(InteractionAction::OutsidePressed);
            }
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u8), LogicalLength::from(20_u8))
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("fixture local size is valid"));
        HitContribution::single_rect(rect)
    }
}

#[derive(Clone, Debug)]
struct InteractionState {
    open_a: bool,
    open_b: bool,
    policy_a: PresentationOutsidePointerPolicy,
    policy_b: PresentationOutsidePointerPolicy,
    modal_a: bool,
    modal_b: bool,
    cancel_a: bool,
    cancel_b: bool,
    prevent_cancel: bool,
    outside_presses: usize,
    dismissals: Vec<(&'static str, PresentationDismissReason)>,
    capture_moves: Rc<RefCell<Vec<(MountedNodeId, Option<MountedNodeId>)>>>,
}

struct InteractionApp;

fn presentation_probe(
    name: &'static str,
    x: f32,
    policy: PresentationOutsidePointerPolicy,
    modal: bool,
    cancel: bool,
    moves: Rc<RefCell<Vec<(MountedNodeId, Option<MountedNodeId>)>>>,
) -> Element<InteractionAction> {
    let presentation = Element::new(InteractionProbe {
        name,
        outside: false,
        prevent_cancel: false,
        capture_on_down: false,
        moves,
    })
    .id(name)
    .key(name)
    .with_layout(fixed(20, 20))
    .surface_presentation(
        SurfacePresentation::new(SurfacePresentationPlacement::new(
            SurfacePresentationSide::Center,
        ))
        .with_anchor(SurfacePresentationAnchor::SurfacePoint(
            LogicalPoint::new(x, 30.0).unwrap_or_else(|_| unreachable!("fixture anchor is finite")),
        ))
        .with_outside_pointer(policy)
        .modal(modal)
        .dismiss_on_cancel_or_back(cancel),
    );
    column(vec![presentation]).into_element()
}

impl UiApp for InteractionApp {
    type State = InteractionState;
    type Action = InteractionAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let outside = Element::new(InteractionProbe {
            name: "outside",
            outside: true,
            prevent_cancel: state.prevent_cancel,
            capture_on_down: false,
            moves: Rc::clone(&state.capture_moves),
        })
        .id("outside")
        .key("outside")
        .with_layout(fixed(20, 20));

        let mut children = vec![outside];
        if state.open_a {
            children.push(presentation_probe(
                "presentation-a",
                60.0,
                state.policy_a,
                state.modal_a,
                state.cancel_a,
                Rc::clone(&state.capture_moves),
            ));
        }
        if state.open_b {
            children.push(presentation_probe(
                "presentation-b",
                70.0,
                state.policy_b,
                state.modal_b,
                state.cancel_b,
                Rc::clone(&state.capture_moves),
            ));
        }
        column(children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            InteractionAction::OutsidePressed => state.outside_presses += 1,
            InteractionAction::Dismissed { name, reason } => {
                state.dismissals.push((name, reason));
                match name {
                    "presentation-a" => state.open_a = false,
                    "presentation-b" => state.open_b = false,
                    _ => unreachable!("fixture presentation name is bounded"),
                }
            }
        }
    }
}

fn interaction_state(policy: PresentationOutsidePointerPolicy) -> InteractionState {
    InteractionState {
        open_a: true,
        open_b: false,
        policy_a: policy,
        policy_b: PresentationOutsidePointerPolicy::Ignore,
        modal_a: false,
        modal_b: false,
        cancel_a: false,
        cancel_b: false,
        prevent_cancel: false,
        outside_presses: 0,
        dismissals: Vec::new(),
        capture_moves: Rc::new(RefCell::new(Vec::new())),
    }
}

fn outside_down(runtime: &mut AppRuntime<InteractionApp>) {
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("interaction publication is admitted"));
    let point = node_center(&publication, "outside");
    runtime
        .submit_pointer(pointer(
            publication.input_context(),
            point,
            PointerPhase::Down,
        ))
        .unwrap_or_else(|_| unreachable!("outside pointer down is accepted"));
    settle(runtime);
}

#[test]
fn outside_pointer_policies_are_exact_and_never_click_through_after_dismissal() {
    let mut ignore = AppRuntime::<InteractionApp>::mount(interaction_state(
        PresentationOutsidePointerPolicy::Ignore,
    ));
    outside_down(&mut ignore);
    assert_eq!(ignore.state().outside_presses, 1);
    assert!(ignore.state().dismissals.is_empty());
    assert!(ignore.state().open_a);
    assert!(
        ignore.focus().focused_node().is_none(),
        "Ignore + nonmodal + default Preserve must permit a Tooltip-style nonfocusable presentation without proxy focus"
    );

    let mut block = AppRuntime::<InteractionApp>::mount(interaction_state(
        PresentationOutsidePointerPolicy::Block,
    ));
    outside_down(&mut block);
    assert_eq!(block.state().outside_presses, 0);
    assert!(block.state().dismissals.is_empty());
    assert!(block.state().open_a);

    let mut dismiss = AppRuntime::<InteractionApp>::mount(interaction_state(
        PresentationOutsidePointerPolicy::DismissAndBlock,
    ));
    outside_down(&mut dismiss);
    assert_eq!(dismiss.state().outside_presses, 0);
    assert_eq!(
        dismiss.state().dismissals,
        [("presentation-a", PresentationDismissReason::OutsidePointer)]
    );
    assert!(!dismiss.state().open_a);
}

#[test]
fn modal_ignore_still_blocks_ordinary_outside_pointer_without_dismissal() {
    let mut state = interaction_state(PresentationOutsidePointerPolicy::Ignore);
    state.modal_a = true;
    let mut runtime = AppRuntime::<InteractionApp>::mount(state);
    outside_down(&mut runtime);
    assert_eq!(runtime.state().outside_presses, 0);
    assert!(runtime.state().dismissals.is_empty());
    assert!(runtime.state().open_a);
}

#[test]
fn visually_topmost_eligible_presentation_owns_outside_decision() {
    let mut state = interaction_state(PresentationOutsidePointerPolicy::Block);
    state.open_b = true;
    state.policy_b = PresentationOutsidePointerPolicy::DismissAndBlock;
    let mut runtime = AppRuntime::<InteractionApp>::mount(state);
    outside_down(&mut runtime);
    assert_eq!(runtime.state().outside_presses, 0);
    assert_eq!(
        runtime.state().dismissals,
        [("presentation-b", PresentationDismissReason::OutsidePointer)]
    );
    assert!(runtime.state().open_a);
    assert!(!runtime.state().open_b);
}

#[test]
fn cancel_or_back_claim_is_suppressed_by_prior_routed_prevent_default() {
    let environment = StyleEnvironment::default();

    let mut unprevented_state = interaction_state(PresentationOutsidePointerPolicy::Ignore);
    unprevented_state.cancel_a = true;
    let mut unprevented = AppRuntime::<InteractionApp>::mount(unprevented_state);
    let publication = unprevented
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("cancel publication is admitted"));
    let outside = node_id(&publication, "outside");
    unprevented
        .submit_command(
            outside,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("cancel command is accepted"));
    settle(&mut unprevented);
    assert_eq!(
        unprevented.state().dismissals,
        [("presentation-a", PresentationDismissReason::CancelOrBack)]
    );

    let mut topmost_state = interaction_state(PresentationOutsidePointerPolicy::Ignore);
    topmost_state.open_b = true;
    topmost_state.cancel_a = true;
    topmost_state.cancel_b = true;
    let mut topmost = AppRuntime::<InteractionApp>::mount(topmost_state);
    let publication = topmost
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("topmost cancel publication is admitted"));
    let outside = node_id(&publication, "outside");
    topmost
        .submit_command(
            outside,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("topmost cancel command is accepted"));
    settle(&mut topmost);
    assert_eq!(
        topmost.state().dismissals,
        [("presentation-b", PresentationDismissReason::CancelOrBack)],
        "CancelOrBack must be claimed by the visually topmost eligible presentation"
    );
    assert!(topmost.state().open_a);
    assert!(!topmost.state().open_b);

    let mut prevented_state = interaction_state(PresentationOutsidePointerPolicy::Ignore);
    prevented_state.cancel_a = true;
    prevented_state.prevent_cancel = true;
    let mut prevented = AppRuntime::<InteractionApp>::mount(prevented_state);
    let publication = prevented
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("prevented cancel publication is admitted"));
    let outside = node_id(&publication, "outside");
    prevented
        .submit_command(
            outside,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("prevented cancel command is accepted"));
    settle(&mut prevented);
    assert!(prevented.state().dismissals.is_empty());
    assert!(prevented.state().open_a);

    let fallback_state = interaction_state(PresentationOutsidePointerPolicy::Ignore);
    let mut fallback = AppRuntime::<InteractionApp>::mount(fallback_state);
    let publication = fallback
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("fallback cancel publication is admitted"));
    let outside = node_id(&publication, "outside");
    fallback
        .submit_command(
            outside,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("unclaimed cancel command is accepted"));
    settle(&mut fallback);
    assert!(fallback.state().dismissals.is_empty());
    assert!(
        fallback.state().open_a,
        "when no presentation claims CancelOrBack, presentation lifecycle must leave the existing default path untouched"
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusAction {
    Open,
    Close,
}

#[derive(Clone, Debug)]
struct FocusModel {
    open: bool,
    policy: PresentationFocusPolicy,
    duplicate_preferred: bool,
    preferred: bool,
}

struct FocusLifecycleApp;

impl UiApp for FocusLifecycleApp {
    type State = FocusModel;
    type Action = FocusAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let trigger = button("trigger")
            .on_activate(|| FocusAction::Open)
            .id("trigger")
            .key("trigger")
            .with_layout(fixed(20, 20))
            .into_element();
        let outside = button("outside focus")
            .on_activate(|| FocusAction::Close)
            .id("focus-outside")
            .key("focus-outside")
            .with_layout(fixed(20, 20))
            .into_element();
        let mut children = vec![trigger, outside];
        if state.open {
            let first = button("first")
                .on_activate(|| FocusAction::Close)
                .id("focus-first")
                .key("focus-first")
                .with_layout(fixed(20, 20))
                .into_element()
                .presentation_focus_preferred(state.duplicate_preferred);
            let preferred = button("preferred")
                .on_activate(|| FocusAction::Close)
                .id("focus-preferred")
                .key("focus-preferred")
                .with_layout(fixed(20, 20))
                .into_element()
                .presentation_focus_preferred(state.preferred);
            let presentation = column(vec![first, preferred])
                .id("focus-presentation")
                .key("focus-presentation")
                .surface_presentation(
                    SurfacePresentation::new(SurfacePresentationPlacement::new(
                        SurfacePresentationSide::Center,
                    ))
                    .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                        LogicalPoint::new(70.0, 30.0)
                            .unwrap_or_else(|_| unreachable!("focus anchor is finite")),
                    ))
                    .with_focus_policy(state.policy),
                )
                .into_element();
            children.push(
                column(vec![presentation])
                    .id("focus-presentation-owner")
                    .key("focus-presentation-owner")
                    .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            FocusAction::Open => state.open = true,
            FocusAction::Close => state.open = false,
        }
    }
}

fn focus_trigger(runtime: &mut AppRuntime<FocusLifecycleApp>, environment: &StyleEnvironment) {
    let publication = runtime
        .publish_surface(&context(environment, 100, 80))
        .unwrap_or_else(|_| unreachable!("focus baseline publication is admitted"));
    let trigger = node_id(&publication, "trigger");
    runtime
        .submit_command(
            trigger,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("trigger focus request is accepted"));
    settle(runtime);
}

fn open_and_publish(
    runtime: &mut AppRuntime<FocusLifecycleApp>,
    environment: &StyleEnvironment,
) -> runenui_runtime::SurfacePublication {
    runtime
        .submit_action(FocusAction::Open)
        .unwrap_or_else(|_| unreachable!("open action is accepted"));
    settle(runtime);
    let publication = runtime
        .publish_surface(&context(environment, 100, 80))
        .unwrap_or_else(|_| unreachable!("open presentation publishes"));
    settle(runtime);
    publication
}

#[test]
fn preserve_focus_never_moves_real_focus_when_presentation_activates() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<FocusLifecycleApp>::mount(FocusModel {
        open: false,
        policy: PresentationFocusPolicy::Preserve,
        duplicate_preferred: false,
        preferred: true,
    });
    focus_trigger(&mut runtime, &environment);
    let trigger = runtime
        .focus()
        .focused_node()
        .cloned()
        .unwrap_or_else(|| unreachable!("trigger is focused"));
    let _ = open_and_publish(&mut runtime, &environment);
    assert_eq!(runtime.focus().focused_node(), Some(&trigger));
}

#[test]
fn enter_and_restore_uses_preferred_focus_and_exact_restoration() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<FocusLifecycleApp>::mount(FocusModel {
        open: false,
        policy: PresentationFocusPolicy::EnterAndRestore,
        duplicate_preferred: false,
        preferred: true,
    });
    focus_trigger(&mut runtime, &environment);
    let trigger = runtime
        .focus()
        .focused_node()
        .cloned()
        .unwrap_or_else(|| unreachable!("trigger is focused"));
    let publication = open_and_publish(&mut runtime, &environment);
    let preferred = node_id(&publication, "focus-preferred");
    assert_eq!(runtime.focus().focused_node(), Some(&preferred));

    runtime
        .submit_action(FocusAction::Close)
        .unwrap_or_else(|_| unreachable!("close action is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&trigger));
    assert_eq!(
        runtime.focus().reason(),
        Some(FocusReason::PresentationRestoration)
    );
}

#[test]
fn enter_and_restore_falls_back_to_first_eligible_when_no_preferred_target_is_authored() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<FocusLifecycleApp>::mount(FocusModel {
        open: false,
        policy: PresentationFocusPolicy::EnterAndRestore,
        duplicate_preferred: false,
        preferred: false,
    });
    focus_trigger(&mut runtime, &environment);
    let publication = open_and_publish(&mut runtime, &environment);
    let first = node_id(&publication, "focus-first");
    assert_eq!(
        runtime.focus().focused_node(),
        Some(&first),
        "without one preferred marker, entry must choose the deterministic first eligible descendant"
    );
}

#[test]
fn duplicate_preferred_focus_fails_closed_and_diagnoses() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<FocusLifecycleApp>::mount(FocusModel {
        open: false,
        policy: PresentationFocusPolicy::EnterAndRestore,
        duplicate_preferred: true,
        preferred: true,
    });
    focus_trigger(&mut runtime, &environment);
    let trigger = runtime
        .focus()
        .focused_node()
        .cloned()
        .unwrap_or_else(|| unreachable!("trigger is focused"));
    let _ = open_and_publish(&mut runtime, &environment);

    assert_eq!(runtime.focus().focused_node(), Some(&trigger));
    assert!(
        runtime
            .reconciliation_report()
            .diagnostics()
            .iter()
            .any(|diagnostic| {
                matches!(
                    diagnostic,
                    ReconciliationDiagnostic::MultiplePreferredPresentationFocusTargets { .. }
                )
            })
    );
}

#[test]
fn intentional_focus_move_outside_is_not_stolen_back_when_presentation_closes() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<FocusLifecycleApp>::mount(FocusModel {
        open: false,
        policy: PresentationFocusPolicy::EnterAndRestore,
        duplicate_preferred: false,
        preferred: true,
    });
    focus_trigger(&mut runtime, &environment);
    let publication = open_and_publish(&mut runtime, &environment);
    let outside = node_id(&publication, "focus-outside");
    runtime
        .submit_command(
            outside.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("outside focus request is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&outside));

    runtime
        .submit_action(FocusAction::Close)
        .unwrap_or_else(|_| unreachable!("close action is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.focus().focused_node(), Some(&outside));
}

fn singular_presentation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::ZERO,
        PresentationScale::new(0.0, 1.0)
            .unwrap_or_else(|_| unreachable!("zero x scale is representable")),
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

#[derive(Clone, Debug)]
struct AnchorState {
    requests: usize,
}

#[derive(Clone, Copy, Debug)]
enum AnchorAction {
    Requested,
}

#[derive(Clone, Debug)]
struct AnchorProbe;

impl Widget<AnchorAction> for AnchorProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        event_context: &mut EventContext<'_, AnchorAction>,
    ) -> WidgetEventOutput {
        if event_context.phase() == EventPhase::Target
            && event.as_semantic_command().is_some_and(|command| {
                command.command()
                    == SemanticCommand::PresentationDismiss(
                        PresentationDismissReason::AnchorUnavailable,
                    )
            })
        {
            event_context.emit(AnchorAction::Requested);
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u8), LogicalLength::from(20_u8))
    }
}

struct AnchorUnavailableApp;

impl UiApp for AnchorUnavailableApp {
    type State = AnchorState;
    type Action = AnchorAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        let popup = Element::new(AnchorProbe)
            .id("anchor-popup")
            .key("anchor-popup")
            .surface_presentation(SurfacePresentation::new(SurfacePresentationPlacement::new(
                SurfacePresentationSide::Bottom,
            )));
        column(vec![popup])
            .id("singular-owner")
            .key("singular-owner")
            .with_layout(fixed(40, 20))
            .presentation(singular_presentation())
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        state.requests += 1;
    }
}

#[test]
fn anchor_unavailable_dismiss_request_is_emitted_once_per_exact_lifetime() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<AnchorUnavailableApp>::mount(AnchorState { requests: 0 });
    for _ in 0..3 {
        let _ = runtime
            .publish_surface(&context(&environment, 80, 60))
            .unwrap_or_else(|_| unreachable!("unavailable anchor surface remains publishable"));
        settle(&mut runtime);
    }
    assert_eq!(runtime.state().requests, 1);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaptureAction {
    Open,
}

#[derive(Clone, Debug)]
struct CaptureState {
    open: bool,
    moves: Rc<RefCell<Vec<(MountedNodeId, Option<MountedNodeId>)>>>,
}

struct CaptureModalApp;

impl UiApp for CaptureModalApp {
    type State = CaptureState;
    type Action = CaptureAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let capture = Element::new(InteractionProbe {
            name: "capture",
            outside: false,
            prevent_cancel: false,
            capture_on_down: true,
            moves: Rc::clone(&state.moves),
        })
        .id("capture")
        .key("capture")
        .with_layout(fixed(20, 20))
        .map_action(|action| match action {
            InteractionAction::OutsidePressed | InteractionAction::Dismissed { .. } => {
                CaptureAction::Open
            }
        });
        let mut children = vec![capture.into_element()];
        if state.open {
            let modal = Element::new(InteractionProbe {
                name: "modal",
                outside: false,
                prevent_cancel: false,
                capture_on_down: false,
                moves: Rc::clone(&state.moves),
            })
            .id("modal")
            .key("modal")
            .with_layout(fixed(20, 20))
            .surface_presentation(
                SurfacePresentation::new(SurfacePresentationPlacement::new(
                    SurfacePresentationSide::Center,
                ))
                .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                    LogicalPoint::new(70.0, 20.0)
                        .unwrap_or_else(|_| unreachable!("modal anchor is finite")),
                ))
                .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
                .modal(true),
            )
            .map_action(|action| match action {
                InteractionAction::OutsidePressed | InteractionAction::Dismissed { .. } => {
                    CaptureAction::Open
                }
            })
            .into_element();
            children.push(
                column(vec![modal])
                    .id("modal-owner")
                    .key("modal-owner")
                    .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        state.open = true;
    }
}

#[test]
fn active_pointer_capture_remains_authoritative_after_modal_presentation_opens() {
    let environment = StyleEnvironment::default();
    let moves = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<CaptureModalApp>::mount(CaptureState {
        open: false,
        moves: Rc::clone(&moves),
    });
    let first = runtime
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("capture baseline publishes"));
    let capture = node_id(&first, "capture");
    let capture_point = node_center(&first, "capture");
    let input = first.input_context().clone();
    runtime
        .submit_pointer(pointer(&input, capture_point, PointerPhase::Down))
        .unwrap_or_else(|_| unreachable!("capture down is accepted"));
    settle(&mut runtime);

    runtime
        .submit_action(CaptureAction::Open)
        .unwrap_or_else(|_| unreachable!("modal open is accepted"));
    settle(&mut runtime);
    let second = runtime
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("modal publication succeeds"));
    let modal = node_id(&second, "modal");
    let modal_point =
        LogicalPoint::new(70.0, 20.0).unwrap_or_else(|_| unreachable!("modal point is finite"));
    runtime
        .submit_pointer(pointer(
            second.input_context(),
            modal_point,
            PointerPhase::Move,
        ))
        .unwrap_or_else(|_| unreachable!("captured move is accepted"));
    settle(&mut runtime);

    assert_eq!(
        moves.borrow().as_slice(),
        [(capture, Some(modal))],
        "new modal presentation may be the physical target but must not steal the active capture"
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CompositionPresentationAction {
    Dismissed,
}

#[derive(Clone, Debug)]
struct CompositionPresentationProbe {
    text_input: bool,
    dismiss_target: bool,
}

impl Widget<CompositionPresentationAction> for CompositionPresentationProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        event_context: &mut EventContext<'_, CompositionPresentationAction>,
    ) -> WidgetEventOutput {
        if self.dismiss_target
            && event_context.phase() == EventPhase::Target
            && event.as_semantic_command().is_some_and(|command| {
                matches!(
                    command.command(),
                    SemanticCommand::PresentationDismiss(PresentationDismissReason::CancelOrBack)
                )
            })
        {
            event_context.emit(CompositionPresentationAction::Dismissed);
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        if self.text_input {
            WidgetActivation::actionable(true)
        } else {
            WidgetActivation::NONE
        }
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        if self.text_input {
            WidgetTextInput::new(true, true)
        } else {
            WidgetTextInput::NONE
        }
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u8), LogicalLength::from(20_u8))
    }
}

#[derive(Clone, Debug)]
struct CompositionPresentationState {
    open: bool,
    dismissals: usize,
}

struct CompositionPresentationApp;

impl UiApp for CompositionPresentationApp {
    type State = CompositionPresentationState;
    type Action = CompositionPresentationAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let editor = Element::new(CompositionPresentationProbe {
            text_input: true,
            dismiss_target: false,
        })
        .id("composition-editor")
        .key("composition-editor")
        .focusable(true)
        .with_layout(fixed(20, 20));

        let mut children = vec![editor];
        if state.open {
            children.push(
                Element::new(CompositionPresentationProbe {
                    text_input: false,
                    dismiss_target: true,
                })
                .id("composition-popup")
                .key("composition-popup")
                .with_layout(fixed(20, 20))
                .surface_presentation(
                    SurfacePresentation::new(SurfacePresentationPlacement::new(
                        SurfacePresentationSide::Center,
                    ))
                    .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                        LogicalPoint::new(70.0, 20.0)
                            .unwrap_or_else(|_| unreachable!("fixture anchor is finite")),
                    ))
                    .dismiss_on_cancel_or_back(true),
                ),
            );
        }
        column(children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            CompositionPresentationAction::Dismissed => {
                state.dismissals += 1;
                state.open = false;
            }
        }
    }
}

#[test]
fn composition_active_escape_remains_text_owned_and_does_not_dismiss_presentation() {
    let environment = StyleEnvironment::default();
    let mut runtime =
        AppRuntime::<CompositionPresentationApp>::mount(CompositionPresentationState {
            open: true,
            dismissals: 0,
        });
    let publication = runtime
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("composition presentation publishes"));
    let editor = node_id(&publication, "composition-editor");
    runtime
        .submit_command(
            editor,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("editor focus request is accepted"));
    settle(&mut runtime);

    runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("focused text owner accepts composition start"));
    settle(&mut runtime);

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("Escape")),
            LogicalKey::Escape,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Active,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("composition-active Escape is accepted"));
    settle(&mut runtime);

    assert!(runtime.state().open);
    assert_eq!(runtime.state().dismissals, 0);
}

#[derive(Clone, Debug)]
struct NestedInteractionState {
    inner_open: bool,
    interaction_presses: usize,
    dismissals: Vec<(&'static str, PresentationDismissReason)>,
    moves: Rc<RefCell<Vec<(MountedNodeId, Option<MountedNodeId>)>>>,
}

struct NestedInteractionApp;

impl UiApp for NestedInteractionApp {
    type State = NestedInteractionState;
    type Action = InteractionAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let outside = Element::new(InteractionProbe {
            name: "nested-outside",
            outside: true,
            prevent_cancel: false,
            capture_on_down: false,
            moves: Rc::clone(&state.moves),
        })
        .id("nested-outside")
        .key("nested-outside")
        .with_layout(fixed(20, 20));

        let owner_content = Element::new(InteractionProbe {
            name: "nested-owner-content",
            outside: true,
            prevent_cancel: false,
            capture_on_down: false,
            moves: Rc::clone(&state.moves),
        })
        .id("nested-owner-content")
        .key("nested-owner-content")
        .with_layout(fixed(20, 20));

        let mut outer_children = vec![owner_content];
        if state.inner_open {
            outer_children.push(presentation_probe(
                "nested-inner",
                75.0,
                PresentationOutsidePointerPolicy::DismissAndBlock,
                false,
                false,
                Rc::clone(&state.moves),
            ));
        }
        let outer = column(outer_children)
            .id("nested-outer")
            .key("nested-outer")
            .with_layout(fixed(40, 30))
            .surface_presentation(
                SurfacePresentation::new(SurfacePresentationPlacement::new(
                    SurfacePresentationSide::Center,
                ))
                .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                    LogicalPoint::new(60.0, 35.0)
                        .unwrap_or_else(|_| unreachable!("nested outer anchor is finite")),
                ))
                .with_outside_pointer(PresentationOutsidePointerPolicy::Block),
            )
            .into_element();

        let outer_owner = column(vec![outer])
            .id("nested-outer-owner")
            .key("nested-outer-owner")
            .into_element();
        column(vec![outside, outer_owner])
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            InteractionAction::OutsidePressed => state.interaction_presses += 1,
            InteractionAction::Dismissed { name, reason } => {
                state.dismissals.push((name, reason));
                if name == "nested-inner" {
                    state.inner_open = false;
                }
            }
        }
    }
}

#[test]
fn nested_presentation_owner_chain_is_inside_and_unrelated_content_dismisses_only_topmost() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<NestedInteractionApp>::mount(NestedInteractionState {
        inner_open: true,
        interaction_presses: 0,
        dismissals: Vec::new(),
        moves: Rc::new(RefCell::new(Vec::new())),
    });
    let publication = runtime
        .publish_surface(&context(&environment, 110, 80))
        .unwrap_or_else(|_| unreachable!("nested interaction publication is admitted"));
    settle(&mut runtime);

    let outer = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "nested-outer")
        })
        .unwrap_or_else(|| unreachable!("nested outer presentation is published"));
    let placed = outer
        .surface_presentation()
        .unwrap_or_else(|| unreachable!("nested outer placement resolves"))
        .placed_bounds();
    let owner_point = LogicalPoint::new(placed.x() + 10.0, placed.y() + 10.0)
        .unwrap_or_else(|_| unreachable!("nested owner point is finite"));
    runtime
        .submit_pointer(pointer(
            publication.input_context(),
            owner_point,
            PointerPhase::Down,
        ))
        .unwrap_or_else(|_| unreachable!("owner-family pointer down is accepted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().interaction_presses, 1);
    assert!(runtime.state().dismissals.is_empty());
    assert!(runtime.state().inner_open);

    let publication = runtime
        .publish_surface(&context(&environment, 110, 80))
        .unwrap_or_else(|_| unreachable!("nested interaction republishes"));
    let outside_point = node_center(&publication, "nested-outside");
    runtime
        .submit_pointer(pointer(
            publication.input_context(),
            outside_point,
            PointerPhase::Down,
        ))
        .unwrap_or_else(|_| unreachable!("unrelated outside pointer down is accepted"));
    settle(&mut runtime);

    assert_eq!(
        runtime.state().interaction_presses,
        1,
        "unrelated outside input must be blocked rather than click through"
    );
    assert_eq!(
        runtime.state().dismissals,
        [("nested-inner", PresentationDismissReason::OutsidePointer,)],
        "the visually topmost nested presentation owns the outside decision"
    );
    assert!(!runtime.state().inner_open);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ClickThroughAction {
    Dismiss,
    ActivateUnderlying,
}

#[derive(Clone, Debug)]
struct ClickThroughState {
    open: bool,
    activations: usize,
}

#[derive(Clone, Debug)]
struct ClickThroughDismissProbe;

impl Widget<ClickThroughAction> for ClickThroughDismissProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, ClickThroughAction>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && event.as_semantic_command().is_some_and(|command| {
                command.command()
                    == SemanticCommand::PresentationDismiss(
                        PresentationDismissReason::OutsidePointer,
                    )
            })
        {
            context.emit(ClickThroughAction::Dismiss);
        }
        WidgetEventOutput::none()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(20_u8), LogicalLength::from(20_u8))
    }
}

struct ClickThroughApp;

impl UiApp for ClickThroughApp {
    type State = ClickThroughState;
    type Action = ClickThroughAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let underlying = button("underlying")
            .on_activate(|| ClickThroughAction::ActivateUnderlying)
            .id("click-through-underlying")
            .key("click-through-underlying")
            .with_layout(fixed(20, 20))
            .into_element();
        let mut children = vec![underlying];
        if state.open {
            let presentation = Element::new(ClickThroughDismissProbe)
                .id("click-through-presentation")
                .key("click-through-presentation")
                .with_layout(fixed(20, 20))
                .surface_presentation(
                    SurfacePresentation::new(SurfacePresentationPlacement::new(
                        SurfacePresentationSide::Center,
                    ))
                    .with_anchor(SurfacePresentationAnchor::SurfacePoint(
                        LogicalPoint::new(70.0, 30.0)
                            .unwrap_or_else(|_| unreachable!("fixture anchor is finite")),
                    ))
                    .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock),
                );
            children.push(
                column(vec![presentation])
                    .id("click-through-presentation-owner")
                    .key("click-through-presentation-owner")
                    .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            ClickThroughAction::Dismiss => state.open = false,
            ClickThroughAction::ActivateUnderlying => state.activations += 1,
        }
    }
}

#[test]
fn dismissed_outside_pointer_stream_cannot_activate_underlying_button_on_matching_up() {
    let environment = StyleEnvironment::default();
    let mut runtime = AppRuntime::<ClickThroughApp>::mount(ClickThroughState {
        open: true,
        activations: 0,
    });
    let first = runtime
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("click-through baseline publishes"));
    let point = node_center(&first, "click-through-underlying");
    runtime
        .submit_pointer(pointer(first.input_context(), point, PointerPhase::Down))
        .unwrap_or_else(|_| unreachable!("blocked outside Down is accepted"));
    settle(&mut runtime);
    assert!(!runtime.state().open);
    assert_eq!(runtime.state().activations, 0);

    let second = runtime
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("post-dismiss surface publishes"));
    runtime
        .submit_pointer(pointer(second.input_context(), point, PointerPhase::Up))
        .unwrap_or_else(|_| unreachable!("matching pointer Up is accepted"));
    settle(&mut runtime);
    assert_eq!(
        runtime.state().activations,
        0,
        "a blocked outside Down must not seed standard Button pressed ownership that could activate on the matching Up"
    );
}

#[test]
fn presentation_lifecycle_trace_export_uses_bounded_stable_tokens() {
    let trace_config = || RuntimeConfig::default().with_trace_config(TraceConfig::new(512));

    let mut outside = AppRuntime::<InteractionApp>::mount_with_config(
        interaction_state(PresentationOutsidePointerPolicy::DismissAndBlock),
        trace_config(),
    );
    outside_down(&mut outside);
    let outside_json = outside.trace().export_jsonl();
    assert!(outside_json.contains("presentation_outside_decision"));
    assert!(outside_json.contains("dismiss_requested"));
    assert!(outside_json.contains("presentation_dismiss"));
    assert!(outside_json.contains("outside_pointer"));

    let mut cancel_state = interaction_state(PresentationOutsidePointerPolicy::Ignore);
    cancel_state.cancel_a = true;
    let mut cancel = AppRuntime::<InteractionApp>::mount_with_config(cancel_state, trace_config());
    let environment = StyleEnvironment::default();
    let publication = cancel
        .publish_surface(&context(&environment, 100, 60))
        .unwrap_or_else(|_| unreachable!("trace cancel publication is admitted"));
    let outside_target = node_id(&publication, "outside");
    cancel
        .submit_command(
            outside_target,
            SemanticCommand::CancelOrBack,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("trace cancel command is accepted"));
    settle(&mut cancel);
    let cancel_json = cancel.trace().export_jsonl();
    assert!(cancel_json.contains("presentation_cancel_or_back_decision"));
    assert!(cancel_json.contains("\"claimed\":true"));
    assert!(cancel_json.contains("cancel_or_back"));

    let mut focus = AppRuntime::<FocusLifecycleApp>::mount_with_config(
        FocusModel {
            open: false,
            policy: PresentationFocusPolicy::EnterAndRestore,
            duplicate_preferred: false,
            preferred: true,
        },
        trace_config(),
    );
    focus_trigger(&mut focus, &environment);
    let _ = open_and_publish(&mut focus, &environment);
    focus
        .submit_action(FocusAction::Close)
        .unwrap_or_else(|_| unreachable!("trace focus close is accepted"));
    settle(&mut focus);
    let focus_json = focus.trace().export_jsonl();
    assert!(focus_json.contains("presentation_initial_focus_resolved"));
    assert!(focus_json.contains("\"outcome\":\"preferred\""));
    assert!(focus_json.contains("presentation_restoration_resolved"));
    assert!(focus_json.contains("\"outcome\":\"exact\""));
    assert!(focus_json.contains("presentation_restoration"));

    let mut anchor = AppRuntime::<AnchorUnavailableApp>::mount_with_config(
        AnchorState { requests: 0 },
        trace_config(),
    );
    let _ = anchor
        .publish_surface(&context(&environment, 80, 60))
        .unwrap_or_else(|_| unreachable!("trace unavailable-anchor surface publishes"));
    settle(&mut anchor);
    let anchor_json = anchor.trace().export_jsonl();
    assert!(anchor_json.contains("presentation_anchor_unavailable_retired"));
    assert!(anchor_json.contains("anchor_unavailable"));
}
