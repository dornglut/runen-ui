use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlSurface2DInputMode {
    KeyboardPan,
    KeyboardZoom,
    KeyboardFitContent,
    PointerCapture,
    WheelScroll,
    TrackpadPinchStatus,
    TouchPanZoomStatus,
    ControllerNavigationStatus,
}

impl ControlSurface2DInputMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeyboardPan => "keyboard-pan",
            Self::KeyboardZoom => "keyboard-zoom",
            Self::KeyboardFitContent => "keyboard-fit-content",
            Self::PointerCapture => "pointer-capture",
            Self::WheelScroll => "wheel-scroll",
            Self::TrackpadPinchStatus => "trackpad-pinch-status",
            Self::TouchPanZoomStatus => "touch-pan-zoom-status",
            Self::ControllerNavigationStatus => "controller-navigation-status",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlSurface2DLayerKind {
    Background,
    Grid,
    DiagnosticOverlay,
    SelectionBox,
}

impl ControlSurface2DLayerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Grid => "grid",
            Self::DiagnosticOverlay => "diagnostic-overlay",
            Self::SelectionBox => "selection-box",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ControlSurface2DBudgetEvidenceKind {
    TransformProjection,
    PanZoomUpdate,
    HoverCoordinateUpdate,
    SelectionRectangleUpdate,
    FitContentCalculation,
    LargeContentBoundsProjection,
    RuntimeReportGeneration,
    StaticMountReportGeneration,
    PrimitiveCount,
}

impl ControlSurface2DBudgetEvidenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TransformProjection => "transform-projection",
            Self::PanZoomUpdate => "pan-zoom-update",
            Self::HoverCoordinateUpdate => "hover-coordinate-update",
            Self::SelectionRectangleUpdate => "selection-rectangle-update",
            Self::FitContentCalculation => "fit-content-calculation",
            Self::LargeContentBoundsProjection => "large-content-bounds-projection",
            Self::RuntimeReportGeneration => "runtime-report-generation",
            Self::StaticMountReportGeneration => "static-mount-report-generation",
            Self::PrimitiveCount => "primitive-count",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSurface2DAccessibilitySupport {
    pub keyboard_pan: bool,
    pub keyboard_zoom: bool,
    pub keyboard_fit_content: bool,
    pub focus_visible: bool,
    pub inspection_readable_name_and_bounds: bool,
    pub reduced_motion_navigation: bool,
}

impl ControlSurface2DAccessibilitySupport {
    pub const fn complete() -> Self {
        Self {
            keyboard_pan: true,
            keyboard_zoom: true,
            keyboard_fit_content: true,
            focus_visible: true,
            inspection_readable_name_and_bounds: true,
            reduced_motion_navigation: true,
        }
    }

    pub const fn is_complete(&self) -> bool {
        self.keyboard_pan
            && self.keyboard_zoom
            && self.keyboard_fit_content
            && self.focus_visible
            && self.inspection_readable_name_and_bounds
            && self.reduced_motion_navigation
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSurface2DInteractionSupport {
    pub content_bounds: bool,
    pub viewport_bounds: bool,
    pub world_screen_transform: bool,
    pub pan_zoom_state: bool,
    pub fit_content: bool,
    pub hover_coordinate: bool,
    pub selection_rectangle: bool,
    pub pointer_capture: bool,
    pub gesture_cancel_commit: bool,
    pub invalid_transform_diagnostic: bool,
}

impl ControlSurface2DInteractionSupport {
    pub const fn complete() -> Self {
        Self {
            content_bounds: true,
            viewport_bounds: true,
            world_screen_transform: true,
            pan_zoom_state: true,
            fit_content: true,
            hover_coordinate: true,
            selection_rectangle: true,
            pointer_capture: true,
            gesture_cancel_commit: true,
            invalid_transform_diagnostic: true,
        }
    }

    pub const fn is_complete(&self) -> bool {
        self.content_bounds
            && self.viewport_bounds
            && self.world_screen_transform
            && self.pan_zoom_state
            && self.fit_content
            && self.hover_coordinate
            && self.selection_rectangle
            && self.pointer_capture
            && self.gesture_cancel_commit
            && self.invalid_transform_diagnostic
    }
}
