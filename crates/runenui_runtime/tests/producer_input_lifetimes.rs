#![allow(refining_impl_trait)]
#![allow(clippy::assert_is_empty)] // Public immutable ownership slices are the test oracle.
#![allow(clippy::unwrap_used, clippy::panic)] // Retained public RED/GREEN qualification fixtures.

use runenui_core::*;
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeConfig, RuntimeStatus, SurfaceBuildContext,
};

struct Form;
enum Action {
    Noop,
    Edit,
}
impl UiApp for Form {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &usize) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &usize, _: &Action) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
    }
    fn interaction_producer_budget(_: &usize) -> ProducerBudget {
        ProducerBudget::new(14, 12)
            .with_reconciliation(0, 1, 0)
            .with_work_starts(WorkStartBudget::new(0, 0, 0, 0, 2))
            .with_work_residency(WorkResidencyBudget::new(0, 0, 0, 0, 0, 2))
    }
    fn root(_: &usize) -> impl View<Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(475), TextDocumentRevision::new(0));
        let position = TextPosition::new(snapshot, "", 0, TextAffinity::Downstream).unwrap();
        text_field(
            snapshot,
            "",
            TextSelection::collapsed(position),
            SemanticEditableMode::SingleLine,
            |_| Action::Edit,
        )
        .unwrap()
        .with_layout(
            LayoutStyle::default()
                .with_width(LayoutDimension::length(LogicalLength::from(100_u16)))
                .with_height(LayoutDimension::length(LogicalLength::from(40_u16))),
        )
    }
    fn update(state: &mut usize, action: Action) {
        assert!(matches!(action, Action::Noop));
        *state += 1;
    }
}
const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);

fn saturated_with_one_free_slot() -> (AppRuntime<Form>, SurfaceInputContext, usize) {
    let mut runtime =
        AppRuntime::<Form>::mount_with_config(0, RuntimeConfig::default().with_queue_capacity(16))
            .unwrap();
    runtime.pump(FULL).unwrap();
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap();
    runtime.pump(FULL).unwrap();
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    let env = StyleEnvironment::default();
    let surface = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &env,
            LogicalSize::try_new(100.0, 40.0).unwrap(),
        ))
        .unwrap();
    let point = LogicalPoint::new(5.0, 5.0).unwrap();
    assert_eq!(surface.hit_test_scene().target_at(point), Some(&owner));
    runtime.pump(FULL).unwrap();
    let mut accepted = 0;
    loop {
        match runtime.submit_action(Action::Noop) {
            Ok(_) => {
                accepted += 1;
                assert!(accepted <= 16);
            }
            Err(error) => {
                assert_eq!(error.kind(), runenui_runtime::SubmitActionErrorKind::Full);
                assert!(matches!(error.into_action(), Action::Noop));
                break;
            }
        }
    }
    assert!(accepted > 1);
    let progress = runtime.pump(PumpBudget::new(1, 0, 0, 0)).unwrap();
    assert_eq!(progress.report().processed_envelopes(), 1);
    assert_eq!(*runtime.state(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    eprintln!(
        "U475_SOURCE_CLOSURE admitted_noop_roots={accepted} freed_slots=1 declared_interaction_frontier=14"
    );
    (runtime, surface.input_context().clone(), accepted - 1)
}

fn preserve_pressure_through_predecessors<App: UiApp<State = usize, Action = Action>>(
    runtime: &mut AppRuntime<App>,
    predecessors: usize,
) {
    for _ in 0..predecessors {
        let boundary = runtime.pump(PumpBudget::new(1, 0, 0, 0)).unwrap();
        assert_eq!(boundary.report().processed_envelopes(), 1);
        runtime.submit_action(Action::Noop).unwrap();
    }
    let reached = runtime.pump(PumpBudget::new(1, 0, 0, 0)).unwrap();
    eprintln!(
        "U475_SOURCE_CLOSURE reached_input={:?} status={:?}",
        reached.ordered_records(),
        runtime.status()
    );
}

#[derive(Debug)]
struct CompositionEditor;
impl Widget<Action> for CompositionEditor {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(if phase == WidgetProducerPhase::Event {
            2
        } else {
            0
        })
    }
    fn text_input(&self, (): &()) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if matches!(event, UiEvent::Composition(_)) {
            let _ = context.try_emit(Action::Noop);
            let _ = context.try_emit(Action::Noop);
        }
        WidgetEventOutput::none()
    }
}
struct CompositionApp;
impl UiApp for CompositionApp {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(state: &usize) -> ProducerBudget {
        Form::initial_producer_budget(state)
    }
    fn action_producer_budget(state: &usize, action: &Action) -> ProducerBudget {
        Form::action_producer_budget(state, action)
    }
    fn interaction_producer_budget(state: &usize) -> ProducerBudget {
        Form::interaction_producer_budget(state)
    }
    fn root(_: &usize) -> impl View<Action> {
        Element::new(CompositionEditor).focusable(true)
    }
    fn update(state: &mut usize, action: Action) {
        Form::update(state, action);
    }
}

