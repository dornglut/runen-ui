//! Deterministic value-level motion sampling shared with the mounted runtime.
//!
//! This module owns no clock, live lifetime, reconciliation, redraw, or publication
//! state. It only applies the exact M9 value/easing semantics to already-validated
//! neutral RunenUI values. The runtime remains the sole owner of when and why a
//! sample is taken.

use crate::{
    Brush, CubicBezier, DropShadow, EdgeInsets, FlexBasis, GradientStop, GradientStops,
    LayoutBound, LayoutDimension, LayoutFactor, LayoutGap, LinearGradient, LogicalLength,
    MotionEasing, MotionValue, PresentationOrigin, PresentationRotation, PresentationScale,
    PresentationTransform, PresentationTranslation, RadialGradient, Radius, SceneOpacity,
    UnitInterval,
};

/// Applies one accepted easing function to normalized progress.
///
/// This is exported only through the doc-hidden core/runtime bridge. It owns no
/// time source and therefore cannot create a second animation clock.
#[must_use]
pub fn ease_motion(easing: MotionEasing, progress: UnitInterval) -> UnitInterval {
    match easing {
        MotionEasing::Linear => progress,
        MotionEasing::CubicBezier(curve) => cubic_bezier_ease(curve, progress),
    }
}

/// Samples two values for one exact motion target at already-eased progress.
///
/// `None` is reserved for an impossible/mismatched target pair. Incompatible
/// endpoint domains are valid discrete motion: they hold the start value until
/// exact terminal progress and then switch to the end value.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "the closed M9 target vocabulary is intentionally sampled in one auditable dispatch"
)]
pub fn interpolate_motion_value(
    start: &MotionValue,
    end: &MotionValue,
    progress: UnitInterval,
) -> Option<MotionValue> {
    if start.target() != end.target() {
        return None;
    }
    if progress == UnitInterval::ZERO {
        return Some(start.clone());
    }
    if progress == UnitInterval::ONE {
        return Some(end.clone());
    }

    let discrete = || Some(start.clone());
    match (start, end) {
        (MotionValue::Foreground(Some(start)), MotionValue::Foreground(Some(end))) => Some(
            MotionValue::Foreground(Some(interpolate_color(*start, *end, progress))),
        ),
        (MotionValue::Foreground(_), MotionValue::Foreground(_)) => discrete(),
        (MotionValue::Background(Some(start)), MotionValue::Background(Some(end))) => {
            interpolate_brush(start, end, progress)
                .map(|brush| MotionValue::Background(Some(brush)))
                .or_else(discrete)
        }
        (MotionValue::Background(_), MotionValue::Background(_)) => discrete(),
        (MotionValue::Padding(Some(start)), MotionValue::Padding(Some(end))) => {
            interpolate_edge_insets(*start, *end, progress)
                .map(|value| MotionValue::Padding(Some(value)))
        }
        (MotionValue::Padding(_), MotionValue::Padding(_)) => discrete(),
        (MotionValue::Radius(Some(start)), MotionValue::Radius(Some(end))) => {
            interpolate_radius(*start, *end, progress)
                .map(|value| MotionValue::Radius(Some(value)))
        }
        (MotionValue::Radius(_), MotionValue::Radius(_)) => discrete(),
        (MotionValue::Typography(_), MotionValue::Typography(_)) => discrete(),
        (MotionValue::Shadows(start), MotionValue::Shadows(end)) => {
            interpolate_shadows(start, end, progress)
                .map(MotionValue::Shadows)
                .or_else(discrete)
        }
        (MotionValue::Opacity(start), MotionValue::Opacity(end)) => {
            interpolate_scene_opacity(*start, *end, progress).map(MotionValue::Opacity)
        }
        (MotionValue::Presentation(Some(start)), MotionValue::Presentation(Some(end))) => {
            interpolate_presentation(*start, *end, progress)
                .map(|value| MotionValue::Presentation(Some(value)))
        }
        (MotionValue::Presentation(_), MotionValue::Presentation(_)) => discrete(),
        (MotionValue::Width(start), MotionValue::Width(end)) => {
            Some(MotionValue::Width(interpolate_dimension(*start, *end, progress)?))
        }
        (MotionValue::Height(start), MotionValue::Height(end)) => {
            Some(MotionValue::Height(interpolate_dimension(*start, *end, progress)?))
        }
        (MotionValue::MinWidth(start), MotionValue::MinWidth(end)) => Some(
            MotionValue::MinWidth(interpolate_bound(*start, *end, progress)?),
        ),
        (MotionValue::MinHeight(start), MotionValue::MinHeight(end)) => Some(
            MotionValue::MinHeight(interpolate_bound(*start, *end, progress)?),
        ),
        (MotionValue::MaxWidth(start), MotionValue::MaxWidth(end)) => Some(
            MotionValue::MaxWidth(interpolate_bound(*start, *end, progress)?),
        ),
        (MotionValue::MaxHeight(start), MotionValue::MaxHeight(end)) => Some(
            MotionValue::MaxHeight(interpolate_bound(*start, *end, progress)?),
        ),
        (MotionValue::Margin(start), MotionValue::Margin(end)) => {
            interpolate_edge_insets(*start, *end, progress).map(MotionValue::Margin)
        }
        (MotionValue::Gap(start), MotionValue::Gap(end)) => {
            interpolate_gap(*start, *end, progress).map(MotionValue::Gap)
        }
        (MotionValue::FlexGrow(start), MotionValue::FlexGrow(end)) => {
            interpolate_layout_factor(*start, *end, progress).map(MotionValue::FlexGrow)
        }
        (MotionValue::FlexShrink(start), MotionValue::FlexShrink(end)) => {
            interpolate_layout_factor(*start, *end, progress).map(MotionValue::FlexShrink)
        }
        (MotionValue::FlexBasis(start), MotionValue::FlexBasis(end)) => {
            Some(MotionValue::FlexBasis(interpolate_flex_basis(*start, *end, progress)?))
        }
        _ => None,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::suboptimal_flops,
    reason = "ADR 0016 requires one f64 convex interpolation, one final f32 conversion, and forbids FMA/mul_add"
)]
fn interpolate_f32(start: f32, end: f32, progress: UnitInterval) -> f32 {
    if progress == UnitInterval::ZERO {
        return start;
    }
    if progress == UnitInterval::ONE {
        return end;
    }
    let progress = f64::from(progress.get());
    let left = (1.0 - progress) * f64::from(start);
    let right = progress * f64::from(end);
    (left + right) as f32
}

