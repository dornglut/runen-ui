//! Renderer-neutral M9 visual descriptor vocabulary.

mod brush;
mod image;
mod outline;
mod presentation;
mod shadow;
mod stroke;

pub use brush::{
    Brush, GradientGeometryError, GradientStop, GradientStops, GradientStopsError, LinearGradient,
    RadialGradient, UnitInterval, UnitIntervalError,
};
pub use image::{
    ImageAlignment, ImageCrop, ImageCropError, ImageDescriptor, ImageDestinationInsets, ImageFit,
    ImageIntrinsicSize, ImageMapping, ImageMappingError, ImagePaintDescriptor, ImageSourceInsets,
    ImageSourceInsetsError,
};
pub use outline::Outline;
pub use presentation::{
    PresentationOrigin, PresentationRotation, PresentationScalarError, PresentationScale,
    PresentationTransform, PresentationTranslation,
};
pub use shadow::{DropShadow, NonFiniteVisualScalar};
pub use stroke::{StrokeCap, StrokeJoin, StrokeStyle, StrokeStyleError};