fn saturated_composition_editor() -> (AppRuntime<CompositionApp>, usize) {
    let mut runtime = AppRuntime::<CompositionApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_queue_capacity(16),
    )
    .unwrap();
    runtime.pump(FULL).unwrap();
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap();
    runtime.pump(FULL).unwrap();
    let mut accepted = 0;
    loop {
        match runtime.submit_action(Action::Noop) {
            Ok(_) => {
                accepted += 1;
                assert!(accepted <= 16);
            }
            Err(error) => {
                assert_eq!(error.kind(), runenui_runtime::SubmitActionErrorKind::Full);
                break;
            }
        }
    }
    runtime.pump(PumpBudget::new(1, 0, 0, 0)).unwrap();
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    eprintln!(
        "U475_COMPOSITION_SOURCE admitted_noop_roots={accepted} freed_slots=1 event_outputs=2"
    );
    (runtime, accepted - 1)
}

#[test]
fn composition_must_refuse_before_lifetime_allocation_when_only_envelope_fits() {
    let (mut runtime, predecessors) = saturated_composition_editor();
    let result = runtime.start_composition(None);
    if let Ok(start) = &result {
        preserve_pressure_through_predecessors(&mut runtime, predecessors);
        let cancellation = runtime.cancel_composition(start.generation().clone());
        eprintln!(
            "U475_SOURCE_CLOSURE active_composition_cancel={:?}",
            cancellation
                .as_ref()
                .map(|_| "accepted")
                .map_err(runenui_runtime::SubmitCompositionError::kind)
        );
    }
    assert!(
        result.is_err(),
        "UNQUALIFIED: composition admitted with only one free slot and no fourteen-credit producer reservation"
    );
}

