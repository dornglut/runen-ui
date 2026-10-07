use runenui_core::{
    Element, ElementId, ImageDescriptor, ImageFit, ImageIntrinsicSize, NoHostProtocol,
    ResourceKind, ResourceRef, SemanticOrientation, SemanticRole, UiApp, View,
    image, separator, column,
};
use runenui_testing::{SemanticQuery, TestHarness};

#[derive(Clone, Debug)]
struct State {
    descriptor: ImageDescriptor,
    meaningful: bool,
}

struct ContentApp;

impl UiApp for ContentApp {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut picture = image(state.descriptor.clone()).fit(ImageFit::Contain);
        if state.meaningful {
            picture = picture.alt_text("Overview").description("A production resource");
        }
        let children: Vec<Element<()>> = vec![
            separator(SemanticOrientation::Horizontal)
                .id("separator.horizontal")
                .into_element(),
            separator(SemanticOrientation::Vertical)
                .id("separator.vertical")
                .into_element(),
            picture.id("content.image").into_element(),
        ];
        column(children)
    }

    fn update(
        _: &mut Self::State,
        _: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {}
}

fn state(meaningful: bool) -> (State, ResourceRef) {
    let resource = ResourceRef::new(ResourceKind::Image);
    let intrinsic = ImageIntrinsicSize::new(48, 24)
        .unwrap_or_else(|| unreachable!("fixture size is positive"));
    let descriptor = ImageDescriptor::new(resource.clone(), intrinsic)
        .unwrap_or_else(|_| unreachable!("fixture resource is image-kind"));
    (State { descriptor, meaningful }, resource)
}

#[test]
fn semantic_separator_and_meaningful_image_publish_via_ordinary_runtime() {
    let (state, resource) = state(true);
    let mut harness = TestHarness::<ContentApp>::mount(state);
    let publication = harness.publish()
        .unwrap_or_else(|_| unreachable!("content publication succeeds"));
    for orientation in [SemanticOrientation::Horizontal, SemanticOrientation::Vertical] {
        let nodes = publication.semantic_publication().snapshot().nodes();
        assert!(nodes.iter().any(|node|
            node.role() == SemanticRole::Separator && node.orientation() == Some(orientation)
        ));
    }
    let query = SemanticQuery::new().with_role(SemanticRole::Image).with_name("Overview");
    assert!(harness.unique_semantic_target(&query).is_ok());

    let image_node_id = ElementId::new("content.image")
        .unwrap_or_else(|_| unreachable!("fixture element id"));
    let publication = harness.publication().unwrap_or_else(|| unreachable!("published"));
    let node = publication.frame().nodes().iter()
        .find(|node| node.authored_id() == Some(&image_node_id))
        .unwrap_or_else(|| unreachable!("image remains mounted in ordinary frame"));
    assert!(node.bounds().width() > 0.0);
    assert!(node.bounds().height() > 0.0);
    let painted = publication.paint_scene().items().iter()
        .find_map(|item| item.primitive().as_image())
        .unwrap_or_else(|| unreachable!("public Image contributes ordinary M9 image paint"));
    assert_eq!(painted.resource_ref(), &resource);
    assert_eq!(painted.resolved_intrinsic_size(),
        ImageIntrinsicSize::new(48, 24));
}

#[test]
fn decorative_image_remains_painted_without_a_false_semantic_node() {
    let (state, resource) = state(false);
    let mut harness = TestHarness::<ContentApp>::mount(state);
    let publication = harness.publish()
        .unwrap_or_else(|_| unreachable!("decorative image publishes"));
    assert!(publication.semantic_publication().snapshot().nodes().iter()
        .all(|node| node.role() != SemanticRole::Image));
    let painted = publication.paint_scene().items().iter()
        .find_map(|item| item.primitive().as_image())
        .unwrap_or_else(|| unreachable!("decorative image still paints"));
    assert_eq!(painted.resource_ref(), &resource);
}
