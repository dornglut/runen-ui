//! Renderer-neutral intrinsic text measurement contracts.

use crate::{LayoutConstraints, LogicalSize, MountedNodeId};
use core::{error::Error, fmt};
use runenui_core::LogicalLength;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextMeasurementKind {
    Text,
    ControlLabel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextMeasurementRequest<'a> {
    content: &'a str,
    constraints: LayoutConstraints,
    node_id: Option<MountedNodeId>,
    kind: TextMeasurementKind,
}

impl<'a> TextMeasurementRequest<'a> {
    #[must_use]
    pub const fn new(
        content: &'a str,
        constraints: LayoutConstraints,
        kind: TextMeasurementKind,
    ) -> Self {
        Self {
            content,
            constraints,
            node_id: None,
            kind,
        }
    }
    #[must_use]
    pub fn with_node_id(mut self, node_id: MountedNodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }
    #[must_use]
    pub const fn content(&self) -> &'a str {
        self.content
    }
    #[must_use]
    pub const fn constraints(&self) -> LayoutConstraints {
        self.constraints
    }
    #[must_use]
    pub const fn node_id(&self) -> Option<&MountedNodeId> {
        self.node_id.as_ref()
    }
    #[must_use]
    pub const fn kind(&self) -> TextMeasurementKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineError {
    NotFinite,
    Negative,
    ExceedsHeight,
}

impl fmt::Display for BaselineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("baseline must be finite"),
            Self::Negative => formatter.write_str("baseline must not be negative"),
            Self::ExceedsHeight => formatter.write_str("baseline must not exceed measured height"),
        }
    }
}

impl Error for BaselineError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMeasurement {
    size: LogicalSize,
    first_baseline: Option<f32>,
    last_baseline: Option<f32>,
}

impl TextMeasurement {
    #[must_use]
    pub const fn new(size: LogicalSize) -> Self {
        Self {
            size,
            first_baseline: None,
            last_baseline: None,
        }
    }
    /// Adds a validated first baseline.
    ///
    /// # Errors
    ///
    /// Returns [`BaselineError`] when the baseline is non-finite, negative, or
    /// exceeds the measured height.
    pub fn with_first_baseline(mut self, baseline: f32) -> Result<Self, BaselineError> {
        validate_baseline(baseline, self.size.height())?;
        self.first_baseline = Some(baseline);
        Ok(self)
    }
    /// Adds a validated last baseline.
    ///
    /// # Errors
    ///
    /// Returns [`BaselineError`] under the same rules as
    /// [`Self::with_first_baseline`].
    pub fn with_last_baseline(mut self, baseline: f32) -> Result<Self, BaselineError> {
        validate_baseline(baseline, self.size.height())?;
        self.last_baseline = Some(baseline);
        Ok(self)
    }
    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }
    #[must_use]
    pub const fn first_baseline(&self) -> Option<f32> {
        self.first_baseline
    }
    #[must_use]
    pub const fn last_baseline(&self) -> Option<f32> {
        self.last_baseline
    }
}

fn validate_baseline(value: f32, height: f32) -> Result<(), BaselineError> {
    if !value.is_finite() {
        Err(BaselineError::NotFinite)
    } else if value < 0.0 {
        Err(BaselineError::Negative)
    } else if value > height {
        Err(BaselineError::ExceedsHeight)
    } else {
        Ok(())
    }
}

/// Open synchronous measurement service; providers are intentionally downstream-implementable.
pub trait MeasurementProvider {
    /// Stable identity for publication-cache compatibility.
    ///
    /// The provider must change this identity or [`Self::cache_revision`]
    /// whenever any behavior that can affect a measurement changes. Reusing
    /// both values is an explicit promise that cached measurements remain
    /// compatible.
    fn cache_identity(&self) -> u64;
    /// Revision of measurement behavior for the stable identity.
    fn cache_revision(&self) -> u64;
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeterministicMeasurementProvider {
    char_width: LogicalLength,
    line_height: LogicalLength,
}

impl Default for DeterministicMeasurementProvider {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl DeterministicMeasurementProvider {
    pub const DEFAULT: Self = Self {
        char_width: match LogicalLength::new(8.0) {
            Ok(value) => value,
            Err(_) => LogicalLength::ZERO,
        },
        line_height: match LogicalLength::new(20.0) {
            Ok(value) => value,
            Err(_) => LogicalLength::ZERO,
        },
    };
    #[must_use]
    pub const fn new(char_width: LogicalLength, line_height: LogicalLength) -> Self {
        Self {
            char_width,
            line_height,
        }
    }
    #[must_use]
    pub const fn char_width(&self) -> f32 {
        self.char_width.get()
    }
    #[must_use]
    pub const fn line_height(&self) -> f32 {
        self.line_height.get()
    }
}

impl MeasurementProvider for DeterministicMeasurementProvider {
    fn cache_identity(&self) -> u64 {
        (u64::from(self.char_width.get().to_bits()) << 32)
            | u64::from(self.line_height.get().to_bits())
    }

    fn cache_revision(&self) -> u64 {
        0
    }

    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        let count = f32::from(u16::try_from(request.content().chars().count()).unwrap_or(u16::MAX));
        let width = count * self.char_width.get();
        let width = if width.is_finite() { width } else { f32::MAX };
        let width = LogicalLength::new(width)
            .unwrap_or_else(|_| unreachable!("bounded text measurement width is a valid extent"));
        let intrinsic = LogicalSize::new(width, self.line_height);
        TextMeasurement::new(request.constraints().constrain(intrinsic))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BaselineError, DeterministicMeasurementProvider, MeasurementProvider, TextMeasurement,
        TextMeasurementKind, TextMeasurementRequest,
    };
    use crate::{LayoutConstraints, LogicalSize};
    use runenui_core::LogicalLength;

    fn length(value: f32) -> LogicalLength {
        LogicalLength::new(value).unwrap_or_default()
    }
    fn size(width: f32, height: f32) -> LogicalSize {
        LogicalSize::new(length(width), length(height))
    }

    #[test]
    fn invalid_baselines_are_rejected() {
        let measurement = TextMeasurement::new(size(20.0, 10.0));
        assert_eq!(
            measurement.with_first_baseline(f32::NAN),
            Err(BaselineError::NotFinite)
        );
        assert_eq!(
            measurement.with_first_baseline(-1.0),
            Err(BaselineError::Negative)
        );
        assert_eq!(
            measurement.with_first_baseline(11.0),
            Err(BaselineError::ExceedsHeight)
        );
        assert!(measurement.with_first_baseline(10.0).is_ok());
    }

    #[test]
    fn deterministic_provider_is_constrained_and_object_safe() {
        let provider = DeterministicMeasurementProvider::new(length(10.0), length(18.0));
        let provider: &dyn MeasurementProvider = &provider;
        let request = TextMeasurementRequest::new(
            "1234567890",
            LayoutConstraints::loose(size(60.0, 12.0)),
            TextMeasurementKind::Text,
        );
        assert_eq!(provider.measure_text(&request).size(), size(60.0, 12.0));
    }
}