#[test]
fn pointer_down_must_refuse_before_admission_when_only_envelope_fits() {
    let (mut runtime, context, predecessors) = saturated_with_one_free_slot();
    let event = PointerEvent::new(
        PointerId::new(475).unwrap(),
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        LogicalPoint::new(5.0, 5.0).unwrap(),
        context,
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    let result = runtime.submit_pointer(event);
    if result.is_ok() {
        preserve_pressure_through_predecessors(&mut runtime, predecessors);
    }
    assert!(
        result.is_err(),
        "UNQUALIFIED: pointer Down admitted with only one free slot and no fourteen-credit producer reservation"
    );
}

#[test]
fn accepted_composition_retains_capacity_for_required_cancellation() {
    let mut runtime = AppRuntime::<CompositionApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_queue_capacity(16),
    )
    .unwrap();
    runtime.pump(FULL).unwrap();
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap();
    runtime.pump(FULL).unwrap();
    let start = runtime.start_composition(None).unwrap();
    runtime.pump(FULL).unwrap();
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    let mut followers = 0;
    while runtime.submit_action(Action::Noop).is_ok() {
        followers += 1;
        assert!(followers <= 16);
    }
    let cancellation = runtime.cancel_composition(start.generation().clone());
    eprintln!(
        "U475_LIFETIME followers={followers} cancel={:?} status={:?}",
        cancellation
            .as_ref()
            .map(|_| "accepted")
            .map_err(runenui_runtime::SubmitCompositionError::kind),
        runtime.status()
    );
    assert!(
        cancellation.is_ok(),
        "UNQUALIFIED: valid admitted composition has no retained cancellation frontier"
    );
    runtime.pump(FULL).unwrap();
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn accepted_pointer_press_retains_capacity_for_required_release() {
    let mut runtime = AppRuntime::<ButtonApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_queue_capacity(17),
    )
    .unwrap();
    runtime.pump(FULL).unwrap();
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap();
    runtime.pump(FULL).unwrap();
    let env = StyleEnvironment::default();
    let surface = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &env,
            LogicalSize::try_new(100.0, 40.0).unwrap(),
        ))
        .unwrap();
    runtime.pump(FULL).unwrap();
    let pointer = PointerId::new(476).unwrap();
    let event = |phase| {
        PointerEvent::new(
            pointer,
            PointerDeviceKind::Mouse,
            phase,
            LogicalPoint::new(5.0, 5.0).unwrap(),
            surface.input_context().clone(),
        )
        .with_changed_button(PointerButton::Primary)
    };
    let receipt = runtime
        .submit_pointer(
            event(PointerPhase::Down).with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap();
    let committed = runtime.pump(FULL).unwrap();
    eprintln!(
        "U475_POINTER_LIFETIME down={:?} records={:?} pointers={:?}",
        receipt,
        committed.ordered_records(),
        committed.final_ownership().pointers()
    );
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(!committed.final_ownership().pointers().is_empty());
    let mut followers = 0;
    while runtime.submit_action(Action::Noop).is_ok() {
        followers += 1;
        assert!(followers <= 17);
    }
    let release = runtime.submit_pointer(event(PointerPhase::Up));
    eprintln!(
        "U475_POINTER_LIFETIME followers={followers} release={:?}",
        release
            .as_ref()
            .map(|_| "accepted")
            .map_err(runenui_runtime::SubmitPointerError::kind)
    );
    assert!(
        release.is_ok(),
        "UNQUALIFIED: committed pointer press has no retained release frontier"
    );
    let released = runtime.pump(FULL).unwrap();
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(released.final_ownership().pointers().is_empty());
}

struct ButtonApp;
impl UiApp for ButtonApp {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(state: &usize) -> ProducerBudget {
        Form::initial_producer_budget(state)
    }
    fn action_producer_budget(state: &usize, action: &Action) -> ProducerBudget {
        Form::action_producer_budget(state, action)
    }
    fn interaction_producer_budget(state: &usize) -> ProducerBudget {
        Form::interaction_producer_budget(state)
    }
    fn root(_: &usize) -> impl View<Action> {
        button("Press").on_activate(|| Action::Noop).with_layout(
            LayoutStyle::default()
                .with_width(LayoutDimension::length(LogicalLength::from(100_u16)))
                .with_height(LayoutDimension::length(LogicalLength::from(40_u16))),
        )
    }
    fn update(state: &mut usize, action: Action) {
        Form::update(state, action);
    }
}

fn button_runtime(capacity: usize) -> (AppRuntime<ButtonApp>, SurfaceInputContext) {
    let mut app = AppRuntime::<ButtonApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_queue_capacity(capacity),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    let owner = app.index().nodes()[0].id().clone();
    app.submit_command(
        owner,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    let environment = StyleEnvironment::default();
    let publication = app
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(100.0, 40.0).unwrap(),
        ))
        .unwrap();
    app.pump(FULL).unwrap();
    (app, publication.input_context().clone())
}

