//! File: domain/ui/ui_render_data/src/frame/output_evidence.rs
//! Crate: ui_render_data
//! Purpose: Renderer-neutral output evidence and primitive-family summaries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{UiFrame, UiPrimitive, UiSurface};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiPrimitiveFamily {
    Rect,
    Border,
    GlyphRun,
    Image,
    Stroke,
    ViewportSurfaceEmbed,
    ProductSurface,
    Clip,
}

impl UiPrimitiveFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Border => "border",
            Self::GlyphRun => "glyph-run",
            Self::Image => "image",
            Self::Stroke => "stroke",
            Self::ViewportSurfaceEmbed => "viewport-surface-embed",
            Self::ProductSurface => "product-surface",
            Self::Clip => "clip",
        }
    }

    pub const fn from_primitive(primitive: &UiPrimitive) -> Self {
        match primitive {
            UiPrimitive::Rect(_) => Self::Rect,
            UiPrimitive::Border(_) => Self::Border,
            UiPrimitive::GlyphRun(_) => Self::GlyphRun,
            UiPrimitive::Image(_) => Self::Image,
            UiPrimitive::Stroke(_) => Self::Stroke,
            UiPrimitive::ViewportSurfaceEmbed(_) => Self::ViewportSurfaceEmbed,
            UiPrimitive::ProductSurface(_) => Self::ProductSurface,
            UiPrimitive::Clip(_) => Self::Clip,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPrimitiveFamilyCount {
    pub family: UiPrimitiveFamily,
    pub count: u32,
}

impl UiPrimitiveFamilyCount {
    pub const fn new(family: UiPrimitiveFamily, count: u32) -> Self {
        Self { family, count }
    }

    pub fn label(&self) -> String {
        format!("{}:{}", self.family.as_str(), self.count)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiExpectedPrimitiveCount {
    pub family: UiPrimitiveFamily,
    pub min_count: u32,
    #[serde(default)]
    pub max_count: Option<u32>,
}

impl UiExpectedPrimitiveCount {
    pub const fn at_least(family: UiPrimitiveFamily, min_count: u32) -> Self {
        Self {
            family,
            min_count,
            max_count: None,
        }
    }

    pub const fn exactly(family: UiPrimitiveFamily, count: u32) -> Self {
        Self {
            family,
            min_count: count,
            max_count: Some(count),
        }
    }

    pub const fn with_max(mut self, max_count: u32) -> Self {
        self.max_count = Some(max_count);
        self
    }

    pub fn accepts(&self, count: u32) -> bool {
        count >= self.min_count && self.max_count.map(|max| count <= max).unwrap_or(true)
    }

    pub fn label(&self) -> String {
        match self.max_count {
            Some(max_count) if max_count == self.min_count => {
                format!("{}:{}", self.family.as_str(), self.min_count)
            }
            Some(max_count) => {
                format!("{}:{}..{}", self.family.as_str(), self.min_count, max_count)
            }
            None => format!("{}:{}..", self.family.as_str(), self.min_count),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSurfaceOutputSummary {
    pub surface_id: u64,
    pub layer_count: u32,
    pub primitive_count: u32,
    pub primitive_families: Vec<UiPrimitiveFamilyCount>,
}

impl UiSurfaceOutputSummary {
    pub fn from_surface(surface: &UiSurface) -> Self {
        let mut counts = BTreeMap::<UiPrimitiveFamily, u32>::new();
        let mut primitive_count = 0u32;
        for layer in &surface.layers {
            for primitive in &layer.primitives {
                primitive_count += 1;
                *counts
                    .entry(UiPrimitiveFamily::from_primitive(primitive))
                    .or_insert(0) += 1;
            }
        }
        Self {
            surface_id: surface.id.0,
            layer_count: surface.layers.len() as u32,
            primitive_count,
            primitive_families: counts
                .into_iter()
                .map(|(family, count)| UiPrimitiveFamilyCount::new(family, count))
                .collect(),
        }
    }

    pub fn count_for_family(&self, family: UiPrimitiveFamily) -> u32 {
        self.primitive_families
            .iter()
            .find(|entry| entry.family == family)
            .map(|entry| entry.count)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiFrameOutputSummary {
    pub surface_count: u32,
    pub layer_count: u32,
    pub primitive_count: u32,
    pub primitive_families: Vec<UiPrimitiveFamilyCount>,
    pub surfaces: Vec<UiSurfaceOutputSummary>,
}

impl UiFrameOutputSummary {
    pub fn from_frame(frame: &UiFrame) -> Self {
        let surfaces = frame
            .surfaces
            .iter()
            .map(UiSurfaceOutputSummary::from_surface)
            .collect::<Vec<_>>();
        let mut counts = BTreeMap::<UiPrimitiveFamily, u32>::new();
        let mut layer_count = 0u32;
        let mut primitive_count = 0u32;
        for surface in &surfaces {
            layer_count += surface.layer_count;
            primitive_count += surface.primitive_count;
            for family_count in &surface.primitive_families {
                *counts.entry(family_count.family).or_insert(0) += family_count.count;
            }
        }
        Self {
            surface_count: surfaces.len() as u32,
            layer_count,
            primitive_count,
            primitive_families: counts
                .into_iter()
                .map(|(family, count)| UiPrimitiveFamilyCount::new(family, count))
                .collect(),
            surfaces,
        }
    }

    pub fn count_for_family(&self, family: UiPrimitiveFamily) -> u32 {
        self.primitive_families
            .iter()
            .find(|entry| entry.family == family)
            .map(|entry| entry.count)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRenderOutputProvenance {
    pub producer_id: String,
    pub source_id: String,
}

impl UiRenderOutputProvenance {
    pub fn new(producer_id: impl Into<String>, source_id: impl Into<String>) -> Self {
        Self {
            producer_id: producer_id.into(),
            source_id: source_id.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiRenderOutputDiagnosticKind {
    EmptyOutput,
    MissingPrimitiveCount,
    ExcessPrimitiveCount,
    ExpectedFailure,
}

impl UiRenderOutputDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyOutput => "empty-output",
            Self::MissingPrimitiveCount => "missing-primitive-count",
            Self::ExcessPrimitiveCount => "excess-primitive-count",
            Self::ExpectedFailure => "expected-failure",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRenderOutputDiagnostic {
    pub diagnostic_id: String,
    pub kind: UiRenderOutputDiagnosticKind,
    pub message: String,
}

impl UiRenderOutputDiagnostic {
    pub fn new(
        diagnostic_id: impl Into<String>,
        kind: UiRenderOutputDiagnosticKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            diagnostic_id: diagnostic_id.into(),
            kind,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRenderOutputEvidence {
    pub evidence_id: String,
    pub provenance: UiRenderOutputProvenance,
    pub frame_summary: UiFrameOutputSummary,
    #[serde(default)]
    pub expected_primitive_counts: Vec<UiExpectedPrimitiveCount>,
    #[serde(default)]
    pub diagnostics: Vec<UiRenderOutputDiagnostic>,
}

impl UiRenderOutputEvidence {
    pub fn from_frame(
        evidence_id: impl Into<String>,
        provenance: UiRenderOutputProvenance,
        frame: &UiFrame,
        expected_primitive_counts: impl IntoIterator<Item = UiExpectedPrimitiveCount>,
    ) -> Self {
        let frame_summary = UiFrameOutputSummary::from_frame(frame);
        let expected_primitive_counts = expected_primitive_counts.into_iter().collect::<Vec<_>>();
        let diagnostics = diagnostics_for_summary(&frame_summary, &expected_primitive_counts);
        Self {
            evidence_id: evidence_id.into(),
            provenance,
            frame_summary,
            expected_primitive_counts,
            diagnostics,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

fn diagnostics_for_summary(
    summary: &UiFrameOutputSummary,
    expected_counts: &[UiExpectedPrimitiveCount],
) -> Vec<UiRenderOutputDiagnostic> {
    let mut diagnostics = Vec::new();
    if summary.primitive_count == 0 {
        diagnostics.push(UiRenderOutputDiagnostic::new(
            "ui.render.output.empty",
            UiRenderOutputDiagnosticKind::EmptyOutput,
            "render output emitted no primitives",
        ));
    }
    for expected in expected_counts {
        let actual_count = summary.count_for_family(expected.family);
        if actual_count < expected.min_count {
            diagnostics.push(UiRenderOutputDiagnostic::new(
                format!("ui.render.output.{}.missing", expected.family.as_str()),
                UiRenderOutputDiagnosticKind::MissingPrimitiveCount,
                format!(
                    "expected at least {} {} primitives, found {}",
                    expected.min_count,
                    expected.family.as_str(),
                    actual_count
                ),
            ));
        } else if expected
            .max_count
            .map(|max| actual_count > max)
            .unwrap_or(false)
        {
            let max_count = expected.max_count.expect("max checked above");
            diagnostics.push(UiRenderOutputDiagnostic::new(
                format!("ui.render.output.{}.excess", expected.family.as_str()),
                UiRenderOutputDiagnosticKind::ExcessPrimitiveCount,
                format!(
                    "expected at most {} {} primitives, found {}",
                    max_count,
                    expected.family.as_str(),
                    actual_count
                ),
            ));
        }
    }
    diagnostics
}
