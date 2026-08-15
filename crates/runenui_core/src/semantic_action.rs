//! Surface-scoped semantic action request and runtime-issued target metadata.

use crate::{SemanticAction, SemanticKey, SemanticNodeId, SurfaceId};

/// Exact public request to execute one semantic action against one current surface.
///
/// The request deliberately carries no semantic revision. Runtime admission
/// evaluates the current committed semantic product and current action readiness.
#[must_use]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SemanticActionRequest {
    surface: SurfaceId,
    target: SemanticNodeId,
    action: SemanticAction,
}

impl SemanticActionRequest {
    /// Creates one exact surface-scoped semantic action request.
    pub const fn new(surface: SurfaceId, target: SemanticNodeId, action: SemanticAction) -> Self {
        Self {
            surface,
            target,
            action,
        }
    }

    /// Returns the exact logical surface named by this request.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface
    }

    /// Returns the exact semantic-node lifetime named by this request.
    #[must_use]
    pub const fn target(&self) -> &SemanticNodeId {
        &self.target
    }

    /// Returns the requested semantic action.
    #[must_use]
    pub const fn action(&self) -> SemanticAction {
        self.action
    }
}

/// Immutable semantic-origin metadata attached to canonical routed callbacks.
///
/// Runtime issues this value only after exact semantic request admission. It
/// exposes semantic identity and the owner-local semantic key needed by a custom
/// widget to distinguish virtual semantic targets, but it exposes no mounted
/// owner or semantic-to-mounted routing conversion.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SemanticActionTarget {
    surface: SurfaceId,
    target: SemanticNodeId,
    key: SemanticKey,
    action: SemanticAction,
}

impl SemanticActionTarget {
    /// Creates runtime-issued semantic target metadata.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        surface: SurfaceId,
        target: SemanticNodeId,
        key: SemanticKey,
        action: SemanticAction,
    ) -> Self {
        Self {
            surface,
            target,
            key,
            action,
        }
    }

    /// Returns the exact logical surface that admitted the semantic request.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface
    }

    /// Returns the exact semantic-node lifetime that originated this route.
    #[must_use]
    pub const fn target(&self) -> &SemanticNodeId {
        &self.target
    }

    /// Returns the stable owner-local semantic key for the exact target.
    ///
    /// The key is semantic authoring identity only. It does not expose the
    /// private mounted owner or provide a routing shortcut.
    #[must_use]
    pub const fn semantic_key(&self) -> &SemanticKey {
        &self.key
    }

    /// Returns the original semantic action admitted for this target.
    #[must_use]
    pub const fn action(&self) -> SemanticAction {
        self.action
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        __runtime::RuntimeNamespace, SemanticAction, SemanticActionRequest, SemanticActionTarget,
        SemanticKey,
    };

    #[test]
    fn request_and_runtime_target_preserve_exact_semantic_facts() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let node = namespace.__runtime_semantic_id(3, 7);
        let key = SemanticKey::from_static("virtual")
            .unwrap_or_else(|_| unreachable!("test semantic key is valid"));
        let request =
            SemanticActionRequest::new(surface.clone(), node.clone(), SemanticAction::Activate);
        assert_eq!(request.surface_id(), &surface);
        assert_eq!(request.target(), &node);
        assert_eq!(request.action(), SemanticAction::Activate);

        let target = SemanticActionTarget::__runtime_new(
            surface.clone(),
            node.clone(),
            key.clone(),
            SemanticAction::Activate,
        );
        assert_eq!(target.surface_id(), &surface);
        assert_eq!(target.target(), &node);
        assert_eq!(target.semantic_key(), &key);
        assert_eq!(target.action(), SemanticAction::Activate);
    }
}
