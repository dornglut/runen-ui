use core::{error::Error, fmt};
use std::sync::Arc;

use runenui_core::{LogicalPoint, ResourceKind, ResourceRef};
use runenui_runtime::RasterScale;

/// Validation failure for renderer-edge immutable raster payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadValidationError {
    /// An image payload declared a zero width or height.
    ZeroImageExtent,
    /// The required byte length cannot be represented on this target.
    ByteLengthOverflow,
    /// The supplied bytes do not exactly match the declared raster extent.
    ByteLengthMismatch { expected: usize, actual: usize },
}

impl fmt::Display for PayloadValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroImageExtent => formatter.write_str("image extent must be non-zero"),
            Self::ByteLengthOverflow => {
                formatter.write_str("declared raster extent exceeds addressable byte length")
            }
            Self::ByteLengthMismatch { expected, actual } => write!(
                formatter,
                "raster byte length mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl Error for PayloadValidationError {}

/// Immutable normalized image source for renderer realization.
///
/// Bytes are row-major, tightly packed, unpremultiplied RGBA8 in sRGB encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImagePayload {
    width: u32,
    height: u32,
    rgba8_srgb: Arc<[u8]>,
}

impl ImagePayload {
    /// Validates one non-zero RGBA8 image payload.
    ///
    /// # Errors
    ///
    /// Returns [`PayloadValidationError`] for zero extent, unrepresentable byte
    /// length, or a byte slice whose length is not exactly `width * height * 4`.
    pub fn new(
        width: u32,
        height: u32,
        rgba8_srgb: impl Into<Arc<[u8]>>,
    ) -> Result<Self, PayloadValidationError> {
        if width == 0 || height == 0 {
            return Err(PayloadValidationError::ZeroImageExtent);
        }
        let rgba8_srgb = rgba8_srgb.into();
        validate_byte_length(width, height, 4, rgba8_srgb.len())?;
        Ok(Self {
            width,
            height,
            rgba8_srgb,
        })
    }

    /// Returns the non-zero pixel width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the non-zero pixel height.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns tightly packed unpremultiplied RGBA8 sRGB bytes.
    #[must_use]
    pub fn rgba8_srgb(&self) -> &[u8] {
        &self.rgba8_srgb
    }
}

/// Scale-specific immutable alpha coverage for one already-shaped text resource.
///
/// `logical_origin` is the resource-local logical coordinate of the coverage
/// raster's top-left boundary. One raster pixel spans `1 / raster_scale` logical
/// units on each axis. Foreground color is not part of this payload; it remains
/// ordinary scene-owned paint state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedRunRaster {
    logical_origin: LogicalPoint,
    width: u32,
    height: u32,
    raster_scale: RasterScale,
    alpha8: Arc<[u8]>,
}

impl ShapedRunRaster {
    /// Validates one alpha8 coverage raster.
    ///
    /// Zero width or height is allowed and represents an empty coverage result.
    ///
    /// # Errors
    ///
    /// Returns [`PayloadValidationError`] when the declared extent cannot be
    /// represented or the alpha byte length is not exactly `width * height`.
    pub fn new(
        logical_origin: LogicalPoint,
        width: u32,
        height: u32,
        raster_scale: RasterScale,
        alpha8: impl Into<Arc<[u8]>>,
    ) -> Result<Self, PayloadValidationError> {
        let alpha8 = alpha8.into();
        validate_byte_length(width, height, 1, alpha8.len())?;
        Ok(Self {
            logical_origin,
            width,
            height,
            raster_scale,
            alpha8,
        })
    }

    /// Returns the resource-local logical top-left coverage boundary.
    #[must_use]
    pub const fn logical_origin(&self) -> LogicalPoint {
        self.logical_origin
    }

    /// Returns the coverage raster width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the coverage raster height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the exact scale at which this coverage was realized.
    #[must_use]
    pub const fn raster_scale(&self) -> RasterScale {
        self.raster_scale
    }

    /// Returns tightly packed row-major alpha8 coverage bytes.
    #[must_use]
    pub fn alpha8(&self) -> &[u8] {
        &self.alpha8
    }
}

/// Renderer request made to the caller-owned logical resource provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceRequest {
    /// Resolve an immutable normalized image source.
    Image,
    /// Resolve one already-shaped run as scale-specific alpha coverage.
    ShapedTextRun { raster_scale: RasterScale },
}

impl ResourceRequest {
    /// Returns the exact neutral resource kind required by this request.
    #[must_use]
    pub const fn resource_kind(self) -> ResourceKind {
        match self {
            Self::Image => ResourceKind::Image,
            Self::ShapedTextRun { .. } => ResourceKind::ShapedTextRun,
        }
    }
}

