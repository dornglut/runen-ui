use runenui_core::LogicalRect;
use skrifa::{
    FontRef, MetadataProvider,
    instance::{LocationRef, NormalizedCoord, Size},
    outline::{DrawSettings, pen::ControlBoundsPen},
    raw::TableProvider,
};

use crate::ShapedTextResource;

/// Conservative renderer-neutral logical ink bounds for one immutable shaped resource.
///
/// The value is derived only from the exact retained font/index/variation/synthesis and
/// positioned-glyph facts. It contains no raster scale, atlas, tessellation, device, or cache
/// state. `Unbounded` is the conservative top value when a finite logical AABB cannot be proven
/// without inventing unsupported glyph semantics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextInkBounds {
    /// The shaped resource has no ordinary outline ink coverage.
    Empty,
    /// A finite conservative resource-local logical AABB. Zero extent remains representable.
    Finite(LogicalRect),
    /// No finite conservative logical AABB can be proven from accepted neutral facts.
    Unbounded,
}

impl TextInkBounds {
    /// Returns the finite logical rectangle when one is available.
    #[must_use]
    pub const fn finite_rect(self) -> Option<LogicalRect> {
        match self {
            Self::Finite(rect) => Some(rect),
            Self::Empty | Self::Unbounded => None,
        }
    }

    /// Returns whether this resource has no ordinary outline ink coverage.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns whether only the conservative top bound is available.
    #[must_use]
    pub const fn is_unbounded(self) -> bool {
        matches!(self, Self::Unbounded)
    }
}

