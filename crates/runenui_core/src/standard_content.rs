//! Standard passive content views over the open widget, semantic, and M9 paint contracts.
use crate::{
    Focusability, ImageAlignment, ImageCrop, ImageDescriptor, ImageFit, ImageMapping,
    ImageMappingError, ImagePaintDescriptor, LayoutDimension, LayoutStyle, LogicalLength,
    LogicalRect, PaintContribution, PaintContributionContext, PaintContributionItem,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution,
    SemanticOrientation, SemanticRole, WidgetInvalidation, WidgetMeasure, WidgetMeasureInput,
    WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::Widget,
};

/// Passive, semantic divider. Color and decoration remain ordinary style authoring.
///
/// Horizontal separators fill the available width by default; vertical separators
/// fill the available height. Their intrinsic cross-axis thickness is one logical
/// unit. Applications may override either axis using normal layout authoring.
#[derive(Clone, Debug, PartialEq)]
pub struct Separator {
    orientation: SemanticOrientation,
    common: CommonNodeAuthoring,
}

impl Separator {
    #[must_use]
    pub fn new(orientation: SemanticOrientation) -> Self {
        let common = CommonNodeAuthoring {
            layout: match orientation {
                SemanticOrientation::Horizontal => {
                    LayoutStyle::default().with_width(LayoutDimension::Fill)
                }
                SemanticOrientation::Vertical => {
                    LayoutStyle::default().with_height(LayoutDimension::Fill)
                }
            },
            ..CommonNodeAuthoring::default()
        };
        Self {
            orientation,
            common,
        }
    }

    common_node_builder_methods!();
}

#[derive(Debug)]
struct SeparatorWidget {
    orientation: SemanticOrientation,
}

impl<Action> Widget<Action> for SeparatorWidget {
    type State = SemanticOrientation;

    fn create_state(&self) -> Self::State {
        self.orientation
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if *state != self.orientation {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
            *state = self.orientation;
        }
    }

    fn measure(&self, state: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        match state {
            SemanticOrientation::Horizontal => {
                WidgetMeasure::measured(LogicalLength::ZERO, LogicalLength::from(1_u16))
            }
            SemanticOrientation::Vertical => {
                WidgetMeasure::measured(LogicalLength::from(1_u16), LogicalLength::ZERO)
            }
        }
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Separator).with_orientation(*state),
        )
    }
}

impl<Action: 'static> View<Action> for Separator {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(SeparatorWidget {
                orientation: self.orientation,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

/// Standard non-interactive image view using the existing M9 image policy.
///
/// A missing or empty alternative text means decorative content and contributes
/// no Image semantic node. Resource ownership/loading/realization stays external.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    descriptor: ImageDescriptor,
    mapping: ImageMapping,
    alternative: Option<String>,
    description: Option<String>,
    common: CommonNodeAuthoring,
}

impl Image {
    #[must_use]
    pub fn new(descriptor: ImageDescriptor) -> Self {
        Self {
            descriptor,
            mapping: ImageMapping::default(),
            alternative: None,
            description: None,
            common: CommonNodeAuthoring::default(),
        }
    }

    common_node_builder_methods!();

    /// Preserves the existing M9 crop/fit/alignment/nine-slice mapping vocabulary.
    ///
    /// # Errors
    ///
    /// Invalid source-pixel nine-slice insets reject before the widget is authored.
    pub fn with_mapping(mut self, mapping: ImageMapping) -> Result<Self, ImageMappingError> {
        let zero = LogicalRect::try_new(0.0, 0.0, 0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("zero destination is finite"));
        ImagePaintDescriptor::new(self.descriptor.clone(), zero, mapping)?;
        self.mapping = mapping;
        Ok(self)
    }

    /// Selects ordinary fit without rewriting the image's opaque resource identity.
    /// Switching from `NineSlice` starts with the default full-source centered fit.
    #[must_use]
    pub const fn fit(mut self, fit: ImageFit) -> Self {
        let (crop, alignment) = match self.mapping {
            ImageMapping::Fit {
                crop, alignment, ..
            } => (crop, alignment),
            ImageMapping::NineSlice { .. } => (ImageCrop::FULL, ImageAlignment::CENTER),
        };
        self.mapping = ImageMapping::Fit {
            crop,
            alignment,
            fit,
        };
        self
    }

    /// A nonempty alternative text makes this image meaningful to accessibility.
    /// An empty alternative explicitly makes it decorative.
    #[must_use]
    pub fn alt_text(mut self, alternative: impl Into<String>) -> Self {
        let alternative = alternative.into();
        self.alternative = (!alternative.is_empty()).then_some(alternative);
        self
    }

