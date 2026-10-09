#![allow(refining_impl_trait)]

use runenui_core::{
    Element, NoHostProtocol, SemanticContribution, SemanticContributionContext,
    SemanticNodeContribution, SemanticNumber, SemanticRange, SemanticRole, UiApp, View, Widget,
    column, progress,
};
use runenui_testing::TestHarness;

fn number(value: f64) -> SemanticNumber {
    SemanticNumber::new(value).unwrap_or_else(|_| unreachable!("finite fixture"))
}

struct PublicProgress {
    range: SemanticRange,
}
impl Widget<()> for PublicProgress {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn semantics(
        &self,
        (): &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Progress)
                .with_name("Custom download")
                .with_range(self.range.clone()),
        )
    }
}

struct App;
impl UiApp for App {
    type State = Option<SemanticNumber>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(current: &Self::State) -> impl View<Self::Action> {
        let range = SemanticRange::new(Some(number(0.0)), Some(number(10.0)), *current)
            .unwrap_or_else(|_| unreachable!("valid range"));
        column([
            progress(number(0.0), number(10.0), *current)
                .unwrap_or_else(|_| unreachable!("valid progress"))
                .accessible_name("Standard download")
                .id("standard")
                .into_element(),
            Element::new(PublicProgress { range })
                .id("custom")
                .into_element(),
        ])
    }

    fn update(_: &mut Self::State, _: Self::Action) {}
}

#[test]
fn public_progress_matches_downstream_widget_semantics_without_implicit_actions() {
    for value in [Some(number(4.0)), None] {
        let mut harness = TestHarness::<App>::mount(value);
        assert!(harness.publish().is_ok());
        let snapshot = harness.semantic_snapshot().unwrap_or_else(|_| unreachable!("published"));
        let facts = ["Standard download", "Custom download"].map(|name| {
            let node = snapshot.nodes().iter()
                .find(|node| node.role() == SemanticRole::Progress && node.name() == Some(name))
                .unwrap_or_else(|| unreachable!("each authored Progress published"));
            assert!(node.supported_actions().is_empty());
            assert_eq!(node.state().expanded(), None);
            node.range().cloned().unwrap_or_else(|| unreachable!("typed range published"))
        });
        assert_eq!(facts[0], facts[1]);
        assert_eq!(facts[0].current(), value);
    }
}