fn interpolate_logical_length(
    start: LogicalLength,
    end: LogicalLength,
    progress: UnitInterval,
) -> Option<LogicalLength> {
    LogicalLength::new(interpolate_f32(start.get(), end.get(), progress)).ok()
}

fn interpolate_layout_factor(
    start: LayoutFactor,
    end: LayoutFactor,
    progress: UnitInterval,
) -> Option<LayoutFactor> {
    LayoutFactor::new(interpolate_f32(start.get(), end.get(), progress)).ok()
}

fn interpolate_scene_opacity(
    start: SceneOpacity,
    end: SceneOpacity,
    progress: UnitInterval,
) -> Option<SceneOpacity> {
    SceneOpacity::new(interpolate_f32(start.get(), end.get(), progress)).ok()
}

fn interpolate_unit_interval(
    start: UnitInterval,
    end: UnitInterval,
    progress: UnitInterval,
) -> Option<UnitInterval> {
    UnitInterval::new(interpolate_f32(start.get(), end.get(), progress)).ok()
}

fn interpolate_edge_insets(
    start: EdgeInsets,
    end: EdgeInsets,
    progress: UnitInterval,
) -> Option<EdgeInsets> {
    Some(EdgeInsets::new(
        interpolate_logical_length(start.top(), end.top(), progress)?,
        interpolate_logical_length(start.right(), end.right(), progress)?,
        interpolate_logical_length(start.bottom(), end.bottom(), progress)?,
        interpolate_logical_length(start.left(), end.left(), progress)?,
    ))
}

