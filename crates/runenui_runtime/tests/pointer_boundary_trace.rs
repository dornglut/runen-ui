#![allow(refining_impl_trait)]

use runenui_core::{
    Element, EventContext, LogicalDelta, LogicalLength, LogicalPoint, NoHostProtocol,
    PointerBoundaryKind, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, StyleTokens, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetEventOutput, WidgetMeasure,
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

fn record<'a>(
    records: &[&'a TraceRecord],
    sequence: runenui_core::WorkSequence,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a TraceRecord {
    records
        .iter()
        .copied()
        .find(|record| record.work_sequence() == Some(sequence) && predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("required pointer boundary fact is retained"))
}

#[test]
fn initial_enter_reconstructs_exact_boundary_plan_and_delivery() {
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
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let pointer_id = PointerId::new(1)
        .unwrap_or_else(|| unreachable!("test pointer identity is non-zero"));
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        publication.input_context().clone(),
    )
    .with_changed_button(PointerButton::Primary)
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_scroll_delta(LogicalDelta::ZERO);

    let submission = runtime
        .submit_pointer(event)
        .unwrap_or_else(|_| unreachable!("pointer submission is accepted"));
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent());

    let records = runtime.trace().records().collect::<Vec<_>>();
    let physical = record(&records, submission.sequence(), |kind| {
        matches!(kind, TraceRecordKind::PointerPhysicalTargetResolved)
    });
    let bundle = record(&records, submission.sequence(), |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerBoundaryBundlePlanned { notifications: 1 }
        )
    });
    let routed = record(&records, submission.sequence(), |kind| {
        matches!(kind, TraceRecordKind::RoutedEventStarted)
    });
    let resolved = record(&records, submission.sequence(), |kind| {
        matches!(
            kind,
            TraceRecordKind::PointerBoundaryNotificationResolved {
                kind: PointerBoundaryKind::Enter,
            }
        )
    });

    let bundle_event = bundle
        .context()
        .event()
        .unwrap_or_else(|| unreachable!("boundary plan owns event classification"));
    assert_eq!(bundle_event.family(), TraceEventFamily::PointerBoundary);
    assert!(!bundle_event.is_cancelable());
    assert_eq!(bundle.context().delivery(), None);
    assert_eq!(bundle.context().route(), None);
    assert_eq!(
        bundle
            .context()
            .pointer()
            .map(|pointer| pointer.pointer_id()),
        Some(&pointer_id)
    );
    assert_eq!(
        bundle
            .context()
            .surface()
            .map(|surface| surface.snapshot()),
        Some(Some(TraceSurfaceSnapshotKind::Current))
    );
    assert_eq!(
        bundle
            .context()
            .physical_path()
            .map(TracePointerPath::targets)
            .and_then(|path| path.first())
            .map(TraceTarget::mounted_node_id),
        Some(&target)
    );
    let bundle_transition = bundle
        .context()
        .target_transition()
        .unwrap_or_else(|| unreachable!("boundary plan owns exact transition"));
    assert_eq!(bundle_transition.previous(), None);
    assert_eq!(
        bundle_transition
            .current()
            .map(TraceTarget::mounted_node_id),
        Some(&target)
    );

    assert_eq!(resolved.context().delivery(), Some(TraceDeliveryOutcome::Delivered));
    let route = resolved
        .context()
        .route()
        .unwrap_or_else(|| unreachable!("delivered boundary owns target-only route"));
    assert_eq!(
        route.targets()
            .first()
            .map(TraceTarget::mounted_node_id),
        Some(&target)
    );
    assert_eq!(route.targets().len(), 1);
    assert_eq!(route.related_target(), None);
    let transition = resolved
        .context()
        .target_transition()
        .unwrap_or_else(|| unreachable!("delivered boundary owns exact transition"));
    assert_eq!(transition.previous(), None);
    assert_eq!(
        transition.current().map(TraceTarget::mounted_node_id),
        Some(&target)
    );
    assert_eq!(
        resolved.target().map(TraceTarget::mounted_node_id),
        Some(&target)
    );

    assert_eq!(bundle.causal_parent(), Some(physical.sequence()));
    assert_eq!(routed.causal_parent(), Some(bundle.sequence()));
    assert!(resolved.sequence() > routed.sequence());
    assert_eq!(bundle.instant(), physical.instant());
    assert_eq!(resolved.instant(), bundle.instant());
}
