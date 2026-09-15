//! Host-neutral deterministic motion description values.
//!
//! This module owns authored transition/timeline vocabulary only. Live motion
//! state, sampling time, reconciliation, redraw, and publication remain runtime
//! responsibilities.

use core::{error::Error, fmt, num::NonZeroU64};
use std::{sync::Arc, time::Duration};

use crate::{
    Brush, Color, DropShadow, EdgeInsets, FlexBasis, IdentifierError, LayoutBound, LayoutDimension,
    LayoutFactor, LayoutGap, PresentationTransform, Radius, SceneOpacity, Typography, UnitInterval,
    identity::{IdentifierText, validate_identifier},
};

/// Validated owner-local identity for one explicit motion declaration.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AnimationId(IdentifierText);

impl AnimationId {
    /// Validates and owns a dynamic animation identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] under the canonical authored-identifier grammar.
    pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(IdentifierText::owned(value)))
    }

    /// Validates a static animation identifier without allocation.
    ///
    /// # Errors
    ///
    /// Returns [`IdentifierError`] under the canonical authored-identifier grammar.
    pub const fn from_static(value: &'static str) -> Result<Self, IdentifierError> {
        match validate_identifier(value) {
            Ok(()) => Ok(Self(IdentifierText::from_static(value))),
            Err(error) => Err(error),
        }
    }

    /// Returns the authored identifier text.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Closed initial M9 property identity accepted by deterministic motion.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MotionTarget {
    Foreground,
    Background,
    Padding,
    Radius,
    Typography,
    Shadows,
    Opacity,
    Presentation,
    Width,
    Height,
    MinWidth,
    MinHeight,
    MaxWidth,
    MaxHeight,
    Margin,
    Gap,
    FlexGrow,
    FlexShrink,
    FlexBasis,
}

impl MotionTarget {
    /// Returns whether this target belongs to authored structural layout.
    #[must_use]
    pub const fn is_structural_layout(self) -> bool {
        matches!(
            self,
            Self::Width
                | Self::Height
                | Self::MinWidth
                | Self::MinHeight
                | Self::MaxWidth
                | Self::MaxHeight
                | Self::Margin
                | Self::Gap
                | Self::FlexGrow
                | Self::FlexShrink
                | Self::FlexBasis
        )
    }
}

/// One typed value for exactly one accepted [`MotionTarget`].
///
/// Optional style domains preserve absence explicitly. The framework never
/// rewrites absence to transparent, zero, or identity motion endpoints.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum MotionValue {
    Foreground(Option<Color>),
    Background(Option<Brush>),
    Padding(Option<EdgeInsets>),
    Radius(Option<Radius>),
    Typography(Option<Typography>),
    Shadows(Vec<DropShadow>),
    Opacity(SceneOpacity),
    Presentation(Option<PresentationTransform>),
    Width(LayoutDimension),
    Height(LayoutDimension),
    MinWidth(LayoutBound),
    MinHeight(LayoutBound),
    MaxWidth(LayoutBound),
    MaxHeight(LayoutBound),
    Margin(EdgeInsets),
    Gap(LayoutGap),
    FlexGrow(LayoutFactor),
    FlexShrink(LayoutFactor),
    FlexBasis(FlexBasis),
}

impl MotionValue {
    /// Returns the exact property identity encoded by this value.
    #[must_use]
    pub const fn target(&self) -> MotionTarget {
        match self {
            Self::Foreground(_) => MotionTarget::Foreground,
            Self::Background(_) => MotionTarget::Background,
            Self::Padding(_) => MotionTarget::Padding,
            Self::Radius(_) => MotionTarget::Radius,
            Self::Typography(_) => MotionTarget::Typography,
            Self::Shadows(_) => MotionTarget::Shadows,
            Self::Opacity(_) => MotionTarget::Opacity,
            Self::Presentation(_) => MotionTarget::Presentation,
            Self::Width(_) => MotionTarget::Width,
            Self::Height(_) => MotionTarget::Height,
            Self::MinWidth(_) => MotionTarget::MinWidth,
            Self::MinHeight(_) => MotionTarget::MinHeight,
            Self::MaxWidth(_) => MotionTarget::MaxWidth,
            Self::MaxHeight(_) => MotionTarget::MaxHeight,
            Self::Margin(_) => MotionTarget::Margin,
            Self::Gap(_) => MotionTarget::Gap,
            Self::FlexGrow(_) => MotionTarget::FlexGrow,
            Self::FlexShrink(_) => MotionTarget::FlexShrink,
            Self::FlexBasis(_) => MotionTarget::FlexBasis,
        }
    }
}