fn pointer(
    context: &SurfaceInputContext,
    id: u64,
    phase: PointerPhase,
    buttons: impl IntoIterator<Item = PointerButton>,
    changed: PointerButton,
) -> PointerEvent {
    PointerEvent::new(
        PointerId::new(id).unwrap(),
        PointerDeviceKind::Mouse,
        phase,
        LogicalPoint::new(5.0, 5.0).unwrap(),
        context.clone(),
    )
    .with_buttons(PointerButtons::new(buttons))
    .with_changed_button(changed)
}

fn fill<App: UiApp<State = usize, Action = Action>>(
    app: &mut AppRuntime<App>,
    capacity: usize,
) -> usize {
    let mut count = 0;
    while app.submit_action(Action::Noop).is_ok() {
        count += 1;
        assert!(count <= capacity);
    }
    count
}

#[test]
fn pending_down_and_its_release_share_the_reserved_frontier_without_overtaking() {
    let (mut app, context) = button_runtime(17);
    let down = app
        .submit_pointer(pointer(
            &context,
            701,
            PointerPhase::Down,
            [PointerButton::Primary],
            PointerButton::Primary,
        ))
        .unwrap();
    let followers = fill(&mut app, 17);
    assert_eq!(followers, 3);
    let up = app
        .submit_pointer(pointer(
            &context,
            701,
            PointerPhase::Up,
            [],
            PointerButton::Primary,
        ))
        .unwrap();
    assert!(up.sequence() > down.sequence());
    let first = app.pump(PumpBudget::new(1, 0, 0, 0)).unwrap();
    assert_eq!(first.processed_through(), Some(down.sequence()));
    assert!(!first.final_ownership().pointers().is_empty());
    let complete = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(complete.final_ownership().pointers().is_empty());
    assert_eq!(*app.state(), followers + 1);
}

#[test]
fn unsupported_queued_touch_chord_cannot_hide_the_valid_release_claim() {
    let (mut app, context) = button_runtime(32);
    let touch = |phase, buttons: &[PointerButton], changed| {
        PointerEvent::new(
            PointerId::new(709).unwrap(),
            PointerDeviceKind::Touch,
            phase,
            LogicalPoint::new(5.0, 5.0).unwrap(),
            context.clone(),
        )
        .with_buttons(PointerButtons::new(buttons.iter().copied()))
        .with_changed_button(changed)
    };
    app.submit_pointer(touch(
        PointerPhase::Down,
        &[PointerButton::Primary],
        PointerButton::Primary,
    ))
    .unwrap();
    app.pump(FULL).unwrap();
    app.submit_pointer(touch(
        PointerPhase::Down,
        &[PointerButton::Primary, PointerButton::Secondary],
        PointerButton::Secondary,
    ))
    .unwrap();
    fill(&mut app, 32);
    app.submit_pointer(touch(PointerPhase::Up, &[], PointerButton::Primary))
        .unwrap_or_else(|error| {
            panic!("rejected touch predecessor cannot consume ownership: {error:?}")
        });
    let settled = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(settled.final_ownership().pointers().is_empty());
}

#[test]
fn simultaneous_pointers_release_under_pressure_and_reused_ids_get_new_claims() {
    let (mut app, context) = button_runtime(32);
    for id in [711, 712] {
        app.submit_pointer(pointer(
            &context,
            id,
            PointerPhase::Down,
            [PointerButton::Primary],
            PointerButton::Primary,
        ))
        .unwrap();
        app.pump(FULL).unwrap();
    }
    fill(&mut app, 32);
    for id in [711, 712] {
        app.submit_pointer(pointer(
            &context,
            id,
            PointerPhase::Up,
            [],
            PointerButton::Primary,
        ))
        .unwrap();
    }
    assert!(
        app.pump(FULL)
            .unwrap()
            .final_ownership()
            .pointers()
            .is_empty()
    );
    assert_eq!(app.status(), RuntimeStatus::Running);
    for _ in 0..8 {
        app.submit_pointer(pointer(
            &context,
            711,
            PointerPhase::Down,
            [PointerButton::Primary],
            PointerButton::Primary,
        ))
        .unwrap();
        app.pump(FULL).unwrap();
        app.submit_pointer(pointer(
            &context,
            711,
            PointerPhase::Cancel,
            [],
            PointerButton::Primary,
        ))
        .unwrap();
        app.pump(FULL).unwrap();
        assert_eq!(app.status(), RuntimeStatus::Running);
    }
}