    /// Adds a description only when the image also has meaningful alternative text.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Suppresses Image semantics while preserving the same paint contribution.
    #[must_use]
    pub fn decorative(mut self) -> Self {
        self.alternative = None;
        self.description = None;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ImageState {
    descriptor: ImageDescriptor,
    mapping: ImageMapping,
    alternative: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
struct ImageWidget {
    authored: ImageState,
}

impl<Action> Widget<Action> for ImageWidget {
    type State = ImageState;

    fn create_state(&self) -> Self::State {
        self.authored.clone()
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.descriptor.intrinsic_size() != self.authored.descriptor.intrinsic_size() {
            context.invalidate(WidgetInvalidation::LAYOUT);
        }
        if state.descriptor != self.authored.descriptor || state.mapping != self.authored.mapping {
            context.invalidate(WidgetInvalidation::PAINT);
        }
        if state.alternative != self.authored.alternative
            || state.description != self.authored.description
        {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        state.clone_from(&self.authored);
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "layout uses finite f32 logical lengths; exact u32 intrinsic pixels stay in the image descriptor"
    )]
    fn measure(&self, state: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        let size = state.descriptor.intrinsic_size();
        let width = LogicalLength::new(size.width() as f32)
            .unwrap_or_else(|_| unreachable!("u32 image width converts to finite f32"));
        let height = LogicalLength::new(size.height() as f32)
            .unwrap_or_else(|_| unreachable!("u32 image height converts to finite f32"));
        WidgetMeasure::measured(width, height)
    }

    fn paint(&self, state: &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        let destination = LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("final widget size is finite and nonnegative"));
        let policy =
            ImagePaintDescriptor::new(state.descriptor.clone(), destination, state.mapping)
                .unwrap_or_else(|_| unreachable!("image mapping was validated when authored"));
        PaintContribution::single(PaintContributionItem::image(policy))
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let Some(alternative) = &state.alternative else {
            return SemanticContribution::empty();
        };
        let mut image =
            SemanticNodeContribution::primary(SemanticRole::Image).with_name(alternative.clone());
        if let Some(description) = &state.description {
            image = image.with_description(description.clone());
        }
        SemanticContribution::single(image)
    }
}

impl<Action: 'static> View<Action> for Image {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(ImageWidget {
                authored: ImageState {
                    descriptor: self.descriptor,
                    mapping: self.mapping,
                    alternative: self.alternative,
                    description: self.description,
                },
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

/// Passive horizontal or vertical divider, with ordinary layout and style.
#[must_use]
pub fn separator(orientation: SemanticOrientation) -> Separator {
    Separator::new(orientation)
}

/// Passive image using a caller-owned M9 resource descriptor.
#[must_use]
pub fn image(descriptor: ImageDescriptor) -> Image {
    Image::new(descriptor)
}

#[cfg(test)]
mod tests {
    use super::{image, separator};
    use crate::{
        ImageAlignment, ImageCrop, ImageDescriptor, ImageDestinationInsets, ImageFit,
        ImageIntrinsicSize, ImageMapping, ImageSourceInsets, LogicalSize, PaintContributionContext,
        PaintPrimitive, ResourceKind, ResourceRef, SemanticContributionContext,
        SemanticOrientation, SemanticRole, View, WidgetAvailableSpace, WidgetMeasure,
        WidgetMeasureInput,
    };

    fn fixture_image() -> (ImageDescriptor, ResourceRef) {
        let resource = ResourceRef::new(ResourceKind::Image);
        let intrinsic = ImageIntrinsicSize::new(48, 24)
            .unwrap_or_else(|| unreachable!("fixture intrinsic pixels are nonzero"));
        let descriptor = ImageDescriptor::new(resource.clone(), intrinsic)
            .unwrap_or_else(|_| unreachable!("fixture resource has image kind"));
        (descriptor, resource)
    }

    fn measurement() -> WidgetMeasureInput {
        WidgetMeasureInput::new(
            None,
            None,
            WidgetAvailableSpace::MaxContent,
            WidgetAvailableSpace::MaxContent,
        )
    }

    #[test]
    fn separator_is_semantic_oriented_passive_and_intrinsically_thin() {
        for orientation in [
            SemanticOrientation::Horizontal,
            SemanticOrientation::Vertical,
        ] {
            let element: crate::Element<()> = separator(orientation).into_element();
            let (_, _, _, _, _, _, _, _, widget, _) = element.into_runtime_parts().into_parts();
            let state = widget.create_state();
            let semantic = widget
                .semantics(&state, SemanticContributionContext::default())
                .unwrap_or_else(|_| unreachable!("separator contribution is valid"));
            let node = semantic.roots()[0]
                .as_node()
                .unwrap_or_else(|| unreachable!("separator has one primary role"));
            assert_eq!(node.role(), SemanticRole::Separator);
            assert_eq!(node.orientation(), Some(orientation));
            assert!(node.actions().is_empty());
            assert!(
                !widget
                    .activation(&state)
                    .unwrap_or_else(|_| unreachable!())
                    .is_actionable()
            );
            assert_eq!(
                widget
                    .measure(&state, measurement())
                    .unwrap_or_else(|_| unreachable!()),
                match orientation {
                    SemanticOrientation::Horizontal => WidgetMeasure::measured(
                        crate::LogicalLength::ZERO,
                        crate::LogicalLength::from(1_u16)
                    ),
                    SemanticOrientation::Vertical => WidgetMeasure::measured(
                        crate::LogicalLength::from(1_u16),
                        crate::LogicalLength::ZERO
                    ),
                }
            );
        }
    }

    #[test]
    fn image_preserves_exact_resource_mapping_and_decorative_semantics() {
        let (descriptor, resource) = fixture_image();
        let mapping = ImageMapping::Fit {
            crop: ImageCrop::FULL,
            alignment: ImageAlignment::CENTER,
            fit: ImageFit::Contain,
        };
        let element: crate::Element<()> = image(descriptor)
            .with_mapping(mapping)
            .unwrap_or_else(|_| unreachable!("valid fit mapping"))
            .into_element();
        let (_, _, _, _, _, _, _, _, widget, _) = element.into_runtime_parts().into_parts();
        let state = widget.create_state();
        let semantics = widget
            .semantics(&state, SemanticContributionContext::default())
            .unwrap_or_else(|_| unreachable!("decorative semantics valid"));
        assert!(semantics.roots().is_empty());
        let paint = widget
            .paint(
                &state,
                PaintContributionContext::__runtime_new(
                    LogicalSize::try_new(120.0, 60.0)
                        .unwrap_or_else(|_| unreachable!("valid fixture size")),
                    crate::ComputedStyle::default(),
                ),
            )
            .unwrap_or_else(|_| unreachable!("image paint valid"));
        assert_eq!(paint.items().len(), 1);
        let PaintPrimitive::Image(primitive) = paint.items()[0].primitive() else {
            unreachable!("standard image uses generic image primitive")
        };
        assert_eq!(primitive.resource_ref(), &resource);
        let policy = primitive
            .authored_descriptor()
            .unwrap_or_else(|| unreachable!("authoring contains descriptor"));
        assert_eq!(policy.mapping(), mapping);
        assert_eq!(policy.destination().width(), 120.0);
        assert_eq!(policy.destination().height(), 60.0);
        assert_eq!(policy.image().resource_ref(), &resource);
        assert_eq!(
            widget
                .measure(&state, measurement())
                .unwrap_or_else(|_| unreachable!()),
            WidgetMeasure::measured(
                crate::LogicalLength::from(48_u16),
                crate::LogicalLength::from(24_u16)
            )
        );
    }

    #[test]
    fn image_semantics_require_nonempty_alt_and_invalid_nine_slice_fails_early() {
        let (descriptor, _) = fixture_image();
        let meaningful: crate::Element<()> = image(descriptor.clone())
            .alt_text("Atlas")
            .description("A map")
            .into_element();
        let (_, _, _, _, _, _, _, _, widget, _) = meaningful.into_runtime_parts().into_parts();
        let state = widget.create_state();
        let semantics = widget
            .semantics(&state, SemanticContributionContext::default())
            .unwrap_or_else(|_| unreachable!("meaningful image is valid"));
        let node = semantics.roots()[0]
            .as_node()
            .unwrap_or_else(|| unreachable!("meaningful image semantic node"));
        assert_eq!(node.role(), SemanticRole::Image);
        assert_eq!(node.name(), Some("Atlas"));
        assert_eq!(node.description(), Some("A map"));
        assert!(node.actions().is_empty());

        let decorative: crate::Element<()> = image(descriptor.clone())
            .alt_text("Atlas")
            .alt_text("")
            .into_element();
        let (_, _, _, _, _, _, _, _, widget, _) = decorative.into_runtime_parts().into_parts();
        let state = widget.create_state();
        assert!(
            widget
                .semantics(&state, SemanticContributionContext::default())
                .unwrap_or_else(|_| unreachable!())
                .roots()
                .is_empty()
        );

        let source_insets = ImageSourceInsets::new(50.0, 0.0, 0.0, 50.0)
            .unwrap_or_else(|_| unreachable!("insets finite but overlapping"));
        let invalid_mapping = ImageMapping::NineSlice {
            source: ImageCrop::FULL,
            source_insets,
            destination_insets: ImageDestinationInsets::default(),
        };
        assert!(image(descriptor).with_mapping(invalid_mapping).is_err());
    }
}
