#![allow(refining_impl_trait)]

use runenui_core::{
    Element, ElementId, HitContribution, HitContributionContext, HitRegion, LogicalLength,
    LogicalPoint, LogicalRect, NoHostProtocol, PointerPolicy, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticRole, SemanticState,
    StyleEnvironment, UiApp, View, Widget, WidgetMeasure, column,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

#[derive(Clone, Copy, Debug)]
enum SemanticMode {
    Available,
    Hidden,
    Inert,
}

impl SemanticMode {
    const fn state(self) -> SemanticState {
        match self {
            Self::Available => SemanticState::ENABLED,
            Self::Hidden => SemanticState::ENABLED.with_hidden(true),
            Self::Inert => SemanticState::ENABLED.with_inert(true),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum PhysicalMode {
    Target,
    Omit,
    Block,
}

#[derive(Debug)]
struct PolicyOwner {
    name: &'static str,
    semantic: SemanticMode,
    physical: PhysicalMode,
}

impl Widget<()> for PolicyOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _input: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(10_u16), LogicalLength::from(10_u16))
    }

    fn hit_test(&self, (): &Self::State, _: HitContributionContext) -> HitContribution {
        let rect = LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
            .unwrap_or_else(|_| unreachable!("test hit rectangle is valid"));
        match self.physical {
            PhysicalMode::Target => HitContribution::single_rect(rect),
            PhysicalMode::Omit => HitContribution::empty(),
            PhysicalMode::Block => HitContribution::new(vec![
                HitRegion::rect(rect).with_pointer_policy(PointerPolicy::Block),
            ]),
        }
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_name(self.name)
                .with_state(self.semantic.state()),
        )
    }
}

struct App;

impl UiApp for App {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(vec![
            Element::new(PolicyOwner {
                name: "semantic-hidden-target",
                semantic: SemanticMode::Hidden,
                physical: PhysicalMode::Target,
            })
            .id("semantic-hidden-target")
            .key("semantic-hidden-target"),
            Element::new(PolicyOwner {
                name: "semantic-inert-target",
                semantic: SemanticMode::Inert,
                physical: PhysicalMode::Target,
            })
            .id("semantic-inert-target")
            .key("semantic-inert-target"),
            Element::new(PolicyOwner {
                name: "physical-omitted",
                semantic: SemanticMode::Available,
                physical: PhysicalMode::Omit,
            })
            .id("physical-omitted")
            .key("physical-omitted"),
            Element::new(PolicyOwner {
                name: "physical-block",
                semantic: SemanticMode::Available,
                physical: PhysicalMode::Block,
            })
            .id("physical-block")
            .key("physical-block"),
        ])
        .key("root")
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn authored(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("test authored id is valid"))
}

fn center(rect: LogicalRect) -> LogicalPoint {
    LogicalPoint::new(
        rect.x() + rect.width() / 2.0,
        rect.y() + rect.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("published logical bounds are finite"))
}

#[test]
fn semantic_hidden_and_inert_do_not_lower_hit_policy_but_explicit_omission_and_block_do() {
    let mut runtime = AppRuntime::<App>::mount(());
    let style_environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("hit-policy publication is admitted"));

    let node = |id: &str| {
        publication
            .frame()
            .nodes()
            .iter()
            .find(|node| node.authored_id() == Some(&authored(id)))
            .unwrap_or_else(|| unreachable!("policy owner is published"))
    };
    let hidden = node("semantic-hidden-target");
    let inert = node("semantic-inert-target");
    let omitted = node("physical-omitted");
    let blocked = node("physical-block");
    let scene = publication.hit_test_scene();

    assert_eq!(scene.target_at(center(hidden.bounds())), Some(hidden.id()));
    assert_eq!(scene.target_at(center(inert.bounds())), Some(inert.id()));
    assert_eq!(scene.target_at(center(omitted.bounds())), None);
    assert_eq!(scene.target_at(center(blocked.bounds())), None);
    assert!(
        scene
            .regions()
            .iter()
            .all(|region| region.target() != omitted.id())
    );
    assert!(scene.regions().iter().any(|region| {
        region.target() == blocked.id() && region.pointer_policy() == PointerPolicy::Block
    }));

    let snapshot = publication.semantic_publication().snapshot();
    assert!(
        snapshot
            .nodes()
            .iter()
            .all(|node| node.name() != Some("semantic-hidden-target"))
    );
    let semantic = |name: &str| {
        snapshot
            .nodes()
            .iter()
            .find(|node| node.name() == Some(name))
            .unwrap_or_else(|| unreachable!("visible policy semantics are published"))
    };
    assert!(semantic("semantic-inert-target").state().inert());
    assert!(!semantic("physical-omitted").state().inert());
    assert!(!semantic("physical-block").state().inert());
}
