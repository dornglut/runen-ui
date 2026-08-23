#![allow(refining_impl_trait)]

use std::collections::HashMap;

use runenui_core::{
    Color, Element, LogicalLength, LogicalPoint, LogicalRect, NoHostProtocol, PaintContribution,
    PaintContributionContext, PaintContributionItem, ResourceKind, ResourceRef, StyleTokens, UiApp,
    Widget, WidgetMeasure,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SceneCapabilities, SurfaceBuildContext,
    UnsupportedSceneRequirement,
};

fn rect() -> LogicalRect {
    LogicalRect::try_new(0.0, 0.0, 10.0, 10.0)
        .unwrap_or_else(|_| unreachable!("test destination is valid"))
}

fn origin() -> LogicalPoint {
    LogicalPoint::new(1.0, 2.0).unwrap_or_else(|_| unreachable!("test origin is finite"))
}

#[derive(Debug)]
struct RequirementsOwner {
    image: ResourceRef,
    shaped: ResourceRef,
}

impl Widget<()> for RequirementsOwner {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(20_u16),
            height: LogicalLength::from(20_u16),
        }
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(vec![
            PaintContributionItem::shaped_text_run(self.shaped.clone(), origin(), Color::WHITE)
                .unwrap_or_else(|_| unreachable!("fixture shaped ref has shaped-run kind")),
            PaintContributionItem::fill_rect(rect(), Color::BLACK),
            PaintContributionItem::image(self.image.clone(), rect())
                .unwrap_or_else(|_| unreachable!("fixture image ref has image kind")),
            PaintContributionItem::shaped_text_run(self.shaped.clone(), origin(), Color::BLACK)
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
        Element::new(RequirementsOwner {
            image: state.image.clone(),
            shaped: state.shaped.clone(),
        })
        .key("requirements-owner")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn requirements_are_derived_canonically_and_capabilities_never_rewrite_the_scene() {
    let image = ResourceRef::new(ResourceKind::Image);
    let shaped = ResourceRef::new(ResourceKind::ShapedTextRun);
    let mut runtime = AppRuntime::<App>::mount(State {
        image: image.clone(),
        shaped: shaped.clone(),
    });
    let tokens = StyleTokens::new();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &tokens,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("requirements publication is admitted"));
    let before_checks = publication.paint_scene().clone();
    let requirements = publication.paint_scene().requirements();

    assert_eq!(
        requirements.resource_kinds(),
        &[ResourceKind::Image, ResourceKind::ShapedTextRun]
    );
    assert_eq!(publication.paint_scene().requirements(), requirements);

    let Err(unsupported_image) = SceneCapabilities::default().check_requirements(&requirements)
    else {
        unreachable!("consumer without resource support rejects image requirement first");
    };
    assert_eq!(unsupported_image.resource_kind(), ResourceKind::Image);
    assert_eq!(unsupported_image.code(), UnsupportedSceneRequirement::CODE);

    let image_only = SceneCapabilities::new([ResourceKind::Image]);
    let Err(unsupported_shaped) = image_only.check_requirements(&requirements) else {
        unreachable!("image-only consumer rejects shaped-run requirement");
    };
    assert_eq!(
        unsupported_shaped.resource_kind(),
        ResourceKind::ShapedTextRun
    );

    let complete = SceneCapabilities::new([
        ResourceKind::ShapedTextRun,
        ResourceKind::Image,
        ResourceKind::ShapedTextRun,
    ]);
    assert_eq!(
        complete.resource_kinds(),
        &[ResourceKind::Image, ResourceKind::ShapedTextRun]
    );
    complete
        .check_requirements(&requirements)
        .unwrap_or_else(|_| unreachable!("complete consumer capabilities are sufficient"));
    assert_eq!(publication.paint_scene(), &before_checks);

    let payloads = HashMap::from([(image, "fixture-image"), (shaped, "fixture-shaped-run")]);
    for resource in publication
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| item.primitive().resource_ref())
    {
        assert!(payloads.contains_key(resource));
    }
}