/// Validated cubic-Bezier timing controls with implicit endpoints `(0,0)` and `(1,1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    x1: UnitInterval,
    y1: UnitInterval,
    x2: UnitInterval,
    y2: UnitInterval,
}

impl CubicBezier {
    #[must_use]
    pub const fn new(
        x1: UnitInterval,
        y1: UnitInterval,
        x2: UnitInterval,
        y2: UnitInterval,
    ) -> Self {
        Self { x1, y1, x2, y2 }
    }

    #[must_use]
    pub const fn x1(self) -> UnitInterval {
        self.x1
    }

    #[must_use]
    pub const fn y1(self) -> UnitInterval {
        self.y1
    }

    #[must_use]
    pub const fn x2(self) -> UnitInterval {
        self.x2
    }

    #[must_use]
    pub const fn y2(self) -> UnitInterval {
        self.y2
    }
}

/// Initial deterministic timing functions.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MotionEasing {
    #[default]
    Linear,
    CubicBezier(CubicBezier),
}

/// Explicit reduced-motion strategy retained by one authored motion source.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReducedMotionStrategy {
    SnapToEnd,
    HoldInitial,
    PreserveEssential,
}

/// Total iteration policy for one explicit timeline.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionRepeat {
    Finite(NonZeroU64),
    Forever,
}

impl MotionRepeat {
    pub const ONCE: Self = Self::Finite(NonZeroU64::MIN);

    #[must_use]
    pub const fn finite(iterations: NonZeroU64) -> Self {
        Self::Finite(iterations)
    }
}

impl Default for MotionRepeat {
    fn default() -> Self {
        Self::ONCE
    }
}

/// Validation failure for transition/timeline authored motion.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionSpecError {
    DurationOutOfRange,
    DelayOutOfRange,
    RelativeScheduleOverflow,
    ZeroDurationForever,
    TransitionHoldInitial,
    ForeverSnapToEnd,
    TooFewKeyframes,
    FirstKeyframeNotZero,
    LastKeyframeNotOne,
    KeyframeOffsetsNotIncreasing {
        index: usize,
    },
    KeyframeTargetMismatch {
        index: usize,
        expected: MotionTarget,
        actual: MotionTarget,
    },
    EasingCountMismatch {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for MotionSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DurationOutOfRange => formatter
                .write_str("motion duration must fit the runtime-relative u64 nanosecond domain"),
            Self::DelayOutOfRange => formatter
                .write_str("motion delay must fit the runtime-relative u64 nanosecond domain"),
            Self::RelativeScheduleOverflow => formatter.write_str(
                "finite motion schedule exceeds the runtime-relative u64 nanosecond domain",
            ),
            Self::ZeroDurationForever => {
                formatter.write_str("a forever timeline requires a strictly positive duration")
            }
            Self::TransitionHoldInitial => formatter
                .write_str("style transitions cannot use HoldInitial reduced-motion strategy"),
            Self::ForeverSnapToEnd => formatter
                .write_str("forever timelines cannot use SnapToEnd reduced-motion strategy"),
            Self::TooFewKeyframes => {
                formatter.write_str("a timeline requires at least two keyframes")
            }
            Self::FirstKeyframeNotZero => {
                formatter.write_str("the first timeline keyframe offset must be exactly 0")
            }
            Self::LastKeyframeNotOne => {
                formatter.write_str("the last timeline keyframe offset must be exactly 1")
            }
            Self::KeyframeOffsetsNotIncreasing { index } => write!(
                formatter,
                "timeline keyframe offsets must be strictly increasing at index {index}"
            ),
            Self::KeyframeTargetMismatch {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "timeline keyframe {index} targets {actual:?}, expected {expected:?}"
            ),
            Self::EasingCountMismatch { expected, actual } => write!(
                formatter,
                "timeline requires {expected} segment easings but received {actual}"
            ),
        }
    }
}

