//! Event-time route and post-event capture are independent public facts.

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, EventPhase, HitContribution, HitContributionContext, LogicalPoint,
    LogicalRect, NoHostProtocol, PointerBoundaryKind, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, StyleEnvironment,
    SurfaceInputContext, UiApp, UiEvent, View, Widget, WidgetActivation, WidgetEventOutput,
    WidgetMeasure, row,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, InputPumpBatch, LogicalSize, MountedNodeId, PumpBudget,
    RuntimeConfig, SurfaceBuildContext, TraceConfig, TraceRecordKind, UiInputClaimReason,
    UiInputConflict, UiInputFamily, UiInputFinality, UiInputRoute, UiInputSettlement,
};

const FULL: PumpBudget = PumpBudget::new(64, 64, 64, 64);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservedPointer {
    phase: PointerPhase,
    routed: MountedNodeId,
    physical: Option<MountedNodeId>,
}

struct App;
impl UiApp for App {
    type State = Rc<RefCell<Vec<ObservedPointer>>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(observed: &Self::State) -> impl View<()> {
        row(vec![
            Element::new(Probe {
                transfer_on_enter: false,
                observed: Rc::clone(observed),
            })
            .id("left")
            .key("left")
            .focusable(false),
            Element::new(Probe {
                transfer_on_enter: true,
                observed: Rc::clone(observed),
            })
            .id("right")
            .key("right")
            .focusable(false),
        ])
    }
    fn update(_: &mut Self::State, (): ()) {}
}

#[derive(Debug)]
struct Probe {
    transfer_on_enter: bool,
    observed: Rc<RefCell<Vec<ObservedPointer>>>,
}
impl Widget<()> for Probe {
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn activation(&self, (): &()) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }
    fn measure(&self, (): &(), _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(32_u16.into(), 32_u16.into())
    }
    fn hit_test(&self, (): &(), context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("finite hit geometry")),
        )
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target {
            match event {
                UiEvent::PointerBoundary(boundary)
                    if self.transfer_on_enter && boundary.kind() == PointerBoundaryKind::Enter =>
                {
                    context.capture_pointer();
                }
                UiEvent::Pointer(pointer) => self.observed.borrow_mut().push(ObservedPointer {
                    phase: pointer.phase(),
                    routed: context.original_target().clone(),
                    physical: context.physical_target().cloned(),
                }),
                _ => {}
            }
        }
        WidgetEventOutput::none()
    }
}

struct Fixture {
    runtime: AppRuntime<App>,
    context: SurfaceInputContext,
    left: MountedNodeId,
    right: MountedNodeId,
    left_point: LogicalPoint,
    right_point: LogicalPoint,
    observed: Rc<RefCell<Vec<ObservedPointer>>>,
}

fn fixture(trace_capacity: usize) -> Fixture {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount_with_config(
        Rc::clone(&observed),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
    );
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!("mount"));
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::new(96_u16.into(), 48_u16.into()),
        ))
        .unwrap_or_else(|_| unreachable!("retained geometry"));
    let node = |name: &str| {
        publication
            .frame()
            .nodes()
            .iter()
            .find(|node| node.authored_id().is_some_and(|id| id.as_str() == name))
            .unwrap_or_else(|| unreachable!("published probe"))
    };
    let left = node("left");
    let right = node("right");
    let center = |rect: LogicalRect| {
        LogicalPoint::new(
            rect.x() + rect.width() / 2.0,
            rect.y() + rect.height() / 2.0,
        )
        .unwrap_or_else(|_| unreachable!("finite center"))
    };
    Fixture {
        context: publication.input_context().clone(),
        left: left.id().clone(),
        right: right.id().clone(),
        left_point: center(left.bounds()),
        right_point: center(right.bounds()),
        runtime,
        observed,
    }
}

fn pointer(
    context: &SurfaceInputContext,
    point: LogicalPoint,
    phase: PointerPhase,
) -> PointerEvent {
    let event = PointerEvent::new(
        PointerId::new(1).unwrap_or_else(|| unreachable!("pointer ID")),
        PointerDeviceKind::Mouse,
        phase,
        point,
        context.clone(),
    );
    match phase {
        PointerPhase::Down => event
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        PointerPhase::Move => event.with_buttons(PointerButtons::new([PointerButton::Primary])),
        _ => event,
    }
}

fn settled(batch: &InputPumpBatch) -> &UiInputSettlement {
    let mut inputs = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(input) => Some(input),
            _ => None,
        });
    let input = inputs
        .next()
        .unwrap_or_else(|| unreachable!("one exact native receipt"));
    assert_eq!(inputs.next(), None);
    input
}

fn assert_captured(batch: &InputPumpBatch, routed: &MountedNodeId) {
    let UiInputFinality::Committed(facts) = settled(batch).finality() else {
        unreachable!("valid captured input commits");
    };
    assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
    assert!(
        facts
            .reasons()
            .contains(&UiInputClaimReason::PointerCapture)
    );
    assert_eq!(
        facts.route(),
        &UiInputRoute::Captured {
            target: routed.clone()
        }
    );
}

