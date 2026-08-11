use runenui_core::{
    Element, IntoEffects, LogicalRect, NoHostProtocol, SemanticAction, SemanticBounds,
    SemanticContribution, SemanticContributionContext, SemanticKey, SemanticNodeContribution,
    SemanticRole, SemanticText, SemanticValue, StyleTokens, UiApp, View, Widget, column,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

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

fn published_semantic_child(runtime: &mut AppRuntime<MappedSemanticApp>) -> SemanticContribution {
    let tokens = StyleTokens::new();
    let publication = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::unbounded(),
    ));
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "semantic.child")
        })
        .unwrap_or_else(|| unreachable!("mapped semantic child is published"))
        .semantics()
        .clone()
}

#[test]
fn recursive_action_mapping_preserves_semantic_contribution_exactly() {
    let mut runtime = AppRuntime::<MappedSemanticApp>::mount(());
    let semantics = published_semantic_child(&mut runtime);

    assert_eq!(semantics, expected_contribution());
    assert!(
        semantics
            .roots()
            .first()
            .is_some_and(|item| item.as_node().is_some())
    );
}

#[test]
fn downstream_widget_authors_validated_owner_local_semantic_bounds() {
    let mut runtime = AppRuntime::<MappedSemanticApp>::mount(());
    let semantics = published_semantic_child(&mut runtime);
    let primary = semantics
        .roots()
        .first()
        .and_then(|item| item.as_node())
        .unwrap_or_else(|| unreachable!("semantic probe has a primary node"));
    let detail = primary
        .children()
        .first()
        .and_then(|item| item.as_node())
        .unwrap_or_else(|| unreachable!("semantic probe has a virtual detail node"));

    assert_eq!(
        detail.bounds(),
        SemanticBounds::OwnerLocal(detail_bounds())
    );
    assert!(LogicalRect::try_new(f32::NAN, 0.0, 1.0, 1.0).is_err());
    assert!(LogicalRect::try_new(0.0, 0.0, -1.0, 1.0).is_err());
}
