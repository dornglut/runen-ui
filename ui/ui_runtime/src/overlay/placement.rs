use ui_controls::{
    ControlOverlayPlacementPreference, ControlOverlayPlacementSide, ControlOverlayRequirement,
};
use ui_math::{UiRect, UiSize};

use super::{MountedOverlayControl, OverlayPlacementResolution};

pub fn placement_resolution(
    control: &MountedOverlayControl,
    requirement: &ControlOverlayRequirement,
    request_id: &str,
    viewport_rect: UiRect,
) -> OverlayPlacementResolution {
    let requested = &requirement.placement;
    let mut rect = raw_overlay_rect(control.bounds, requested);
    let before = rect;
    rect.x = rect.x.max(viewport_rect.x + requested.viewport_margin);
    rect.y = rect.y.max(viewport_rect.y + requested.viewport_margin);
    rect.x = rect
        .x
        .min(viewport_rect.x + viewport_rect.width - rect.width - requested.viewport_margin);
    rect.y = rect
        .y
        .min(viewport_rect.y + viewport_rect.height - rect.height - requested.viewport_margin);
    OverlayPlacementResolution {
        anchor_id: control.anchor_id.clone(),
        request_id: request_id.to_owned(),
        anchor_rect: control.bounds,
        requested_side: requested.side.as_str().to_owned(),
        requested_alignment: requested.alignment.as_str().to_owned(),
        resolved_side: requested.side.as_str().to_owned(),
        resolved_alignment: requested.alignment.as_str().to_owned(),
        resolved_rect: rect,
        viewport_rect,
        clamped: rect != before,
        shifted: rect.x != before.x || rect.y != before.y,
        hidden_or_suppressed: false,
    }
}

fn raw_overlay_rect(anchor: UiRect, placement: &ControlOverlayPlacementPreference) -> UiRect {
    let size = UiSize::new(220.0, 132.0);
    match placement.side {
        ControlOverlayPlacementSide::Top => UiRect::new(
            anchor.x,
            anchor.y - size.height - placement.main_axis_offset,
            size.width,
            size.height,
        ),
        ControlOverlayPlacementSide::Right => UiRect::new(
            anchor.x + anchor.width + placement.main_axis_offset,
            anchor.y,
            size.width,
            size.height,
        ),
        ControlOverlayPlacementSide::Bottom => UiRect::new(
            anchor.x,
            anchor.y + anchor.height + placement.main_axis_offset,
            size.width,
            size.height,
        ),
        ControlOverlayPlacementSide::Left => UiRect::new(
            anchor.x - size.width - placement.main_axis_offset,
            anchor.y,
            size.width,
            size.height,
        ),
        ControlOverlayPlacementSide::Center | ControlOverlayPlacementSide::Cursor => {
            UiRect::new(anchor.x, anchor.y, size.width, size.height)
        }
    }
}
