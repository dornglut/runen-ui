//! Renderer-neutral intrinsic text measurement contracts.

use crate::{LayoutConstraints, LogicalSize, RuntimeNodeId};

/// Semantic use of a text measurement request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextMeasurementKind {
    /// Standalone text content.
    Text,
    /// Text used as a button label.
    ButtonLabel,
}

/// Borrowed text measurement input for one publication pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMeasurementRequest<'a> {
    content: &'a str,
    constraints: LayoutConstraints,
    node_id: Option<RuntimeNodeId>,
    kind: TextMeasurementKind,
}

impl<'a> TextMeasurementRequest<'a> {
    /// Creates a text measurement request.
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

    /// Adds runtime identity for diagnostics and observation.
    #[must_use]
    pub const fn with_node_id(mut self, node_id: RuntimeNodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    /// Returns the text content to measure.
    #[must_use]
    pub const fn content(&self) -> &'a str {
        self.content
    }

    /// Returns the content-box constraints supplied by layout.
    #[must_use]
    pub const fn constraints(&self) -> LayoutConstraints {
        self.constraints
    }

    /// Returns optional runtime identity used only for observation.
    #[must_use]
    pub const fn node_id(&self) -> Option<RuntimeNodeId> {
        self.node_id
    }

    /// Returns the semantic text use.
    #[must_use]
    pub const fn kind(&self) -> TextMeasurementKind {
        self.kind
    }
}

/// Logical content measurement returned by a provider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextMeasurement {
    size: LogicalSize,
    first_baseline: Option<f32>,
    last_baseline: Option<f32>,
}

impl TextMeasurement {
    /// Creates a measurement without baseline information.
    #[must_use]
    pub const fn new(size: LogicalSize) -> Self {
        Self {
            size,
            first_baseline: None,
            last_baseline: None,
        }
    }

    /// Adds a first baseline in logical coordinates.
    #[must_use]
    pub const fn with_first_baseline(mut self, baseline: f32) -> Self {
        self.first_baseline = Some(baseline);
        self
    }

    /// Adds a last baseline in logical coordinates.
    #[must_use]
    pub const fn with_last_baseline(mut self, baseline: f32) -> Self {
        self.last_baseline = Some(baseline);
        self
    }

    /// Returns the measured content-box size.
    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    /// Returns the first baseline, when supplied.
    #[must_use]
    pub const fn first_baseline(&self) -> Option<f32> {
        self.first_baseline
    }

    /// Returns the last baseline, when supplied.
    #[must_use]
    pub const fn last_baseline(&self) -> Option<f32> {
        self.last_baseline
    }
}

/// Synchronous renderer-neutral text measurement service.
pub trait MeasurementProvider {
    /// Measures one text request under its content-box constraints.
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement;
}

/// Deterministic character-count provider for tests and headless examples.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeterministicMeasurementProvider {
    char_width: f32,
    line_height: f32,
}

impl Default for DeterministicMeasurementProvider {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl DeterministicMeasurementProvider {
    /// Default deterministic metrics for headless publication.
    pub const DEFAULT: Self = Self {
        char_width: 8.0,
        line_height: 20.0,
    };

    /// Creates a deterministic provider from normalized logical metrics.
    #[must_use]
    pub fn new(char_width: f32, line_height: f32) -> Self {
        Self {
            char_width: normalize_metric(char_width),
            line_height: normalize_metric(line_height),
        }
    }

    /// Returns the logical width assigned to one Unicode scalar value.
    #[must_use]
    pub const fn char_width(&self) -> f32 {
        self.char_width
    }

    /// Returns the fixed logical line height.
    #[must_use]
    pub const fn line_height(&self) -> f32 {
        self.line_height
    }
}

impl MeasurementProvider for DeterministicMeasurementProvider {
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        let character_count = count_as_f32(request.content().chars().count());
        let intrinsic = LogicalSize::new(
            finite_product(character_count, self.char_width),
            self.line_height,
        );

        TextMeasurement::new(request.constraints().constrain(intrinsic))
    }
}

