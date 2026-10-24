//! Public alignment boundary for one committed logical-surface publication.

use runenui_core::SurfaceInputContext;

use crate::{
    HitTestScene, InputOwnershipTransition, PaintPublication, PaintScene, SemanticDiagnosticReport,
    SemanticPublication, SurfaceFrame, SurfaceLayoutReport, SurfaceStyleReport,
};

/// One runtime-issued surface publication aligning distinct immutable sibling products.
///
/// `PaintPublication` owns renderer update identity and canonical paint content;
/// `HitTestScene` owns exact displayed-input context, ordered hit regions, and
/// historical mounted-target membership. Layout/debug, semantics, and diagnostics
/// remain separately typed siblings coordinated by the same successful commit.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePublication {
    paint_publication: PaintPublication,
    hit_test_scene: HitTestScene,
    products: crate::surface::SurfacePublication,
    semantic_publication: SemanticPublication,
    semantic_diagnostics: SemanticDiagnosticReport,
    input_ownership_transition: Option<InputOwnershipTransition>,
}

impl SurfacePublication {
    pub(crate) const fn new(
        paint_publication: PaintPublication,
        hit_test_scene: HitTestScene,
        products: crate::surface::SurfacePublication,
        semantic_publication: SemanticPublication,
        semantic_diagnostics: SemanticDiagnosticReport,
    ) -> Self {
        Self {
            paint_publication,
            hit_test_scene,
            products,
            semantic_publication,
            semantic_diagnostics,
            input_ownership_transition: None,
        }
    }

    pub(crate) fn with_input_ownership_transition(
        mut self,
        transition: InputOwnershipTransition,
    ) -> Self {
        self.input_ownership_transition = Some(transition);
        self
    }

    /// Returns the exact direct ownership transition for this publication when
    /// its preceding state was materialized and no earlier direct boundary was
    /// pending. The transition's after-snapshot retains this publication's
    /// input context. It proves only this invocation; callers must match its
    /// scope and `before_revision()` to their own current witness. `None` means
    /// continuity was not proven for this operation; a host must retain its
    /// conservative revision-gap behavior.
    #[must_use]
    pub const fn input_ownership_transition(&self) -> Option<&InputOwnershipTransition> {
        self.input_ownership_transition.as_ref()
    }

    /// Returns the exact immutable renderer publication aligned with this commit.
    #[must_use]
    pub const fn paint_publication(&self) -> &PaintPublication {
        &self.paint_publication
    }

    /// Returns the reusable canonical paint scene through its owning publication.
    #[must_use]
    pub const fn paint_scene(&self) -> &PaintScene {
        self.paint_publication.scene()
    }

    /// Returns the exact immutable displayed-input scene aligned with this commit.
    #[must_use]
    pub const fn hit_test_scene(&self) -> &HitTestScene {
        &self.hit_test_scene
    }

    /// Returns the exact runtime-issued context owned by the displayed hit scene.
    ///
    /// This convenience accessor derives from [`Self::hit_test_scene`] and stores
    /// no duplicate context authority.
    #[must_use]
    pub const fn input_context(&self) -> &SurfaceInputContext {
        self.hit_test_scene.input_context()
    }

    /// Returns the renderer-independent semantic publication aligned with this surface publication.
    #[must_use]
    pub const fn semantic_publication(&self) -> &SemanticPublication {
        &self.semantic_publication
    }

    /// Returns deterministic semantic diagnostics aligned with this surface publication.
    #[must_use]
    pub const fn semantic_diagnostics(&self) -> &SemanticDiagnosticReport {
        &self.semantic_diagnostics
    }

    /// Compares renderer-facing sibling products only, excluding displayed hit,
    /// semantic publication, and semantic diagnostics.
    #[must_use]
    pub fn renderer_products_eq(&self, other: &Self) -> bool {
        self.paint_publication == other.paint_publication && self.products == other.products
    }

    /// Returns the retained layout/debug frame. It is not paint or pointer-hit authority.
    #[must_use]
    pub fn frame(&self) -> &SurfaceFrame {
        self.products.frame()
    }

    /// Returns the style-resolution report aligned with the publication.
    #[must_use]
    pub fn style_report(&self) -> &SurfaceStyleReport {
        self.products.style_report()
    }

    /// Returns the layout report aligned with the publication.
    #[must_use]
    pub fn layout_report(&self) -> &SurfaceLayoutReport {
        self.products.layout_report()
    }

    /// Consumes this publication into its renderer-facing paint and layout/debug
    /// products, preserving the directly coupled ownership observation as the
    /// final tuple element. A missing observation remains `None` and must not be
    /// inferred from the extracted renderer products.
    #[must_use]
    pub fn into_renderer_products(
        self,
    ) -> (
        PaintPublication,
        SurfaceFrame,
        SurfaceStyleReport,
        SurfaceLayoutReport,
        Option<InputOwnershipTransition>,
    ) {
        let Self {
            paint_publication,
            products,
            input_ownership_transition,
            ..
        } = self;
        let (frame, style_report, layout_report) = products.into_parts();
        (
            paint_publication,
            frame,
            style_report,
            layout_report,
            input_ownership_transition,
        )
    }

    /// Consumes this publication into every independently typed public sibling
    /// product, preserving the directly coupled ownership observation as the
    /// final tuple element.
    #[must_use]
    pub fn into_complete_products(
        self,
    ) -> (
        PaintPublication,
        HitTestScene,
        SurfaceFrame,
        SurfaceStyleReport,
        SurfaceLayoutReport,
        SemanticPublication,
        SemanticDiagnosticReport,
        Option<InputOwnershipTransition>,
    ) {
        let Self {
            paint_publication,
            hit_test_scene,
            products,
            semantic_publication,
            semantic_diagnostics,
            input_ownership_transition,
        } = self;
        let (frame, style_report, layout_report) = products.into_parts();
        (
            paint_publication,
            hit_test_scene,
            frame,
            style_report,
            layout_report,
            semantic_publication,
            semantic_diagnostics,
            input_ownership_transition,
        )
    }
}