fn interpolate_radius(start: Radius, end: Radius, progress: UnitInterval) -> Option<Radius> {
    Some(Radius::new(
        interpolate_logical_length(start.top_left(), end.top_left(), progress)?,
        interpolate_logical_length(start.top_right(), end.top_right(), progress)?,
        interpolate_logical_length(start.bottom_right(), end.bottom_right(), progress)?,
        interpolate_logical_length(start.bottom_left(), end.bottom_left(), progress)?,
    ))
}

fn interpolate_gap(start: LayoutGap, end: LayoutGap, progress: UnitInterval) -> Option<LayoutGap> {
    Some(LayoutGap::new(
        interpolate_logical_length(start.horizontal(), end.horizontal(), progress)?,
        interpolate_logical_length(start.vertical(), end.vertical(), progress)?,
    ))
}

fn interpolate_dimension(
    start: LayoutDimension,
    end: LayoutDimension,
    progress: UnitInterval,
) -> Option<LayoutDimension> {
    match (start, end) {
        (LayoutDimension::Length(start), LayoutDimension::Length(end)) => {
            interpolate_logical_length(start, end, progress).map(LayoutDimension::Length)
        }
        (LayoutDimension::Percent(start), LayoutDimension::Percent(end)) => {
            interpolate_layout_factor(start, end, progress).map(LayoutDimension::Percent)
        }
        _ => Some(start),
    }
}

fn interpolate_bound(
    start: LayoutBound,
    end: LayoutBound,
    progress: UnitInterval,
) -> Option<LayoutBound> {
    match (start, end) {
        (LayoutBound::Length(start), LayoutBound::Length(end)) => {
            interpolate_logical_length(start, end, progress).map(LayoutBound::Length)
        }
        (LayoutBound::Percent(start), LayoutBound::Percent(end)) => {
            interpolate_layout_factor(start, end, progress).map(LayoutBound::Percent)
        }
        _ => Some(start),
    }
}

fn interpolate_flex_basis(
    start: FlexBasis,
    end: FlexBasis,
    progress: UnitInterval,
) -> Option<FlexBasis> {
    match (start, end) {
        (FlexBasis::Length(start), FlexBasis::Length(end)) => {
            interpolate_logical_length(start, end, progress).map(FlexBasis::Length)
        }
        (FlexBasis::Percent(start), FlexBasis::Percent(end)) => {
            interpolate_layout_factor(start, end, progress).map(FlexBasis::Percent)
        }
        _ => Some(start),
    }
}

fn interpolate_color(
    start: crate::Color,
    end: crate::Color,
    progress: UnitInterval,
) -> crate::Color {
    let stops = GradientStops::new(vec![
        GradientStop::new(UnitInterval::ZERO, start),
        GradientStop::new(UnitInterval::ONE, end),
    ])
    .unwrap_or_else(|_| unreachable!("two ordered endpoint stops are always valid"));
    stops.sample(progress)
}

fn compatible_stop_colors(
    start: &GradientStops,
    end: &GradientStops,
    progress: UnitInterval,
) -> Option<GradientStops> {
    let start = start.as_slice();
    let end = end.as_slice();
    if start.len() != end.len()
        || start
            .iter()
            .zip(end)
            .any(|(start, end)| start.offset() != end.offset())
    {
        return None;
    }
    GradientStops::new(
        start
            .iter()
            .zip(end)
            .map(|(start, end)| {
                GradientStop::new(
                    start.offset(),
                    interpolate_color(start.color(), end.color(), progress),
                )
            })
            .collect::<Vec<_>>(),
    )
    .ok()
}

