//! Renderer-private exact outline normalization for already-shaped text.
//!
//! The retained `ShapedTextResource` remains the sole logical text authority. This
//! module performs one deterministic Skrifa traversal into scale-independent `f64`
//! outline verbs. MSDF realization may consume those verbs without narrowing the
//! existing geometry path; ADR 0015 neutral support projects the same verbs into
//! positioned `RunenUI` `ScenePath` geometry. Raster scale, atlas state, sampled alpha,
//! paint alpha, target state, and device state are deliberately absent.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use runenui_core::{LogicalPoint, PathFillRule, PathVerb, ScenePath};
use runenui_text::{ShapedTextResource, TextGlyph};
use skrifa::raw::TableProvider;
use skrifa::{
    FontRef, MetadataProvider,
    color::ColorGlyphFormat,
    instance::{LocationRef, NormalizedCoord, Size},
    outline::{DrawSettings, OutlinePen},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UnsupportedOutlineKind {
    ColrV0,
    ColrV1,
    Bitmap,
    Svg,
    FauxBold,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OutlineResolveFailure {
    UnsupportedGlyph {
        glyph_id: u32,
        kind: UnsupportedOutlineKind,
    },
    InvalidFont,
    InvalidOutline {
        glyph_id: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct OutlinePoint {
    x: f64,
    y: f64,
}

impl OutlinePoint {
    pub(super) const fn x(self) -> f64 {
        self.x
    }

    pub(super) const fn y(self) -> f64 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum OutlineVerb {
    MoveTo(OutlinePoint),
    LineTo(OutlinePoint),
    QuadraticTo {
        control: OutlinePoint,
        to: OutlinePoint,
    },
    CubicTo {
        control1: OutlinePoint,
        control2: OutlinePoint,
        to: OutlinePoint,
    },
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GlyphOutline {
    verbs: Arc<[OutlineVerb]>,
}

impl GlyphOutline {
    pub(super) fn verbs(&self) -> &[OutlineVerb] {
        &self.verbs
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct UniqueGlyphOutline {
    glyph_id: u32,
    outline: Option<GlyphOutline>,
}

impl UniqueGlyphOutline {
    pub(super) const fn glyph_id(&self) -> u32 {
        self.glyph_id
    }

    pub(super) const fn outline(&self) -> Option<&GlyphOutline> {
        self.outline.as_ref()
    }
}

/// Resolves each distinct glyph id exactly once into normalized renderer-private
/// `f64` outline geometry. Missing scalable/intrinsic representation remains valid
/// non-painting content, matching the established shaped-text renderer contract.
pub(super) fn resolve_unique_outlines(
    resource: &ShapedTextResource,
) -> Result<Vec<UniqueGlyphOutline>, OutlineResolveFailure> {
    let font = FontRef::from_index(resource.font().bytes(), resource.font().face_index())
        .map_err(|_| OutlineResolveFailure::InvalidFont)?;
    let normalized = resource
        .font()
        .normalized_coords()
        .iter()
        .copied()
        .map(NormalizedCoord::from_bits)
        .collect::<Vec<_>>();
    let location = LocationRef::new(&normalized);
    let outlines = font.outline_glyphs();
    let colors = font.color_glyphs();
    let bitmaps = font.bitmap_strikes();
    let svg = font.svg().ok();
    let intrinsic_size = Size::new(resource.font_size());
    let mut seen = HashSet::new();
    let mut resolved = Vec::new();

    for glyph in resource.glyphs() {
        if !seen.insert(glyph.id()) {
            continue;
        }
        if resource.font().faux_bold() {
            return Err(OutlineResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind: UnsupportedOutlineKind::FauxBold,
            });
        }

        let glyph_id = skrifa::GlyphId::new(glyph.id());
        if svg
            .as_ref()
            .and_then(|table| table.glyph_data(glyph_id).ok().flatten())
            .is_some()
        {
            return Err(OutlineResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind: UnsupportedOutlineKind::Svg,
            });
        }
        if let Some(color) = colors.get(glyph_id) {
            let kind = match color.format() {
                ColorGlyphFormat::ColrV0 => UnsupportedOutlineKind::ColrV0,
                ColorGlyphFormat::ColrV1 => UnsupportedOutlineKind::ColrV1,
            };
            return Err(OutlineResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind,
            });
        }
        if bitmaps.glyph_for_size(intrinsic_size, glyph_id).is_some() {
            return Err(OutlineResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind: UnsupportedOutlineKind::Bitmap,
            });
        }

        let outline = if let Some(outline) = outlines.get(glyph_id) {
            let mut pen = OutlinePathPen::new(resource.font().faux_skew());
            outline
                .draw(DrawSettings::unhinted(Size::new(1.0), location), &mut pen)
                .map_err(|_| OutlineResolveFailure::InvalidOutline {
                    glyph_id: glyph.id(),
                })?;
            pen.finish()
                .map_err(|()| OutlineResolveFailure::InvalidOutline {
                    glyph_id: glyph.id(),
                })?
        } else {
            None
        };
        resolved.push(UniqueGlyphOutline {
            glyph_id: glyph.id(),
            outline,
        });
    }

    Ok(resolved)
}

/// Resolves painting glyph occurrences into positioned resource-local logical
/// `ScenePath` values. Distinct glyph ids share one `f64` extraction, while every
/// shaped occurrence retains its exact logical position before the final checked
/// narrowing into `RunenUI`'s logical `f32` coordinate vocabulary.
pub(super) fn resolve_positioned_paths(
    resource: &ShapedTextResource,
    run_origin: LogicalPoint,
) -> Result<Vec<ScenePath>, OutlineResolveFailure> {
    let outlines = resolve_unique_outlines(resource)?;
    let by_id = outlines
        .iter()
        .map(|outline| (outline.glyph_id(), outline.outline()))
        .collect::<HashMap<_, _>>();
    let mut paths = Vec::new();
    for glyph in resource.glyphs() {
        let glyph_id = glyph.id();
        let Some(Some(outline)) = by_id.get(&glyph_id).copied() else {
            continue;
        };
        let positioned = positioned_scene_path(outline, *glyph, run_origin, resource.font_size())
            .map_err(|()| OutlineResolveFailure::InvalidOutline { glyph_id })?;
        if !positioned.is_coverage_empty() {
            paths.push(positioned);
        }
    }
    Ok(paths)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContourState {
    Closed,
    OpenMoveOnly,
    OpenSegmentBearing,
}

impl ContourState {
    const fn is_open(self) -> bool {
        !matches!(self, Self::Closed)
    }
}

struct OutlinePathPen {
    verbs: Vec<OutlineVerb>,
    contour: ContourState,
    has_any_segment: bool,
    invalid: bool,
    skew: f64,
}

impl OutlinePathPen {
    fn new(faux_skew: Option<f32>) -> Self {
        let skew = faux_skew.map_or(0.0, f64::from).tan();
        Self {
            verbs: Vec::new(),
            contour: ContourState::Closed,
            has_any_segment: false,
            invalid: !skew.is_finite(),
            skew,
        }
    }

    fn point(&mut self, x: f32, y: f32) -> Option<OutlinePoint> {
        let y = -f64::from(y);
        let x = self.skew.mul_add(y, f64::from(x));
        if !x.is_finite() || !y.is_finite() {
            self.invalid = true;
            return None;
        }
        Some(OutlinePoint { x, y })
    }

    fn finish_contour(&mut self) {
        if matches!(self.contour, ContourState::OpenSegmentBearing) {
            self.verbs.push(OutlineVerb::Close);
        }
        self.contour = ContourState::Closed;
    }

    fn finish(mut self) -> Result<Option<GlyphOutline>, ()> {
        self.finish_contour();
        if self.invalid {
            return Err(());
        }
        Ok(self.has_any_segment.then(|| GlyphOutline {
            verbs: self.verbs.into(),
        }))
    }

    const fn mark_segment(&mut self) {
        self.contour = ContourState::OpenSegmentBearing;
        self.has_any_segment = true;
    }
}

impl OutlinePen for OutlinePathPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish_contour();
        if let Some(point) = self.point(x, y) {
            self.verbs.push(OutlineVerb::MoveTo(point));
            self.contour = ContourState::OpenMoveOnly;
        }
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let Some(to) = self.point(x, y) else {
            return;
        };
        if !self.contour.is_open() {
            self.invalid = true;
            return;
        }
        self.verbs.push(OutlineVerb::LineTo(to));
        self.mark_segment();
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let Some(control) = self.point(cx, cy) else {
            return;
        };
        let Some(to) = self.point(x, y) else {
            return;
        };
        if !self.contour.is_open() {
            self.invalid = true;
            return;
        }
        self.verbs.push(OutlineVerb::QuadraticTo { control, to });
        self.mark_segment();
    }

    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        let Some(control1) = self.point(c0x, c0y) else {
            return;
        };
        let Some(control2) = self.point(c1x, c1y) else {
            return;
        };
        let Some(to) = self.point(x, y) else {
            return;
        };
        if !self.contour.is_open() {
            self.invalid = true;
            return;
        }
        self.verbs.push(OutlineVerb::CubicTo {
            control1,
            control2,
            to,
        });
        self.mark_segment();
    }

    fn close(&mut self) {
        self.finish_contour();
    }
}

fn positioned_scene_path(
    outline: &GlyphOutline,
    glyph: TextGlyph,
    run_origin: LogicalPoint,
    font_size: f32,
) -> Result<ScenePath, ()> {
    let verbs = outline
        .verbs()
        .iter()
        .copied()
        .map(|verb| match verb {
            OutlineVerb::MoveTo(point) => {
                position_point(point, glyph, run_origin, font_size).map(PathVerb::MoveTo)
            }
            OutlineVerb::LineTo(point) => {
                position_point(point, glyph, run_origin, font_size).map(PathVerb::LineTo)
            }
            OutlineVerb::QuadraticTo { control, to } => Ok(PathVerb::QuadraticTo {
                control: position_point(control, glyph, run_origin, font_size)?,
                to: position_point(to, glyph, run_origin, font_size)?,
            }),
            OutlineVerb::CubicTo {
                control1,
                control2,
                to,
            } => Ok(PathVerb::CubicTo {
                control1: position_point(control1, glyph, run_origin, font_size)?,
                control2: position_point(control2, glyph, run_origin, font_size)?,
                to: position_point(to, glyph, run_origin, font_size)?,
            }),
            OutlineVerb::Close => Ok(PathVerb::Close),
        })
        .collect::<Result<Vec<_>, ()>>()?;
    ScenePath::new(verbs, PathFillRule::NonZero).map_err(|_| ())
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "positioned f64 glyph support narrows to logical f32 only after explicit finite/range validation"
)]
fn position_point(
    point: OutlinePoint,
    glyph: TextGlyph,
    run_origin: LogicalPoint,
    font_size: f32,
) -> Result<LogicalPoint, ()> {
    let x = point.x().mul_add(
        f64::from(font_size),
        f64::from(glyph.x()) + f64::from(run_origin.x()),
    );
    let y = point.y().mul_add(
        f64::from(font_size),
        f64::from(glyph.y()) + f64::from(run_origin.y()),
    );
    if !x.is_finite()
        || !y.is_finite()
        || x < f64::from(f32::MIN)
        || x > f64::from(f32::MAX)
        || y < f64::from(f32::MIN)
        || y > f64::from(f32::MAX)
    {
        return Err(());
    }
    LogicalPoint::new(x as f32, y as f32).map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use runenui_core::{FontFamilyName, GenericFontFamily, LogicalPoint, Typography};
    use runenui_text::{FontSourcePolicy, TextConstraints, TextRequest, TextSystem};

    use super::{OutlineVerb, resolve_positioned_paths, resolve_unique_outlines};

    const FONT_BYTES: &[u8] = include_bytes!("../../../tests/fixtures/Cantarell-Regular.ttf");

    fn shaped_resource(text: &str) -> (TextSystem, runenui_text::TextArtifact) {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        assert!(system.register_font_bytes(FONT_BYTES.to_vec()).is_ok());
        let family = FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!());
        assert!(
            system
                .set_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
                .is_ok()
        );
        let request = TextRequest::new(text, Typography::default(), TextConstraints::unbounded());
        let artifact = system
            .layout_text(&mut runenui_text::TextLayoutState::new(), &request)
            .unwrap_or_else(|_| unreachable!("controlled bundled text shapes"))
            .into_artifact();
        (system, artifact)
    }

    #[test]
    fn repeated_glyph_ids_share_one_normalized_outline() {
        let (_system, artifact) = shaped_resource("AAAA");
        let resource = artifact.lines()[0].runs()[0].shaped_resource();
        let outlines = resolve_unique_outlines(resource)
            .unwrap_or_else(|_| unreachable!("Cantarell outlines resolve"));
        assert_eq!(resource.glyphs().len(), 4);
        assert_eq!(outlines.len(), 1);
        let outline = outlines[0]
            .outline()
            .unwrap_or_else(|| unreachable!("A has an outline"));
        assert!(outline.verbs().iter().any(|verb| matches!(
            verb,
            OutlineVerb::LineTo(_) | OutlineVerb::QuadraticTo { .. } | OutlineVerb::CubicTo { .. }
        )));
    }

    #[test]
    fn whitespace_is_valid_non_painting_outline_content() {
        let (_system, artifact) = shaped_resource(" ");
        let resource = artifact.lines()[0].runs()[0].shaped_resource();
        let paths = resolve_positioned_paths(
            resource,
            LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!()),
        )
        .unwrap_or_else(|_| unreachable!("whitespace resolves without painting paths"));
        assert!(paths.is_empty());
    }

    #[test]
    fn positioned_paths_are_deterministic_and_follow_run_origin() {
        let (_system, artifact) = shaped_resource("T");
        let resource = artifact.lines()[0].runs()[0].shaped_resource();
        let first_origin = LogicalPoint::new(2.0, 3.0).unwrap_or_else(|_| unreachable!());
        let second_origin = LogicalPoint::new(12.0, 9.0).unwrap_or_else(|_| unreachable!());
        let first = resolve_positioned_paths(resource, first_origin)
            .unwrap_or_else(|_| unreachable!("first positioned outline resolves"));
        let repeated = resolve_positioned_paths(resource, first_origin)
            .unwrap_or_else(|_| unreachable!("repeated positioned outline resolves"));
        let second = resolve_positioned_paths(resource, second_origin)
            .unwrap_or_else(|_| unreachable!("second positioned outline resolves"));
        assert_eq!(first, repeated);
        assert_eq!(first.len(), second.len());
        assert!(!first.is_empty());
        assert_ne!(first[0].logical_bounds(), second[0].logical_bounds());
    }
}