impl Error for MotionSpecError {}

fn checked_duration_nanos(
    duration: Duration,
    error: MotionSpecError,
) -> Result<u64, MotionSpecError> {
    u64::try_from(duration.as_nanos()).map_err(|_| error)
}

/// One validated style-transition specification.
#[derive(Clone, Debug, PartialEq)]
pub struct TransitionSpec {
    duration: Duration,
    delay: Duration,
    easing: MotionEasing,
    reduced_motion: ReducedMotionStrategy,
}

impl TransitionSpec {
    /// Validates one transition including its complete finite relative schedule.
    ///
    /// `None` selects the accepted default reduced-motion strategy, `SnapToEnd`.
    ///
    /// # Errors
    ///
    /// Rejects unrepresentable timing, relative terminal overflow, or
    /// `HoldInitial`, which is not valid for style transitions.
    pub fn new(
        duration: Duration,
        delay: Duration,
        easing: MotionEasing,
        reduced_motion: Option<ReducedMotionStrategy>,
    ) -> Result<Self, MotionSpecError> {
        let duration_nanos = checked_duration_nanos(duration, MotionSpecError::DurationOutOfRange)?;
        let delay_nanos = checked_duration_nanos(delay, MotionSpecError::DelayOutOfRange)?;
        delay_nanos
            .checked_add(duration_nanos)
            .ok_or(MotionSpecError::RelativeScheduleOverflow)?;
        let reduced_motion = reduced_motion.unwrap_or(ReducedMotionStrategy::SnapToEnd);
        if reduced_motion == ReducedMotionStrategy::HoldInitial {
            return Err(MotionSpecError::TransitionHoldInitial);
        }
        Ok(Self {
            duration,
            delay,
            easing,
            reduced_motion,
        })
    }

    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    #[must_use]
    pub const fn delay(&self) -> Duration {
        self.delay
    }

    #[must_use]
    pub const fn easing(&self) -> MotionEasing {
        self.easing
    }

    #[must_use]
    pub const fn reduced_motion(&self) -> ReducedMotionStrategy {
        self.reduced_motion
    }
}

/// One style-layer contribution for a target's transition policy.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum TransitionPolicy {
    Disabled,
    Enabled(TransitionSpec),
}

/// One validated explicit timeline keyframe.
#[derive(Clone, Debug, PartialEq)]
pub struct MotionKeyframe {
    offset: UnitInterval,
    value: MotionValue,
}

impl MotionKeyframe {
    #[must_use]
    pub const fn new(offset: UnitInterval, value: MotionValue) -> Self {
        Self { offset, value }
    }

    #[must_use]
    pub const fn offset(&self) -> UnitInterval {
        self.offset
    }

    #[must_use]
    pub const fn value(&self) -> &MotionValue {
        &self.value
    }
}

/// Immutable validated one-target explicit timeline specification.
#[derive(Clone, Debug, PartialEq)]
pub struct TimelineSpec {
    target: MotionTarget,
    keyframes: Arc<[MotionKeyframe]>,
    easings: Arc<[MotionEasing]>,
    duration: Duration,
    delay: Duration,
    repeat: MotionRepeat,
    reduced_motion: ReducedMotionStrategy,
}

