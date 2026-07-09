//! File: domain/ui/ui_widgets/src/color_picker.rs
//! Purpose: Bounded ColorPicker ControlPackage proof for UiProgram Stage 6D.

use ui_theme::UiColor;
use ui_tree::WidgetId;

pub const COLOR_PICKER_PACKAGE_ID: &str = "runenwerk.ui.color-picker";
pub const COLOR_PICKER_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.color-picker";
pub const COLOR_PICKER_PROPERTY_SCHEMA_ID: &str = "runenwerk.ui.color-picker.properties";
pub const COLOR_PICKER_STATE_SCHEMA_ID: &str = "runenwerk.ui.color-picker.state";
pub const COLOR_PICKER_EVENT_SCHEMA_ID: &str = "runenwerk.ui.color-picker.events";
pub const COLOR_PICKER_LAYOUT_KERNEL_ID: &str = "runenwerk.ui.color-picker.layout.wheel-triangle";
pub const COLOR_PICKER_INTERACTION_KERNEL_ID: &str =
    "runenwerk.ui.color-picker.interaction.wheel-triangle";
pub const COLOR_PICKER_VISUAL_KERNEL_ID: &str = "runenwerk.ui.color-picker.visual.wheel-triangle";
pub const COLOR_PICKER_FIXTURE_ID: &str = "runenwerk.ui.fixtures.color-picker.wheel-triangle";
pub const COLOR_PICKER_DIAGNOSTIC_ID: &str = "runenwerk.ui.diagnostics.color-picker";
pub const COLOR_PICKER_ROUTE_PREVIEW_CHANGED: &str = "ui.color-picker.preview-changed";
pub const COLOR_PICKER_ROUTE_COMMITTED: &str = "ui.color-picker.committed";
pub const COLOR_PICKER_ROUTE_SCHEMA_VERSION: u32 = 1;
pub const COLOR_PICKER_ROUTE_CAPABILITY: &str = "ui.color-picker.write-preview";

