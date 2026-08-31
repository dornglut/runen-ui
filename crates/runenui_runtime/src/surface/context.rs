use core::{error::Error, fmt};

use runenui_core::StyleEnvironment;

use crate::{
    DeterministicMeasurementProvider, LayoutConstraints, LogicalSize, MeasurementProvider,
};

static DEFAULT_MEASUREMENT_PROVIDER: DeterministicMeasurementProvider =
    DeterministicMeasurementProvider::DEFAULT;

/// Error returned when a renderer raster scale is not finite and strictly positive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RasterScaleError {
    /// The supplied scale was NaN or positive/negative infinity.
    NotFinite,
    /// The supplied finite scale was zero or negative.
    NotPositive,
}

impl fmt::Display for RasterScaleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("raster scale must be finite"),
            Self::NotPositive => formatter.write_str("raster scale must be strictly positive"),
        }
    }
}

impl Error for RasterScaleError {}

/// Finite strictly-positive renderer raster scale for one logical surface publication.
///
/// Raster scale affects renderer realization only. It never rescales logical scene,
/// layout, or hit-test coordinates.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct RasterScale(f32);

impl RasterScale {
    /// Deterministic default one-to-one logical-to-raster scale.
    pub const ONE: Self = Self(1.0);

    /// Validates one neutral raster scale.
    ///
    /// # Errors
    ///
    /// Returns [`RasterScaleError`] for non-finite, zero, or negative values.
    pub const fn new(value: f32) -> Result<Self, RasterScaleError> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            Err(RasterScaleError::NotFinite)
        } else if value <= 0.0 {
            Err(RasterScaleError::NotPositive)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the validated scalar raster scale.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Eq for RasterScale {}

impl Default for RasterScale {
    fn default() -> Self {
        Self::ONE
    }
}

/// Explicit inputs used to publish one surface snapshot.
#[derive(Clone, Copy)]
pub struct SurfaceBuildContext<'a> {
    style_environment: &'a StyleEnvironment,
    root_constraints: LayoutConstraints,
    measurement_provider: &'a dyn MeasurementProvider,
    raster_scale: RasterScale,
}

impl<'a> SurfaceBuildContext<'a> {
    #[must_use]
    pub fn new(
        style_environment: &'a StyleEnvironment,
        root_constraints: LayoutConstraints,
    ) -> Self {
        Self {
            style_environment,
            root_constraints,
            measurement_provider: &DEFAULT_MEASUREMENT_PROVIDER,
            raster_scale: RasterScale::ONE,
        }
    }

    #[must_use]
    pub fn tight(style_environment: &'a StyleEnvironment, size: LogicalSize) -> Self {
        Self::new(style_environment, LayoutConstraints::tight(size))
    }

    #[must_use]
    pub const fn with_root_constraints(mut self, root_constraints: LayoutConstraints) -> Self {
        self.root_constraints = root_constraints;
        self
    }

    #[must_use]
    pub fn with_measurement_provider(
        mut self,
        measurement_provider: &'a dyn MeasurementProvider,
    ) -> Self {
        self.measurement_provider = measurement_provider;
        self
    }

    /// Sets the exact neutral renderer raster scale for this publication attempt.
    #[must_use]
    pub const fn with_raster_scale(mut self, raster_scale: RasterScale) -> Self {
        self.raster_scale = raster_scale;
        self
    }

    /// Returns the complete host-neutral style environment for this publication.
    #[must_use]
    pub const fn style_environment(&self) -> &'a StyleEnvironment {
        self.style_environment
    }

    #[must_use]
    pub const fn root_constraints(&self) -> LayoutConstraints {
        self.root_constraints
    }

    #[must_use]
    pub const fn measurement_provider(&self) -> &'a dyn MeasurementProvider {
        self.measurement_provider
    }

    /// Returns the exact validated renderer raster scale.
    #[must_use]
    pub const fn raster_scale(&self) -> RasterScale {
        self.raster_scale
    }
}
