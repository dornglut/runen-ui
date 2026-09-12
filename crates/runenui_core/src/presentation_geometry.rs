use crate::{LogicalPoint, LogicalRect, LogicalSize, LogicalTransform};

/// Returns the exact finite axis-aligned bounds of a logical rectangle after an
/// affine transform. Arithmetic overflow returns `None`; callers must not fall
/// back to untransformed geometry.
#[must_use]
pub fn transform_rect_aabb(transform: LogicalTransform, rect: LogicalRect) -> Option<LogicalRect> {
    let max_x = checked_add(rect.x(), rect.width())?;
    let max_y = checked_add(rect.y(), rect.height())?;
    let corners = [
        LogicalPoint::new(rect.x(), rect.y()).ok()?,
        LogicalPoint::new(max_x, rect.y()).ok()?,
        LogicalPoint::new(rect.x(), max_y).ok()?,
        LogicalPoint::new(max_x, max_y).ok()?,
    ];
    let mut projected = corners
        .into_iter()
        .map(|point| transform.transform_point(point));
    let first = projected.next()??;
    let mut min_x = first.x();
    let mut max_x = first.x();
    let mut min_y = first.y();
    let mut max_y = first.y();
    for point in projected {
        let point = point?;
        min_x = min_x.min(point.x());
        max_x = max_x.max(point.x());
        min_y = min_y.min(point.y());
        max_y = max_y.max(point.y());
    }
    let width = checked_sub(max_x, min_x)?;
    let height = checked_sub(max_y, min_y)?;
    Some(LogicalRect::new(
        LogicalPoint::new(min_x, min_y).ok()?,
        LogicalSize::try_new(width, height).ok()?,
    ))
}

fn checked_add(left: f32, right: f32) -> Option<f32> {
    let value = left + right;
    value.is_finite().then_some(value)
}

fn checked_sub(left: f32, right: f32) -> Option<f32> {
    let value = left - right;
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::transform_rect_aabb;
    use crate::{LogicalPoint, LogicalRect, LogicalSize, LogicalTransform};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::new(
            LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!()),
            LogicalSize::try_new(width, height).unwrap_or_else(|_| unreachable!()),
        )
    }

    #[test]
    fn uses_all_four_transformed_corners() {
        let rotation = LogicalTransform::try_new(0.0, 1.0, -1.0, 0.0, 10.0, 20.0)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(
            transform_rect_aabb(rotation, rect(0.0, 0.0, 4.0, 2.0)),
            Some(rect(8.0, 20.0, 2.0, 4.0))
        );
    }

    #[test]
    fn rejects_unrepresentable_transformed_geometry() {
        let scale = LogicalTransform::try_new(f32::MAX, 0.0, 0.0, 1.0, 0.0, 0.0)
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(transform_rect_aabb(scale, rect(0.0, 0.0, 2.0, 1.0)), None);
    }
}