const PROPERTY_FIELDS: &[&str] = &["committed_rgba", "allow_alpha", "route_namespace"];
const STATE_FIELDS: &[&str] = &[
    "hue_degrees",
    "triangle_saturation",
    "triangle_value",
    "preview_rgba",
];
const EVENT_FIELDS: &[&str] = &["rgba", "preview", "committed"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerTrianglePoint {
    pub saturation: f32,
    pub value: f32,
}

impl ColorPickerTrianglePoint {
    pub fn new(saturation: f32, value: f32) -> Self {
        Self {
            saturation: saturation.clamp(0.0, 1.0),
            value: value.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerProperties {
    pub committed: UiColor,
    pub allow_alpha: bool,
}

impl ColorPickerProperties {
    pub const fn new(committed: UiColor, allow_alpha: bool) -> Self {
        Self {
            committed,
            allow_alpha,
        }
    }
}

impl Default for ColorPickerProperties {
    fn default() -> Self {
        Self::new(UiColor::new(1.0, 1.0, 1.0, 1.0), true)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerState {
    pub hue_degrees: f32,
    pub triangle: ColorPickerTrianglePoint,
    pub preview: UiColor,
    pub committed: UiColor,
}

impl ColorPickerState {
    pub fn wheel_plus_triangle(
        hue_degrees: f32,
        triangle: ColorPickerTrianglePoint,
        properties: ColorPickerProperties,
    ) -> Self {
        let hue = normalize_hue(hue_degrees);
        let preview = hsv_to_rgb(
            hue,
            triangle.saturation,
            triangle.value,
            properties.committed.a,
        );
        Self {
            hue_degrees: hue,
            triangle,
            preview,
            committed: properties.committed,
        }
    }

    pub fn commit(mut self) -> Self {
        self.committed = self.preview;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPickerSchema {
    pub id: &'static str,
    pub version: u32,
    pub fields: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPickerControlPackage {
    pub package_id: &'static str,
    pub control_kind_id: &'static str,
    pub property_schema: ColorPickerSchema,
    pub state_schema: ColorPickerSchema,
    pub event_payload_schema: ColorPickerSchema,
    pub layout_kernel_id: &'static str,
    pub interaction_kernel_id: &'static str,
    pub visual_kernel_id: &'static str,
    pub fixture_id: &'static str,
    pub diagnostic_id: &'static str,
    pub migration_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerControl {
    pub id: WidgetId,
    pub properties: ColorPickerProperties,
    pub state: ColorPickerState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerEventPayload {
    pub rgba: UiColor,
    pub preview: bool,
    pub committed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPickerEventPacket {
    pub route_id: &'static str,
    pub route_schema_version: u32,
    pub route_capability: &'static str,
    pub payload_schema_id: &'static str,
    pub payload: ColorPickerEventPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPickerDiagnostic {
    pub id: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPickerDeferredFeature {
    pub id: &'static str,
    pub reason: &'static str,
}

pub fn color_picker(
    id: WidgetId,
    properties: ColorPickerProperties,
    state: ColorPickerState,
) -> ColorPickerControl {
    ColorPickerControl {
        id,
        properties,
        state,
    }
}

pub fn color_picker_control_package() -> ColorPickerControlPackage {
    ColorPickerControlPackage {
        package_id: COLOR_PICKER_PACKAGE_ID,
        control_kind_id: COLOR_PICKER_CONTROL_KIND_ID,
        property_schema: ColorPickerSchema {
            id: COLOR_PICKER_PROPERTY_SCHEMA_ID,
            version: 1,
            fields: PROPERTY_FIELDS,
        },
        state_schema: ColorPickerSchema {
            id: COLOR_PICKER_STATE_SCHEMA_ID,
            version: 1,
            fields: STATE_FIELDS,
        },
        event_payload_schema: ColorPickerSchema {
            id: COLOR_PICKER_EVENT_SCHEMA_ID,
            version: 1,
            fields: EVENT_FIELDS,
        },
        layout_kernel_id: COLOR_PICKER_LAYOUT_KERNEL_ID,
        interaction_kernel_id: COLOR_PICKER_INTERACTION_KERNEL_ID,
        visual_kernel_id: COLOR_PICKER_VISUAL_KERNEL_ID,
        fixture_id: COLOR_PICKER_FIXTURE_ID,
        diagnostic_id: COLOR_PICKER_DIAGNOSTIC_ID,
        migration_version: 1,
    }
}

pub fn color_picker_fixture() -> ColorPickerControl {
    let properties = ColorPickerProperties::new(UiColor::new(0.8, 0.2, 0.1, 1.0), true);
    let state = ColorPickerState::wheel_plus_triangle(
        24.0,
        ColorPickerTrianglePoint::new(0.75, 0.8),
        properties,
    );
    color_picker(WidgetId(6_010), properties, state)
}

pub fn color_picker_preview_changed_packet(state: ColorPickerState) -> ColorPickerEventPacket {
    ColorPickerEventPacket {
        route_id: COLOR_PICKER_ROUTE_PREVIEW_CHANGED,
        route_schema_version: COLOR_PICKER_ROUTE_SCHEMA_VERSION,
        route_capability: COLOR_PICKER_ROUTE_CAPABILITY,
        payload_schema_id: COLOR_PICKER_EVENT_SCHEMA_ID,
        payload: ColorPickerEventPayload {
            rgba: state.preview,
            preview: true,
            committed: false,
        },
    }
}

pub fn color_picker_committed_packet(state: ColorPickerState) -> ColorPickerEventPacket {
    ColorPickerEventPacket {
        route_id: COLOR_PICKER_ROUTE_COMMITTED,
        route_schema_version: COLOR_PICKER_ROUTE_SCHEMA_VERSION,
        route_capability: COLOR_PICKER_ROUTE_CAPABILITY,
        payload_schema_id: COLOR_PICKER_EVENT_SCHEMA_ID,
        payload: ColorPickerEventPayload {
            rgba: state.committed,
            preview: false,
            committed: true,
        },
    }
}

pub fn color_picker_diagnostics(control: &ColorPickerControl) -> Vec<ColorPickerDiagnostic> {
    let mut diagnostics = Vec::new();
    if !control.state.preview.a.is_finite() {
        diagnostics.push(ColorPickerDiagnostic {
            id: COLOR_PICKER_DIAGNOSTIC_ID,
            message: "preview alpha must be finite",
        });
    }
    diagnostics
}

pub fn color_picker_rgb_cube_status() -> ColorPickerDeferredFeature {
    ColorPickerDeferredFeature {
        id: "runenwerk.ui.color-picker.rgb-cube",
        reason: "RGB cube projection is deferred until after the wheel-plus-triangle ControlPackage proof.",
    }
}

fn normalize_hue(hue_degrees: f32) -> f32 {
    if hue_degrees.is_finite() {
        hue_degrees.rem_euclid(360.0)
    } else {
        0.0
    }
}

fn hsv_to_rgb(hue_degrees: f32, saturation: f32, value: f32, alpha: f32) -> UiColor {
    let chroma = value * saturation;
    let hue_prime = hue_degrees / 60.0;
    let x = chroma * (1.0 - ((hue_prime % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if hue_prime < 1.0 {
        (chroma, x, 0.0)
    } else if hue_prime < 2.0 {
        (x, chroma, 0.0)
    } else if hue_prime < 3.0 {
        (0.0, chroma, x)
    } else if hue_prime < 4.0 {
        (0.0, x, chroma)
    } else if hue_prime < 5.0 {
        (x, 0.0, chroma)
    } else {
        (chroma, 0.0, x)
    };
    let m = value - chroma;
    UiColor::new(r1 + m, g1 + m, b1 + m, alpha.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_picker_package_records_stable_contract_ids() {
        let package = color_picker_control_package();

        assert_eq!(package.package_id, COLOR_PICKER_PACKAGE_ID);
        assert_eq!(package.control_kind_id, COLOR_PICKER_CONTROL_KIND_ID);
        assert_eq!(package.property_schema.fields, PROPERTY_FIELDS);
        assert_eq!(package.state_schema.fields, STATE_FIELDS);
        assert_eq!(package.event_payload_schema.fields, EVENT_FIELDS);
        assert_eq!(package.layout_kernel_id, COLOR_PICKER_LAYOUT_KERNEL_ID);
        assert_eq!(
            package.interaction_kernel_id,
            COLOR_PICKER_INTERACTION_KERNEL_ID
        );
        assert_eq!(package.visual_kernel_id, COLOR_PICKER_VISUAL_KERNEL_ID);
    }

    #[test]
    fn color_picker_uses_wheel_plus_triangle_and_defers_rgb_cube() {
        let fixture = color_picker_fixture();
        let deferred = color_picker_rgb_cube_status();

        assert_eq!(fixture.id, WidgetId(6_010));
        assert_eq!(fixture.state.hue_degrees, 24.0);
        assert_eq!(deferred.id, "runenwerk.ui.color-picker.rgb-cube");
        assert!(deferred.reason.contains("deferred"));
    }

    #[test]
    fn color_picker_emits_route_based_event_packets() {
        let fixture = color_picker_fixture();
        let packet = color_picker_preview_changed_packet(fixture.state);

        assert_eq!(packet.route_id, COLOR_PICKER_ROUTE_PREVIEW_CHANGED);
        assert_eq!(
            packet.route_schema_version,
            COLOR_PICKER_ROUTE_SCHEMA_VERSION
        );
        assert_eq!(packet.route_capability, COLOR_PICKER_ROUTE_CAPABILITY);
        assert_eq!(packet.payload_schema_id, COLOR_PICKER_EVENT_SCHEMA_ID);
        assert!(packet.payload.preview);
        assert!(!packet.payload.committed);
    }

    #[test]
    fn color_picker_keeps_diagnostics_fixture_local() {
        let fixture = color_picker_fixture();

        assert!(color_picker_diagnostics(&fixture).is_empty());
    }
}
