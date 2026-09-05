#![allow(refining_impl_trait)]

use core::num::NonZeroUsize;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use runenui_core::{
    Element, EventContext, HitContribution, HitContributionContext, LogicalLength, LogicalPoint,
    LogicalRect, NoHostProtocol, PointerButton, PointerButtons, PointerCaptureKind,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment,
    SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetActivationContext,
    WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeConfig, SurfaceBuildContext, SurfacePublication,
    TracePointerRejection, TraceRecordKind, TraceReplay,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Observation {
    Pointer {
        phase: PointerPhase,
        buttons: Vec<PointerButton>,
        changed_button: Option<PointerButton>,
    },
    Capture(PointerCaptureKind),
}

#[derive(Clone)]
struct State {
    observations: Rc<RefCell<Vec<Observation>>>,
    activations: Rc<Cell<usize>>,
}

#[derive(Clone, Copy)]
enum Action {
    Activated,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe {
            observations: Rc::clone(&state.observations),
        })
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Activated => state.activations.set(state.activations.get() + 1),
        }
    }
}

#[derive(Debug)]
struct Probe {
    observations: Rc<RefCell<Vec<Observation>>>,
}

impl Widget<Action> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        _context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        match event {
            UiEvent::Pointer(pointer) => {
                self.observations.borrow_mut().push(Observation::Pointer {
                    phase: pointer.phase(),
                    buttons: pointer.buttons().iter().collect(),
                    changed_button: pointer.changed_button(),
                });
            }
            UiEvent::PointerCapture(capture) => self
                .observations
                .borrow_mut()
                .push(Observation::Capture(capture.kind())),
            _ => {}
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, _state: &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        _state: &mut Self::State,
        _context: &mut WidgetActivationContext<Action>,
    ) -> WidgetActivationOutput<Action> {
        WidgetActivationOutput::action(Action::Activated)
    }

    fn measure(
        &self,
        _state: &Self::State,
        _input: runenui_core::WidgetMeasureInput,
    ) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::new(32.0).unwrap_or_default(),
            LogicalLength::new(32.0).unwrap_or_default(),
        )
    }

    fn hit_test(&self, _state: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a valid hit rectangle"));
        HitContribution::single_rect(rect)
    }
}

struct Harness {
    runtime: AppRuntime<App>,
    context: SurfaceInputContext,
    point: LogicalPoint,
    observations: Rc<RefCell<Vec<Observation>>>,
    activations: Rc<Cell<usize>>,
}

fn harness(config: RuntimeConfig) -> Harness {
    let observations = Rc::new(RefCell::new(Vec::new()));
    let activations = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            observations: Rc::clone(&observations),
            activations: Rc::clone(&activations),
        },
        config,
    );
    let publication = publish(&mut runtime);
    let node = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the probe is published"));
    let bounds = node.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    pump_all(&mut runtime);
    Harness {
        runtime,
        context: publication.input_context().clone(),
        point,
        observations,
        activations,
    }
}

fn publish(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    runtime
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|_| unreachable!("surface publication is admitted"))
}

fn event(
    pointer_id: u64,
    phase: PointerPhase,
    changed_button: Option<PointerButton>,
    buttons: impl IntoIterator<Item = PointerButton>,
    context: &SurfaceInputContext,
    point: LogicalPoint,
) -> PointerEvent {
    let pointer_id = PointerId::new(pointer_id)
        .unwrap_or_else(|| unreachable!("test pointer identities are non-zero"));
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        phase,
        point,
        context.clone(),
    )
    .with_buttons(PointerButtons::new(buttons));
    match changed_button {
        Some(button) => event.with_changed_button(button),
        None => event,
    }
}

fn pump_all(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(
                usize::MAX,
                usize::MAX,
                usize::MAX,
                usize::MAX,
            ))
            .is_quiescent()
    );
}

fn submit(harness: &mut Harness, pointer: PointerEvent) {
    harness
        .runtime
        .submit_pointer(pointer)
        .unwrap_or_else(|_| unreachable!("the pointer envelope is admitted"));
    pump_all(&mut harness.runtime);
}

fn capture_lost_count(harness: &Harness) -> usize {
    harness
        .observations
        .borrow()
        .iter()
        .filter(|observation| matches!(observation, Observation::Capture(PointerCaptureKind::Lost)))
        .count()
}

fn stream_closed_count(harness: &Harness, pointer_id: u64) -> usize {
    harness
        .runtime
        .trace()
        .kinds()
        .filter(|kind| {
            matches!(
                kind,
                TraceRecordKind::PointerStreamClosed { pointer_id: id } if id.get() == pointer_id
            )
        })
        .count()
}

