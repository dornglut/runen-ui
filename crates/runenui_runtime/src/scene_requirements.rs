//! Derived renderer-neutral scene requirements and external consumer capabilities.

use core::fmt;

use runenui_core::ResourceKind;

use crate::PaintScene;

/// Canonical requirements derived from one exact [`PaintScene`] value.
///
/// Requirements are a read-only projection of scene content. They are not stored,
/// versioned, or independently mutable publication authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SceneRequirements {
    resource_kinds: Vec<ResourceKind>,
}

impl SceneRequirements {
    fn from_scene(scene: &PaintScene) -> Self {
        let resource_kinds = scene
            .items()
            .iter()
            .filter_map(|item| {
                item.primitive()
                    .resource_ref()
                    .map(runenui_core::ResourceRef::kind)
            })
            .collect();
        Self {
            resource_kinds: canonical_resource_kinds(resource_kinds),
        }
    }

    /// Returns required resource kinds in deterministic canonical order.
    #[must_use]
    pub const fn resource_kinds(&self) -> &[ResourceKind] {
        self.resource_kinds.as_slice()
    }

    /// Returns whether this scene has no resource-kind requirements.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.resource_kinds.is_empty()
    }
}

/// Renderer-neutral resource capabilities declared by one external scene consumer.
///
/// Capability values are consumer input only. They never rewrite, lower, or become
/// part of canonical [`PaintScene`] identity.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SceneCapabilities {
    resource_kinds: Vec<ResourceKind>,
}

impl SceneCapabilities {
    /// Creates capabilities from supported resource kinds.
    ///
    /// Input order and duplicates do not affect the resulting value.
    #[must_use]
    pub fn new(resource_kinds: impl IntoIterator<Item = ResourceKind>) -> Self {
        Self {
            resource_kinds: canonical_resource_kinds(resource_kinds.into_iter().collect()),
        }
    }

    /// Returns supported resource kinds in deterministic canonical order.
    #[must_use]
    pub const fn resource_kinds(&self) -> &[ResourceKind] {
        self.resource_kinds.as_slice()
    }

    /// Returns whether this consumer supports one exact resource kind.
    #[must_use]
    pub fn supports_resource_kind(&self, kind: ResourceKind) -> bool {
        self.resource_kinds.binary_search(&kind).is_ok()
    }

    /// Checks one derived requirement view without mutating or rewriting its scene.
    ///
    /// # Errors
    ///
    /// Returns the first unsupported kind in canonical requirement order.
    pub fn check_requirements(
        &self,
        requirements: &SceneRequirements,
    ) -> Result<(), UnsupportedSceneRequirement> {
        for &kind in requirements.resource_kinds() {
            if !self.supports_resource_kind(kind) {
                return Err(UnsupportedSceneRequirement { resource_kind: kind });
            }
        }
        Ok(())
    }
}

/// Deterministic rejection of one unsupported renderer-neutral scene requirement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedSceneRequirement {
    resource_kind: ResourceKind,
}

impl UnsupportedSceneRequirement {
    /// Stable diagnostic code for unsupported resource-kind requirements.
    pub const CODE: &'static str = "runenui.scene.unsupported-resource-kind";

    /// Returns the unsupported resource kind.
    #[must_use]
    pub const fn resource_kind(self) -> ResourceKind {
        self.resource_kind
    }

    /// Returns the stable renderer-neutral diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        Self::CODE
    }
}

impl fmt::Display for UnsupportedSceneRequirement {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported scene resource requirement: {:?}",
            self.resource_kind
        )
    }
}

impl std::error::Error for UnsupportedSceneRequirement {}

impl PaintScene {
    /// Derives the exact canonical renderer-neutral requirements from scene content.
    ///
    /// The result is recomputed from canonical primitives on every call; no hidden
    /// requirements cache or independent requirements version exists.
    #[must_use]
    pub fn requirements(&self) -> SceneRequirements {
        SceneRequirements::from_scene(self)
    }
}

fn canonical_resource_kinds(mut kinds: Vec<ResourceKind>) -> Vec<ResourceKind> {
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

#[cfg(test)]
mod tests {
    use super::SceneCapabilities;
    use runenui_core::ResourceKind;

    #[test]
    fn capabilities_are_canonicalized_by_kind() {
        let capabilities = SceneCapabilities::new([
            ResourceKind::ShapedTextRun,
            ResourceKind::Image,
            ResourceKind::ShapedTextRun,
        ]);
        assert_eq!(
            capabilities.resource_kinds(),
            &[ResourceKind::Image, ResourceKind::ShapedTextRun]
        );
    }
}
