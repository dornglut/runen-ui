use runenui_core::{
    CommandOrigin, IntoEffects, LogicalLength, NoHostProtocol, SemanticCommand, StyleTokens, UiApp,
    View, button,
};
use runenui_runtime::{
    AppRuntime, LogicalPoint, LogicalSize, PumpBudget, RuntimeConfig, SurfaceBuildContext,
    TraceConfig,
};

struct TraceDisabledApp;

impl UiApp for TraceDisabledApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        button("Activate").on_activate(|| {})
    }

    fn update(
        state: &mut Self::State,
        _action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

fn pump_all(runtime: &mut AppRuntime<TraceDisabledApp>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

#[test]
fn disabled_trace_preserves_checked_surface_command_behavior() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(0));
    let mut runtime = AppRuntime::<TraceDisabledApp>::mount_with_config(0, config);
    pump_all(&mut runtime);

    let tokens = StyleTokens::new();
    let size = LogicalSize::new(LogicalLength::from(160_u16), LogicalLength::from(48_u16));
    let build = SurfaceBuildContext::tight(&tokens, size);
    let publication = runtime.publish_surface(&build);
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