fn interpolate_brush(start: &Brush, end: &Brush, progress: UnitInterval) -> Option<Brush> {
    match (start, end) {
        (Brush::Solid(start), Brush::Solid(end)) => {
            Some(Brush::Solid(interpolate_color(*start, *end, progress)))
        }
        (Brush::Linear(start), Brush::Linear(end))
            if start.start() == end.start() && start.end() == end.end() =>
        {
            let stops = compatible_stop_colors(start.stops(), end.stops(), progress)?;
            LinearGradient::new(start.start(), start.end(), stops)
                .ok()
                .map(Brush::Linear)
        }
        (Brush::Radial(start), Brush::Radial(end))
            if start.center() == end.center() && start.radius() == end.radius() =>
        {
            let stops = compatible_stop_colors(start.stops(), end.stops(), progress)?;
            RadialGradient::new(start.center(), start.radius(), stops)
                .ok()
                .map(Brush::Radial)
        }
        _ => None,
    }
}

fn interpolate_shadows(
    start: &[DropShadow],
    end: &[DropShadow],
    progress: UnitInterval,
) -> Option<Vec<DropShadow>> {
    if start.len() != end.len() {
        return None;
    }
    start
        .iter()
        .zip(end)
        .map(|(start, end)| {
            DropShadow::new(
                interpolate_f32(start.offset_x(), end.offset_x(), progress),
                interpolate_f32(start.offset_y(), end.offset_y(), progress),
                interpolate_logical_length(start.sigma(), end.sigma(), progress)?,
                interpolate_f32(start.spread(), end.spread(), progress),
                interpolate_color(start.color(), end.color(), progress),
            )
            .ok()
        })
        .collect()
}

fn interpolate_presentation(
    start: PresentationTransform,
    end: PresentationTransform,
    progress: UnitInterval,
) -> Option<PresentationTransform> {
    let start_translation = start.translation();
    let end_translation = end.translation();
    let translation = PresentationTranslation::new(
        interpolate_f32(start_translation.x(), end_translation.x(), progress),
        interpolate_f32(start_translation.y(), end_translation.y(), progress),
    )
    .ok()?;

    let start_scale = start.scale();
    let end_scale = end.scale();
    let scale = PresentationScale::new(
        interpolate_f32(start_scale.x(), end_scale.x(), progress),
        interpolate_f32(start_scale.y(), end_scale.y(), progress),
    )
    .ok()?;

    let rotation = PresentationRotation::radians(interpolate_f32(
        start.rotation().get_radians(),
        end.rotation().get_radians(),
        progress,
    ))
    .ok()?;

    let start_origin = start.origin();
    let end_origin = end.origin();
    let origin = PresentationOrigin::new(
        interpolate_unit_interval(start_origin.x(), end_origin.x(), progress)?,
        interpolate_unit_interval(start_origin.y(), end_origin.y(), progress)?,
    );

    Some(PresentationTransform::new(
        translation,
        scale,
        rotation,
        origin,
    ))
}

