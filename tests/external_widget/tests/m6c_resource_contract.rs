#![allow(refining_impl_trait)]

use std::collections::HashMap;

use runenui_core::{
    Color, Element, ImagePrimitive, LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, PaintPrimitive,
    ResourceKind, ResourceRef, ShapedTextRunPrimitive, StyleTokens, UiApp, Widget, WidgetMeasure,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

#[derive(Default)]
struct FixtureProvider {
    refs: HashMap<&'static str, ResourceRef>,
}

impl FixtureProvider {
    fn issue(&mut self, key: &'static str, kind: ResourceKind) -> ResourceRef {
        let reference = ResourceRef::new(kind);
        self.refs.insert(key, reference.clone());
        reference
    }

    fn replace(&mut self, key: &'static str, kind: ResourceKind) -> ResourceRef {
        self.issue(key, kind)
    }
}

fn rect() -> LogicalRect {
    LogicalRect::try_new(2.0, 3.0, 40.0, 50.0)
        .unwrap_or_else(|_| unreachable!("test destination is valid"))
}

fn origin() -> LogicalPoint {
    LogicalPoint::new(7.0, 11.0).unwrap_or_else(|_| unreachable!("test origin is finite"))
}

fn image_point(image: &ImagePrimitive, u: f32, v: f32) -> LogicalPoint {
    let destination = image.destination();
    LogicalPoint::new(
        u.mul_add(destination.width(), destination.x()),
        v.mul_add(destination.height(), destination.y()),
    )
    .unwrap_or_else(|_| unreachable!("test image mapping remains finite"))
}

fn shaped_point(run: &ShapedTextRunPrimitive, x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(run.origin().x() + x, run.origin().y() + y)
        .unwrap_or_else(|_| unreachable!("test shaped-run mapping remains finite"))
}

#[derive(Debug)]
struct ResourceOwner {
    image: ResourceRef,
    shaped: ResourceRef,
}

impl Widget<()> for ResourceOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(80_u16),
            height: LogicalLength::from(80_u16),
        }
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(vec![
            PaintContributionItem::image(self.image.clone(), rect())
                .unwrap_or_else(|_| unreachable!("fixture image ref has image kind")),
            PaintContributionItem::shaped_text_run(
                self.shaped.clone(),
                origin(),
                Color::rgba(10, 20, 30, 255),
            )
            .unwrap_or_else(|_| unreachable!("fixture shaped ref has shaped-run kind")),
            PaintContributionItem::shaped_text_run(
                self.shaped.clone(),
                origin(),
                Color::rgba(30, 20, 10, 255),
            )
            .unwrap_or_else(|_| unreachable!("fixture shaped ref has shaped-run kind")),
        ])
    }
}

#[derive(Debug)]
struct State {
    image: ResourceRef,
    shaped: ResourceRef,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(ResourceOwner {
            image: state.image.clone(),
            shaped: state.shaped.clone(),
        })
        .key("resource-owner")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn opaque_refs_disambiguate_providers_and_resource_primitives_preserve_exact_logical_semantics() {
    let mut provider_a = FixtureProvider::default();
    let mut provider_b = FixtureProvider::default();
    let image_a = provider_a.issue("same-local-key", ResourceKind::Image);
    let image_b = provider_b.issue("same-local-key", ResourceKind::Image);
    let shaped = provider_a.issue("shaped", ResourceKind::ShapedTextRun);

    assert_ne!(image_a, image_b);
    assert_eq!(image_a, image_a.clone());
    let replaced = provider_a.replace("same-local-key", ResourceKind::Image);
    assert_ne!(image_a, replaced);

    assert!(PaintContributionItem::image(shaped.clone(), rect()).is_err());
    assert!(
        PaintContributionItem::shaped_text_run(image_a.clone(), origin(), Color::WHITE,).is_err()
    );

    let mut runtime = AppRuntime::<App>::mount(State {
        image: image_a.clone(),
        shaped: shaped.clone(),
    });
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("resource publication is admitted"));
    let items = publication.paint_scene().items();
    assert_eq!(items.len(), 3);

    let image = items[0]
        .primitive()
        .as_image()
        .unwrap_or_else(|| unreachable!("first primitive is image"));
    assert_eq!(image.resource_ref(), &image_a);
    assert_eq!(image.destination(), rect());
    let mapped_origin = image_point(image, 0.0, 0.0);
    let mapped_far = image_point(image, 1.0, 1.0);
    assert_eq!((mapped_origin.x(), mapped_origin.y()), (2.0, 3.0));
    assert_eq!((mapped_far.x(), mapped_far.y()), (42.0, 53.0));

    let first_run = items[1]
        .primitive()
        .as_shaped_text_run()
        .unwrap_or_else(|| unreachable!("second primitive is shaped run"));
    let second_run = items[2]
        .primitive()
        .as_shaped_text_run()
        .unwrap_or_else(|| unreachable!("third primitive is shaped run"));
    assert_eq!(first_run.resource_ref(), &shaped);
    assert_eq!(second_run.resource_ref(), &shaped);
    assert_ne!(first_run.foreground(), second_run.foreground());
    assert_ne!(items[1].primitive(), items[2].primitive());
    let mapped_local = shaped_point(first_run, 3.0, 5.0);
    assert_eq!(
        (mapped_local.x(), mapped_local.y()),
        (origin().x() + 3.0, origin().y() + 5.0)
    );
    assert!(matches!(items[0].primitive(), PaintPrimitive::Image(_)));
    assert!(matches!(
        items[1].primitive(),
        PaintPrimitive::ShapedTextRun(_)
    ));
}