#[test]
fn queued_chords_keep_each_buttons_claim_and_malformed_release_cannot_spend_it() {
    let (mut app, context) = button_runtime(32);
    app.submit_pointer(pointer(
        &context,
        721,
        PointerPhase::Down,
        [PointerButton::Primary],
        PointerButton::Primary,
    ))
    .unwrap();
    app.submit_pointer(pointer(
        &context,
        721,
        PointerPhase::Down,
        [PointerButton::Primary, PointerButton::Secondary],
        PointerButton::Secondary,
    ))
    .unwrap();
    fill(&mut app, 32);
    // This is not the exact post-release button set. It cannot borrow a
    // lifetime claim; the saturated queue refuses the owned malformed payload.
    let malformed = pointer(&context, 721, PointerPhase::Up, [], PointerButton::Primary);
    let refusal = app.submit_pointer(malformed.clone()).unwrap_err();
    assert_eq!(refusal.into_event(), malformed);
    app.submit_pointer(pointer(
        &context,
        721,
        PointerPhase::Up,
        [PointerButton::Primary],
        PointerButton::Secondary,
    ))
    .unwrap();
    app.submit_pointer(pointer(
        &context,
        721,
        PointerPhase::Up,
        [],
        PointerButton::Primary,
    ))
    .unwrap();
    let complete = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(complete.final_ownership().pointers().is_empty());
}

#[test]
fn pending_composition_cancellation_drains_descendants_and_releases_its_entire_root() {
    let mut app = AppRuntime::<CompositionApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_queue_capacity(16),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    let owner = app.index().nodes()[0].id().clone();
    app.submit_command(
        owner,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    let start = app.start_composition(None).unwrap();
    let followers = fill(&mut app, 16);
    app.cancel_composition(start.generation().clone()).unwrap();
    assert_eq!(
        app.pump(PumpBudget::new(0, 0, 0, 0))
            .unwrap()
            .report()
            .processed_envelopes(),
        0
    );
    app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert_eq!(*app.state(), followers + 4);
    assert!(app.cancel_composition(start.generation().clone()).is_err());
    assert_eq!(
        fill(&mut app, 16),
        16,
        "retired lifetime cannot retain idle credits"
    );
}

#[test]
fn unavailable_context_release_uses_its_claim_and_preserves_integrity_only_finality() {
    let (mut app, context) = button_runtime(64);
    app.submit_pointer(pointer(
        &context,
        731,
        PointerPhase::Down,
        [PointerButton::Primary],
        PointerButton::Primary,
    ))
    .unwrap();
    app.pump(FULL).unwrap();
    let environment = StyleEnvironment::default();
    for width in [101.0, 102.0, 103.0] {
        app.publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(width, 40.0).unwrap(),
        ))
        .unwrap();
        app.pump(FULL).unwrap();
    }
    let followers = fill(&mut app, 64);
    app.submit_pointer(pointer(
        &context,
        731,
        PointerPhase::Up,
        [],
        PointerButton::Primary,
    ))
    .unwrap();
    let complete = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(complete.final_ownership().pointers().is_empty());
    assert_eq!(
        *app.state(),
        followers,
        "unavailable geometry must not activate"
    );
    assert!(
        complete
            .ordered_records()
            .iter()
            .any(|record| matches!(record,
        runenui_runtime::InputArbitrationRecord::InputSettled(settlement)
            if matches!(settlement.finality(), runenui_runtime::UiInputFinality::Committed(_))))
    );
}