fn drain_derived(f: &mut Fixture) {
    let batch = f
        .runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("derived pointer work"));
    assert!(
        batch
            .ordered_records()
            .iter()
            .all(|record| !matches!(record, InputArbitrationRecord::InputSettled(_))),
        "stationary re-hit and follow-ups cannot invent native receipts"
    );
}

fn press(f: &mut Fixture) -> InputPumpBatch {
    let down_receipt = f
        .runtime
        .submit_pointer(pointer(&f.context, f.left_point, PointerPhase::Down))
        .unwrap_or_else(|_| unreachable!("Down"));
    let down = f
        .runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("press"));
    let UiInputFinality::Committed(facts) = settled(&down).finality() else {
        unreachable!("physical Down commits before capture acquisition");
    };
    assert_eq!(
        facts.route(),
        &UiInputRoute::Routed {
            target: f.left.clone()
        }
    );
    assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
    assert!(
        facts
            .reasons()
            .contains(&UiInputClaimReason::PointerCapture)
    );
    assert_eq!(settled(&down).sequence(), down_receipt.sequence());
    assert_eq!(
        down.final_ownership().pointers()[0].capture_owner(),
        Some(&f.left)
    );
    // A press may queue a focus request; its separate boundary must be
    // drained before the one-envelope transfer witness.
    assert_eq!(
        f.runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("press follow-up"))
            .ordered_records(),
        []
    );
    down
}

fn route_subsequent_input_and_close(f: &mut Fixture) {
    drain_derived(f);
    f.observed.borrow_mut().clear();
    let next = f
        .runtime
        .submit_pointer(pointer(&f.context, f.left_point, PointerPhase::Move))
        .unwrap_or_else(|_| unreachable!("next Move"));
    let after = f
        .runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("new capture routes next input"));
    assert_captured(&after, &f.right);
    assert_eq!(settled(&after).sequence(), next.sequence());
    assert_eq!(
        *f.observed.borrow(),
        [ObservedPointer {
            phase: PointerPhase::Move,
            routed: f.right.clone(),
            physical: Some(f.left.clone()),
        }]
    );
    drain_derived(f);
    let cancel = f
        .runtime
        .submit_pointer(pointer(&f.context, f.left_point, PointerPhase::Cancel))
        .unwrap_or_else(|_| unreachable!("Cancel"));
    let closed = f
        .runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("capture cleanup"));
    assert_captured(&closed, &f.right);
    assert_eq!(settled(&closed).sequence(), cancel.sequence());
    assert_eq!(closed.final_ownership().pointers(), []);
    assert_eq!(
        f.runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("no replay"))
            .ordered_records(),
        []
    );
}

#[test]
fn boundary_capture_transfer_does_not_rewrite_the_current_native_move_route() {
    for trace_capacity in [0, 1, 512] {
        let mut f = fixture(trace_capacity);
        let down = press(&mut f);
        f.observed.borrow_mut().clear();
        let receipt = f
            .runtime
            .submit_pointer(pointer(&f.context, f.right_point, PointerPhase::Move))
            .unwrap_or_else(|_| unreachable!("transfer Move"));
        let transfer = f
            .runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("transfer commits"));
        assert_eq!(
            *f.observed.borrow(),
            [ObservedPointer {
                phase: PointerPhase::Move,
                routed: f.left.clone(),
                physical: Some(f.right.clone()),
            }]
        );
        assert_eq!(
            transfer.final_ownership().pointers()[0].capture_owner(),
            Some(&f.right)
        );
        assert!(
            matches!(transfer.ordered_records(), [InputArbitrationRecord::OwnershipChanged(change), InputArbitrationRecord::InputSettled(input)]
            if change.before_revision() == down.final_ownership().revision()
                && change.after().revision() == input.ownership_revision()
                && input.ownership_revision() == transfer.final_ownership().revision())
        );
        assert_captured(&transfer, &f.left);
        let input = settled(&transfer);
        assert_eq!(input.scope(), down.final_ownership().scope());
        assert_eq!(input.sequence(), receipt.sequence());
        assert_eq!(input.family(), UiInputFamily::Pointer);
        assert_eq!(
            input.pointer_id(),
            Some(PointerId::new(1).unwrap_or_else(|| unreachable!()))
        );
        assert_eq!(input.surface_id(), Some(f.context.surface_id()));
        assert_eq!(input.device_id(), None);
        if trace_capacity == 512 {
            assert!(
                f.runtime
                    .trace()
                    .records()
                    .any(|record| record.work_sequence() == Some(receipt.sequence())
                        && matches!(record.kind(), TraceRecordKind::RoutedEventStarted)
                        && record.original_target() == Some(&f.left))
            );
        }
        route_subsequent_input_and_close(&mut f);
        assert_eq!(
            down.final_ownership().pointers()[0].capture_owner(),
            Some(&f.left)
        );
        assert_eq!(
            transfer.final_ownership().pointers()[0].capture_owner(),
            Some(&f.right)
        );
        match trace_capacity {
            0 => assert_eq!(f.runtime.trace().len(), 0),
            1 => assert_eq!(f.runtime.trace().len(), 1),
            _ => assert!(f.runtime.trace().len() <= trace_capacity),
        }
    }
}