impl ShapedTextResource {
    /// Derives conservative resource-local logical ink bounds from the exact shaped binding.
    ///
    /// Supported scalable outlines are drawn unhinted at the retained logical font size and
    /// measured with Skrifa's control-bounds pen. Bézier control bounds conservatively enclose
    /// curve coverage without any flattening tolerance becoming observable semantics. The
    /// retained faux-skew synthesis is then applied with the same logical shear convention used
    /// by the renderer. Faux bold and intrinsic color/bitmap/SVG glyphs remain unsupported
    /// breadth and therefore produce [`TextInkBounds::Unbounded`] rather than fabricated ink.
    #[must_use]
    pub fn logical_ink_bounds(&self) -> TextInkBounds {
        if self.glyphs().is_empty() || self.font_size() == 0.0 {
            return TextInkBounds::Empty;
        }
        if self.font_size() < 0.0 || self.font().faux_bold() {
            return TextInkBounds::Unbounded;
        }

        let Ok(font) = FontRef::from_index(self.font().bytes(), self.font().face_index()) else {
            return TextInkBounds::Unbounded;
        };
        let normalized = self
            .font()
            .normalized_coords()
            .iter()
            .copied()
            .map(NormalizedCoord::from_bits)
            .collect::<Vec<_>>();
        let location = LocationRef::new(&normalized);
        let size = Size::new(self.font_size());
        let outlines = font.outline_glyphs();
        let colors = font.color_glyphs();
        let bitmaps = font.bitmap_strikes();
        let svg = font.svg().ok();
        let skew = self
            .font()
            .faux_skew()
            .map_or(0.0_f64, |angle| f64::from(angle).tan());
        if !skew.is_finite() {
            return TextInkBounds::Unbounded;
        }

        let mut bounds = None::<Bounds>;
        for glyph in self.glyphs() {
            let glyph_id = skrifa::GlyphId::new(glyph.id());
            if colors.get(glyph_id).is_some()
                || bitmaps.glyph_for_size(size, glyph_id).is_some()
                || svg
                    .as_ref()
                    .and_then(|table| table.glyph_data(glyph_id).ok().flatten())
                    .is_some()
            {
                return TextInkBounds::Unbounded;
            }

            let Some(outline) = outlines.get(glyph_id) else {
                // Missing scalable and intrinsic representation is valid non-painting content
                // (for example ordinary whitespace), matching current renderer semantics.
                continue;
            };
            let mut pen = ControlBoundsPen::default();
            if outline
                .draw(DrawSettings::unhinted(size, location), &mut pen)
                .is_err()
            {
                return TextInkBounds::Unbounded;
            }
            let Some(glyph_bounds) = pen.bounding_box() else {
                continue;
            };

            for (x, y) in [
                (glyph_bounds.x_min, glyph_bounds.y_min),
                (glyph_bounds.x_min, glyph_bounds.y_max),
                (glyph_bounds.x_max, glyph_bounds.y_min),
                (glyph_bounds.x_max, glyph_bounds.y_max),
            ] {
                let y = -f64::from(y);
                let x = skew.mul_add(y, f64::from(x));
                let x = x + f64::from(glyph.x());
                let y = y + f64::from(glyph.y());
                if !x.is_finite() || !y.is_finite() {
                    return TextInkBounds::Unbounded;
                }
                bounds
                    .get_or_insert_with(|| Bounds::point(x, y))
                    .include(x, y);
            }
        }

        bounds.map_or(TextInkBounds::Empty, |bounds| {
            bounds
                .logical_rect()
                .map_or(TextInkBounds::Unbounded, TextInkBounds::Finite)
        })
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    const fn point(x: f64, y: f64) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    const fn include(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn logical_rect(self) -> Option<LogicalRect> {
        logical_rect_from_edges(self.min_x, self.min_y, self.max_x, self.max_y)
    }
}

fn logical_rect_from_edges(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Option<LogicalRect> {
    if ![min_x, min_y, max_x, max_y].into_iter().all(f64::is_finite)
        || max_x < min_x
        || max_y < min_y
    {
        return None;
    }
    let min_x = checked_f32_down(min_x)?;
    let min_y = checked_f32_down(min_y)?;
    let max_x = checked_f32_up(max_x)?;
    let max_y = checked_f32_up(max_y)?;
    let width = checked_f32_up(f64::from(max_x) - f64::from(min_x))?;
    let height = checked_f32_up(f64::from(max_y) - f64::from(min_y))?;
    LogicalRect::try_new(min_x, min_y, width, height).ok()
}

#[allow(clippy::cast_possible_truncation)]
fn checked_f32(value: f64) -> Option<f32> {
    if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
        return None;
    }
    Some(value as f32)
}

fn checked_f32_down(value: f64) -> Option<f32> {
    let rounded = checked_f32(value)?;
    Some(if f64::from(rounded) > value {
        rounded.next_down()
    } else {
        rounded
    })
}

fn checked_f32_up(value: f64) -> Option<f32> {
    let rounded = checked_f32(value)?;
    Some(if f64::from(rounded) < value {
        rounded.next_up()
    } else {
        rounded
    })
}

#[cfg(test)]
mod tests {
    use runenui_core::{FontFamily, LogicalLength, Typography};

    use crate::{
        FontSourcePolicy, TextConstraints, TextInkBounds, TextLayoutState, TextLine, TextRequest,
        TextRun, TextSystem,
    };

    const CANTARELL: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");

    fn typography() -> Typography {
        Typography::new(
            FontFamily::named("Cantarell").unwrap_or_else(|_| unreachable!()),
            LogicalLength::new(20.0).unwrap_or_else(|_| unreachable!()),
        )
    }

    #[test]
    fn supported_outline_resource_has_deterministic_finite_ink_bounds() {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        let mut state = TextLayoutState::new();
        system
            .register_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("fixture font registers"));
        let artifact = system
            .layout_text(
                &mut state,
                &TextRequest::new("RunenUI", typography(), TextConstraints::unbounded()),
            )
            .unwrap_or_else(|_| unreachable!("fixture text lays out"))
            .into_artifact();
        let resource = artifact.lines()[0].runs()[0].shaped_resource();
        let first = resource.logical_ink_bounds();
        let second = resource.logical_ink_bounds();
        assert_eq!(first, second);
        let TextInkBounds::Finite(rect) = first else {
            unreachable!("ordinary outline text must have finite ink bounds");
        };
        assert!(rect.width() > 0.0);
        assert!(rect.height() > 0.0);
    }

    #[test]
    fn whitespace_without_outline_ink_is_empty() {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        let mut state = TextLayoutState::new();
        system
            .register_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("fixture font registers"));
        let artifact = system
            .layout_text(
                &mut state,
                &TextRequest::new(" ", typography(), TextConstraints::unbounded()),
            )
            .unwrap_or_else(|_| unreachable!("fixture whitespace lays out"))
            .into_artifact();
        let mut saw_resource = false;
        for resource in artifact
            .lines()
            .iter()
            .flat_map(TextLine::runs)
            .map(TextRun::shaped_resource)
        {
            saw_resource = true;
            assert!(resource.logical_ink_bounds().is_empty());
        }
        assert!(saw_resource);
    }
}
