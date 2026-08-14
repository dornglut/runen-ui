use runenui_core::{
    Element, IntoEffects, LogicalRect, NoHostProtocol, SemanticAction, SemanticBounds,
    SemanticContribution, SemanticContributionContext, SemanticContributionError, SemanticKey,
    SemanticNodeContribution, SemanticRole, SemanticText, SemanticValue, StyleTokens, UiApp, View,
    Widget, WidgetActivation, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SemanticDiagnostic, SemanticOwnerWithdrawalReason,
    SemanticPublication, SurfaceBuildContext,
};

#[derive(Clone, Debug)]
struct ChildAction;

#[derive(Clone, Debug)]
struct MiddleAction;

#[derive(Clone, Debug)]
struct AppAction;

#[derive(Debug)]
struct SemanticProbe;

fn detail_bounds() -> LogicalRect {
    LogicalRect::try_new(4.0, 6.0, 18.0, 10.0)
        .unwrap_or_else(|_| unreachable!("owner-local semantic bounds are valid"))
}

fn expected_contribution() -> SemanticContribution {
    let detail = SemanticKey::from_static("detail")
        .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
    SemanticContribution::single(
        SemanticNodeContribution::primary(SemanticRole::Group)
            .with_name("mapped semantic probe")
            .with_action(SemanticAction::Activate)
            .with_child(
                SemanticNodeContribution::new(detail, SemanticRole::Text)
                    .with_name("detail")
                    .with_value(SemanticValue::Integer(7))
                    .with_bounds(SemanticBounds::OwnerLocal(detail_bounds()))
                    .with_text(SemanticText::plain("mapped detail")),
            ),
    )
}

impl Widget<ChildAction> for SemanticProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        expected_contribution()
    }
}

struct MappedSemanticApp;

impl UiApp for MappedSemanticApp {
    type State = ();
    type Action = AppAction;
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        let child: Element<ChildAction> = Element::new(SemanticProbe).id("semantic.child");
        column(vec![child])
            .key("semantic.root")
            .into_element()
            .map_action(|_: ChildAction| MiddleAction)
            .map_action(|_: MiddleAction| AppAction)
    }

    fn update(
        (): &mut Self::State,
        _: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[derive(Debug)]
struct InvalidSemanticProbe;

impl Widget<()> for InvalidSemanticProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::new(vec![
            runenui_core::SemanticItem::node(SemanticNodeContribution::primary(
                SemanticRole::Group,
            )),
            runenui_core::SemanticItem::node(SemanticNodeContribution::primary(SemanticRole::Text)),
        ])
    }
}

struct InvalidSemanticApp;

impl UiApp for InvalidSemanticApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(InvalidSemanticProbe).id("semantic.invalid")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn published_semantic_child(
    runtime: &mut AppRuntime<MappedSemanticApp>,
) -> (SemanticPublication, LogicalRect) {
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("M5 semantic conformance publication is admitted"));
    let owner_bounds = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "semantic.child")
        })
        .unwrap_or_else(|| unreachable!("mapped semantic child owner is published"))
        .bounds();
    (publication.semantic_publication().clone(), owner_bounds)
}

#[test]
fn recursive_action_mapping_preserves_composed_semantic_product() {
    let mut runtime = AppRuntime::<MappedSemanticApp>::mount(());
    let (semantics, _) = published_semantic_child(&mut runtime);
    let snapshot = semantics.snapshot();
    let primary = snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some("mapped semantic probe"))
        .unwrap_or_else(|| unreachable!("mapped semantic primary is published"));
    assert_eq!(primary.role(), SemanticRole::Group);
    assert!(
        primary
            .supported_actions()
            .contains(&SemanticAction::Activate)
    );

    let detail_id = primary
        .children()
        .first()
        .unwrap_or_else(|| unreachable!("mapped semantic primary retains virtual child"));
    let detail = snapshot
        .node(detail_id)
        .unwrap_or_else(|| unreachable!("mapped semantic detail is published"));
    assert_eq!(detail.role(), SemanticRole::Text);
    assert_eq!(detail.name(), Some("detail"));
    assert_eq!(detail.value(), Some(&SemanticValue::Integer(7)));
    let expected_text = SemanticText::plain("mapped detail");
    assert_eq!(detail.text(), Some(&expected_text));
}

#[test]
fn downstream_widget_owner_local_bounds_publish_as_absolute_semantic_bounds() {
    let mut runtime = AppRuntime::<MappedSemanticApp>::mount(());
    let (semantics, owner_bounds) = published_semantic_child(&mut runtime);
    let detail = semantics
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("detail"))
        .unwrap_or_else(|| unreachable!("semantic detail is published"));
    let local = detail_bounds();
    let expected = LogicalRect::try_new(
        owner_bounds.x() + local.x(),
        owner_bounds.y() + local.y(),
        local.width(),
        local.height(),
    )
    .unwrap_or_else(|_| unreachable!("absolute semantic bounds remain valid"));

    assert_eq!(detail.bounds(), expected);
    assert!(LogicalRect::try_new(f32::NAN, 0.0, 1.0, 1.0).is_err());
    assert!(LogicalRect::try_new(0.0, 0.0, -1.0, 1.0).is_err());
}

#[test]
fn invalid_owner_semantics_publish_typed_fail_closed_diagnostic() {
    let mut runtime = AppRuntime::<InvalidSemanticApp>::mount(());
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| {
            unreachable!("invalid semantic authoring fails closed, not publication")
        });
    let snapshot = publication.semantic_publication().snapshot();
    let report = publication.semantic_diagnostics();

    assert!(snapshot.nodes().is_empty());
    assert_eq!(report.surface_id(), snapshot.surface_id());
    assert_eq!(report.diagnostics().len(), 1);
    match &report.diagnostics()[0] {
        SemanticDiagnostic::OwnerWithdrawn {
            authored_id: Some(authored_id),
            reason:
                SemanticOwnerWithdrawalReason::InvalidContribution(
                    SemanticContributionError::DuplicateKey { key },
                ),
        } => {
            assert_eq!(authored_id.as_str(), "semantic.invalid");
            assert_eq!(key, &SemanticKey::PRIMARY);
        }
        diagnostic => panic!("unexpected semantic diagnostic: {diagnostic:?}"),
    }
}
