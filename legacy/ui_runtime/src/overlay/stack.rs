use ui_controls::{ControlOverlayFocusPolicy, ControlOverlayKind, ControlOverlayRequirement};

use super::MountedOverlayControl;

pub fn overlay_scope(requirement: &ControlOverlayRequirement) -> String {
    match requirement.kind {
        ControlOverlayKind::Menu => "scope.menu".to_owned(),
        ControlOverlayKind::Tooltip => "scope.tooltip".to_owned(),
        ControlOverlayKind::FocusContainingOverlay => "scope.focus-containing".to_owned(),
        _ => "scope.anchored".to_owned(),
    }
}

pub fn focus_return_anchor(
    control: &MountedOverlayControl,
    requirement: &ControlOverlayRequirement,
) -> Option<String> {
    match requirement.focus_policy {
        ControlOverlayFocusPolicy::ReturnToAnchor
        | ControlOverlayFocusPolicy::ReturnToPrevious
        | ControlOverlayFocusPolicy::ContainFocus => Some(control.anchor_id.clone()),
        _ => None,
    }
}
