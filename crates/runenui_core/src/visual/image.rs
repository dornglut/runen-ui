use core::{error::Error, fmt, num::NonZeroU32};

use crate::{LogicalLength, LogicalRect, ResourceKind, ResourceKindMismatch, ResourceRef};

use super::UnitInterval;

/// Exact non-zero intrinsic image pixel extent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ImageIntrinsicSize {
    width: NonZeroU32,
    height: NonZeroU32,
}

impl ImageIntrinsicSize {
    /// Creates one exact non-zero intrinsic extent.
    ///
    /// Returns `None` when either axis is zero.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        let Some(width) = NonZeroU32::new(width) else {
            return None;
        };
        let Some(height) = NonZeroU32::new(height) else {
            return None;
        };
        Some(Self { width, height })
    }

    /// Returns intrinsic pixel width.
    #[must_use]
    pub const fn width(self) -> u32 {
        self.width.get()
    }

    /// Returns intrinsic pixel height.
    #[must_use]
    pub const fn height(self) -> u32 {
        self.height.get()
    }
}

/// Validated normalized image crop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageCrop {
    x: UnitInterval,
    y: UnitInterval,
    width: UnitInterval,
    height: UnitInterval,
}

/// Validation failure for one normalized crop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageCropError {
    /// Crop width or height was zero.
    Empty,
    /// Crop extends beyond normalized source extent.
    OutsideSource,
}

impl fmt::Display for ImageCropError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "image crop must have positive width and height",
            Self::OutsideSource => "image crop must remain within normalized source extent",
        })
    }
}

impl Error for ImageCropError {}

impl ImageCrop {
    /// Complete normalized source crop.
    pub const FULL: Self = Self {
        x: UnitInterval::ZERO,
        y: UnitInterval::ZERO,
        width: UnitInterval::ONE,
        height: UnitInterval::ONE,
    };

    /// Validates a normalized positive crop.
    ///
    /// # Errors
    ///
    /// Returns [`ImageCropError`] when empty or outside the source.
    pub fn new(
        x: UnitInterval,
        y: UnitInterval,
        width: UnitInterval,
        height: UnitInterval,
    ) -> Result<Self, ImageCropError> {
        if width == UnitInterval::ZERO || height == UnitInterval::ZERO {
            return Err(ImageCropError::Empty);
        }
        if f64::from(x.get()) + f64::from(width.get()) > 1.0
            || f64::from(y.get()) + f64::from(height.get()) > 1.0
        {
            return Err(ImageCropError::OutsideSource);
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    /// Returns normalized crop start on x.
    #[must_use]
    pub const fn x(self) -> UnitInterval {
        self.x
    }

    /// Returns normalized crop start on y.
    #[must_use]
    pub const fn y(self) -> UnitInterval {
        self.y
    }

    /// Returns normalized crop width.
    #[must_use]
    pub const fn width(self) -> UnitInterval {
        self.width
    }

    /// Returns normalized crop height.
    #[must_use]
    pub const fn height(self) -> UnitInterval {
        self.height
    }
}

/// Finite normalized image alignment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageAlignment {
    x: UnitInterval,
    y: UnitInterval,
}

impl ImageAlignment {
    /// Center alignment.
    pub const CENTER: Self = Self {
        x: UnitInterval::HALF,
        y: UnitInterval::HALF,
    };

    /// Creates one already-validated normalized alignment.
    #[must_use]
    pub const fn new(x: UnitInterval, y: UnitInterval) -> Self {
        Self { x, y }
    }

    /// Returns horizontal alignment.
    #[must_use]
    pub const fn x(self) -> UnitInterval {
        self.x
    }

    /// Returns vertical alignment.
    #[must_use]
    pub const fn y(self) -> UnitInterval {
        self.y
    }
}

/// Ordinary image fit policy.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ImageFit {
    /// Independently scale each axis to fill destination.
    #[default]
    Fill,
    /// Uniformly fit completely inside destination.
    Contain,
    /// Uniformly cover destination and crop overflow.
    Cover,
    /// Preserve intrinsic logical scale `1`.
    None,
    /// Preserve intrinsic scale when it fits, otherwise contain.
    ScaleDown,
}

/// Complete immutable logical image identity plus intrinsic metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageDescriptor {
    resource: ResourceRef,
    intrinsic_size: ImageIntrinsicSize,
}

