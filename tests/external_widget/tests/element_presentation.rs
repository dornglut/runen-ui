use runenui_core::{
    Element, LogicalPoint, PresentationOrigin, PresentationRotation, PresentationScale,
    PresentationTransform, PresentationTranslation, PresentationValue, SurfacePresentation,
    SurfacePresentationAnchor, SurfacePresentationPlacement, SurfacePresentationSide, UnitInterval,
    Widget,
};

#[derive(Debug)]
struct ExternalWidget;

impl Widget<()> for ExternalWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}
}

fn presentation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(5.0, 7.0)
            .unwrap_or_else(|_| unreachable!("controlled translation is finite")),
        PresentationScale::IDENTITY,
        PresentationRotation::ZERO,
        PresentationOrigin::new(
            UnitInterval::new(0.5).unwrap_or_else(|_| unreachable!("controlled unit is valid")),
            UnitInterval::new(0.5).unwrap_or_else(|_| unreachable!("controlled unit is valid")),
        ),
    )
}

#[test]
fn downstream_custom_widget_can_author_generic_element_presentation() {
    let expected = presentation();
    let element: Element<()> = Element::new(ExternalWidget).presentation(expected);

    assert_eq!(
        element
            .style()
            .presentation()
            .and_then(PresentationValue::as_literal),
        Some(expected)
    );
}


#[test]
fn downstream_custom_widget_can_author_same_surface_presentation_without_runtime_identity() {
    let anchor = SurfacePresentationAnchor::SurfacePoint(
        LogicalPoint::new(12.0, 18.0)
            .unwrap_or_else(|_| unreachable!("controlled point is finite")),
    );
    let first = SurfacePresentationPlacement::new(SurfacePresentationSide::Bottom);
    let fallback = SurfacePresentationPlacement::new(SurfacePresentationSide::Top);
    let authored = SurfacePresentation::new(first)
        .with_anchor(anchor)
        .with_fallback(fallback);

    let element: Element<()> =
        Element::new(ExternalWidget).surface_presentation(authored.clone());

    assert_eq!(element.surface_presentation_config(), Some(&authored));
    assert_eq!(authored.candidates(), [first, fallback]);
}