#[allow(
    clippy::suboptimal_flops,
    reason = "ADR 0016 freezes these ordered de Casteljau operations and explicitly forbids FMA/mul_add"
)]
fn de_casteljau(p0: f64, p1: f64, p2: f64, p3: f64, u: f64) -> f64 {
    let v = 1.0 - u;
    let q0 = v * p0 + u * p1;
    let q1 = v * p1 + u * p2;
    let q2 = v * p2 + u * p3;
    let r0 = v * q0 + u * q1;
    let r1 = v * q1 + u * q2;
    v * r0 + u * r1
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the accepted easing result is clamped to [0,1] before the single f64-to-f32 conversion"
)]
fn cubic_bezier_ease(curve: CubicBezier, progress: UnitInterval) -> UnitInterval {
    if progress == UnitInterval::ZERO || progress == UnitInterval::ONE {
        return progress;
    }
    let target_x = f64::from(progress.get());
    let x1 = f64::from(curve.x1().get());
    let x2 = f64::from(curve.x2().get());
    let y1 = f64::from(curve.y1().get());
    let y2 = f64::from(curve.y2().get());
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for _ in 0..32 {
        let mid = (lo + hi) / 2.0;
        let x = de_casteljau(0.0, x1, x2, 1.0, mid);
        if x < target_x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let u = (lo + hi) / 2.0;
    let y = de_casteljau(0.0, y1, y2, 1.0, u).clamp(0.0, 1.0);
    UnitInterval::new(y as f32)
        .unwrap_or_else(|_| unreachable!("clamped deterministic Bezier output is normalized"))
}

#[cfg(test)]
mod tests {
    use core::f32::consts::{PI, TAU};

    use super::{ease_motion, interpolate_f32, interpolate_motion_value};
    use crate::{
        Color, CubicBezier, LayoutDimension, LogicalLength, MotionEasing, MotionValue,
        PresentationOrigin, PresentationRotation, PresentationScale, PresentationTransform,
        PresentationTranslation, UnitInterval,
    };

    fn unit(value: f32) -> UnitInterval {
        UnitInterval::new(value).unwrap_or_else(|_| unreachable!("test unit value is valid"))
    }

    fn transform(rotation: f32) -> PresentationTransform {
        PresentationTransform::new(
            PresentationTranslation::ZERO,
            PresentationScale::IDENTITY,
            PresentationRotation::radians(rotation)
                .unwrap_or_else(|_| unreachable!("test rotation is finite")),
            PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
        )
    }

    #[test]
    fn scalar_rule_avoids_false_opposite_sign_overflow() {
        let sample = interpolate_f32(f32::MAX, f32::MIN, UnitInterval::HALF);
        assert!(sample.is_finite());
        assert_eq!(sample, 0.0);
    }

    #[test]
    fn color_motion_reuses_accepted_linear_srgb_sampling() {
        let sampled = interpolate_motion_value(
            &MotionValue::Foreground(Some(Color::BLACK)),
            &MotionValue::Foreground(Some(Color::WHITE)),
            UnitInterval::HALF,
        )
        .unwrap_or_else(|| unreachable!("matching color target is sampleable"));
        assert_eq!(sampled, MotionValue::Foreground(Some(Color::rgb(188, 188, 188))));
    }

    #[test]
    fn incompatible_layout_domain_is_discrete_at_terminal_boundary() {
        let end = LayoutDimension::Length(LogicalLength::from(10_u16));
        let start = MotionValue::Width(LayoutDimension::Auto);
        let end = MotionValue::Width(end);
        assert_eq!(
            interpolate_motion_value(&start, &end, UnitInterval::HALF),
            Some(start.clone())
        );
        assert_eq!(
            interpolate_motion_value(&start, &end, UnitInterval::ONE),
            Some(end)
        );
    }

    #[test]
    fn presentation_rotation_interpolates_raw_authored_radians() {
        let sampled = interpolate_motion_value(
            &MotionValue::Presentation(Some(transform(0.0))),
            &MotionValue::Presentation(Some(transform(TAU))),
            UnitInterval::HALF,
        )
        .unwrap_or_else(|| unreachable!("matching presentation target is sampleable"));
        let MotionValue::Presentation(Some(sampled)) = sampled else {
            unreachable!("sample retains presentation target")
        };
        assert!((sampled.rotation().get_radians() - PI).abs() <= 1.0e-6);
    }

    #[test]
    fn cubic_bezier_uses_exact_endpoints_and_fixed_inversion() {
        let curve = CubicBezier::new(unit(0.25), unit(0.1), unit(0.25), unit(1.0));
        assert_eq!(
            ease_motion(MotionEasing::CubicBezier(curve), UnitInterval::ZERO),
            UnitInterval::ZERO
        );
        assert_eq!(
            ease_motion(MotionEasing::CubicBezier(curve), UnitInterval::ONE),
            UnitInterval::ONE
        );
        let middle = ease_motion(MotionEasing::CubicBezier(curve), UnitInterval::HALF);
        assert!(middle.get() > 0.5 && middle.get() < 1.0);
    }
}