impl ImageDescriptor {
    /// Creates one image descriptor from the complete image [`ResourceRef`].
    ///
    /// # Errors
    ///
    /// Returns [`ResourceKindMismatch`] when the resource is not image-kind.
    pub fn new(
        resource: ResourceRef,
        intrinsic_size: ImageIntrinsicSize,
    ) -> Result<Self, ResourceKindMismatch> {
        if resource.kind() != ResourceKind::Image {
            return Err(ResourceKindMismatch::new(
                ResourceKind::Image,
                resource.kind(),
            ));
        }
        Ok(Self {
            resource,
            intrinsic_size,
        })
    }

    /// Returns the complete opaque image identity.
    #[must_use]
    pub const fn resource_ref(&self) -> &ResourceRef {
        &self.resource
    }

    /// Returns exact non-zero intrinsic pixel metadata.
    #[must_use]
    pub const fn intrinsic_size(&self) -> ImageIntrinsicSize {
        self.intrinsic_size
    }
}

/// Four non-negative source-pixel insets for nine-slice mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageSourceInsets {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

/// Validation failure for source-pixel insets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageSourceInsetsError {
    /// One inset was NaN or infinite.
    NotFinite,
    /// One inset was negative.
    Negative,
}

impl fmt::Display for ImageSourceInsetsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFinite => "image source insets must be finite",
            Self::Negative => "image source insets must be non-negative",
        })
    }
}

impl Error for ImageSourceInsetsError {}

impl ImageSourceInsets {
    /// Validates non-negative finite source-pixel insets.
    ///
    /// # Errors
    ///
    /// Returns [`ImageSourceInsetsError`] for non-finite or negative values.
    pub fn new(
        top: f32,
        right: f32,
        bottom: f32,
        left: f32,
    ) -> Result<Self, ImageSourceInsetsError> {
        if ![top, right, bottom, left].into_iter().all(f32::is_finite) {
            return Err(ImageSourceInsetsError::NotFinite);
        }
        if [top, right, bottom, left]
            .into_iter()
            .any(|value| value < 0.0)
        {
            return Err(ImageSourceInsetsError::Negative);
        }
        Ok(Self {
            top,
            right,
            bottom,
            left,
        })
    }

    /// Returns top source-pixel inset.
    #[must_use]
    pub const fn top(self) -> f32 {
        self.top
    }

    /// Returns right source-pixel inset.
    #[must_use]
    pub const fn right(self) -> f32 {
        self.right
    }

    /// Returns bottom source-pixel inset.
    #[must_use]
    pub const fn bottom(self) -> f32 {
        self.bottom
    }

    /// Returns left source-pixel inset.
    #[must_use]
    pub const fn left(self) -> f32 {
        self.left
    }
}

/// Four logical destination edge widths for nine-slice mapping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImageDestinationInsets {
    top: LogicalLength,
    right: LogicalLength,
    bottom: LogicalLength,
    left: LogicalLength,
}

impl ImageDestinationInsets {
    /// Creates destination edge widths.
    #[must_use]
    pub const fn new(
        top: LogicalLength,
        right: LogicalLength,
        bottom: LogicalLength,
        left: LogicalLength,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Returns top logical edge width.
    #[must_use]
    pub const fn top(self) -> LogicalLength {
        self.top
    }

    /// Returns right logical edge width.
    #[must_use]
    pub const fn right(self) -> LogicalLength {
        self.right
    }

    /// Returns bottom logical edge width.
    #[must_use]
    pub const fn bottom(self) -> LogicalLength {
        self.bottom
    }

    /// Returns left logical edge width.
    #[must_use]
    pub const fn left(self) -> LogicalLength {
        self.left
    }
}

/// Image mapping mode before runtime resolves exact source/destination geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImageMapping {
    /// Ordinary crop/fit/alignment mapping.
    Fit {
        /// Normalized source crop.
        crop: ImageCrop,
        /// Normalized destination alignment.
        alignment: ImageAlignment,
        /// Ordinary fit policy.
        fit: ImageFit,
    },
    /// Stretched nine-slice mapping. Tiling/repeat is intentionally absent.
    NineSlice {
        /// Normalized source crop.
        source: ImageCrop,
        /// Source-pixel slice insets.
        source_insets: ImageSourceInsets,
        /// Logical destination edge widths.
        destination_insets: ImageDestinationInsets,
    },
}

impl Default for ImageMapping {
    fn default() -> Self {
        Self::Fit {
            crop: ImageCrop::FULL,
            alignment: ImageAlignment::CENTER,
            fit: ImageFit::Fill,
        }
    }
}

