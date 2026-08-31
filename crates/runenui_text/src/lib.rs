#![forbid(unsafe_code)]
//! Renderer-neutral production text-system foundations.
//!
//! `RunenUI` owns the public contracts in this crate. Parley and Fontique remain
//! private implementation dependencies and must not become public API authority.

mod artifact;
#[cfg(test)]
mod layout_extract;
mod source_identity;

use core::{error::Error, fmt};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

#[cfg(test)]
use parley::LayoutContext;
use parley::{
    FontContext,
    fontique::{Blob, Collection, CollectionOptions, SourceCache},
};
use runenui_core::{LogicalLength, ResourceRef};

pub use artifact::{
    ShapedTextLease, ShapedTextResource, TextArtifact, TextCluster, TextFontBinding, TextGlyph,
    TextLine, TextLineMetrics, TextRun,
};
pub use source_identity::{FontSourceIdentity, FontSourceSnapshot};

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

/// Monotonic revision within one [`FontSourceIdentity`] universe.
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
/// The underlying Parley/Fontique contexts are deliberately private. Consumers configure font
/// sources through `RunenUI`-owned operations rather than reaching into the dependency stack.
/// Artifacts and explicit [`ShapedTextLease`] values own shaped payload lifetimes; this system
/// keeps only weak lookup bindings so dead logical resources can be reclaimed.
pub struct TextSystem {
    font_context: FontContext,
    #[cfg(test)]
    layout_context: LayoutContext,
    shaped_resources: HashMap<ResourceRef, Weak<ShapedTextResource>>,
    source_policy: FontSourcePolicy,
    source_identity: FontSourceIdentity,
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
            #[cfg(test)]
            layout_context: LayoutContext::new(),
            shaped_resources: HashMap::new(),
            source_policy,
            source_identity: FontSourceIdentity::fresh(),
            source_revision: FontSourceRevision::ZERO,
        }
    }

    /// Returns the explicit font-source policy used by this text system.
    #[must_use]
    pub const fn source_policy(&self) -> FontSourcePolicy {
        self.source_policy
    }

    /// Returns the current exact font-source cache-compatibility snapshot.
    #[must_use]
    pub fn source_snapshot(&self) -> FontSourceSnapshot {
        FontSourceSnapshot::new(self.source_identity.clone(), self.source_revision)
    }

    /// Returns the revision within this text system's font-source universe.
    #[must_use]
    pub const fn source_revision(&self) -> FontSourceRevision {
        self.source_revision
    }

    /// Acquires a strong lifetime token for one live scale-independent shaped resource.
    ///
    /// Runtime caches or retained publications keep the returned lease while the corresponding
    /// [`ResourceRef`] must remain resolvable. Raster scale is intentionally not an input.
    #[must_use]
    pub fn lease_shaped_run(&mut self, resource: &ResourceRef) -> Option<ShapedTextLease> {
        let shaped = self.shaped_resources.get(resource).and_then(Weak::upgrade);
        if shaped.is_none() {
            self.shaped_resources.remove(resource);
        }
        shaped.map(ShapedTextLease::new)
    }

    /// Registers immutable bundled font bytes and advances the source revision.
    ///
    /// The returned value is the number of font faces discovered in the source.
    ///
    /// # Errors
    ///
    /// Returns [`FontRegistrationError::NoFonts`] when the bytes contain no registerable font
    /// faces, or [`FontRegistrationError::RevisionExhausted`] when the monotonic source revision
    /// cannot advance.
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
        use parley::{Alignment, AlignmentOptions, FontFamily, StyleProperty};

        let source_snapshot = self.source_snapshot();
        let mut builder =
            self.layout_context
                .ranged_builder(&mut self.font_context, text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(FontFamily::named(family)));
        builder.push_default(StyleProperty::FontSize(font_size));
        let mut layout = builder.build(text);
        layout.break_all_lines(constraints.max_inline().map(LogicalLength::get));
        layout.align(Alignment::Start, AlignmentOptions::default());
        layout_extract::extract_layout(&layout, source_snapshot, &mut self.shaped_resources)
    }
}

#[cfg(test)]
mod tests {
    use super::{FontSourcePolicy, FontSourceRevision, TextConstraints, TextSystem};
    use runenui_core::{LogicalLength, ResourceKind};

    const CANTARELL: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");

    #[test]
    fn independent_font_source_universes_never_alias_at_the_same_revision() {
        let first = TextSystem::new(FontSourcePolicy::SystemAndBundled);
        let second = TextSystem::new(FontSourcePolicy::SystemAndBundled);

        assert_eq!(first.source_revision(), FontSourceRevision::ZERO);
        assert_eq!(second.source_revision(), FontSourceRevision::ZERO);
        assert_ne!(first.source_snapshot(), second.source_snapshot());
    }

    #[test]
    fn bundled_registration_advances_revision_without_replacing_source_identity() {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        let before = system.source_snapshot();
        system
            .register_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|error| panic!("Cantarell fixture registration failed: {error}"));
        let after = system.source_snapshot();

        assert_eq!(before.identity(), after.identity());
        assert_eq!(before.revision(), FontSourceRevision::ZERO);
        assert_eq!(after.revision().get(), 1);
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

        assert_eq!(artifact.source_snapshot(), &system.source_snapshot());
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
    fn explicit_lease_preserves_retry_binding_and_dead_resources_are_reclaimable() {
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
        let lease = system
            .lease_shaped_run(&resource)
            .unwrap_or_else(|| panic!("artifact-backed resource must be leaseable"));

        drop(artifact);
        assert_eq!(lease.resource_ref(), &resource);
        assert_eq!(lease.shaped_resource().glyphs().len(), glyph_count);
        assert_eq!(lease.shaped_resource().font().bytes(), CANTARELL);

        drop(lease);
        assert!(system.lease_shaped_run(&resource).is_none());
    }
}
