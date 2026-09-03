use core::{error::Error, fmt};

use parley::fontique::GenericFamily as BackendGenericFamily;
use runenui_core::{FontFamilyName, GenericFontFamily};

use crate::TextSystem;

/// Failure while replacing one generic-family mapping in a [`TextSystem`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenericFamilyMappingError {
    /// One requested concrete family is not available in this text system's source universe.
    UnknownFamily(FontFamilyName),
    /// A future core generic family has no reviewed mapping in the adopted font backend.
    UnsupportedGenericFamily(GenericFontFamily),
    /// The font-source revision cannot advance without wrapping.
    RevisionExhausted,
}

impl fmt::Display for GenericFamilyMappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFamily(family) => write!(
                formatter,
                "font family `{}` is not available in this text system",
                family.as_str()
            ),
            Self::UnsupportedGenericFamily(family) => write!(
                formatter,
                "generic font family {family:?} has no reviewed backend mapping"
            ),
            Self::RevisionExhausted => formatter.write_str("font-source revision is exhausted"),
        }
    }
}

impl Error for GenericFamilyMappingError {}

impl TextSystem {
    /// Replaces one generic family with an ordered chain of already-available named families.
    ///
    /// Input names are resolved to the backend's canonical family identity before mutation.
    /// Duplicate names or aliases resolving to the same family are collapsed while preserving
    /// first occurrence. Reapplying the same canonical chain is a no-op and does not advance the
    /// font-source revision. Passing an empty slice explicitly clears the mapping.
    ///
    /// This operation never creates a second `RunenUI` fallback registry: the private Fontique
    /// collection remains the sole concrete mapping store inside this text system.
    ///
    /// # Errors
    ///
    /// Returns [`GenericFamilyMappingError::UnknownFamily`] before mutation when any requested
    /// named family is unavailable, [`GenericFamilyMappingError::UnsupportedGenericFamily`] for
    /// a future core generic family without a reviewed backend mapping, or
    /// [`GenericFamilyMappingError::RevisionExhausted`] when a semantic mapping change cannot be
    /// represented by the monotonic font-source revision.
    pub fn set_generic_family_mapping(
        &mut self,
        generic: GenericFontFamily,
        families: &[FontFamilyName],
    ) -> Result<bool, GenericFamilyMappingError> {
        let backend_generic = backend_generic_family(generic)
            .ok_or(GenericFamilyMappingError::UnsupportedGenericFamily(generic))?;
        let mut resolved = Vec::with_capacity(families.len());
        for family in families {
            let family_id = self
                .font_context
                .collection
                .family_id(family.as_str())
                .ok_or_else(|| GenericFamilyMappingError::UnknownFamily(family.clone()))?;
            if !resolved.contains(&family_id) {
                resolved.push(family_id);
            }
        }

        let current = self
            .font_context
            .collection
            .generic_families(backend_generic)
            .collect::<Vec<_>>();
        if current == resolved {
            return Ok(false);
        }

        let next_revision = self
            .source_revision
            .next()
            .ok_or(GenericFamilyMappingError::RevisionExhausted)?;
        self.font_context
            .collection
            .set_generic_families(backend_generic, resolved.into_iter());
        self.source_revision = next_revision;
        Ok(true)
    }
}

