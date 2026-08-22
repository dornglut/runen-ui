//! Context-bearing public surface publication product.

use runenui_core::SurfaceInputContext;

use crate::{
    SemanticDiagnosticReport, SemanticPublication, SurfaceFrame, SurfaceLayoutReport,
    SurfaceStyleReport,
};

/// One runtime-issued displayed surface publication.
///
/// Equality compares every published renderer, semantic, and semantic-diagnostic
/// product, but not the runtime-issued input context. Compare
/// [`Self::input_context`] explicitly when exact displayed-snapshot identity is
/// part of the assertion.
#[derive(Clone, Debug)]
pub struct SurfacePublication {
    input_context: SurfaceInputContext,
    products: crate::surface::SurfacePublication,
    semantic_publication: SemanticPublication,
    semantic_diagnostics: SemanticDiagnosticReport,
}

impl PartialEq for SurfacePublication {
    fn eq(&self, other: &Self) -> bool {
        self.products == other.products
            && self.semantic_publication == other.semantic_publication
            && self.semantic_diagnostics == other.semantic_diagnostics
    }
}

impl SurfacePublication {
    pub(crate) fn new(
        input_context: SurfaceInputContext,
        products: crate::surface::SurfacePublication,
        semantic_publication: SemanticPublication,
        semantic_diagnostics: SemanticDiagnosticReport,
    ) -> Self {
        Self {
            input_context,
            products,
            semantic_publication,
            semantic_diagnostics,
        }
    }

    /// Returns the exact runtime-issued context for this displayed hit-test snapshot.
    #[must_use]
    pub const fn input_context(&self) -> &SurfaceInputContext {
        &self.input_context
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

    /// Compares only the renderer-facing products, deliberately excluding
    /// semantic publication, semantic diagnostics, and input-context identity.
    #[must_use]
    pub fn renderer_products_eq(&self, other: &Self) -> bool {
        self.products == other.products
    }

    /// Returns the renderer-facing frame.
    #[must_use]
    pub fn frame(&self) -> &SurfaceFrame {
        self.products.frame()
    }

    /// Returns the style-resolution report aligned with the frame.
    #[must_use]
    pub fn style_report(&self) -> &SurfaceStyleReport {
        self.products.style_report()
    }

    /// Returns the layout report aligned with the frame.
    #[must_use]
    pub fn layout_report(&self) -> &SurfaceLayoutReport {
        self.products.layout_report()
    }

    /// Consumes this complete publication while explicitly retaining only the
    /// renderer-facing products.
    #[must_use]
    pub fn into_renderer_products(self) -> (SurfaceFrame, SurfaceStyleReport, SurfaceLayoutReport) {
        self.products.into_parts()
    }

    /// Consumes this publication into every public sibling product.
    #[must_use]
    pub fn into_complete_products(
        self,
    ) -> (
        SurfaceInputContext,
        SurfaceFrame,
        SurfaceStyleReport,
        SurfaceLayoutReport,
        SemanticPublication,
        SemanticDiagnosticReport,
    ) {
        let Self {
            input_context,
            products,
            semantic_publication,
            semantic_diagnostics,
        } = self;
        let (frame, style_report, layout_report) = products.into_parts();
        (
            input_context,
            frame,
            style_report,
            layout_report,
            semantic_publication,
            semantic_diagnostics,
        )
    }
}
