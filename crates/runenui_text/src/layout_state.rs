//! Caller-owned reusable state for one logical text-layout stream.

use core::fmt;
use std::sync::Arc;

use parley::Layout;

use crate::{FontSourceSnapshot, TextArtifact, TextRequest};

/// Reusable renderer-neutral state for one logical text-layout stream.
///
/// The caller owns placement, lifetime, and invalidation of this value. `runenui_text`
/// owns only the private shaping/layout representation stored inside it. Cloning is
/// cheap immutable sharing; a later re-linebreak uses copy-on-write so staged runtime
/// work cannot mutate an accepted cache before commit.
///
/// This value carries no mounted identity, runtime topology, publication state, or
/// renderer state.
#[derive(Clone, Default)]
pub struct TextLayoutState {
    pub(super) cached: Option<Arc<CachedTextLayout>>,
}

impl TextLayoutState {
    /// Creates an empty reusable text-layout state.
    #[must_use]
    pub const fn new() -> Self {
        Self { cached: None }
    }

    /// Drops reusable shaping/layout state without affecting already-issued artifacts
    /// or shaped-resource leases.
    pub fn clear(&mut self) {
        self.cached = None;
    }

    /// Returns whether this state currently retains reusable private layout work.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cached.is_none()
    }
}

impl fmt::Debug for TextLayoutState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextLayoutState")
            .field("has_cached_layout", &self.cached.is_some())
            .finish()
    }
}

#[derive(Clone)]
pub struct CachedTextLayout {
    pub(super) layout: Layout<[u8; 4]>,
    pub(super) request: TextRequest,
    pub(super) source_snapshot: FontSourceSnapshot,
    pub(super) artifact: TextArtifact,
}

impl CachedTextLayout {
    pub(super) const fn new(
        layout: Layout<[u8; 4]>,
        request: TextRequest,
        source_snapshot: FontSourceSnapshot,
        artifact: TextArtifact,
    ) -> Self {
        Self {
            layout,
            request,
            source_snapshot,
            artifact,
        }
    }
}

/// Work performed to satisfy one logical text-layout request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextLayoutDecision {
    /// The exact prior immutable artifact was reused; no resource was reissued.
    Reused,
    /// Existing shaped layout state was re-line-broken/re-aligned without rebuilding shaping.
    Relinebroken,
    /// Text/font/style/source inputs required a fresh shaping/layout build.
    Reshaped,
}

/// Result of one logical text-layout request plus cache/reflow diagnostics.
#[derive(Clone, Debug)]
pub struct TextLayoutOutcome {
    artifact: TextArtifact,
    decision: TextLayoutDecision,
    issued_resource_count: usize,
}

impl TextLayoutOutcome {
    pub(super) const fn new(
        artifact: TextArtifact,
        decision: TextLayoutDecision,
        issued_resource_count: usize,
    ) -> Self {
        Self {
            artifact,
            decision,
            issued_resource_count,
        }
    }

    /// Returns the immutable artifact used for both measurement and later paint facts.
    #[must_use]
    pub const fn artifact(&self) -> &TextArtifact {
        &self.artifact
    }

    /// Consumes the outcome and returns its immutable artifact.
    #[must_use]
    pub fn into_artifact(self) -> TextArtifact {
        self.artifact
    }

    /// Returns whether this request reused, re-line-broke, or reshaped private text state.
    #[must_use]
    pub const fn decision(&self) -> TextLayoutDecision {
        self.decision
    }

    /// Returns the number of new logical shaped-run resources issued by this request.
    #[must_use]
    pub const fn issued_resource_count(&self) -> usize {
        self.issued_resource_count
    }
}
