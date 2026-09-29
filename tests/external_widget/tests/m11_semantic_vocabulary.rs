use runenui_core::{
    Element, NoHostProtocol, SemanticAutocomplete, SemanticCollectionPosition,
    SemanticContribution, SemanticContributionContext, SemanticInvalidState, SemanticItem,
    SemanticKey, SemanticNodeContribution, SemanticNumber, SemanticOrientation, SemanticPopupKind,
    SemanticRange, SemanticReference, SemanticRelationship, SemanticRelationshipKind, SemanticRole,
    SemanticSelectionMode, SemanticState, StyleEnvironment, UiApp, View, Widget,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

#[derive(Debug)]
struct TypedSemanticProbe;

impl Widget<()> for TypedSemanticProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let combo_key = SemanticKey::from_static("combo")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let listbox_key = SemanticKey::from_static("listbox")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let option_key = SemanticKey::from_static("option")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let error_key = SemanticKey::from_static("error")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));
        let slider_key = SemanticKey::from_static("slider")
            .unwrap_or_else(|_| unreachable!("static semantic key is valid"));

        let combo = SemanticNodeContribution::new(combo_key, SemanticRole::ComboBox)
            .with_name("Choice")
            .with_state(
                SemanticState::ENABLED
                    .with_expanded(true)
                    .with_required(true)
                    .with_invalid(SemanticInvalidState::Grammar),
            )
            .with_popup(SemanticPopupKind::ListBox)
            .with_placeholder("Filter choices")
            .with_autocomplete(SemanticAutocomplete::List)
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::Controls,
                SemanticReference::Local(listbox_key.clone()),
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ActiveDescendant,
                SemanticReference::Local(option_key.clone()),
            ))
            .with_relationship(SemanticRelationship::new(
                SemanticRelationshipKind::ErrorMessage,
                SemanticReference::Local(error_key.clone()),
            ));

        let option = SemanticNodeContribution::new(option_key, SemanticRole::Option)
            .with_name("One")
            .with_state(SemanticState::ENABLED.with_selected(true))
            .with_collection_position(
                SemanticCollectionPosition::new(0, Some(1))
                    .unwrap_or_else(|_| unreachable!("controlled collection position is valid")),
            );
        let listbox = SemanticNodeContribution::new(listbox_key, SemanticRole::ListBox)
            .with_orientation(SemanticOrientation::Vertical)
            .with_selection_mode(SemanticSelectionMode::Single)
            .with_child(option);

        let minimum = SemanticNumber::new(0.0)
            .unwrap_or_else(|_| unreachable!("controlled range minimum is finite"));
        let maximum = SemanticNumber::new(10.0)
            .unwrap_or_else(|_| unreachable!("controlled range maximum is finite"));
        let current = SemanticNumber::new(4.0)
            .unwrap_or_else(|_| unreachable!("controlled range value is finite"));
        let slider = SemanticNodeContribution::new(slider_key, SemanticRole::Slider)
            .with_orientation(SemanticOrientation::Horizontal)
            .with_range(
                SemanticRange::new(Some(minimum), Some(maximum), Some(current))
                    .unwrap_or_else(|_| unreachable!("controlled range is valid")),
            );

        let error = SemanticNodeContribution::new(error_key, SemanticRole::Text)
            .with_name("Choice is invalid");

        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group).with_children(vec![
                SemanticItem::node(combo),
                SemanticItem::node(listbox),
                SemanticItem::node(slider),
                SemanticItem::node(error),
            ]),
        )
    }
}

struct ProbeApp;

impl UiApp for ProbeApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        Element::new(TypedSemanticProbe)
            .id("typed-semantic-probe")
            .key("typed-semantic-probe")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn downstream_widget_publishes_selected_m11_typed_semantics_through_public_apis() {
    let mut runtime = AppRuntime::<ProbeApp>::mount(());
    let style = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &style,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("downstream semantic publication is admitted"));
    let snapshot = publication.semantic_publication().snapshot();

    let combo = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("downstream ComboBox semantics are published"));
    assert_eq!(combo.state().expanded(), Some(true));
    assert_eq!(combo.state().required(), Some(true));
    assert_eq!(combo.state().invalid(), Some(SemanticInvalidState::Grammar));
    assert_eq!(combo.popup(), Some(SemanticPopupKind::ListBox));
    assert_eq!(combo.placeholder(), Some("Filter choices"));
    assert_eq!(combo.autocomplete(), Some(SemanticAutocomplete::List));
    assert_eq!(combo.relationships().len(), 3);

    let listbox = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ListBox)
        .unwrap_or_else(|| unreachable!("downstream ListBox semantics are published"));
    assert_eq!(listbox.orientation(), Some(SemanticOrientation::Vertical));
    assert_eq!(listbox.selection_mode(), Some(SemanticSelectionMode::Single));

    let option = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Option)
        .unwrap_or_else(|| unreachable!("downstream Option semantics are published"));
    assert_eq!(option.state().selected(), Some(true));
    assert_eq!(
        option.collection_position().map(|position| position.index()),
        Some(0)
    );
    assert_eq!(
        option
            .collection_position()
            .and_then(|position| position.known_size()),
        Some(1)
    );

    let slider = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Slider)
        .unwrap_or_else(|| unreachable!("downstream Slider semantics are published"));
    assert_eq!(slider.orientation(), Some(SemanticOrientation::Horizontal));
    assert_eq!(
        slider.range().and_then(SemanticRange::current),
        SemanticNumber::new(4.0).ok()
    );
}