/// Immutable logical payload returned by a caller-owned resource provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourcePayload {
    Image(ImagePayload),
    ShapedTextRun(ShapedRunRaster),
}

impl ResourcePayload {
    /// Returns the neutral kind represented by this payload.
    #[must_use]
    pub const fn resource_kind(&self) -> ResourceKind {
        match self {
            Self::Image(_) => ResourceKind::Image,
            Self::ShapedTextRun(_) => ResourceKind::ShapedTextRun,
        }
    }
}

/// Structured caller-provider failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceProviderErrorKind {
    Missing,
    Unavailable,
    Malformed,
}

impl ResourceProviderErrorKind {
    /// Stable diagnostic code for this provider failure category.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Missing => "runenui.renderer.resource.missing",
            Self::Unavailable => "runenui.renderer.resource.unavailable",
            Self::Malformed => "runenui.renderer.resource.malformed",
        }
    }
}

/// Structured error returned by a caller-owned resource provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceProviderError {
    kind: ResourceProviderErrorKind,
    detail: Arc<str>,
}

impl ResourceProviderError {
    #[must_use]
    pub fn new(kind: ResourceProviderErrorKind, detail: impl Into<Arc<str>>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ResourceProviderErrorKind {
        self.kind
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.kind.code()
    }

    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for ResourceProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.detail)
    }
}

impl Error for ResourceProviderError {}

/// Caller-owned logical resource lookup.
///
/// The complete opaque [`ResourceRef`] is the lookup key. Implementations must
/// not derive a provider/domain/cache key from its debug representation or kind.
pub trait ResourceProvider {
    /// Resolves one logical resource for the exact renderer request.
    ///
    /// # Errors
    ///
    /// Returns a structured provider error when the resource is missing,
    /// temporarily unavailable, or cannot produce the requested logical payload.
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError>;
}

/// Deterministic contract failure while resolving one renderer resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceResolveError {
    ReferenceKindMismatch {
        expected: ResourceKind,
        actual: ResourceKind,
    },
    PayloadKindMismatch {
        expected: ResourceKind,
        actual: ResourceKind,
    },
    Provider(ResourceProviderError),
}

impl fmt::Display for ResourceResolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReferenceKindMismatch { expected, actual } => write!(
                formatter,
                "resource reference kind mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::PayloadKindMismatch { expected, actual } => write!(
                formatter,
                "resource payload kind mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::Provider(error) => error.fmt(formatter),
        }
    }
}

impl Error for ResourceResolveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider(error) => Some(error),
            Self::ReferenceKindMismatch { .. } | Self::PayloadKindMismatch { .. } => None,
        }
    }
}

/// Resolves one resource while enforcing complete-ref and kind consistency.
///
/// # Errors
///
/// Returns deterministic contract failures for reference/request mismatch,
/// provider failure, or provider payload kind mismatch.
pub fn resolve_resource(
    provider: &impl ResourceProvider,
    resource: &ResourceRef,
    request: ResourceRequest,
) -> Result<ResourcePayload, ResourceResolveError> {
    let expected = request.resource_kind();
    let actual_reference = resource.kind();
    if actual_reference != expected {
        return Err(ResourceResolveError::ReferenceKindMismatch {
            expected,
            actual: actual_reference,
        });
    }

    let payload = provider
        .load(resource, request)
        .map_err(ResourceResolveError::Provider)?;
    let actual_payload = payload.resource_kind();
    if actual_payload != expected {
        return Err(ResourceResolveError::PayloadKindMismatch {
            expected,
            actual: actual_payload,
        });
    }
    Ok(payload)
}

