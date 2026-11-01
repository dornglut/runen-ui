#![allow(refining_impl_trait)]
use runenui_core::*;
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, RuntimeConfig, RuntimeStatus, SurfaceBuildContext,
    TraceConfig,
};
use std::{cell::Cell, rc::Rc};
const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
#[derive(Debug)]
enum Action {
    Disable,
    Noop,
}
struct State {
    enabled: bool,
    losses: Rc<Cell<usize>>,
    noops: usize,
}
struct App;
#[derive(Debug)]
struct Control {
    enabled: bool,
    losses: Rc<Cell<usize>>,
}
impl Widget<Action> for Control {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(usize::from(phase == WidgetProducerPhase::Event))
    }
    fn activation(&self, (): &()) -> WidgetActivation {
        WidgetActivation::actionable(self.enabled)
    }
    fn measure(&self, (): &(), _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(40_u16), LogicalLength::from(40_u16))
    }
    fn hit_test(&self, (): &(), context: HitContributionContext) -> HitContribution {
        let s = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0., 0., s.width(), s.height())
                .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}")),
        )
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let UiEvent::PointerCapture(capture) = event
            && capture.kind() == PointerCaptureKind::Lost
        {
            self.losses.set(self.losses.get() + 1);
            let _ = context.try_emit(Action::Noop);
        }
        WidgetEventOutput::none()
    }
}
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &State, action: &Action) -> ProducerBudget {
        match action {
            Action::Noop => ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0),
            Action::Disable => ProducerBudget::new(16, 8)
                .with_application_outputs(0)
                .with_reconciliation(0, 1, 0)
                .with_work_starts(WorkStartBudget::new(0, 0, 0, 0, 2))
                .with_work_residency(WorkResidencyBudget::new(0, 0, 0, 0, 0, 4)),
        }
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(32, 16)
            .with_application_outputs(0)
            .with_reconciliation(0, 1, 0)
            .with_work_starts(WorkStartBudget::new(0, 0, 0, 0, 2))
            .with_work_residency(WorkResidencyBudget::new(0, 0, 0, 0, 0, 4))
    }
    fn root(state: &State) -> impl View<Action> {
        Element::new(Control {
            enabled: state.enabled,
            losses: state.losses.clone(),
        })
    }
    fn update(state: &mut State, action: Action) {
        match action {
            Action::Disable => state.enabled = false,
            Action::Noop => state.noops += 1,
        }
    }
}
fn captured_runtime(trace_capacity: usize) -> (AppRuntime<App>, SurfaceInputContext) {
    let losses = Rc::new(Cell::new(0));
    let mut app = AppRuntime::<App>::mount_with_config(
        State {
            enabled: true,
            losses,
            noops: 0,
        },
        RuntimeConfig::default()
            .with_queue_capacity(64)
            .with_trace_config(TraceConfig::new(trace_capacity)),
    )
    .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
    app.pump(FULL)
        .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
    let env = StyleEnvironment::default();
    let published = app
        .publish_surface(&SurfaceBuildContext::tight(
            &env,
            LogicalSize::try_new(40., 40.)
                .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}")),
        ))
        .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
    app.pump(FULL)
        .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
    let event = PointerEvent::new(
        PointerId::new(1).unwrap_or_else(|| unreachable!("pointer identity is nonzero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        LogicalPoint::new(5., 5.)
            .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}")),
        published.input_context().clone(),
    )
    .with_changed_button(PointerButton::Primary)
    .with_buttons(PointerButtons::new([PointerButton::Primary]));
    app.submit_pointer(event)
        .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
    let down = app
        .pump(FULL)
        .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
    assert!(
        down.final_ownership().pointers()[0]
            .capture_owner()
            .is_some()
    );
    (app, published.input_context().clone())
}

fn acknowledge_state_services(app: &mut AppRuntime<App>) {
    let pending = app.pending_framework_services();
    let completions = pending
        .iter()
        .map(|service| {
            let response = match service.request() {
                FrameworkServiceRequest::Cursor { .. } => FrameworkServiceResponse::Cursor(Ok(())),
                FrameworkServiceRequest::InputMethod { .. } => {
                    FrameworkServiceResponse::InputMethod(Ok(()))
                }
                _ => unreachable!("fixture only synchronizes host state"),
            };
            (service.token(), response)
        })
        .collect::<Vec<_>>();
    for (token, response) in completions {
        app.complete_framework_service(&token, response)
            .unwrap_or_else(|_| unreachable!("host acknowledges live service"));
    }
    app.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert_eq!(app.pending_framework_services().len(), 0);
}

// U475-REVIEW-CAPTURE-RETIREMENT-01: same admitted update and callback,
// with and without unrelated followers, retaining the original C=64 bound.
#[test]
fn funded_capture_loss_survives_compatible_disablement_under_queue_pressure() {
    for (saturated, trace_capacity) in [(false, 0), (true, 0), (false, 512), (true, 512)] {
        let (mut app, context) = captured_runtime(trace_capacity);
        let losses = Rc::clone(&app.state().losses);
        app.submit_action(Action::Disable)
            .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
        let mut followers = 0;
        if saturated {
            while app.submit_action(Action::Noop).is_ok() {
                followers += 1;
                assert!(followers <= 64);
            }
        }
        app.pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|error| unreachable!("fixture admission: {error:?}"));
        let facts = app
            .trace()
            .kinds()
            .filter(|kind| {
                matches!(
                    kind,
                    runenui_runtime::TraceRecordKind::RoutedEventAdmissionRejected { .. }
                        | runenui_runtime::TraceRecordKind::RuntimeTerminal { .. }
                )
            })
            .collect::<Vec<_>>();
        eprintln!(
            "CAPTURE_RETIRE trace={trace_capacity} saturated={saturated} followers={followers} enabled={} losses={} status={:?} facts={facts:?}",
            app.state().enabled,
            losses.get(),
            app.status()
        );
        assert!(!app.state().enabled);
        assert_eq!(app.status(), RuntimeStatus::Running);
        assert_eq!(losses.get(), 1);
        assert_eq!(
            app.state().noops,
            0,
            "callback output stays at the FIFO tail"
        );
        app.pump(FULL)
            .unwrap_or_else(|error| unreachable!("cleanup descendants: {error:?}"));
        assert_eq!(app.state().noops, followers + 1);
        let release = PointerEvent::new(
            PointerId::new(1).unwrap_or_else(|| unreachable!()),
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            LogicalPoint::new(5., 5.).unwrap_or_else(|_| unreachable!()),
            context,
        )
        .with_changed_button(PointerButton::Primary);
        app.submit_pointer(release)
            .unwrap_or_else(|error| unreachable!("funded release: {error:?}"));
        let retired = app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(retired.final_ownership().pointers(), []);
        assert_eq!(losses.get(), 1, "late release cannot duplicate CaptureLost");
        acknowledge_state_services(&mut app);
        let mut recovered = 0;
        while app.submit_action(Action::Noop).is_ok() {
            recovered += 1;
            assert!(recovered <= 64);
        }
        assert_eq!(
            recovered, 64,
            "every retired producer credit is available again"
        );
    }
}
