use runenui_core::{
    Element, NoHostProtocol, SemanticAction, SemanticContribution, SemanticContributionContext,
    SemanticItem, SemanticKey, SemanticNodeContribution, SemanticRole, SemanticText, SemanticValue,
    StyleTokens, UiApp, View, Widget, column,
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

    fn root((): &Self::State) -> Element<Self::Action> {
        let child: Element<ChildAction> = Element::new(SemanticProbe).id("semantic.child");
        column(vec![child])
            .key("semantic.root")
            .into_element()
            .map_action(|_: ChildAction| MiddleAction)
            .map_action(|_: MiddleAction| AppAction)
    }

    fn update((): &mut Self::State, _: Self::Action) {}
}

#[test]
fn recursive_action_mapping_preserves_semantic_contribution_exactly() {
    let mut runtime = AppRuntime::<MappedSemanticApp>::mount(());
    let tokens = StyleTokens::new();
    let publication = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::unbounded(),
    ));
    let semantic_child = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == "semantic.child")
        })
        .unwrap_or_else(|| unreachable!("mapped semantic child is published"));

    assert_eq!(semantic_child.semantics(), &expected_contribution());
    assert!(matches!(
        semantic_child.semantics().roots(),
        [SemanticItem::Node(_)]
    ));
}