fn validate_byte_length(
    width: u32,
    height: u32,
    channels: u64,
    actual: usize,
) -> Result<(), PayloadValidationError> {
    let expected_u64 = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(channels))
        .ok_or(PayloadValidationError::ByteLengthOverflow)?;
    let expected =
        usize::try_from(expected_u64).map_err(|_| PayloadValidationError::ByteLengthOverflow)?;
    if actual == expected {
        Ok(())
    } else {
        Err(PayloadValidationError::ByteLengthMismatch { expected, actual })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use runenui_core::{LogicalPoint, ResourceKind, ResourceRef};
    use runenui_runtime::RasterScale;

    use super::{
        ImagePayload, PayloadValidationError, ResourcePayload, ResourceProvider,
        ResourceProviderError, ResourceProviderErrorKind, ResourceRequest, ResourceResolveError,
        ShapedRunRaster, resolve_resource,
    };

    #[derive(Default)]
    struct MapProvider {
        payloads: HashMap<ResourceRef, ResourcePayload>,
    }

    impl ResourceProvider for MapProvider {
        fn load(
            &self,
            resource: &ResourceRef,
            _: ResourceRequest,
        ) -> Result<ResourcePayload, ResourceProviderError> {
            self.payloads.get(resource).cloned().ok_or_else(|| {
                ResourceProviderError::new(ResourceProviderErrorKind::Missing, "fixture resource")
            })
        }
    }

    fn image_payload() -> ImagePayload {
        ImagePayload::new(1, 1, vec![1, 2, 3, 4])
            .unwrap_or_else(|_| unreachable!("fixture image payload is valid"))
    }

    #[test]
    fn image_payload_requires_non_zero_exact_rgba8_extent() {
        assert_eq!(
            ImagePayload::new(0, 1, Vec::<u8>::new()),
            Err(PayloadValidationError::ZeroImageExtent)
        );
        assert_eq!(
            ImagePayload::new(1, 1, vec![1, 2, 3]),
            Err(PayloadValidationError::ByteLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );

        let payload = image_payload();
        assert_eq!(payload.width(), 1);
        assert_eq!(payload.height(), 1);
        assert_eq!(payload.rgba8_srgb(), &[1, 2, 3, 4]);
    }

    #[test]
    fn shaped_run_raster_preserves_requested_scale_and_allows_empty_coverage() {
        let origin = LogicalPoint::new(-1.0, 2.0)
            .unwrap_or_else(|_| unreachable!("fixture origin is finite"));
        let scale =
            RasterScale::new(2.0).unwrap_or_else(|_| unreachable!("fixture raster scale is valid"));
        let raster = ShapedRunRaster::new(origin, 2, 1, scale, vec![0, 255])
            .unwrap_or_else(|_| unreachable!("fixture shaped raster is valid"));
        assert_eq!(raster.logical_origin(), origin);
        assert_eq!(raster.width(), 2);
        assert_eq!(raster.height(), 1);
        assert_eq!(raster.raster_scale(), scale);
        assert_eq!(raster.alpha8(), &[0, 255]);

        let empty = ShapedRunRaster::new(origin, 0, 0, scale, Vec::<u8>::new())
            .unwrap_or_else(|_| unreachable!("empty shaped coverage is valid"));
        assert!(empty.alpha8().is_empty());
    }

    #[test]
    fn complete_resource_ref_is_the_provider_lookup_identity() {
        let stored = ResourceRef::new(ResourceKind::Image);
        let same = stored.clone();
        let fresh = ResourceRef::new(ResourceKind::Image);
        let mut provider = MapProvider::default();
        provider
            .payloads
            .insert(stored, ResourcePayload::Image(image_payload()));

        let resolved = resolve_resource(&provider, &same, ResourceRequest::Image)
            .unwrap_or_else(|_| unreachable!("cloned complete ref resolves stored resource"));
        assert!(matches!(resolved, ResourcePayload::Image(_)));

        let missing = resolve_resource(&provider, &fresh, ResourceRequest::Image);
        assert!(matches!(
            missing,
            Err(ResourceResolveError::Provider(error))
                if error.kind() == ResourceProviderErrorKind::Missing
        ));
    }

    #[test]
    fn resolver_rejects_reference_and_payload_kind_mismatch() {
        let shaped = ResourceRef::new(ResourceKind::ShapedTextRun);
        let provider = MapProvider::default();
        assert!(matches!(
            resolve_resource(&provider, &shaped, ResourceRequest::Image),
            Err(ResourceResolveError::ReferenceKindMismatch {
                expected: ResourceKind::Image,
                actual: ResourceKind::ShapedTextRun,
            })
        ));

        let image = ResourceRef::new(ResourceKind::Image);
        let origin = LogicalPoint::new(0.0, 0.0)
            .unwrap_or_else(|_| unreachable!("fixture origin is finite"));
        let scale = RasterScale::ONE;
        let shaped_payload = ShapedRunRaster::new(origin, 0, 0, scale, Vec::<u8>::new())
            .unwrap_or_else(|_| unreachable!("empty shaped payload is valid"));
        let mut wrong_provider = MapProvider::default();
        wrong_provider.payloads.insert(
            image.clone(),
            ResourcePayload::ShapedTextRun(shaped_payload),
        );

        assert!(matches!(
            resolve_resource(&wrong_provider, &image, ResourceRequest::Image),
            Err(ResourceResolveError::PayloadKindMismatch {
                expected: ResourceKind::Image,
                actual: ResourceKind::ShapedTextRun,
            })
        ));
    }
}
