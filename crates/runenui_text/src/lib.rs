#![forbid(unsafe_code)]
//! Renderer-neutral production text-system foundations.
//!
//! `RunenUI` owns the public contracts in this crate. Parley and Fontique remain
//! private implementation dependencies and must not become public API authority.

mod artifact;
mod layout_extract;

use core::{error::Error, fmt};
use std::{collections::HashMap, sync::Arc};

use parley::{
    FontContext, LayoutContext,
    fontique::{Blob, Collection, CollectionOptions, SourceCache},
};
use runenui_core::{LogicalLength, ResourceRef};

pub use artifact::{
    ShapedTextResource, TextArtifact, TextCluster, TextFontBinding, TextGlyph, TextLine,
    TextLineMetrics, TextRun,
};

/// Explicit font-source policy for one text system.
///
/// Deterministic consumers use [`Self::BundledOnly`]. Production hosts that
/// intentionally permit ambient system discovery use [`Self::SystemAndBundled`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontSourcePolicy {
    BundledOnly,
    SystemAndBundled,
}

impl FontSourcePolicy {
    const fn discovers_system_fonts(self) -> bool {
        matches!(self, Self::SystemAndBundled)
    }
}

/// Cache-visible revision of the configured font-source set.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FontSourceRevision(u64);

impl FontSourceRevision {
    pub const ZERO: Self = Self(0);

    /// Returns the opaque monotonic revision value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    fn next(self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }
}

/// Text-specific logical constraints independent of runtime layout types.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextConstraints {
    max_inline: Option<LogicalLength>,
}

impl TextConstraints {
    /// Unbounded inline layout.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self { max_inline: None }
    }

    /// Layout constrained to at most `max_inline` logical units.
    #[must_use]
    pub const fn limited(max_inline: LogicalLength) -> Self {
        Self {
            max_inline: Some(max_inline),
        }
    }

    /// Returns the available inline extent, or `None` when unbounded.
    #[must_use]
    pub const fn max_inline(self) -> Option<LogicalLength> {
        self.max_inline
    }
}

/// Failure while changing the explicit font-source set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontRegistrationError {
    NoFonts,
    RevisionExhausted,
}

impl fmt::Display for FontRegistrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFonts => formatter.write_str("font data contains no registerable font faces"),
            Self::RevisionExhausted => formatter.write_str("font-source revision is exhausted"),
        }
    }
}

impl Error for FontRegistrationError {}

/// Coarse-grained renderer-neutral text-system authority.
///
/// The underlying Parley/Fontique contexts are deliberately private. Consumers
/// configure font sources through `RunenUI`-owned operations rather than reaching
/// into the dependency stack. Live shaped [`ResourceRef`] values are retained here
/// with their immutable logical payloads so measurement/publication retry cannot
/// outlive the content binding.
pub struct TextSystem {
    font_context: FontContext,
    layout_context: LayoutContext,
    shaped_resources: HashMap<ResourceRef, Arc<ShapedTextResource>>,
    source_policy: FontSourcePolicy,
    source_revision: FontSourceRevision,
}

impl TextSystem {
    /// Creates one text system with explicit ambient-font policy.
    #[must_use]
    pub fn new(source_policy: FontSourcePolicy) -> Self {
        let font_context = FontContext {
            collection: Collection::new(CollectionOptions {
                shared: false,
                system_fonts: source_policy.discovers_system_fonts(),
            }),
            source_cache: SourceCache::default(),
        };
        Self {
            font_context,
            layout_context: LayoutContext::new(),
            shaped_resources: HashMap::new(),
            source_policy,
            source_revision: FontSourceRevision::ZERO,
        }
    }

    /// Returns the explicit font-source policy used by this text system.
    #[must_use]
    pub const fn source_policy(&self) -> FontSourcePolicy {
        self.source_policy
    }

    /// Returns the revision participating in text cache compatibility.
    #[must_use]
    pub const fn source_revision(&self) -> FontSourceRevision {
        self.source_revision
    }

    /// Resolves one live scale-independent shaped resource by its sole opaque identity.
    ///
    /// The returned strong reference preserves the immutable logical glyph/font binding while a
    /// renderer realizes or retries the resource. Raster scale is intentionally not an input.
    #[must_use]
    pub fn resolve_shaped_run(
        &self,
        resource: &ResourceRef,
    ) -> Option<Arc<ShapedTextResource>> {
        self.shaped_resources.get(resource).cloned()
    }