impl TimelineSpec {
    /// Validates a complete explicit timeline description.
    ///
    /// `easings` contains exactly one timing function per adjacent keyframe
    /// segment. `None` selects `SnapToEnd` for finite timelines and
    /// `HoldInitial` for forever timelines.
    ///
    /// # Errors
    ///
    /// Rejects malformed keyframes, mixed targets, timing overflow, invalid
    /// forever zero-duration timing, or an invalid reduced-motion strategy.
    pub fn new(
        keyframes: impl Into<Vec<MotionKeyframe>>,
        easings: impl Into<Vec<MotionEasing>>,
        duration: Duration,
        delay: Duration,
        repeat: MotionRepeat,
        reduced_motion: Option<ReducedMotionStrategy>,
    ) -> Result<Self, MotionSpecError> {
        let keyframes = keyframes.into();
        if keyframes.len() < 2 {
            return Err(MotionSpecError::TooFewKeyframes);
        }
        if keyframes[0].offset() != UnitInterval::ZERO {
            return Err(MotionSpecError::FirstKeyframeNotZero);
        }
        if keyframes[keyframes.len() - 1].offset() != UnitInterval::ONE {
            return Err(MotionSpecError::LastKeyframeNotOne);
        }
        for (index, pair) in keyframes.windows(2).enumerate() {
            if pair[0].offset().get() >= pair[1].offset().get() {
                return Err(MotionSpecError::KeyframeOffsetsNotIncreasing { index: index + 1 });
            }
        }

        let target = keyframes[0].value().target();
        for (index, keyframe) in keyframes.iter().enumerate().skip(1) {
            let actual = keyframe.value().target();
            if actual != target {
                return Err(MotionSpecError::KeyframeTargetMismatch {
                    index,
                    expected: target,
                    actual,
                });
            }
        }

        let easings = easings.into();
        let expected_easings = keyframes.len() - 1;
        if easings.len() != expected_easings {
            return Err(MotionSpecError::EasingCountMismatch {
                expected: expected_easings,
                actual: easings.len(),
            });
        }

        let duration_nanos = checked_duration_nanos(duration, MotionSpecError::DurationOutOfRange)?;
        let delay_nanos = checked_duration_nanos(delay, MotionSpecError::DelayOutOfRange)?;
        let reduced_motion = match repeat {
            MotionRepeat::Finite(iterations) => {
                let active_nanos = duration_nanos
                    .checked_mul(iterations.get())
                    .ok_or(MotionSpecError::RelativeScheduleOverflow)?;
                delay_nanos
                    .checked_add(active_nanos)
                    .ok_or(MotionSpecError::RelativeScheduleOverflow)?;
                reduced_motion.unwrap_or(ReducedMotionStrategy::SnapToEnd)
            }
            MotionRepeat::Forever => {
                if duration_nanos == 0 {
                    return Err(MotionSpecError::ZeroDurationForever);
                }
                let strategy = reduced_motion.unwrap_or(ReducedMotionStrategy::HoldInitial);
                if strategy == ReducedMotionStrategy::SnapToEnd {
                    return Err(MotionSpecError::ForeverSnapToEnd);
                }
                strategy
            }
        };

        Ok(Self {
            target,
            keyframes: Arc::from(keyframes),
            easings: Arc::from(easings),
            duration,
            delay,
            repeat,
            reduced_motion,
        })
    }

    #[must_use]
    pub const fn target(&self) -> MotionTarget {
        self.target
    }

    #[must_use]
    pub fn keyframes(&self) -> &[MotionKeyframe] {
        self.keyframes.as_ref()
    }

    #[must_use]
    pub fn easings(&self) -> &[MotionEasing] {
        self.easings.as_ref()
    }

    #[must_use]
    pub const fn duration(&self) -> Duration {
        self.duration
    }

    #[must_use]
    pub const fn delay(&self) -> Duration {
        self.delay
    }

    #[must_use]
    pub const fn repeat(&self) -> MotionRepeat {
        self.repeat
    }

    #[must_use]
    pub const fn reduced_motion(&self) -> ReducedMotionStrategy {
        self.reduced_motion
    }
}

/// One owner-local declarative explicit timeline.
#[derive(Clone, Debug, PartialEq)]
pub struct ExplicitTimeline {
    id: AnimationId,
    spec: TimelineSpec,
}

impl ExplicitTimeline {
    #[must_use]
    pub const fn new(id: AnimationId, spec: TimelineSpec) -> Self {
        Self { id, spec }
    }

    #[must_use]
    pub const fn id(&self) -> &AnimationId {
        &self.id
    }

    #[must_use]
    pub const fn spec(&self) -> &TimelineSpec {
        &self.spec
    }

