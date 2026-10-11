//! A host correlates all native families across unrelated `UiApp` types using opaque public keys.
use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, EventContext, EventPhase, HitContribution,
    HitContributionContext, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, LogicalPoint, LogicalRect, NoHostProtocol, PhysicalKey,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
    SemanticCommand, StyleEnvironment, SurfaceInputContext, UiApp, UiEvent, View, Widget,
    WidgetEventOutput, WidgetMeasure, WidgetTextInput, WorkSequence,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, InputArbitrationScope, InputPumpBatch, LogicalSize,
    PumpBudget, RuntimeConfig, SurfaceBuildContext, TraceConfig, UiInputFamily, UiInputFinality,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
const FULL: PumpBudget = PumpBudget::new(64, 64, 64, 64);
const FAMILIES: [UiInputFamily; 4] = [
    UiInputFamily::Keyboard,
    UiInputFamily::CommittedText,
    UiInputFamily::Composition,
    UiInputFamily::Pointer,
];
type Observed = Rc<RefCell<Vec<UiInputFamily>>>;
#[derive(Debug)]
struct Probe(Observed);
impl<Action> Widget<Action> for Probe {
    type State = ();
    fn create_state(&self) {}
    fn text_input(&self, (): &()) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
    fn measure(&self, (): &(), _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(32_u16.into(), 32_u16.into())
    }
    fn hit_test(&self, (): &(), context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!()),
        )
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target {
            let family = match event {
                UiEvent::Keyboard(_) => Some(UiInputFamily::Keyboard),
                UiEvent::CommittedText(_) => Some(UiInputFamily::CommittedText),
                UiEvent::Composition(_) => Some(UiInputFamily::Composition),
                UiEvent::Pointer(_) => Some(UiInputFamily::Pointer),
                _ => None,
            };
            if let Some(family) = family {
                self.0.borrow_mut().push(family);
                context.claim_host_input();
            }
        }
        WidgetEventOutput::none()
    }
}
struct AppA;
impl UiApp for AppA {
    type State = Observed;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(state: &Observed) -> impl View<()> {
        Element::new(Probe(Rc::clone(state)))
            .id("probe")
            .focusable(true)
    }
    fn update(_: &mut Observed, (): ()) {
        unreachable!("fixture emits no actions")
    }
}
struct StateB {
    observed: Observed,
}
struct AppB;
impl UiApp for AppB {
    type State = StateB;
    type Action = u8;
    type HostProtocol = NoHostProtocol;
    fn root(state: &StateB) -> impl View<u8> {
        Element::new(Probe(Rc::clone(&state.observed)))
            .id("probe")
            .focusable(true)
    }
    fn update(_: &mut StateB, _: u8) {
        unreachable!("fixture emits no actions")
    }
}
fn prepare<App: UiApp>(runtime: &mut AppRuntime<App>) -> (SurfaceInputContext, LogicalPoint) {
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::new(64_u16.into(), 64_u16.into()),
        ))
        .unwrap_or_else(|_| unreachable!());
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == "probe"))
        .unwrap_or_else(|| unreachable!());
    let target = node.id().clone();
    let bounds = node.bounds();
    let point = LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!());
    let context = publication.input_context().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    (context, point)
}
fn admit<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    context: SurfaceInputContext,
    point: LogicalPoint,
) -> [WorkSequence; 4] {
    let keyboard = runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code("Fixture".into()),
            LogicalKey::Character("x".into()),
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!());
    let text = runtime
        .submit_text(CommittedTextEvent::new("x", None).unwrap_or_else(|_| unreachable!()))
        .unwrap_or_else(|_| unreachable!());
    let composition = runtime
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!());
    let pending = runtime.input_ownership().unwrap_or_else(|_| unreachable!());
    assert_eq!(
        pending.keyboard().composition_generation(),
        Some(composition.generation())
    );
    let pointer = runtime
        .submit_pointer(
            PointerEvent::new(
                PointerId::new(1).unwrap_or_else(|| unreachable!()),
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                point,
                context,
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|_| unreachable!());
    [
        keyboard.sequence(),
        text.sequence(),
        composition.sequence(),
        pointer.sequence(),
    ]
}
type Pending = HashMap<(InputArbitrationScope, WorkSequence), UiInputFamily>;
fn correlate(
    pending: &mut Pending,
    batch: &InputPumpBatch,
    scope: &InputArbitrationScope,
    sequences: &[WorkSequence],
    families: &[UiInputFamily],
) {
    let settled: Vec<_> = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(input) => Some(input),
            _ => None,
        })
        .collect();
    assert_eq!(settled.len(), sequences.len());
    for ((input, sequence), family) in settled.iter().zip(sequences).zip(families) {
        assert_eq!(input.scope(), scope);
        assert_eq!(input.sequence(), *sequence);
        assert_eq!(input.family(), *family);
        assert_eq!(input.device_id(), None);
        assert!(matches!(input.finality(), UiInputFinality::Committed(_)));
        assert_eq!(
            pending.remove(&(input.scope().clone(), input.sequence())),
            Some(*family)
        );
    }
    assert_eq!(batch.processed_through(), sequences.last().copied());
    assert_eq!(batch.final_ownership().scope(), scope);
}
#[test]
fn two_unrelated_slots_correlate_all_four_families_without_sequence_or_generation_aliasing() {
    for capacity in [0, 1, 512] {
        let a_observed = Observed::default();
        let b_observed = Observed::default();
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(capacity));
        let mut a = AppRuntime::<AppA>::mount_with_config(Rc::clone(&a_observed), config);
        let mut b = AppRuntime::<AppB>::mount_with_config(
            StateB {
                observed: Rc::clone(&b_observed),
            },
            config,
        );
        let (a_context, a_point) = prepare(&mut a);
        let (b_context, b_point) = prepare(&mut b);
        let a_sequences = admit(&mut a, a_context, a_point);
        let b_sequences = admit(&mut b, b_context, b_point);
        assert_eq!(
            a_sequences, b_sequences,
            "local sequences deliberately overlap"
        );
        let a_before = a.input_ownership().unwrap_or_else(|_| unreachable!());
        let b_before = b.input_ownership().unwrap_or_else(|_| unreachable!());
        assert_ne!(a_before.scope(), b_before.scope());
        assert_ne!(
            a_before.keyboard().composition_generation(),
            b_before.keyboard().composition_generation()
        );
        let mut pending = Pending::new();
        for (scope, sequences) in [
            (a_before.scope(), a_sequences),
            (b_before.scope(), b_sequences),
        ] {
            for (sequence, family) in sequences.into_iter().zip(FAMILIES) {
                assert_eq!(pending.insert((scope.clone(), sequence), family), None);
            }
        }
        let zero = a
            .pump(PumpBudget::new(0, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(zero.ordered_records(), []);
        assert_eq!(zero.processed_through(), None);
        correlate(
            &mut pending,
            &b.pump(PumpBudget::new(4, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!()),
            b_before.scope(),
            &b_sequences,
            &FAMILIES,
        );
        assert_eq!(pending.len(), 4);
        assert!(a_observed.borrow().is_empty());
        assert_eq!(*b_observed.borrow(), FAMILIES);
        correlate(
            &mut pending,
            &a.pump(PumpBudget::new(2, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!()),
            a_before.scope(),
            &a_sequences[..2],
            &FAMILIES[..2],
        );
        assert_eq!(pending.len(), 2);
        correlate(
            &mut pending,
            &a.pump(PumpBudget::new(2, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!()),
            a_before.scope(),
            &a_sequences[2..],
            &FAMILIES[2..],
        );
        assert!(pending.is_empty());
        assert_eq!(*a_observed.borrow(), FAMILIES);
        for batch in [
            a.pump(FULL).unwrap_or_else(|_| unreachable!()),
            b.pump(FULL).unwrap_or_else(|_| unreachable!()),
        ] {
            assert!(
                batch
                    .ordered_records()
                    .iter()
                    .all(|record| !matches!(record, InputArbitrationRecord::InputSettled(_)))
            );
        }
        assert_eq!(*a_observed.borrow(), FAMILIES);
        assert_eq!(*b_observed.borrow(), FAMILIES);
        for len in [a.trace().len(), b.trace().len()] {
            if capacity <= 1 {
                assert_eq!(len, capacity);
            } else {
                assert!(len <= capacity);
            }
        }
    }
}
