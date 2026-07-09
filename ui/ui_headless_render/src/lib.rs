//! Deterministic headless render proof over the strict runtime view contract.

use serde::{Deserialize, Serialize};
use ui_program::ControlNodeId;
use ui_runtime_view::{RuntimeControlView, UiRuntimeViewReport};
use ui_schema::UiSchemaValue;

pub const DIAGNOSTIC_RUNTIME_VIEW_FAILED: &str = "ui.headless_render.runtime_view_failed";
pub const DIAGNOSTIC_CONTROL_MISSING_VISUAL_OPERATOR: &str =
    "ui.headless_render.control.missing_visual_operator";
pub const DIAGNOSTIC_CONTROL_MISSING_PROPERTY: &str = "ui.headless_render.control.missing_property";
pub const DIAGNOSTIC_CONTROL_MISSING_PROPERTY_VALUE: &str =
    "ui.headless_render.control.missing_property_value";
pub const DIAGNOSTIC_LAYOUT_HEADLESS_FIXED_LAYOUT: &str =
    "ui.headless_render.layout.headless_fixed_layout";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessRenderReport {
    frame: Option<HeadlessUiFrame>,
    diagnostics: Vec<HeadlessRenderDiagnostic>,
}

impl HeadlessRenderReport {
    pub fn from_runtime_view_report(
        report: &UiRuntimeViewReport,
        viewport: HeadlessRenderViewport,
    ) -> Self {
        if !report.passed() {
            return Self {
                frame: None,
                diagnostics: vec![HeadlessRenderDiagnostic::error(
                    DIAGNOSTIC_RUNTIME_VIEW_FAILED,
                    "headless render refused failed runtime view report",
                    None,
                )],
            };
        }

        let mut diagnostics = vec![HeadlessRenderDiagnostic::info(
            DIAGNOSTIC_LAYOUT_HEADLESS_FIXED_LAYOUT,
            "headless render used HeadlessFixedLayout deterministic bounds",
            None,
        )];
        let mut primitives = Vec::new();
        let mut primitive_order = 0u32;

        for control in report.view.controls() {
            if control.visual.is_empty() {
                diagnostics.push(HeadlessRenderDiagnostic::error(
                    DIAGNOSTIC_CONTROL_MISSING_VISUAL_OPERATOR,
                    format!(
                        "control {} has no visual operator row",
                        control.control_id().as_str()
                    ),
                    control.control.source_map_index,
                ));
                continue;
            }

            let Some(property) = control.property() else {
                diagnostics.push(HeadlessRenderDiagnostic::error(
                    DIAGNOSTIC_CONTROL_MISSING_PROPERTY,
                    format!(
                        "control {} has no runtime property row",
                        control.control_id().as_str()
                    ),
                    control.control.source_map_index,
                ));
                continue;
            };

            let Some(label) = required_string_property(control, "label", &mut diagnostics) else {
                continue;
            };
            let variant = optional_string_property_or(control, "variant", "secondary");
            let tone = optional_string_property_or(control, "tone", "neutral");
            let density = optional_string_property_or(control, "density", "normal");
            let size = optional_string_property_or(control, "size", "md");

            let style = HeadlessRenderStyle::from_button_properties(variant, tone, density, size);
            let layout = HeadlessFixedLayout::for_control_index(primitives.len() as u32, &style);
            let source_map_index = property
                .source_map_index
                .or(control.control.source_map_index);

            primitives.push(HeadlessUiPrimitive::Rectangle {
                primitive_id: primitive_id(control.control_id(), "background"),
                control_id: control.control_id().as_str().to_owned(),
                role: HeadlessPrimitiveRole::ButtonBackground,
                bounds: layout.bounds,
                radius: style.corner_radius,
                color: style.background,
                sort_key: HeadlessSortKey::new(0, primitive_order),
                source_map_index,
            });
            primitive_order += 1;

            primitives.push(HeadlessUiPrimitive::Border {
                primitive_id: primitive_id(control.control_id(), "outline"),
                control_id: control.control_id().as_str().to_owned(),
                role: HeadlessPrimitiveRole::ButtonOutline,
                bounds: layout.bounds,
                radius: style.corner_radius,
                width: style.border_width,
                color: style.border,
                sort_key: HeadlessSortKey::new(0, primitive_order),
                source_map_index,
            });
            primitive_order += 1;

            primitives.push(HeadlessUiPrimitive::Text {
                primitive_id: primitive_id(control.control_id(), "label"),
                control_id: control.control_id().as_str().to_owned(),
                role: HeadlessPrimitiveRole::LabelText,
                run: HeadlessTextRun {
                    text: label.to_owned(),
                    bounds: layout.text_bounds,
                    color: style.text,
                    font_size: style.font_size,
                },
                sort_key: HeadlessSortKey::new(0, primitive_order),
                source_map_index,
            });
            primitive_order += 1;
        }

        let frame = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == HeadlessRenderDiagnosticSeverity::Error)
        {
            None
        } else {
            primitives.sort_by_key(HeadlessUiPrimitive::sort_key);
            Some(HeadlessUiFrame {
                frame_id: "headless.frame.root".to_owned(),
                viewport,
                layout_strategy: HeadlessLayoutStrategy::HeadlessFixedLayout,
                primitives,
            })
        };

