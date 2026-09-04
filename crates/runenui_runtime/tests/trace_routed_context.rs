#![allow(refining_impl_trait)]

use runenui_core::{
    ChildBearingWidget, CommandOrigin, Element, EventContext, NoHostProtocol, SemanticCommand,
    UiApp, UiEvent, View, Widget, WidgetActivation, WidgetEventOutput, container,
};
use runenui_runtime::{
    AppRuntime, MountedNodeId, PumpBudget, RuntimeConfig, TraceEventFamily, TraceRecordKind,
};

#[derive(Debug)]
struct RouteWidget {
    actionable: bool,
}

impl Widget<()> for RouteWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        _: &UiEvent,
        _: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        if self.actionable {
            WidgetActivation::actionable(true)
        } else {
            WidgetActivation::NONE
        }
    }
}

impl ChildBearingWidget<()> for RouteWidget {}

struct RouteApp;

impl UiApp for RouteApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        container(
            RouteWidget { actionable: false },
            vec![
                Element::new(RouteWidget { actionable: true })
                    .id("target")
                    .key("target")
                    .focusable(true),
            ],
        )
        .id("root")
        .key("root")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn settle(runtime: &mut AppRuntime<RouteApp>) {
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.is_quiescent(), "fixture did not settle: {report:?}");
}

fn authored_target(runtime: &mut AppRuntime<RouteApp>, authored_id: &str) -> MountedNodeId {
    let authored_id =
        runenui_core::ElementId::new(authored_id).unwrap_or_else(|_| unreachable!("valid id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored_id))
        .unwrap_or_else(|| unreachable!("fixture node is mounted"))
        .id()
        .clone()
}

#[test]
fn semantic_command_retains_event_family_and_exact_ordered_route() {
    let mut runtime = AppRuntime::<RouteApp>::mount_with_config((), RuntimeConfig::default());
    settle(&mut runtime);
    let root = authored_target(&mut runtime, "root");
    let target = authored_target(&mut runtime, "target");
    let retained_before = runtime.trace().len();

    runtime
        .submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("fixture target accepts focus"));
    settle(&mut runtime);

    let records = runtime
        .trace()
        .records()
        .skip(retained_before)
        .collect::<Vec<_>>();
    let started = records
        .iter()
        .find(|record| matches!(record.kind(), TraceRecordKind::RoutedEventStarted))
        .copied()
        .unwrap_or_else(|| unreachable!("routed event start is retained"));
    let snapshot = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::RouteSnapshotCreated { .. })
                && record.causal_parent() == Some(started.sequence())
        })
        .copied()
        .unwrap_or_else(|| unreachable!("causal route snapshot is retained"));

    let event = started
        .context()
        .event()
        .unwrap_or_else(|| unreachable!("started record owns event context"));
    assert_eq!(event.family(), TraceEventFamily::SemanticCommand);
    assert!(event.is_cancelable());
    assert_eq!(snapshot.context().event(), Some(event));
    assert_eq!(snapshot.causal_parent(), Some(started.sequence()));
    assert_eq!(snapshot.work_sequence(), started.work_sequence());
    assert_eq!(snapshot.instant(), started.instant());
    assert_eq!(started.original_target(), Some(&target));
    assert_eq!(snapshot.original_target(), Some(&target));

    let route = snapshot
        .context()
        .route()
        .unwrap_or_else(|| unreachable!("snapshot record owns exact route"));
    assert_eq!(route.related_target(), None);
    assert_eq!(route.targets().len(), 2);
    assert_eq!(route.targets()[0].mounted_node_id(), &root);
    assert_eq!(route.targets()[1].mounted_node_id(), &target);
}
