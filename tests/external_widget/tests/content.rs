use runenui_core::StyleEnvironment;
use runenui_core::{
    Element, ImageDescriptor, ImageIntrinsicSize, ImageMapping, ImagePaintDescriptor,
    LogicalLength, LogicalRect, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, ResourceKind, ResourceRef, SemanticContribution,
    SemanticContributionContext, SemanticNodeContribution, SemanticOrientation, SemanticRole,
    UiApp, View, Widget, WidgetMeasure, WidgetMeasureInput, column, image, separator,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

#[derive(Debug)]
struct DownstreamSeparator;

impl Widget<()> for DownstreamSeparator {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::ZERO, LogicalLength::from(1_u16))
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Separator)
                .with_orientation(SemanticOrientation::Horizontal),
        )
    }
}

#[derive(Debug)]
struct DownstreamImage {
    descriptor: ImageDescriptor,
}

impl Widget<()> for DownstreamImage {
    type State = ImageDescriptor;

    fn create_state(&self) -> Self::State {
        self.descriptor.clone()
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "logical intrinsic dimensions use finite f32, exact pixels remain in the descriptor"
    )]
    fn measure(&self, state: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        let intrinsic = state.intrinsic_size();
        WidgetMeasure::measured(
            LogicalLength::new(intrinsic.width() as f32)
                .unwrap_or_else(|_| unreachable!("pixel extent is finite")),
            LogicalLength::new(intrinsic.height() as f32)
                .unwrap_or_else(|_| unreachable!("pixel extent is finite")),
        )
    }

    fn paint(&self, state: &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        let destination = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("final size is finite"));
        let policy = ImagePaintDescriptor::new(state.clone(), destination, ImageMapping::default())
            .unwrap_or_else(|_| unreachable!("default image mapping is valid"));
        PaintContribution::single(PaintContributionItem::image(policy))
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Image).with_name("Downstream image"),
        )
    }
}

struct ContentParityApp;

impl UiApp for ContentParityApp {
    type State = ImageDescriptor;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(descriptor: &Self::State) -> impl View<Self::Action> {
        let children: Vec<Element<()>> = vec![
            separator(SemanticOrientation::Horizontal)
                .id("standard.separator")
                .into_element(),
            Element::new(DownstreamSeparator).id("custom.separator"),
            image(descriptor.clone())
                .alt_text("Standard image")
                .id("standard.image")
                .into_element(),
            Element::new(DownstreamImage {
                descriptor: descriptor.clone(),
            })
            .id("custom.image"),
        ];
        column(children)
    }

    fn update(
        _: &mut Self::State,
        _: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn downstream_standard_content_parity_requires_no_builtin_runtime_path() {
    let resource = ResourceRef::new(ResourceKind::Image);
    let intrinsic =
        ImageIntrinsicSize::new(24, 12).unwrap_or_else(|| unreachable!("fixture pixels nonzero"));
    let descriptor = ImageDescriptor::new(resource.clone(), intrinsic)
        .unwrap_or_else(|_| unreachable!("fixture resource image-kind"));
    let mut runtime = AppRuntime::<ContentParityApp>::mount(descriptor);
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("downstream content publishes"));
    let nodes = publication.semantic_publication().snapshot().nodes();
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.role() == SemanticRole::Separator)
            .count(),
        2
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.role() == SemanticRole::Image)
            .count(),
        2
    );
    let images = publication
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| item.primitive().as_image())
        .collect::<Vec<_>>();
    assert_eq!(images.len(), 2);
    for image in images {
        assert_eq!(image.resource_ref(), &resource);
        assert_eq!(image.resolved_intrinsic_size(), Some(intrinsic));
    }
}