#[test]
fn composition_and_pointer_lifetimes_keep_independent_cleanup_claims() {
    let mut app =
        AppRuntime::<Form>::mount_with_config(0, RuntimeConfig::default().with_queue_capacity(64))
            .unwrap();
    app.pump(FULL).unwrap();
    let owner = app.index().nodes()[0].id().clone();
    app.submit_command(
        owner,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    let environment = StyleEnvironment::default();
    let surface = app
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(100.0, 40.0).unwrap(),
        ))
        .unwrap();
    app.pump(FULL).unwrap();
    let composition = app.start_composition(None).unwrap();
    app.pump(FULL).unwrap();
    app.submit_pointer(pointer(
        surface.input_context(),
        741,
        PointerPhase::Down,
        [PointerButton::Primary],
        PointerButton::Primary,
    ))
    .unwrap();
    let down = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(!down.final_ownership().pointers().is_empty());
    assert!(
        down.final_ownership()
            .keyboard()
            .composition_generation()
            .is_some()
    );
    fill(&mut app, 64);
    app.cancel_composition(composition.generation().clone())
        .unwrap();
    app.submit_pointer(pointer(
        surface.input_context(),
        741,
        PointerPhase::Up,
        [],
        PointerButton::Primary,
    ))
    .unwrap();
    let complete = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(complete.final_ownership().pointers().is_empty());
    assert!(
        complete
            .final_ownership()
            .keyboard()
            .composition_generation()
            .is_none()
    );
    app.shutdown().unwrap();
}

const fn space(phase: KeyboardPhase) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        PhysicalKey::Space,
        LogicalKey::Space,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
fn space_release_reuses_its_lifetime_frontier_under_unrelated_pressure() {
    let (mut app, _) = button_runtime(17);
    app.submit_keyboard(space(KeyboardPhase::Down)).unwrap();
    let down = app.pump(FULL).unwrap();
    assert!(
        down.final_ownership()
            .keyboard()
            .space_activation_owner()
            .is_some()
    );
    let followers = fill(&mut app, 17);
    app.submit_keyboard(space(KeyboardPhase::Up)).unwrap();
    let complete = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(
        complete
            .final_ownership()
            .keyboard()
            .space_activation_owner()
            .is_none()
    );
    assert_eq!(*app.state(), followers + 1);
}

#[test]
fn pending_space_down_and_release_share_credits_before_the_first_pump() {
    let (mut app, _) = button_runtime(17);
    app.submit_keyboard(space(KeyboardPhase::Down)).unwrap();
    let followers = fill(&mut app, 17);
    app.submit_keyboard(space(KeyboardPhase::Up)).unwrap();
    let complete = app.pump(FULL).unwrap();
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert!(
        complete
            .final_ownership()
            .keyboard()
            .space_activation_owner()
            .is_none()
    );
    assert_eq!(*app.state(), followers + 1);
}

#[derive(Debug)]
struct PreventSpaceRelease;
impl Widget<Action> for PreventSpaceRelease {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, _: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(0)
    }
    fn activation(&self, (): &()) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if event
            .as_keyboard()
            .is_some_and(|event| event.phase() == KeyboardPhase::Up)
        {
            context.prevent_default();
        }
        WidgetEventOutput::none()
    }
}
struct PreventSpaceApp;
impl UiApp for PreventSpaceApp {
    type State = usize;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &usize) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &usize, _: &Action) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0)
    }
    fn interaction_producer_budget(_: &usize) -> ProducerBudget {
        ProducerBudget::new(3, 1)
            .with_application_outputs(0)
            .with_reconciliation(0, 1, 0)
    }
    fn root(_: &usize) -> impl View<Action> {
        Element::new(PreventSpaceRelease).focusable(true)
    }
    fn update(state: &mut usize, action: Action) {
        Form::update(state, action);
    }
}