pub const fn backend_generic_family(family: GenericFontFamily) -> Option<BackendGenericFamily> {
    Some(match family {
        GenericFontFamily::Serif => BackendGenericFamily::Serif,
        GenericFontFamily::SansSerif => BackendGenericFamily::SansSerif,
        GenericFontFamily::Monospace => BackendGenericFamily::Monospace,
        GenericFontFamily::Cursive => BackendGenericFamily::Cursive,
        GenericFontFamily::Fantasy => BackendGenericFamily::Fantasy,
        GenericFontFamily::SystemUi => BackendGenericFamily::SystemUi,
        GenericFontFamily::UiSerif => BackendGenericFamily::UiSerif,
        GenericFontFamily::UiSansSerif => BackendGenericFamily::UiSansSerif,
        GenericFontFamily::UiMonospace => BackendGenericFamily::UiMonospace,
        GenericFontFamily::UiRounded => BackendGenericFamily::UiRounded,
        GenericFontFamily::Emoji => BackendGenericFamily::Emoji,
        GenericFontFamily::Math => BackendGenericFamily::Math,
        GenericFontFamily::FangSong => BackendGenericFamily::FangSong,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use runenui_core::{FontFamilyName, GenericFontFamily, Typography};

    use super::GenericFamilyMappingError;
    use crate::{
        FontSourcePolicy, TextConstraints, TextLayoutDecision, TextLayoutState, TextRequest,
        TextSystem,
    };

    const CANTARELL: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");

    fn cantarell_family() -> Result<FontFamilyName, Box<dyn Error>> {
        Ok(FontFamilyName::new("Cantarell")?)
    }

    fn initial_typography_request(text: &str) -> TextRequest {
        TextRequest::new(text, Typography::default(), TextConstraints::unbounded())
    }

    #[test]
    fn bundled_generic_mapping_is_canonical_revisioned_and_drives_initial_typography()
    -> Result<(), Box<dyn Error>> {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        system.register_font_bytes(CANTARELL.to_vec())?;
        assert_eq!(system.source_revision().get(), 1);

        let cantarell = cantarell_family()?;
        assert!(system.set_generic_family_mapping(
            GenericFontFamily::SansSerif,
            &[cantarell.clone(), cantarell.clone()],
        )?);
        assert_eq!(system.source_revision().get(), 2);

        assert!(!system.set_generic_family_mapping(
            GenericFontFamily::SansSerif,
            std::slice::from_ref(&cantarell),
        )?);
        assert_eq!(system.source_revision().get(), 2);

        let mut state = TextLayoutState::new();
        let artifact = system
            .layout_text(
                &mut state,
                &initial_typography_request("deterministic sans serif"),
            )?
            .into_artifact();
        let run = artifact
            .lines()
            .first()
            .and_then(|line| line.runs().first())
            .ok_or("generic bundled fixture must shape at least one run")?;
        assert_eq!(run.shaped_resource().font().bytes(), CANTARELL);
        Ok(())
    }

    #[test]
    fn unknown_generic_mapping_family_is_transactionally_rejected() -> Result<(), Box<dyn Error>> {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        system.register_font_bytes(CANTARELL.to_vec())?;
        let before = system.source_revision();
        let missing = FontFamilyName::new("RunenUI Missing Fixture")?;

        assert_eq!(
            system.set_generic_family_mapping(
                GenericFontFamily::SansSerif,
                std::slice::from_ref(&missing),
            ),
            Err(GenericFamilyMappingError::UnknownFamily(missing))
        );
        assert_eq!(system.source_revision(), before);
        Ok(())
    }

    #[test]
    fn generic_mapping_revision_change_invalidates_cached_layout() -> Result<(), Box<dyn Error>> {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        system.register_font_bytes(CANTARELL.to_vec())?;
        let cantarell = cantarell_family()?;
        system.set_generic_family_mapping(
            GenericFontFamily::SansSerif,
            std::slice::from_ref(&cantarell),
        )?;
        let request = initial_typography_request("mapping revision");
        let mut state = TextLayoutState::new();

        assert_eq!(
            system.layout_text(&mut state, &request)?.decision(),
            TextLayoutDecision::Reshaped
        );
        assert!(system.set_generic_family_mapping(GenericFontFamily::SansSerif, &[])?);
        assert!(system.set_generic_family_mapping(
            GenericFontFamily::SansSerif,
            std::slice::from_ref(&cantarell),
        )?);
        assert_eq!(
            system.layout_text(&mut state, &request)?.decision(),
            TextLayoutDecision::Reshaped
        );
        Ok(())
    }
}
