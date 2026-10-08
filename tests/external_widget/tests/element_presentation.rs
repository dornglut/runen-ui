use runenui_core::{
    Element, LogicalPoint, PresentationFocusEntry, PresentationFocusPolicy, PresentationOrigin,
    PresentationOutsidePointerPolicy, PresentationRotation, PresentationScale,
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

    let element: Element<()> = Element::new(ExternalWidget).surface_presentation(authored.clone());

    assert_eq!(element.surface_presentation_config(), Some(&authored));
    assert_eq!(authored.candidates(), [first, fallback]);
}

#[test]
fn downstream_custom_widget_can_author_presentation_lifecycle_without_runtime_identity() {
    let placement = SurfacePresentationPlacement::new(SurfacePresentationSide::Bottom);
    let authored = SurfacePresentation::new(placement)
        .with_outside_pointer(PresentationOutsidePointerPolicy::DismissAndBlock)
        .modal(true)
        .dismiss_on_cancel_or_back(true)
        .with_focus_policy(PresentationFocusPolicy::EnterAndRestore);

    let element: Element<()> = Element::new(ExternalWidget)
        .presentation_focus_preferred(true)
        .surface_presentation(authored.clone());

    assert_eq!(element.surface_presentation_config(), Some(&authored));
    assert_eq!(
        element.presentation_focus_entry(),
        PresentationFocusEntry::Preferred
    );
    assert_eq!(
        authored.outside_pointer(),
        PresentationOutsidePointerPolicy::DismissAndBlock
    );
    assert!(authored.is_modal());
    assert!(authored.dismisses_on_cancel_or_back());
    assert_eq!(
        authored.focus_policy(),
        PresentationFocusPolicy::EnterAndRestore
    );

    let tooltip_style = SurfacePresentation::new(placement);
    assert_eq!(
        tooltip_style.outside_pointer(),
        PresentationOutsidePointerPolicy::Ignore
    );
    assert!(!tooltip_style.is_modal());
    assert!(!tooltip_style.dismisses_on_cancel_or_back());
    assert_eq!(
        tooltip_style.focus_policy(),
        PresentationFocusPolicy::Preserve
    );
}
