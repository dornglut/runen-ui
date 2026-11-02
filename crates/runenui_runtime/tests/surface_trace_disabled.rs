use runenui_core::{
    CommandOrigin, LogicalLength, NoHostProtocol, SemanticCommand, StyleEnvironment, UiApp, View,
    button,
};
use runenui_runtime::{
    AppRuntime, LogicalPoint, LogicalSize, PumpBudget, RuntimeConfig, SurfaceBuildContext,
    TraceConfig,
};

struct TraceDisabledApp;

impl UiApp for TraceDisabledApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One button owner, without initial outputs or work.
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
        // Button event/activation output and its pure application turn.
        runenui_core::ProducerBudget::new(5, 5)
            .with_application_outputs(0)
            .with_reconciliation(1, 1, 1)
    }

    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        button("Activate").on_activate(|| {})
    }

    fn update(
        state: &mut Self::State,
        _action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

fn pump_all(runtime: &mut AppRuntime<TraceDisabledApp>) {
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
}

#[test]
#[allow(clippy::assert_is_empty)]
fn disabled_trace_preserves_checked_surface_command_behavior() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(0));
    let mut runtime = AppRuntime::<TraceDisabledApp>::mount_with_config(0, config)
        .unwrap_or_else(|_| unreachable!("trace-disabled fixture admission is valid"));
    pump_all(&mut runtime);

    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::new(LogicalLength::from(160_u16), LogicalLength::from(48_u16));
    let build = SurfaceBuildContext::tight(&style_environment, size);
    let publication = runtime
        .publish_surface(&build)
        .unwrap_or_else(|_| unreachable!("trace-disabled publication is admitted"));
    let root = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("button root is published"));
    let bounds = root.bounds();
    let point = LogicalPoint::new(
        bounds.width().mul_add(0.5, bounds.x()),
        bounds.height().mul_add(0.5, bounds.y()),
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"));

    runtime
        .submit_surface_command(
            publication.input_context().clone(),
            point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("trace-disabled command is accepted"));
    pump_all(&mut runtime);

    assert_eq!(*runtime.state(), 1);
    assert!(runtime.trace().is_empty());
}