#[test]
fn secondary_partial_release_preserves_primary_capture_until_final_primary_release() {
    let mut harness = harness(RuntimeConfig::default());
    let context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            201,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            201,
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            [PointerButton::Primary, PointerButton::Secondary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            201,
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            [PointerButton::Primary],
            &context,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 0);
    assert_eq!(capture_lost_count(&harness), 0);
    assert_eq!(stream_closed_count(&harness, 201), 0);

    submit(
        &mut harness,
        event(
            201,
            PointerPhase::Up,
            Some(PointerButton::Primary),
            [],
            &context,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 1);
    assert_eq!(capture_lost_count(&harness), 1);
    assert_eq!(stream_closed_count(&harness, 201), 1);
}

#[test]
fn primary_partial_release_ends_primary_interaction_but_keeps_secondary_stream_alive() {
    let mut harness = harness(RuntimeConfig::default());
    let context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            202,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            202,
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            [PointerButton::Primary, PointerButton::Secondary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            202,
            PointerPhase::Up,
            Some(PointerButton::Primary),
            [PointerButton::Secondary],
            &context,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 1);
    assert_eq!(capture_lost_count(&harness), 1);
    assert_eq!(stream_closed_count(&harness, 202), 0);

    submit(
        &mut harness,
        event(
            202,
            PointerPhase::Move,
            None,
            [PointerButton::Secondary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            202,
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            [],
            &context,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 1);
    assert_eq!(capture_lost_count(&harness), 1);
    assert_eq!(stream_closed_count(&harness, 202), 1);
}

#[test]
fn secondary_first_chord_uses_one_pointer_stream_through_primary_partial_release() {
    let mut harness = harness(RuntimeConfig::default());
    let context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            203,
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            [PointerButton::Secondary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            203,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary, PointerButton::Secondary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            203,
            PointerPhase::Up,
            Some(PointerButton::Primary),
            [PointerButton::Secondary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            203,
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            [],
            &context,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 1);
    assert_eq!(capture_lost_count(&harness), 1);
    assert_eq!(stream_closed_count(&harness, 203), 1);
    assert_eq!(
        harness
            .runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::PointerStreamRegistered { pointer_id, .. } if pointer_id.get() == 203
            ))
            .count(),
        1
    );
}

#[test]
fn malformed_button_transition_is_rejected_without_mutating_the_live_stream() {
    let mut harness = harness(RuntimeConfig::default());
    let context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            204,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary],
            &context,
            point,
        ),
    );
    let callbacks_before = harness.observations.borrow().len();
    submit(
        &mut harness,
        event(
            204,
            PointerPhase::Move,
            None,
            [PointerButton::Primary, PointerButton::Secondary],
            &context,
            point,
        ),
    );

    assert_eq!(harness.observations.borrow().len(), callbacks_before);
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Move,
            outcome: TracePointerRejection::ButtonTransitionMismatch,
        } if pointer_id.get() == 204
    )));
    assert_eq!(stream_closed_count(&harness, 204), 0);

    let jsonl = harness.runtime.trace().export_jsonl();
    assert!(jsonl.contains("\"outcome\":\"button_transition_mismatch\""));
    let replay = TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| unreachable!("canonical pointer rejection must replay: {error}"));
    assert!(
        replay
            .records()
            .any(|record| { record.kind().as_str() == "pointer_ingress_rejected" })
    );

    submit(
        &mut harness,
        event(
            204,
            PointerPhase::Up,
            Some(PointerButton::Primary),
            [],
            &context,
            point,
        ),
    );
    assert_eq!(harness.activations.get(), 1);
    assert_eq!(stream_closed_count(&harness, 204), 1);
}

#[test]
fn retired_primary_partial_release_clears_primary_interaction_without_closing_stream() {
    let retention =
        NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("test retention is non-zero"));
    let mut harness = harness(RuntimeConfig::default().with_surface_snapshot_retention(retention));
    let old_context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            205,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary],
            &old_context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            205,
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            [PointerButton::Primary, PointerButton::Secondary],
            &old_context,
            point,
        ),
    );
    let current = publish(&mut harness.runtime);
    pump_all(&mut harness.runtime);
    harness.observations.borrow_mut().clear();
    harness.activations.set(0);

    submit(
        &mut harness,
        event(
            205,
            PointerPhase::Up,
            Some(PointerButton::Primary),
            [PointerButton::Secondary],
            &old_context,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 0);
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation::Capture(PointerCaptureKind::Lost)]
    );
    assert_eq!(stream_closed_count(&harness, 205), 0);
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Up,
            outcome: TracePointerRejection::RetiredGeneration,
        } if pointer_id.get() == 205
    )));

    submit(
        &mut harness,
        event(
            205,
            PointerPhase::Move,
            None,
            [PointerButton::Secondary],
            current.input_context(),
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            205,
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            [],
            current.input_context(),
            point,
        ),
    );
    assert_eq!(stream_closed_count(&harness, 205), 1);
}