#[test]
fn prevented_release_keeps_the_actual_space_lifetime_and_its_cancel_claim() {
    let mut app = AppRuntime::<PreventSpaceApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_queue_capacity(4),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    let owner = app.index().nodes()[0].id().clone();
    app.submit_command(
        owner,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    app.submit_keyboard(space(KeyboardPhase::Down)).unwrap();
    app.pump(FULL).unwrap();
    fill(&mut app, 4);
    app.submit_keyboard(space(KeyboardPhase::Up)).unwrap();
    let prevented = app.pump(FULL).unwrap();
    assert!(
        prevented
            .final_ownership()
            .keyboard()
            .space_activation_owner()
            .is_some()
    );
    fill(&mut app, 4);
    app.submit_keyboard(space(KeyboardPhase::Cancel)).unwrap();
    let cancelled = app.pump(FULL).unwrap();
    assert!(
        cancelled
            .final_ownership()
            .keyboard()
            .space_activation_owner()
            .is_none()
    );
    assert_eq!(app.status(), RuntimeStatus::Running);
    assert_eq!(fill(&mut app, 4), 4);
}

// U475-REVIEW-SPACE-QUEUED-CANCEL-01: observe the prevented Up before
// processing Cancel, so successful cancellation is never mistaken for a leak.
#[test]
fn queued_prevented_space_up_and_cancel_share_the_finite_lifetime_frontier() {
    for (capacity, saturated, trace_capacity) in [4, 8].into_iter().flat_map(|capacity| {
        [false, true]
            .into_iter()
            .flat_map(move |saturated| [0, 512].map(move |trace| (capacity, saturated, trace)))
    }) {
        let mut app = AppRuntime::<PreventSpaceApp>::mount_with_config(
            0,
            RuntimeConfig::default()
                .with_queue_capacity(capacity)
                .with_trace_config(runenui_runtime::TraceConfig::new(trace_capacity)),
        )
        .unwrap();
        app.pump(FULL).unwrap();
        let owner = app.index().nodes()[0].id().clone();
        app.submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap();
        app.pump(FULL).unwrap();
        app.submit_keyboard(space(KeyboardPhase::Down)).unwrap();
        app.pump(FULL).unwrap();
        let followers = if saturated {
            fill(&mut app, capacity)
        } else {
            0
        };
        let up = app.submit_keyboard(space(KeyboardPhase::Up)).unwrap();
        let cancel = app.submit_keyboard(space(KeyboardPhase::Cancel)).unwrap();
        assert!(up.sequence() < cancel.sequence());

        // A third overlapping occurrence cannot spend the same guard twice,
        // even if unrelated ambient queue capacity remains available.
        let refused = app
            .submit_keyboard(space(KeyboardPhase::Cancel))
            .unwrap_err();
        assert_eq!(
            refused.kind(),
            runenui_runtime::SubmitKeyboardErrorKind::Full
        );
        assert_eq!(refused.into_event().phase(), KeyboardPhase::Cancel);

        let prevented = app.pump(PumpBudget::new(followers + 1, 0, 0, 0)).unwrap();
        assert!(
            prevented
                .final_ownership()
                .keyboard()
                .space_activation_owner()
                .is_some()
        );
        assert_eq!(prevented.ordered_records().iter().filter(|record| matches!(record,
            runenui_runtime::InputArbitrationRecord::InputSettled(settled)
                if settled.sequence() == up.sequence()
                    && matches!(settled.finality(), runenui_runtime::UiInputFinality::Committed(facts)
                        if facts.default_prevented())
        )).count(), 1);
        let cancelled = app.pump(FULL).unwrap();
        assert!(
            cancelled
                .final_ownership()
                .keyboard()
                .space_activation_owner()
                .is_none()
        );
        assert_eq!(cancelled.ordered_records().iter().filter(|record| matches!(record,
            runenui_runtime::InputArbitrationRecord::InputSettled(settled)
                if settled.sequence() == cancel.sequence()
                    && matches!(settled.finality(), runenui_runtime::UiInputFinality::Committed(_))
        )).count(), 1);
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert_eq!(*app.state(), followers);
        assert_eq!(
            fill(&mut app, capacity),
            capacity,
            "retired root returns every credit"
        );
    }
}
