use core::num::NonZeroUsize;

use runenui_core::{
    CommandOrigin, Element, IntoEffects, LogicalLength, NoHostProtocol, SemanticCommand,
    StyleEnvironment, UiApp, View, button, row,
};
use runenui_runtime::{
    AppRuntime, CommandSubmission, LogicalPoint, LogicalSize, PumpBudget, RuntimeConfig,
    SubmitSurfaceCommandError, SubmitSurfaceCommandErrorKind, SurfaceBuildContext,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct RetentionState {
    mode: u8,
    activations: [usize; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RetentionAction {
    NextMode,
    Activate(usize),
}

struct RetentionApp;

impl UiApp for RetentionApp {
    type State = RetentionState;
    type Action = RetentionAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let order = match state.mode {
            0 => [0, 1, 2],
            1 => [1, 2, 0],
            _ => [2, 0, 1],
        };
        let children = order.map(retention_button);
        row(children)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            RetentionAction::NextMode => state.mode = (state.mode + 1) % 3,
            RetentionAction::Activate(index) => state.activations[index] += 1,
        }
    }
}

fn retention_button(index: usize) -> Element<RetentionAction> {
    let label = match index {
        0 => "A",
        1 => "B",
        _ => "C",
    };
    button(label)
        .key(label)
        .on_activate(move || RetentionAction::Activate(index))
        .into_element()
}

fn rejected(
    result: Result<CommandSubmission, SubmitSurfaceCommandError>,
    message: &str,
) -> SubmitSurfaceCommandError {
    match result {
        Ok(_) => unreachable!("{message}"),
        Err(error) => error,
    }
}

fn pump_all(runtime: &mut AppRuntime<RetentionApp>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

fn publish(runtime: &mut AppRuntime<RetentionApp>) -> runenui_runtime::SurfacePublication {
    let style_environment = StyleEnvironment::default();
    let size = LogicalSize::new(LogicalLength::from(240_u16), LogicalLength::from(48_u16));
    let context = SurfaceBuildContext::tight(&style_environment, size);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("retention geometry publication is admitted"))
}

fn first_center(publication: &runenui_runtime::SurfacePublication) -> LogicalPoint {
    let first_child = publication
        .frame()
        .nodes()
        .get(1)
        .unwrap_or_else(|| unreachable!("row publishes its first button after the root"));
    let bounds = first_child.bounds();
    LogicalPoint::new(
        bounds.width().mul_add(0.5, bounds.x()),
        bounds.height().mul_add(0.5, bounds.y()),
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"))
}

fn activate(
    runtime: &mut AppRuntime<RetentionApp>,
    context: runenui_runtime::SurfaceInputContext,
    point: LogicalPoint,
) {
    runtime
        .submit_surface_command(
            context,
            point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("retained context is accepted"));
    pump_all(runtime);
}

#[test]
fn every_configured_retained_generation_uses_its_own_geometry() {
    let retention = NonZeroUsize::new(3).unwrap_or_else(|| unreachable!());
    let config = RuntimeConfig::default().with_surface_snapshot_retention(retention);
    let mut runtime =
        AppRuntime::<RetentionApp>::mount_with_config(RetentionState::default(), config);
    pump_all(&mut runtime);

    let first = publish(&mut runtime);
    let point = first_center(&first);
    let first_context = first.input_context().clone();

    runtime
        .submit_action(RetentionAction::NextMode)
        .unwrap_or_else(|_| unreachable!("mode action is accepted"));
    pump_all(&mut runtime);
    let second = publish(&mut runtime);
    let second_context = second.input_context().clone();

    runtime
        .submit_action(RetentionAction::NextMode)
        .unwrap_or_else(|_| unreachable!("mode action is accepted"));
    pump_all(&mut runtime);
    let third = publish(&mut runtime);
    let third_context = third.input_context().clone();

    activate(&mut runtime, first_context.clone(), point);
    activate(&mut runtime, second_context.clone(), point);
    activate(&mut runtime, third_context, point);
    assert_eq!(runtime.state().activations, [1, 1, 1]);

    runtime
        .submit_action(RetentionAction::NextMode)
        .unwrap_or_else(|_| unreachable!("mode action is accepted"));
    pump_all(&mut runtime);
    let _ = publish(&mut runtime);

    let retired = rejected(
        runtime.submit_surface_command(
            first_context,
            point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        ),
        "expected rejection",
    );
    assert_eq!(
        retired.kind(),
        SubmitSurfaceCommandErrorKind::RetiredSurfaceContext
    );

    activate(&mut runtime, second_context, point);
    assert_eq!(runtime.state().activations, [1, 2, 1]);
}