#[test]
fn retired_secondary_partial_release_preserves_primary_interaction_for_current_final_release() {
    let retention =
        NonZeroUsize::new(1).unwrap_or_else(|| unreachable!("test retention is non-zero"));
    let mut harness = harness(RuntimeConfig::default().with_surface_snapshot_retention(retention));
    let old_context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            206,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary],
            &old_context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            206,
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            [PointerButton::Primary, PointerButton::Secondary],
            &old_context,
            point,
        ),
    );
    let current = publish(&mut harness.runtime);
    pump_all(&mut harness.runtime);
    harness.observations.borrow_mut().clear();
    harness.activations.set(0);

    submit(
        &mut harness,
        event(
            206,
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            [PointerButton::Primary],
            &old_context,
            point,
        ),
    );

    assert!(harness.observations.borrow().is_empty());
    assert_eq!(capture_lost_count(&harness), 0);
    assert_eq!(stream_closed_count(&harness, 206), 0);

    submit(
        &mut harness,
        event(
            206,
            PointerPhase::Up,
            Some(PointerButton::Primary),
            [],
            current.input_context(),
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 1);
    assert_eq!(capture_lost_count(&harness), 1);
    assert_eq!(stream_closed_count(&harness, 206), 1);
}

#[test]
fn missing_primary_partial_release_clears_primary_interaction_without_closing_stream() {
    let mut harness = harness(RuntimeConfig::default());
    let context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            207,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            207,
            PointerPhase::Down,
            Some(PointerButton::Secondary),
            [PointerButton::Primary, PointerButton::Secondary],
            &context,
            point,
        ),
    );
    let missing = harness.runtime.__surface_context_for_test(
        0,
        1,
        context.coordinate_revision(),
        context.hit_test_generation() + 100,
    );
    harness.observations.borrow_mut().clear();
    harness.activations.set(0);

    submit(
        &mut harness,
        event(
            207,
            PointerPhase::Up,
            Some(PointerButton::Primary),
            [PointerButton::Secondary],
            &missing,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 0);
    assert_eq!(
        harness.observations.borrow().as_slice(),
        [Observation::Capture(PointerCaptureKind::Lost)]
    );
    assert_eq!(stream_closed_count(&harness, 207), 0);
    assert!(harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            phase: PointerPhase::Up,
            outcome: TracePointerRejection::MissingGeneration,
        } if pointer_id.get() == 207
    )));

    submit(
        &mut harness,
        event(
            207,
            PointerPhase::Move,
            None,
            [PointerButton::Secondary],
            &context,
            point,
        ),
    );
    submit(
        &mut harness,
        event(
            207,
            PointerPhase::Up,
            Some(PointerButton::Secondary),
            [],
            &context,
            point,
        ),
    );

    assert_eq!(harness.activations.get(), 0);
    assert_eq!(stream_closed_count(&harness, 207), 1);
}

#[test]
fn initial_down_may_include_preheld_buttons_and_cancel_skips_transition_proof() {
    let mut harness = harness(RuntimeConfig::default());
    let context = harness.context.clone();
    let point = harness.point;

    submit(
        &mut harness,
        event(
            208,
            PointerPhase::Down,
            Some(PointerButton::Primary),
            [PointerButton::Primary, PointerButton::Secondary],
            &context,
            point,
        ),
    );

    assert!(harness.observations.borrow().iter().any(|observation| {
        matches!(
            observation,
            Observation::Pointer {
                phase: PointerPhase::Down,
                buttons,
                changed_button: Some(PointerButton::Primary),
            } if buttons.as_slice() == [PointerButton::Primary, PointerButton::Secondary]
        )
    }));
    assert_eq!(stream_closed_count(&harness, 208), 0);

    submit(
        &mut harness,
        event(
            208,
            PointerPhase::Cancel,
            Some(PointerButton::Secondary),
            [],
            &context,
            point,
        ),
    );

    assert_eq!(stream_closed_count(&harness, 208), 1);
    assert!(!harness.runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::PointerIngressRejected {
            pointer_id,
            outcome: TracePointerRejection::ButtonTransitionMismatch,
            ..
        } if pointer_id.get() == 208
    )));
}
