#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    Element, EventContext, HitContribution, HitContributionContext, LogicalLength, LogicalPoint,
    LogicalRect, NoHostProtocol, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent,
    PointerId, PointerPhase, StyleEnvironment, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, SurfacePhase};

#[derive(Clone)]
struct State {
    phases: Rc<RefCell<Vec<PointerPhase>>>,
}

#[derive(Debug)]
enum Action {
    Observed(PointerPhase),
}

struct App;

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The fixture has at most 1 mounted owners and no initial output.
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Event output budgets sum to 1 across the mounted owners. Bound
        // ordinary/boundary route passes and four focus passes, plus four
        // activation/default outputs. Owner reconciliation and the cursor
        // service outcome/cancel pair stay inside the same finite frontier.
        runenui_core::ProducerBudget::new(16, 12)
            .with_application_outputs(0)
            .with_reconciliation(1, 1, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 1))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 0, 1))
    }

    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> impl View<Self::Action> {
        Element::new(ProbeWidget)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Observed(phase) => state.phases.borrow_mut().push(phase),
        }
    }
}

#[derive(Debug)]
struct ProbeWidget;

impl Widget<Action> for ProbeWidget {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Event => runenui_core::WidgetProducerBudget::new(1),
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        _state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if let UiEvent::Pointer(pointer) = event {
            let _ = context.try_emit(Action::Observed(pointer.phase()));
        }
        WidgetEventOutput::none()
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

struct PassiveApp;

impl UiApp for PassiveApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The fixture has at most 1 mounted owners and no initial output.
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Event output budgets sum to 0 across the mounted owners. Bound
        // ordinary/boundary route passes and four focus passes, plus four
        // activation/default outputs. Owner reconciliation and the cursor
        // service outcome/cancel pair stay inside the same finite frontier.
        runenui_core::ProducerBudget::new(8, 4)
            .with_application_outputs(0)
            .with_reconciliation(1, 1, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 1))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 0, 0, 1))
    }

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(PassiveHitWidget)
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[derive(Debug)]
struct PassiveHitWidget;

impl Widget<()> for PassiveHitWidget {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

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

fn pump_all<Application: UiApp>(runtime: &mut AppRuntime<Application>) {
    let report = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(report.is_quiescent());
}

#[test]
fn hover_stream_accepts_first_button_down_and_rejects_repeated_button_down() {
    let phases = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount(State {
        phases: Rc::clone(&phases),
    })
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(&environment, size))
        .unwrap_or_else(|_| unreachable!("the test surface publication is admitted"));
    let node = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the root is published"));
    let bounds = node.bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let context = publication.input_context().clone();
    let pointer_id =
        PointerId::new(1).unwrap_or_else(|| unreachable!("pointer identity is non-zero"));

    runtime
        .submit_pointer(PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            point,
            context.clone(),
        ))
        .unwrap_or_else(|_| unreachable!("hover ingress is accepted"));
    pump_all(&mut runtime);

    let down = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        context,
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);

    runtime
        .submit_pointer(down.clone())
        .unwrap_or_else(|_| unreachable!("the first button-down is accepted"));
    pump_all(&mut runtime);

    assert_eq!(
        phases.borrow().as_slice(),
        [PointerPhase::Move, PointerPhase::Down]
    );

    runtime
        .submit_pointer(down)
        .unwrap_or_else(|_| unreachable!("processing owns duplicate-down rejection"));
    pump_all(&mut runtime);

    assert_eq!(
        phases.borrow().as_slice(),
        [PointerPhase::Move, PointerPhase::Down]
    );
}

#[test]
fn routed_pointer_projection_change_requests_redraw_without_spurious_downstream_work() {
    let mut runtime = AppRuntime::<PassiveApp>::mount(())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let environment = StyleEnvironment::default();
    let size = LogicalSize::try_new(64.0, 64.0)
        .unwrap_or_else(|_| unreachable!("the test surface size is finite"));
    let context = SurfaceBuildContext::tight(&environment, size);
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("the initial publication is admitted"));
    assert!(runtime.take_redraw_request().is_none());

    let bounds = publication
        .frame()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("the passive root is published"))
        .bounds();
    let point = LogicalPoint::new(bounds.x() + 1.0, bounds.y() + 1.0)
        .unwrap_or_else(|_| unreachable!("published bounds are finite"));
    let input_context = publication.input_context().clone();
    let pointer_id =
        PointerId::new(2).unwrap_or_else(|| unreachable!("pointer identity is non-zero"));
    let hover = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        point,
        input_context,
    );

    runtime
        .submit_pointer(hover.clone())
        .unwrap_or_else(|_| unreachable!("hover ingress is accepted"));
    pump_all(&mut runtime);
    let redraw = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!("new hover membership requests redraw"));
    runtime
        .acknowledge_redraw(&redraw)
        .unwrap_or_else(|_| unreachable!("runtime redraw token remains local"));

    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("interaction-only publication is admitted"));
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Style]
    );
    assert!(runtime.take_redraw_request().is_none());

    runtime
        .submit_pointer(hover)
        .unwrap_or_else(|_| unreachable!("duplicate hover ingress is accepted"));
    pump_all(&mut runtime);
    assert!(
        runtime.take_redraw_request().is_none(),
        "unchanged effective pointer membership must not mint a redraw"
    );
}
