use runenui_core::StyleTokens;

use crate::{
    DeterministicMeasurementProvider, LayoutConstraints, LogicalSize, MeasurementProvider,
};

static DEFAULT_MEASUREMENT_PROVIDER: DeterministicMeasurementProvider =
    DeterministicMeasurementProvider::DEFAULT;

/// Explicit inputs used to publish one surface snapshot.
#[derive(Clone, Copy)]
pub struct SurfaceBuildContext<'a> {
    style_tokens: &'a StyleTokens,
    root_constraints: LayoutConstraints,
    measurement_provider: &'a dyn MeasurementProvider,
}

impl<'a> SurfaceBuildContext<'a> {
    #[must_use]
    pub fn new(style_tokens: &'a StyleTokens, root_constraints: LayoutConstraints) -> Self {
        Self {
            style_tokens,
            root_constraints,
            measurement_provider: &DEFAULT_MEASUREMENT_PROVIDER,
        }
    }

    #[must_use]
    pub fn tight(style_tokens: &'a StyleTokens, size: LogicalSize) -> Self {
        Self::new(style_tokens, LayoutConstraints::tight(size))
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

    #[must_use]
    pub const fn style_tokens(&self) -> &'a StyleTokens {
        self.style_tokens
    }

    #[must_use]
    pub const fn root_constraints(&self) -> LayoutConstraints {
        self.root_constraints
    }

    #[must_use]
    pub const fn measurement_provider(&self) -> &'a dyn MeasurementProvider {
        self.measurement_provider
    }
}