/// Failure while correlating nine-slice source insets with intrinsic source extent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageMappingError {
    /// Opposing source insets overlap within the selected crop.
    OverlappingSourceInsets,
}

impl fmt::Display for ImageMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("nine-slice source insets cannot overlap")
    }
}

impl Error for ImageMappingError {}

/// Complete owner-local image paint descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct ImagePaintDescriptor {
    image: ImageDescriptor,
    destination: LogicalRect,
    mapping: ImageMapping,
}

impl ImagePaintDescriptor {
    /// Creates one image paint descriptor and validates nine-slice source insets.
    ///
    /// # Errors
    ///
    /// Returns [`ImageMappingError`] when source insets overlap in the selected crop.
    pub fn new(
        image: ImageDescriptor,
        destination: LogicalRect,
        mapping: ImageMapping,
    ) -> Result<Self, ImageMappingError> {
        if let ImageMapping::NineSlice {
            source,
            source_insets,
            ..
        } = mapping
        {
            let source_width =
                f64::from(image.intrinsic_size().width()) * f64::from(source.width().get());
            let source_height =
                f64::from(image.intrinsic_size().height()) * f64::from(source.height().get());
            if f64::from(source_insets.left()) + f64::from(source_insets.right()) > source_width
                || f64::from(source_insets.top()) + f64::from(source_insets.bottom())
                    > source_height
            {
                return Err(ImageMappingError::OverlappingSourceInsets);
            }
        }
        Ok(Self {
            image,
            destination,
            mapping,
        })
    }

    /// Returns immutable image identity/metadata.
    #[must_use]
    pub const fn image(&self) -> &ImageDescriptor {
        &self.image
    }

    /// Returns exact owner-local destination rectangle.
    #[must_use]
    pub const fn destination(&self) -> LogicalRect {
        self.destination
    }

    /// Returns unresolved neutral mapping policy.
    #[must_use]
    pub const fn mapping(&self) -> ImageMapping {
        self.mapping
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ImageAlignment, ImageCrop, ImageCropError, ImageDescriptor, ImageDestinationInsets,
        ImageFit, ImageIntrinsicSize, ImageMapping, ImageMappingError, ImagePaintDescriptor,
        ImageSourceInsets,
    };
    use crate::{LogicalRect, ResourceKind, ResourceRef, UnitInterval};

    #[test]
    fn image_descriptor_preserves_resource_identity_across_mapping() {
        let resource = ResourceRef::new(ResourceKind::Image);
        let image = ImageDescriptor::new(
            resource.clone(),
            ImageIntrinsicSize::new(40, 20)
                .unwrap_or_else(|| unreachable!("test intrinsic size is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("test resource has image kind"));
        let destination = LogicalRect::try_new(0.0, 0.0, 100.0, 100.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        let paint = ImagePaintDescriptor::new(
            image,
            destination,
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Contain,
            },
        )
        .unwrap_or_else(|_| unreachable!("test mapping is valid"));
        assert_eq!(paint.image().resource_ref(), &resource);
    }

    #[test]
    fn crop_validation_uses_wider_arithmetic_at_the_source_boundary() {
        let start = UnitInterval::new(0.999_999_94)
            .unwrap_or_else(|_| unreachable!("test start is normalized"));
        let width = UnitInterval::new(0.000_000_1)
            .unwrap_or_else(|_| unreachable!("test width is normalized"));
        assert_eq!(
            ImageCrop::new(start, UnitInterval::ZERO, width, UnitInterval::ONE),
            Err(ImageCropError::OutsideSource)
        );
    }

    #[test]
    fn nine_slice_rejects_overlapping_source_insets() {
        let image = ImageDescriptor::new(
            ResourceRef::new(ResourceKind::Image),
            ImageIntrinsicSize::new(10, 10)
                .unwrap_or_else(|| unreachable!("test intrinsic size is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("test resource has image kind"));
        let source_insets = ImageSourceInsets::new(0.0, 6.0, 0.0, 6.0)
            .unwrap_or_else(|_| unreachable!("test insets are finite"));
        let destination = LogicalRect::try_new(0.0, 0.0, 20.0, 20.0)
            .unwrap_or_else(|_| unreachable!("test destination is valid"));
        assert_eq!(
            ImagePaintDescriptor::new(
                image,
                destination,
                ImageMapping::NineSlice {
                    source: ImageCrop::FULL,
                    source_insets,
                    destination_insets: ImageDestinationInsets::default(),
                },
            ),
            Err(ImageMappingError::OverlappingSourceInsets)
        );
    }
}
