use runenui_core::{Color, LogicalPoint, LogicalRect, PaintPrimitive};

use crate::{ConsumerSnapshot, PaintRecord, rect_contains, shape_contains};

/// Independently evaluates literal fill/stroke coverage and source-over color.
///
/// Resource-backed image and shaped-run payload coverage is intentionally not
/// invented here. M6 exposes their symbolic resource identity and placement,
/// while provider payloads and realization remain outside the protocol.
#[must_use]
pub fn sample_literal_paint(snapshot: &ConsumerSnapshot, point: LogicalPoint) -> [f32; 4] {
    snapshot
        .paint_items
        .iter()
        .filter_map(|item| literal_source(item, point))
        .fold([0.0; 4], source_over)
}

fn literal_source(item: &PaintRecord, surface_point: LogicalPoint) -> Option<(Color, f32)> {
    let local_point = item
        .local_to_surface
        .inverse()
        .and_then(|surface_to_local| surface_to_local.transform_point(surface_point))?;
    if !item.clips.iter().all(|clip| {
        clip.clip_to_surface()
            .inverse()
            .and_then(|surface_to_clip| surface_to_clip.transform_point(surface_point))
            .is_some_and(|clip_point| shape_contains(clip.shape(), clip_point))
    }) {
        return None;
    }

    let color = match &item.primitive {
        PaintPrimitive::FillRect { rect, color } if fill_covers(*rect, local_point) => *color,
        PaintPrimitive::StrokeRect { rect, color, width }
            if stroke_covers(*rect, width.get(), local_point) =>
        {
            *color
        }
        _ => return None,
    };
    Some((color, item.opacity.get()))
}

fn fill_covers(rect: LogicalRect, point: LogicalPoint) -> bool {
    rect.width() > 0.0 && rect.height() > 0.0 && rect_contains(rect, point)
}

fn stroke_covers(rect: LogicalRect, width: f32, point: LogicalPoint) -> bool {
    if width == 0.0 || rect.width() == 0.0 || rect.height() == 0.0 {
        return false;
    }
    let half = width / 2.0;
    let Ok(expanded) = LogicalRect::try_new(
        rect.x() - half,
        rect.y() - half,
        rect.width() + width,
        rect.height() + width,
    ) else {
        return false;
    };
    if !rect_contains(expanded, point) {
        return false;
    }
    if rect.width() <= width || rect.height() <= width {
        return true;
    }
    let Ok(inset) = LogicalRect::try_new(
        rect.x() + half,
        rect.y() + half,
        rect.width() - width,
        rect.height() - width,
    ) else {
        return false;
    };
    !rect_contains(inset, point)
}

fn source_over(destination: [f32; 4], (color, opacity): (Color, f32)) -> [f32; 4] {
    let alpha = (f32::from(color.alpha()) / 255.0) * opacity;
    let one_minus_alpha = 1.0 - alpha;
    [
        destination[0].mul_add(one_minus_alpha, srgb8_to_linear(color.red()) * alpha),
        destination[1].mul_add(one_minus_alpha, srgb8_to_linear(color.green()) * alpha),
        destination[2].mul_add(one_minus_alpha, srgb8_to_linear(color.blue()) * alpha),
        destination[3].mul_add(one_minus_alpha, alpha),
    ]
}

fn srgb8_to_linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}
