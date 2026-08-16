use core::{error::Error, fmt};

use runenui_core::{LogicalSize, StyleTokens};
use runenui_runtime::DeterministicMeasurementProvider;

/// Named non-zero default surface used by deterministic harness publication.
pub const DEFAULT_TEST_SURFACE_SIZE: LogicalSize = match LogicalSize::try_new(800.0, 600.0) {
    Ok(size) => size,
    Err(_) => LogicalSize::ZERO,
};

/// Invalid deterministic test-surface configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestSurfaceConfigError {
    /// Width and height must both be non-zero.
    ZeroExtent,
}

impl fmt::Display for TestSurfaceConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExtent => formatter.write_str("test surface width and height must be non-zero"),
        }
    }
}

impl Error for TestSurfaceConfigError {}

/// Deterministic fixed rectangular surface configuration owned by the harness.
#[derive(Clone, Debug, PartialEq)]
pub struct TestSurfaceConfig {
    size: LogicalSize,
    style_tokens: StyleTokens,
    measurement: DeterministicMeasurementProvider,
}

impl Default for TestSurfaceConfig {
    fn default() -> Self {
        Self {
            size: DEFAULT_TEST_SURFACE_SIZE,
            style_tokens: StyleTokens::new(),
            measurement: DeterministicMeasurementProvider::DEFAULT,
        }
    }
}

impl TestSurfaceConfig {
    /// Creates a deterministic fixed-size surface.
    ///
    /// # Errors
    ///
    /// Returns [`TestSurfaceConfigError::ZeroExtent`] when either extent is zero.
    pub fn new(size: LogicalSize) -> Result<Self, TestSurfaceConfigError> {
        if size.width() == 0.0 || size.height() == 0.0 {
            return Err(TestSurfaceConfigError::ZeroExtent);
        }
        Ok(Self {
            size,
            ..Self::default()
        })
    }

    /// Returns the fixed logical surface size.
    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    /// Returns the style-token set used by ordinary surface publication.
    #[must_use]
    pub const fn style_tokens(&self) -> &StyleTokens {
        &self.style_tokens
    }

    /// Returns the deterministic default measurement provider.
    #[must_use]
    pub const fn measurement_provider(&self) -> DeterministicMeasurementProvider {
        self.measurement
    }

    /// Replaces the harness-owned style-token set.
    #[must_use]
    pub fn with_style_tokens(mut self, style_tokens: StyleTokens) -> Self {
        self.style_tokens = style_tokens;
        self
    }

    /// Replaces the deterministic text metrics used by default publication.
    #[must_use]
    pub const fn with_measurement_provider(
        mut self,
        measurement: DeterministicMeasurementProvider,
    ) -> Self {
        self.measurement = measurement;
        self
    }
}
