//! File: domain/ui/ui_input/src/pointer.rs
//! Purpose: Pointer, mouse, touch, and stylus event primitives.

use ui_math::{UiPoint, UiVector};

pub type PointerPosition = UiPoint;
pub type PointerDelta = UiVector;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerEventKind {
    Move,
    Down,
    Up,
    Enter,
    Leave,
    Scroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PointerDeviceId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerSourceKind {
    #[default]
    Mouse,
    Trackpad,
    Touch,
    Stylus,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerToolKind {
    #[default]
    Mouse,
    Finger,
    Pen,
    Brush,
    Marker,
    Airbrush,
    Eraser,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerContactState {
    Hover,
    #[default]
    Contact,
    OutOfRange,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerSampleRole {
    #[default]
    Raw,
    Coalesced,
    Predicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerLatencyClass {
    #[default]
    Normal,
    LowLatencyPreview,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerTilt {
    pub x_degrees: f32,
    pub y_degrees: f32,
}

impl PointerTilt {
    pub const fn new(x_degrees: f32, y_degrees: f32) -> Self {
        Self {
            x_degrees,
            y_degrees,
        }
    }

    pub fn is_valid(self) -> bool {
        self.x_degrees.is_finite()
            && self.y_degrees.is_finite()
            && (-90.0..=90.0).contains(&self.x_degrees)
            && (-90.0..=90.0).contains(&self.y_degrees)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerBarrelButtons {
    pub primary: bool,
    pub secondary: bool,
}

impl PointerBarrelButtons {
    pub const fn none() -> Self {
        Self {
            primary: false,
            secondary: false,
        }
    }
}

impl Default for PointerBarrelButtons {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerCalibration {
    pub cursor_offset: PointerDelta,
    pub pressure_scale: f32,
    pub pressure_bias: f32,
}

impl PointerCalibration {
    pub const fn identity() -> Self {
        Self {
            cursor_offset: PointerDelta::ZERO,
            pressure_scale: 1.0,
            pressure_bias: 0.0,
        }
    }

    pub fn is_valid(self) -> bool {
        self.cursor_offset.x.is_finite()
            && self.cursor_offset.y.is_finite()
            && self.pressure_scale.is_finite()
            && self.pressure_scale > 0.0
            && self.pressure_bias.is_finite()
    }
}

impl Default for PointerCalibration {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PointerDeviceCapabilities {
    pub pressure: bool,
    pub tilt: bool,
    pub twist: bool,
    pub tangential_pressure: bool,
    pub hover: bool,
    pub eraser: bool,
    pub barrel_buttons: bool,
    pub coalesced_samples: bool,
    pub predicted_samples: bool,
    pub calibration: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerSample {
    pub role: PointerSampleRole,
    pub position: PointerPosition,
    pub delta: PointerDelta,
    pub timestamp_micros: Option<u64>,
    pub pressure: Option<f32>,
    pub tilt: Option<PointerTilt>,
    pub twist_degrees: Option<f32>,
    pub tangential_pressure: Option<f32>,
    pub contact: PointerContactState,
}

impl PointerSample {
    pub const fn new(
        role: PointerSampleRole,
        position: PointerPosition,
        delta: PointerDelta,
    ) -> Self {
        Self {
            role,
            position,
            delta,
            timestamp_micros: None,
            pressure: None,
            tilt: None,
            twist_degrees: None,
            tangential_pressure: None,
            contact: PointerContactState::Contact,
        }
    }

    pub fn with_timestamp_micros(mut self, timestamp_micros: u64) -> Self {
        self.timestamp_micros = Some(timestamp_micros);
        self
    }

    pub fn with_pressure(mut self, pressure: f32) -> Self {
        self.pressure = Some(pressure);
        self
    }

    pub fn with_tilt(mut self, tilt: PointerTilt) -> Self {
        self.tilt = Some(tilt);
        self
    }

    pub fn with_twist_degrees(mut self, twist_degrees: f32) -> Self {
        self.twist_degrees = Some(twist_degrees);
        self
    }

    pub fn with_tangential_pressure(mut self, tangential_pressure: f32) -> Self {
        self.tangential_pressure = Some(tangential_pressure);
        self
    }

    pub fn with_contact(mut self, contact: PointerContactState) -> Self {
        self.contact = contact;
        self
    }

    pub fn is_valid(self) -> bool {
        self.position.x.is_finite()
            && self.position.y.is_finite()
            && self.delta.x.is_finite()
            && self.delta.y.is_finite()
            && self.pressure.is_none_or(unit_value)
            && self.tilt.is_none_or(PointerTilt::is_valid)
            && self
                .twist_degrees
                .is_none_or(|twist| twist.is_finite() && (0.0..=360.0).contains(&twist))
            && self.tangential_pressure.is_none_or(unit_value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PointerPacket {
    pub source_kind: PointerSourceKind,
    pub tool_kind: PointerToolKind,
    pub device_id: Option<PointerDeviceId>,
    pub timestamp_micros: Option<u64>,
    pub contact: PointerContactState,
    pub pressure: Option<f32>,
    pub tilt: Option<PointerTilt>,
    pub twist_degrees: Option<f32>,
    pub tangential_pressure: Option<f32>,
    pub eraser: bool,
    pub barrel_buttons: PointerBarrelButtons,
    pub capabilities: PointerDeviceCapabilities,
    pub calibration: Option<PointerCalibration>,
    pub latency_class: PointerLatencyClass,
    pub coalesced_samples: Vec<PointerSample>,
    pub predicted_samples: Vec<PointerSample>,
}

impl PointerPacket {
    pub fn mouse() -> Self {
        Self::default()
    }

    pub fn stylus(device_id: PointerDeviceId, tool_kind: PointerToolKind) -> Self {
        Self {
            source_kind: PointerSourceKind::Stylus,
            tool_kind,
            device_id: Some(device_id),
            capabilities: PointerDeviceCapabilities {
                pressure: true,
                tilt: true,
                twist: true,
                tangential_pressure: true,
                hover: true,
                eraser: true,
                barrel_buttons: true,
                coalesced_samples: true,
                predicted_samples: true,
                calibration: true,
            },
            ..Self::default()
        }
    }

    pub fn with_timestamp_micros(mut self, timestamp_micros: u64) -> Self {
        self.timestamp_micros = Some(timestamp_micros);
        self
    }

    pub fn with_capabilities(mut self, capabilities: PointerDeviceCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_pressure(mut self, pressure: f32) -> Self {
        self.pressure = Some(pressure);
        self.capabilities.pressure = true;
        self
    }

    pub fn with_tilt(mut self, tilt: PointerTilt) -> Self {
        self.tilt = Some(tilt);
        self.capabilities.tilt = true;
        self
    }

    pub fn with_twist_degrees(mut self, twist_degrees: f32) -> Self {
        self.twist_degrees = Some(twist_degrees);
        self.capabilities.twist = true;
        self
    }

    pub fn with_tangential_pressure(mut self, tangential_pressure: f32) -> Self {
        self.tangential_pressure = Some(tangential_pressure);
        self.capabilities.tangential_pressure = true;
        self
    }

    pub fn with_eraser(mut self, eraser: bool) -> Self {
        self.eraser = eraser;
        if eraser {
            self.tool_kind = PointerToolKind::Eraser;
        }
        self.capabilities.eraser |= eraser;
        self
    }

    pub fn with_barrel_buttons(mut self, barrel_buttons: PointerBarrelButtons) -> Self {
        self.barrel_buttons = barrel_buttons;
        self.capabilities.barrel_buttons |= barrel_buttons.primary || barrel_buttons.secondary;
        self
    }

    pub fn with_contact(mut self, contact: PointerContactState) -> Self {
        self.contact = contact;
        if contact == PointerContactState::Hover {
            self.capabilities.hover = true;
        }
        self
    }

    pub fn with_calibration(mut self, calibration: PointerCalibration) -> Self {
        self.calibration = Some(calibration);
        self.capabilities.calibration = true;
        self
    }

    pub fn with_latency_class(mut self, latency_class: PointerLatencyClass) -> Self {
        self.latency_class = latency_class;
        self
    }

    pub fn with_coalesced_samples(
        mut self,
        samples: impl IntoIterator<Item = PointerSample>,
    ) -> Self {
        self.coalesced_samples = samples.into_iter().collect();
        self.capabilities.coalesced_samples |= !self.coalesced_samples.is_empty();
        self
    }

    pub fn with_predicted_samples(
        mut self,
        samples: impl IntoIterator<Item = PointerSample>,
    ) -> Self {
        self.predicted_samples = samples.into_iter().collect();
        self.capabilities.predicted_samples |= !self.predicted_samples.is_empty();
        self
    }

    pub fn is_pointer_fallback(&self) -> bool {
        self.source_kind == PointerSourceKind::Mouse
            && self.device_id.is_none()
            && self.pressure.is_none()
            && self.tilt.is_none()
            && self.twist_degrees.is_none()
            && self.tangential_pressure.is_none()
            && !self.eraser
    }

    pub fn is_valid(&self) -> bool {
        self.pressure.is_none_or(unit_value)
            && self.tilt.is_none_or(PointerTilt::is_valid)
            && self
                .twist_degrees
                .is_none_or(|twist| twist.is_finite() && (0.0..=360.0).contains(&twist))
            && self.tangential_pressure.is_none_or(unit_value)
            && self.calibration.is_none_or(PointerCalibration::is_valid)
            && self
                .coalesced_samples
                .iter()
                .all(|sample| sample.role == PointerSampleRole::Coalesced && sample.is_valid())
            && self
                .predicted_samples
                .iter()
                .all(|sample| sample.role == PointerSampleRole::Predicted && sample.is_valid())
    }
}

impl Default for PointerPacket {
    fn default() -> Self {
        Self {
            source_kind: PointerSourceKind::Mouse,
            tool_kind: PointerToolKind::Mouse,
            device_id: None,
            timestamp_micros: None,
            contact: PointerContactState::Contact,
            pressure: None,
            tilt: None,
            twist_degrees: None,
            tangential_pressure: None,
            eraser: false,
            barrel_buttons: PointerBarrelButtons::none(),
            capabilities: PointerDeviceCapabilities::default(),
            calibration: None,
            latency_class: PointerLatencyClass::Normal,
            coalesced_samples: Vec::new(),
            predicted_samples: Vec::new(),
        }
    }
}

fn unit_value(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
