#![allow(refining_impl_trait)]

use runenui_core::{
    Element, EventContext, LogicalDelta, LogicalLength, LogicalPoint, NoHostProtocol,
    PointerBoundaryKind, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, StyleTokens, SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
    WorkSequence,
};
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, TraceDeliveryOutcome,
    TraceEventFamily, TracePointerPath, TraceRecord, TraceRecordKind, TraceRouteSnapshot,
    TraceSurfaceSnapshotKind, TraceTarget, TraceTargetTransition,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Activated,
}

struct App;

impl UiApp for App {
    type State = bool;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> impl View<Self::Action> {
        Element::new(Probe)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Activated => *state = true,
        }
    }
}

#[derive(Debug)]
struct Probe;

impl Widget<Action> for Probe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        _event: &UiEvent,
        _context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
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

    fn measure(&self, _state: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::new(32.0).unwrap_or_default(),
            height: LogicalLength::new(32.0).unwrap_or_default(),
        }
    }
}

struct Harness {
    runtime: AppRuntime<App>,
    context: SurfaceInputContext,
    target: runenui_core::MountedNodeId,
    inside: LogicalPoint,
    outside: LogicalPoint,
}

fn harness() -> Harness {
    let mut runtime = AppRuntime::<App>::mount(false);
    let tokens = StyleTokens::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("test surface size is finite"));
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(&tokens, size));
    let node = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("root is published"));
    let target = node.id().clone();
    let bounds = node.bounds();
    let inside = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let outside = LogicalPoint::new(size.width() + 1.0, size.height() + 1.0)
        .unwrap_or_else(|_| unreachable!("outside surface coordinates are finite"));
    Harness {
        runtime,
        context: publication.input_context().clone(),
        target,
        inside,
        outside,
    }
}

fn pointer_event(
    harness: &Harness,
    pointer_id: u64,
    phase: PointerPhase,
    point: LogicalPoint,
) -> PointerEvent {
    let pointer_id = PointerId::new(pointer_id)
        .unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        phase,
        point,
        harness.context.clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_scroll_delta(LogicalDelta::ZERO);
    if phase == PointerPhase::Down {
        event.with_changed_button(PointerButton::Primary)
    } else {
        event
    }
}

fn submit_and_pump(harness: &mut Harness, event: PointerEvent) -> WorkSequence {
    let submission = harness
        .runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("pointer submission is accepted"));
    let report = harness.runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());
    submission.sequence()
}

fn record<'a>(
    records: &[&'a TraceRecord],
    sequence: WorkSequence,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| record.work_sequence() == Some(sequence) && predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("required pointer boundary fact is retained"))
}

fn trace_path(record: &TraceRecord) -> &[TraceTarget] {
    record
        .context()
        .physical_path()
        .map(TracePointerPath::targets)
        .unwrap_or_else(|| unreachable!("pointer boundary fact owns its physical path"))
}

fn trace_route(record: &TraceRecord) -> &TraceRouteSnapshot {
    record
        .context()
        .route()
        .unwrap_or_else(|| unreachable!("resolved boundary owns target-only route"))
}

fn trace_transition(record: &TraceRecord) -> &TraceTargetTransition {
    record
        .context()
        .target_transition()
        .unwrap_or_else(|| unreachable!("pointer boundary fact owns exact transition"))
}

fn assert_plan_identity(bundle: &TraceRecord, pointer_id: &PointerId) {
    let event = bundle
        .context()
        .event()
        .unwrap_or_else(|| unreachable!("boundary plan owns event classification"));
    assert_eq!(event.family(), TraceEventFamily::PointerBoundary);
    assert!(!event.is_cancelable());
    assert_eq!(bundle.context().delivery(), None);
    assert_eq!(bundle.context().route(), None);
    assert_eq!(
        bundle
            .context()
            .pointer()
            .map(|pointer| pointer.pointer_id()),
        Some(pointer_id)
    );
    assert_eq!(
        bundle.context().surface().map(|surface| surface.snapshot()),
        Some(Some(TraceSurfaceSnapshotKind::Current))
    );
}

