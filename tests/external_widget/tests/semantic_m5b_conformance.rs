use runenui_core::{
    Element, IntoEffects, NoHostProtocol, SemanticAction, SemanticContribution,
    SemanticContributionContext, SemanticKey, SemanticNodeContribution, SemanticRole,
    SemanticState, StyleTokens, UiApp, View, Widget, WidgetActivation, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SemanticNode, SemanticSnapshot, SurfaceBuildContext,
};

fn all_m5_actions(node: SemanticNodeContribution) -> SemanticNodeContribution {
    node.with_action(SemanticAction::Activate)
        .with_action(SemanticAction::RequestFocus)
        .with_action(SemanticAction::OpenMenu)
        .with_action(SemanticAction::OpenContextMenu)
}

#[derive(Debug)]
struct SupportProbe {
    prefix: &'static str,
    activation: WidgetActivation,
    inert: bool,
}

impl Widget<()> for SupportProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        self.activation
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let named = SemanticKey::from_static("named")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let primary = all_m5_actions(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name(format!("{}-primary", self.prefix))
                .with_state(SemanticState::ENABLED.with_inert(self.inert)),
        )
        .with_child(
            all_m5_actions(SemanticNodeContribution::new(named, SemanticRole::Button))
                .with_name(format!("{}-named", self.prefix)),
        );
        SemanticContribution::single(primary)
    }
}

struct SupportApp;

impl UiApp for SupportApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        column(vec![
            Element::new(SupportProbe {
                prefix: "auto-actionable",
                activation: WidgetActivation::actionable(true),
                inert: false,
            })
            .id("support.auto-actionable")
            .key("auto-actionable"),
            Element::new(SupportProbe {
                prefix: "auto-passive",
                activation: WidgetActivation::NONE,
                inert: false,
            })
            .id("support.auto-passive")
            .key("auto-passive"),
            Element::new(SupportProbe {
                prefix: "explicit-focus",
                activation: WidgetActivation::NONE,
                inert: false,
            })
            .id("support.explicit-focus")
            .key("explicit-focus")
            .focusable(true),
            Element::new(SupportProbe {
                prefix: "disabled-actionable",
                activation: WidgetActivation::actionable(false),
                inert: true,
            })
            .id("support.disabled-actionable")
            .key("disabled-actionable"),
        ])
        .key("support.root")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn named_node<'a>(snapshot: &'a SemanticSnapshot, name: &str) -> &'a SemanticNode {
    snapshot
        .nodes()
        .iter()
        .find(|node| node.name() == Some(name))
        .unwrap_or_else(|| unreachable!("named semantic conformance node is published"))
}

#[test]
fn public_support_matrix_separates_support_from_current_availability() {
    let mut runtime = AppRuntime::<SupportApp>::mount(());
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("support conformance publication is admitted"));
    let snapshot = publication.semantic_publication().snapshot();

    assert_eq!(
        named_node(snapshot, "auto-actionable-primary").supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::RequestFocus,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    assert_eq!(
        named_node(snapshot, "auto-actionable-named").supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    assert_eq!(
        named_node(snapshot, "auto-passive-primary").supported_actions(),
        &[SemanticAction::OpenMenu, SemanticAction::OpenContextMenu]
    );
    assert_eq!(
        named_node(snapshot, "auto-passive-named").supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    assert_eq!(
        named_node(snapshot, "explicit-focus-primary").supported_actions(),
        &[
            SemanticAction::RequestFocus,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );

    let disabled = named_node(snapshot, "disabled-actionable-primary");
    assert!(disabled.state().disabled());
    assert!(disabled.state().inert());
    assert_eq!(
        disabled.supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::RequestFocus,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
    let disabled_named = named_node(snapshot, "disabled-actionable-named");
    assert!(disabled_named.state().disabled());
    assert_eq!(
        disabled_named.supported_actions(),
        &[
            SemanticAction::Activate,
            SemanticAction::OpenMenu,
            SemanticAction::OpenContextMenu,
        ]
    );
}