        Self { frame, diagnostics }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != HeadlessRenderDiagnosticSeverity::Error)
    }

    pub fn frame(&self) -> Option<&HeadlessUiFrame> {
        self.frame.as_ref()
    }

    pub fn diagnostics(&self) -> &[HeadlessRenderDiagnostic] {
        &self.diagnostics
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessUiFrame {
    pub frame_id: String,
    pub viewport: HeadlessRenderViewport,
    pub layout_strategy: HeadlessLayoutStrategy,
    pub primitives: Vec<HeadlessUiPrimitive>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HeadlessUiPrimitive {
    Rectangle {
        primitive_id: String,
        control_id: String,
        role: HeadlessPrimitiveRole,
        bounds: HeadlessRect,
        radius: f32,
        color: HeadlessColor,
        sort_key: HeadlessSortKey,
        #[serde(default)]
        source_map_index: Option<u32>,
    },
    Border {
        primitive_id: String,
        control_id: String,
        role: HeadlessPrimitiveRole,
        bounds: HeadlessRect,
        radius: f32,
        width: f32,
        color: HeadlessColor,
        sort_key: HeadlessSortKey,
        #[serde(default)]
        source_map_index: Option<u32>,
    },
    Text {
        primitive_id: String,
        control_id: String,
        role: HeadlessPrimitiveRole,
        run: HeadlessTextRun,
        sort_key: HeadlessSortKey,
        #[serde(default)]
        source_map_index: Option<u32>,
    },
}

impl HeadlessUiPrimitive {
    pub fn sort_key(&self) -> HeadlessSortKey {
        match self {
            Self::Rectangle { sort_key, .. }
            | Self::Border { sort_key, .. }
            | Self::Text { sort_key, .. } => *sort_key,
        }
    }

    pub fn source_map_index(&self) -> Option<u32> {
        match self {
            Self::Rectangle {
                source_map_index, ..
            }
            | Self::Border {
                source_map_index, ..
            }
            | Self::Text {
                source_map_index, ..
            } => *source_map_index,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessPrimitiveRole {
    ButtonBackground,
    ButtonOutline,
    LabelText,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessRenderDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: HeadlessRenderDiagnosticSeverity,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl HeadlessRenderDiagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        source_map_index: Option<u32>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: HeadlessRenderDiagnosticSeverity::Error,
            source_map_index,
        }
    }

    pub fn info(
        code: impl Into<String>,
        message: impl Into<String>,
        source_map_index: Option<u32>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: HeadlessRenderDiagnosticSeverity::Info,
            source_map_index,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessRenderDiagnosticSeverity {
    Info,
    Warning,
    #[default]
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessRenderViewport {
    pub width: f32,
    pub height: f32,
}

impl HeadlessRenderViewport {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl HeadlessRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl HeadlessColor {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HeadlessSortKey {
    pub layer_order: u32,
    pub primitive_order: u32,
}

impl HeadlessSortKey {
    pub const fn new(layer_order: u32, primitive_order: u32) -> Self {
        Self {
            layer_order,
            primitive_order,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessLayoutStrategy {
    HeadlessFixedLayout,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessTextRun {
    pub text: String,
    pub bounds: HeadlessRect,
    pub color: HeadlessColor,
    pub font_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessRenderStyle {
    pub background: HeadlessColor,
    pub border: HeadlessColor,
    pub text: HeadlessColor,
    pub corner_radius: f32,
    pub border_width: f32,
    pub font_size: f32,
    pub horizontal_padding: f32,
    pub height: f32,
    pub min_width: f32,
}

impl HeadlessRenderStyle {
    fn from_button_properties(variant: &str, tone: &str, density: &str, size: &str) -> Self {
        let (background, border, text) = button_palette(variant, tone);
        let horizontal_padding = match density {
            "compact" => 12.0,
            "normal" => 16.0,
            "spacious" => 20.0,
            _ => 16.0,
        };
        let (height, min_width, font_size) = match size {
            "sm" => (32.0, 80.0, 13.0),
            "md" => (40.0, 104.0, 14.0),
            "lg" => (48.0, 128.0, 16.0),
            _ => (40.0, 104.0, 14.0),
        };

        Self {
            background,
            border,
            text,
            corner_radius: 6.0,
            border_width: 1.0,
            font_size,
            horizontal_padding,
            height,
            min_width,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HeadlessFixedLayout {
    pub bounds: HeadlessRect,
    pub text_bounds: HeadlessRect,
}

impl HeadlessFixedLayout {
    fn for_control_index(index: u32, style: &HeadlessRenderStyle) -> Self {
        let x = 24.0;
        let y = 24.0 + index as f32 * (style.height + 12.0);
        let width = style.min_width;
        let bounds = HeadlessRect::new(x, y, width, style.height);
        let text_bounds = HeadlessRect::new(
            x + style.horizontal_padding,
            y,
            width - style.horizontal_padding * 2.0,
            style.height,
        );

        Self {
            bounds,
            text_bounds,
        }
    }
}

fn button_palette(variant: &str, tone: &str) -> (HeadlessColor, HeadlessColor, HeadlessColor) {
    match (variant, tone) {
        ("secondary", "neutral") => (
            HeadlessColor::rgba(0.88, 0.90, 0.93, 1.0),
            HeadlessColor::rgba(0.45, 0.49, 0.55, 1.0),
            HeadlessColor::rgba(0.08, 0.10, 0.13, 1.0),
        ),
        ("secondary", _) => (
            HeadlessColor::rgba(0.86, 0.90, 0.98, 1.0),
            HeadlessColor::rgba(0.38, 0.48, 0.78, 1.0),
            HeadlessColor::rgba(0.08, 0.12, 0.22, 1.0),
        ),
        ("primary", "accent") => (
            HeadlessColor::rgba(0.18, 0.39, 0.88, 1.0),
            HeadlessColor::rgba(0.12, 0.27, 0.62, 1.0),
            HeadlessColor::rgba(1.0, 1.0, 1.0, 1.0),
        ),
        ("primary", _) => (
            HeadlessColor::rgba(0.20, 0.24, 0.31, 1.0),
            HeadlessColor::rgba(0.12, 0.15, 0.20, 1.0),
            HeadlessColor::rgba(1.0, 1.0, 1.0, 1.0),
        ),
        _ => (
            HeadlessColor::rgba(0.92, 0.93, 0.95, 1.0),
            HeadlessColor::rgba(0.58, 0.61, 0.66, 1.0),
            HeadlessColor::rgba(0.10, 0.12, 0.16, 1.0),
        ),
    }
}

fn required_string_property<'a>(
    control: &'a RuntimeControlView,
    property_name: &str,
    diagnostics: &mut Vec<HeadlessRenderDiagnostic>,
) -> Option<&'a str> {
    let property = control.property()?;
    let value = property
        .snapshot
        .value
        .get(property_name)
        .and_then(UiSchemaValue::as_str);
    if value.is_none() {
        diagnostics.push(HeadlessRenderDiagnostic::error(
            DIAGNOSTIC_CONTROL_MISSING_PROPERTY_VALUE,
            format!(
                "control {} is missing string property {}",
                control.control_id().as_str(),
                property_name
            ),
            property.source_map_index,
        ));
    }

    value
}

fn optional_string_property_or<'a>(
    control: &'a RuntimeControlView,
    property_name: &str,
    fallback: &'a str,
) -> &'a str {
    control
        .property()
        .and_then(|property| property.snapshot.value.get(property_name))
        .and_then(UiSchemaValue::as_str)
        .unwrap_or(fallback)
}

fn primitive_id(control_id: &ControlNodeId, primitive_kind: &str) -> String {
    format!("headless.{}.{}", control_id.as_str(), primitive_kind)
}
