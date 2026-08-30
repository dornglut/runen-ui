use runenui_core::{
    CommandOrigin, Element, ElementId, IntoEffects, LogicalLength, NoHostProtocol, SemanticCommand,
    StyleEnvironment, StyleTokens, UiApp, View, button, row,
};
use runenui_runtime::{
    AppRuntime, CommandSubmission, LogicalPoint, LogicalSize, MountedNodeId, PumpBudget,
    RuntimeConfig, SubmitSurfaceCommandError, SurfaceBuildContext, SurfaceInputContext,
    SurfacePublication, TraceRecord, TraceRecordKind, TraceSurfaceIngressKind,
    TraceSurfaceRejection,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceState {
    pub swapped: bool,
    pub show_primary: bool,
    pub show_extra: bool,
    pub primary_activations: usize,
    pub secondary_activations: usize,
    pub extra_activations: usize,
}

impl SurfaceState {
    const fn new() -> Self {
        Self {
            swapped: false,
            show_primary: true,
            show_extra: false,
            primary_activations: 0,
            secondary_activations: 0,
            extra_activations: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceAction {
    ActivatePrimary,
    ActivateSecondary,
    ActivateExtra,
    Swap,
    ShowExtra,
    HidePrimary,
}

pub struct SurfaceApp;

impl UiApp for SurfaceApp {
    type State = SurfaceState;
    type Action = SurfaceAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        surface_root(state)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            SurfaceAction::ActivatePrimary => state.primary_activations += 1,
            SurfaceAction::ActivateSecondary => state.secondary_activations += 1,
            SurfaceAction::ActivateExtra => state.extra_activations += 1,
            SurfaceAction::Swap => state.swapped = true,
            SurfaceAction::ShowExtra => state.show_extra = true,
            SurfaceAction::HidePrimary => state.show_primary = false,
        }
    }
}

fn primary_button() -> Element<SurfaceAction> {
    button("Alpha")
        .id("surface.primary")
        .key("surface.primary")
        .on_activate(|| SurfaceAction::ActivatePrimary)
        .into_element()
}

fn secondary_button() -> Element<SurfaceAction> {
    button("Bravo")
        .id("surface.secondary")
        .key("surface.secondary")
        .on_activate(|| SurfaceAction::ActivateSecondary)
        .into_element()
}

fn extra_button() -> Element<SurfaceAction> {
    button("Extra")
        .id("surface.extra")
        .key("surface.extra")
        .on_activate(|| SurfaceAction::ActivateExtra)
        .into_element()
}

fn surface_root(state: &SurfaceState) -> Element<SurfaceAction> {
    let mut children = Vec::new();
    if state.swapped {
        children.push(secondary_button());
        if state.show_primary {
            children.push(primary_button());
        }
    } else {
        if state.show_primary {
            children.push(primary_button());
        }
        children.push(secondary_button());
    }
    if state.show_extra {
        children.push(extra_button());
    }
    row(children)
        .id("surface.root")
        .key("surface.root")
        .gap(8_u16)
        .into_element()
}

fn surface_size() -> LogicalSize {
    LogicalSize::new(LogicalLength::from(320_u16), LogicalLength::from(80_u16))
}

pub fn rejected(
    result: Result<CommandSubmission, SubmitSurfaceCommandError>,
    message: &str,
) -> SubmitSurfaceCommandError {
    match result {
        Ok(_) => unreachable!("{message}"),
        Err(error) => error,
    }
}

pub fn pump_all(runtime: &mut AppRuntime<SurfaceApp>) {
    let _ = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
}

pub fn mounted_with(config: RuntimeConfig) -> AppRuntime<SurfaceApp> {
    let mut runtime = AppRuntime::<SurfaceApp>::mount_with_config(SurfaceState::new(), config);
    pump_all(&mut runtime);
    runtime
}

pub fn mounted() -> AppRuntime<SurfaceApp> {
    mounted_with(RuntimeConfig::default())
}

pub fn publication(
    runtime: &mut AppRuntime<SurfaceApp>,
    tokens: &StyleTokens,
) -> SurfacePublication {
    let environment = StyleEnvironment::from_tokens(tokens.clone());
    let context = SurfaceBuildContext::tight(&environment, surface_size());
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("surface context publication is admitted"))
}

pub fn authored_target(publication: &SurfacePublication, authored: &str) -> MountedNodeId {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored target is published"))
        .id()
        .clone()
}

pub fn mounted_target(runtime: &mut AppRuntime<SurfaceApp>, authored: &str) -> MountedNodeId {
    let authored = ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("authored target is mounted"))
        .id()
        .clone()
}

pub fn authored_center(publication: &SurfacePublication, authored: &str) -> LogicalPoint {
    let node = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == authored))
        .unwrap_or_else(|| unreachable!("authored target is published"));
    let bounds = node.bounds();
    LogicalPoint::new(
        bounds.width().mul_add(0.5, bounds.x()),
        bounds.height().mul_add(0.5, bounds.y()),
    )
    .unwrap_or_else(|_| unreachable!("published bounds are finite"))
}

pub fn activate_point(
    runtime: &mut AppRuntime<SurfaceApp>,
    context: SurfaceInputContext,
    point: LogicalPoint,
) {
    runtime
        .submit_surface_command(
            context,
            point,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("checked logical command is accepted"));
    pump_all(runtime);
}

pub fn activate_target(
    runtime: &mut AppRuntime<SurfaceApp>,
    context: SurfaceInputContext,
    target: MountedNodeId,
) {
    runtime
        .submit_resolved_surface_command(
            context,
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("checked resolved command is accepted"));
    pump_all(runtime);
}

pub fn trace_record(
    runtime: &AppRuntime<SurfaceApp>,
    sequence: runenui_runtime::WorkSequence,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &TraceRecord {
    runtime
        .trace()
        .records()
        .find(|record| record.work_sequence() == Some(sequence) && predicate(record.kind()))
        .unwrap_or_else(|| unreachable!("required causal trace record is retained"))
}

pub fn has_rejection(
    runtime: &AppRuntime<SurfaceApp>,
    ingress: TraceSurfaceIngressKind,
    outcome: TraceSurfaceRejection,
) -> bool {
    runtime.trace().kinds().any(|kind| {
        matches!(
            kind,
            TraceRecordKind::SurfaceCommandRejected {
                ingress: recorded_ingress,
                outcome: recorded_outcome,
            } if *recorded_ingress == ingress && *recorded_outcome == outcome
        )
    })
}