#[test]
fn initial_enter_reconstructs_empty_previous_and_exact_current_path() {
    let mut harness = harness();
    let pointer_id =
        PointerId::new(1).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
    let event = pointer_event(
        &harness,
        pointer_id.get(),
        PointerPhase::Down,
        harness.inside,
    );
    let sequence = submit_and_pump(&mut harness, event);

    let records = harness.runtime.trace().records().collect::<Vec<_>>();
    let physical = record(&records, sequence, |kind| {
        matches!(kind, TraceRecordKind::PointerPhysicalTargetResolved)
    });
    let bundle = record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerBoundaryBundlePlanned { notifications: 1 }
        )
    });
    let routed = record(&records, sequence, |kind| {
        matches!(kind, TraceRecordKind::RoutedEventStarted)
    });
    let resolved = record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerBoundaryNotificationResolved {
                kind: PointerBoundaryKind::Enter,
            }
        )
    });

    assert_plan_identity(bundle, &pointer_id);
    assert!(trace_path(bundle).is_empty());
    assert_eq!(
        trace_path(physical)
            .first()
            .map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    let transition = trace_transition(bundle);
    assert_eq!(transition.previous(), None);
    assert_eq!(
        transition.current().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );

    assert_eq!(
        resolved.context().delivery(),
        Some(TraceDeliveryOutcome::Delivered)
    );
    let route = trace_route(resolved);
    assert_eq!(route.targets().len(), 1);
    assert_eq!(
        route.targets().first().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(route.related_target(), None);
    assert_eq!(
        resolved.target().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );

    assert_eq!(bundle.causal_parent(), Some(physical.sequence()));
    assert_eq!(routed.causal_parent(), Some(bundle.sequence()));
    assert!(resolved.sequence() > routed.sequence());
    assert_eq!(bundle.instant(), physical.instant());
    assert_eq!(resolved.instant(), bundle.instant());
}

#[test]
fn leave_reconstructs_exact_previous_and_empty_current_path() {
    let mut harness = harness();
    let move_inside = pointer_event(&harness, 2, PointerPhase::Move, harness.inside);
    submit_and_pump(&mut harness, move_inside);
    let pointer_id =
        PointerId::new(2).unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
    let move_outside = pointer_event(
        &harness,
        pointer_id.get(),
        PointerPhase::Move,
        harness.outside,
    );
    let sequence = submit_and_pump(&mut harness, move_outside);

    let records = harness.runtime.trace().records().collect::<Vec<_>>();
    let physical = record(&records, sequence, |kind| {
        matches!(kind, TraceRecordKind::PointerPhysicalTargetResolved)
    });
    let bundle = record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerBoundaryBundlePlanned { notifications: 1 }
        )
    });
    let resolved = record(&records, sequence, |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerBoundaryNotificationResolved {
                kind: PointerBoundaryKind::Leave,
            }
        )
    });

    assert_plan_identity(bundle, &pointer_id);
    assert_eq!(
        trace_path(bundle).first().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert!(trace_path(physical).is_empty());
    assert_eq!(physical.target(), None);
    let transition = trace_transition(bundle);
    assert_eq!(
        transition.previous().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(transition.current(), None);

    assert_eq!(
        resolved.context().delivery(),
        Some(TraceDeliveryOutcome::Delivered)
    );
    let route = trace_route(resolved);
    assert_eq!(route.targets().len(), 1);
    assert_eq!(
        route.targets().first().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(route.related_target(), None);
    assert_eq!(
        resolved.target().map(TraceTarget::mounted_node_id),
        Some(&harness.target)
    );
    assert_eq!(bundle.causal_parent(), Some(physical.sequence()));
    assert_eq!(resolved.instant(), bundle.instant());
}