    /// Registers immutable bundled font bytes and advances the source revision.
    ///
    /// The returned value is the number of font faces discovered in the source.
    ///
    /// # Errors
    ///
    /// Returns [`FontRegistrationError::NoFonts`] when the bytes contain no
    /// registerable faces, or [`FontRegistrationError::RevisionExhausted`] when
    /// the monotonic source revision cannot advance.
    pub fn register_font_bytes(&mut self, bytes: Vec<u8>) -> Result<usize, FontRegistrationError> {
        let next_revision = self
            .source_revision
            .next()
            .ok_or(FontRegistrationError::RevisionExhausted)?;
        let blob = Blob::new(Arc::new(bytes));
        let registered = self.font_context.collection.register_fonts(blob, None);
        let face_count = registered
            .iter()
            .map(|(_, fonts)| fonts.len())
            .sum::<usize>();
        if face_count == 0 {
            return Err(FontRegistrationError::NoFonts);
        }
        self.source_revision = next_revision;
        Ok(face_count)
    }

    #[cfg(test)]
    fn shape_fixture(
        &mut self,
        text: &str,
        family: &str,
        font_size: f32,
        constraints: TextConstraints,
    ) -> Option<TextArtifact> {
        use parley::{Alignment, AlignmentOptions, FontFamily, Layout, StyleProperty};

        let mut builder = self
            .layout_context
            .ranged_builder(&mut self.font_context, text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(FontFamily::named(family)));
        builder.push_default(StyleProperty::FontSize(font_size));
        let mut layout: Layout<()> = builder.build(text);
        layout.break_all_lines(constraints.max_inline().map(LogicalLength::get));
        layout.align(Alignment::Start, AlignmentOptions::default());
        layout_extract::extract_layout(
            &layout,
            self.source_revision,
            &mut self.shaped_resources,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{FontSourcePolicy, FontSourceRevision, TextConstraints, TextSystem};
    use runenui_core::{LogicalLength, ResourceKind};

    const CANTARELL: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");

    #[test]
    fn font_source_policy_and_initial_revision_are_explicit() {
        let deterministic = TextSystem::new(FontSourcePolicy::BundledOnly);
        assert_eq!(deterministic.source_policy(), FontSourcePolicy::BundledOnly);
        assert_eq!(deterministic.source_revision(), FontSourceRevision::ZERO);

        let production = TextSystem::new(FontSourcePolicy::SystemAndBundled);
        assert_eq!(
            production.source_policy(),
            FontSourcePolicy::SystemAndBundled
        );
        assert_eq!(production.source_revision(), FontSourceRevision::ZERO);
    }

    #[test]
    fn text_constraints_are_renderer_and_runtime_neutral_values() {
        assert_eq!(TextConstraints::unbounded().max_inline(), None);
        let width = LogicalLength::new(320.0)
            .unwrap_or_else(|_| unreachable!("fixture width is a valid logical extent"));
        assert_eq!(TextConstraints::limited(width).max_inline(), Some(width));
    }

    #[test]
    fn bundled_shaping_produces_one_measure_and_resource_artifact() {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        let faces = system
            .register_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|error| panic!("Cantarell fixture registration failed: {error}"));
        assert!(faces > 0);

        let artifact = system
            .shape_fixture(
                "RunenUI shaping",
                "Cantarell",
                18.0,
                TextConstraints::unbounded(),
            )
            .unwrap_or_else(|| panic!("fixture shaping must yield a valid logical artifact"));

        assert_eq!(artifact.source_revision(), system.source_revision());
        assert!(artifact.size().width() > 0.0);
        assert!(artifact.size().height() > 0.0);
        let run = artifact
            .lines()
            .first()
            .and_then(|line| line.runs().first())
            .unwrap_or_else(|| panic!("fixture must produce a positioned shaped run"));
        assert_eq!(run.resource_ref().kind(), ResourceKind::ShapedTextRun);
        assert!(!run.shaped_resource().glyphs().is_empty());
        assert_eq!(run.shaped_resource().font().bytes(), CANTARELL);
    }

    #[test]
    fn shaped_resource_binding_survives_artifact_drop_without_raster_scale() {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        system
            .register_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|error| panic!("Cantarell fixture registration failed: {error}"));
        let artifact = system
            .shape_fixture(
                "retry safe",
                "Cantarell",
                16.0,
                TextConstraints::unbounded(),
            )
            .unwrap_or_else(|| panic!("fixture shaping must yield a valid logical artifact"));
        let resource = artifact.lines()[0].runs()[0].resource_ref().clone();
        let glyph_count = artifact.lines()[0].runs()[0]
            .shaped_resource()
            .glyphs()
            .len();

        drop(artifact);

        let retained = system
            .resolve_shaped_run(&resource)
            .unwrap_or_else(|| panic!("live shaped identity must retain immutable content"));
        assert_eq!(retained.resource_ref(), &resource);
        assert_eq!(retained.glyphs().len(), glyph_count);
        assert_eq!(retained.font().bytes(), CANTARELL);
    }
}