    #[must_use]
    pub const fn target(&self) -> MotionTarget {
        self.spec.target()
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;
    use std::time::Duration;

    use super::{
        MotionEasing, MotionKeyframe, MotionRepeat, MotionSpecError, MotionTarget, MotionValue,
        ReducedMotionStrategy, TimelineSpec, TransitionSpec,
    };
    use crate::{Color, UnitInterval};

    fn color_keyframe(offset: UnitInterval, color: Color) -> MotionKeyframe {
        MotionKeyframe::new(offset, MotionValue::Foreground(Some(color)))
    }

    #[test]
    fn motion_value_preserves_absent_typography_endpoint() {
        assert_eq!(
            MotionValue::Typography(None).target(),
            MotionTarget::Typography
        );
    }

    #[test]
    fn transition_validates_relative_schedule_and_strategy() {
        assert_eq!(
            TransitionSpec::new(
                Duration::ZERO,
                Duration::ZERO,
                MotionEasing::Linear,
                Some(ReducedMotionStrategy::HoldInitial),
            ),
            Err(MotionSpecError::TransitionHoldInitial)
        );
        assert_eq!(
            TransitionSpec::new(
                Duration::from_nanos(u64::MAX),
                Duration::from_nanos(1),
                MotionEasing::Linear,
                None,
            ),
            Err(MotionSpecError::RelativeScheduleOverflow)
        );
    }

    #[test]
    fn timeline_requires_exact_ordered_endpoints_and_one_target() {
        let half = UnitInterval::HALF;
        let valid = TimelineSpec::new(
            vec![
                color_keyframe(UnitInterval::ZERO, Color::BLACK),
                color_keyframe(half, Color::WHITE),
                color_keyframe(UnitInterval::ONE, Color::BLACK),
            ],
            vec![MotionEasing::Linear, MotionEasing::Linear],
            Duration::from_millis(100),
            Duration::ZERO,
            MotionRepeat::ONCE,
            None,
        )
        .unwrap_or_else(|_| unreachable!("test timeline is valid"));
        assert_eq!(valid.target(), MotionTarget::Foreground);

        assert_eq!(
            TimelineSpec::new(
                vec![
                    color_keyframe(UnitInterval::ZERO, Color::BLACK),
                    MotionKeyframe::new(
                        UnitInterval::ONE,
                        MotionValue::Opacity(crate::SceneOpacity::OPAQUE),
                    ),
                ],
                vec![MotionEasing::Linear],
                Duration::from_millis(100),
                Duration::ZERO,
                MotionRepeat::ONCE,
                None,
            ),
            Err(MotionSpecError::KeyframeTargetMismatch {
                index: 1,
                expected: MotionTarget::Foreground,
                actual: MotionTarget::Opacity,
            })
        );
    }

    #[test]
    fn forever_timing_and_reduced_motion_are_source_validated() {
        assert_eq!(
            TimelineSpec::new(
                vec![
                    color_keyframe(UnitInterval::ZERO, Color::BLACK),
                    color_keyframe(UnitInterval::ONE, Color::WHITE),
                ],
                vec![MotionEasing::Linear],
                Duration::ZERO,
                Duration::ZERO,
                MotionRepeat::Forever,
                None,
            ),
            Err(MotionSpecError::ZeroDurationForever)
        );
        assert_eq!(
            TimelineSpec::new(
                vec![
                    color_keyframe(UnitInterval::ZERO, Color::BLACK),
                    color_keyframe(UnitInterval::ONE, Color::WHITE),
                ],
                vec![MotionEasing::Linear],
                Duration::from_millis(1),
                Duration::ZERO,
                MotionRepeat::Forever,
                Some(ReducedMotionStrategy::SnapToEnd),
            ),
            Err(MotionSpecError::ForeverSnapToEnd)
        );

        let finite = MotionRepeat::finite(
            NonZeroU64::new(2).unwrap_or_else(|| unreachable!("two is non-zero")),
        );
        let held = TimelineSpec::new(
            vec![
                color_keyframe(UnitInterval::ZERO, Color::BLACK),
                color_keyframe(UnitInterval::ONE, Color::WHITE),
            ],
            vec![MotionEasing::Linear],
            Duration::from_millis(1),
            Duration::ZERO,
            finite,
            Some(ReducedMotionStrategy::HoldInitial),
        )
        .unwrap_or_else(|_| unreachable!("finite hold strategy is valid"));
        assert_eq!(held.reduced_motion(), ReducedMotionStrategy::HoldInitial);
    }
}