fn normalize_metric(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn finite_product(left: f32, right: f32) -> f32 {
    let product = left * right;
    if product.is_finite() {
        product
    } else {
        f32::MAX
    }
}

fn count_as_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

#[cfg(test)]
mod tests {
    use super::{
        DeterministicMeasurementProvider, MeasurementProvider, TextMeasurement,
        TextMeasurementKind, TextMeasurementRequest,
    };
    use crate::{LayoutConstraints, LogicalSize, RuntimeNodeId};

    fn assert_f32_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= f32::EPSILON,
            "expected {expected}, got {actual}",
        );
    }

    #[test]
    fn request_preserves_content_constraints_kind_and_observation_identity() {
        let constraints = LayoutConstraints::loose(LogicalSize::new(100.0, 40.0));
        let node_id = RuntimeNodeId::from_index(7);
        let request =
            TextMeasurementRequest::new("Counter", constraints, TextMeasurementKind::ButtonLabel)
                .with_node_id(node_id);

        assert_eq!(request.content(), "Counter");
        assert_eq!(request.constraints(), constraints);
        assert_eq!(request.kind(), TextMeasurementKind::ButtonLabel);
        assert_eq!(request.node_id(), Some(node_id));
    }

    #[test]
    fn measurement_can_carry_optional_baselines() {
        let measurement = TextMeasurement::new(LogicalSize::new(40.0, 20.0))
            .with_first_baseline(14.0)
            .with_last_baseline(18.0);

        assert_eq!(measurement.size(), LogicalSize::new(40.0, 20.0));
        assert_f32_eq(measurement.first_baseline().unwrap_or_default(), 14.0);
        assert_f32_eq(measurement.last_baseline().unwrap_or_default(), 18.0);
    }

    #[test]
    fn deterministic_provider_uses_one_text_path_for_all_text_kinds() {
        let provider = DeterministicMeasurementProvider::new(10.0, 18.0);
        let constraints = LayoutConstraints::unbounded();
        let text = TextMeasurementRequest::new("ABC", constraints, TextMeasurementKind::Text);
        let button =
            TextMeasurementRequest::new("ABC", constraints, TextMeasurementKind::ButtonLabel);

        assert_eq!(
            provider.measure_text(&text).size(),
            LogicalSize::new(30.0, 18.0),
        );
        assert_eq!(provider.measure_text(&button), provider.measure_text(&text));
    }

    #[test]
    fn deterministic_provider_applies_request_constraints() {
        let provider = DeterministicMeasurementProvider::new(10.0, 18.0);
        let loose = TextMeasurementRequest::new(
            "1234567890",
            LayoutConstraints::loose(LogicalSize::new(60.0, 12.0)),
            TextMeasurementKind::Text,
        );
        let tight = TextMeasurementRequest::new(
            "A",
            LayoutConstraints::tight(LogicalSize::new(50.0, 30.0)),
            TextMeasurementKind::Text,
        );

        assert_eq!(
            provider.measure_text(&loose).size(),
            LogicalSize::new(60.0, 12.0),
        );
        assert_eq!(
            provider.measure_text(&tight).size(),
            LogicalSize::new(50.0, 30.0),
        );
    }

    #[test]
    fn invalid_deterministic_metrics_normalize_to_zero() {
        let provider = DeterministicMeasurementProvider::new(f32::NAN, -4.0);
        let request = TextMeasurementRequest::new(
            "ABC",
            LayoutConstraints::unbounded(),
            TextMeasurementKind::Text,
        );

        assert_f32_eq(provider.char_width(), 0.0);
        assert_f32_eq(provider.line_height(), 0.0);
        assert_eq!(
            provider.measure_text(&request).size(),
            LogicalSize::new(0.0, 0.0),
        );
    }

    #[test]
    fn provider_contract_is_object_safe() {
        let provider = DeterministicMeasurementProvider::default();
        let provider: &dyn MeasurementProvider = &provider;
        let request = TextMeasurementRequest::new(
            "A",
            LayoutConstraints::unbounded(),
            TextMeasurementKind::Text,
        );

        assert_eq!(
            provider.measure_text(&request).size(),
            LogicalSize::new(8.0, 20.0),
        );
    }
}
