#![forbid(unsafe_code)]
//! Renderer-neutral production text-system foundations.
//!
//! RunenUI owns the public contracts in this crate. Parley and Fontique remain
//! private implementation dependencies and must not become public API authority.

use core::{error::Error, fmt};
use std::sync::Arc;

use parley::{
    FontContext,
    fontique::{Blob, Collection, CollectionOptions, SourceCache},
};
use runenui_core::LogicalLength;

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
/// configure font sources through RunenUI-owned operations rather than reaching
/// into the dependency stack.
pub struct TextSystem {
    font_context: FontContext,
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
}

#[cfg(test)]
mod tests {
    use super::{FontSourcePolicy, FontSourceRevision, TextConstraints, TextSystem};
    use runenui_core::LogicalLength;

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
}
