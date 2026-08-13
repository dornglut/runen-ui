//! Context-bearing public surface publication product.

use std::sync::Arc;

use runenui_core::SurfaceInputContext;

use crate::{SemanticPublication, SurfaceFrame, SurfaceLayoutReport, SurfaceStyleReport};

/// One runtime-issued displayed surface publication.
///
/// The aggregate always carries both renderer-facing products and the exact
/// current renderer-independent semantic publication. Aggregate equality
/// compares both product families while intentionally excluding the
/// runtime-issued input context; use [`Self::renderer_eq`] when only renderer
/// products are relevant, and compare [`Self::input_context`] explicitly when
/// exact displayed-snapshot identity is part of the assertion.
#[derive(Clone, Debug)]
pub struct SurfacePublication {
    input_context: SurfaceInputContext,
    products: crate::surface::SurfacePublication,
    semantics: Arc<SemanticPublication>,
}

impl PartialEq for SurfacePublication {
    fn eq(&self, other: &Self) -> bool {
        self.products == other.products && self.semantics == other.semantics
    }
}

impl SurfacePublication {
    pub(crate) const fn new(
        input_context: SurfaceInputContext,
        products: crate::surface::SurfacePublication,
        semantics: Arc<SemanticPublication>,
    ) -> Self {
        Self {
            input_context,
            products,
            semantics,
        }
    }

    /// Returns the exact runtime-issued context for this displayed hit-test snapshot.
    #[must_use]
    pub const fn input_context(&self) -> &SurfaceInputContext {
        &self.input_context
    }

    /// Returns the renderer-facing frame.
    #[must_use]
    pub const fn frame(&self) -> &SurfaceFrame {
        self.products.frame()
    }

    /// Returns the style-resolution report aligned with the frame.
    #[must_use]
    pub const fn style_report(&self) -> &SurfaceStyleReport {
        self.products.style_report()
    }

    /// Returns the layout report aligned with the frame.
    #[must_use]
    pub const fn layout_report(&self) -> &SurfaceLayoutReport {
        self.products.layout_report()
    }

    /// Returns the exact current renderer-independent semantic publication.
    #[must_use]
    pub fn semantics(&self) -> &SemanticPublication {
        self.semantics.as_ref()
    }

    /// Compares only renderer-facing frame/style/layout products.
    ///
    /// This is the explicit opt-in comparison for renderer-only tests and
    /// adapters. Ordinary [`PartialEq`] also compares semantic publication.
    #[must_use]
    pub fn renderer_eq(&self, other: &Self) -> bool {
        self.products == other.products
    }

    /// Consumes the publication into renderer-facing products only.
    ///
    /// This method intentionally omits input-context and semantic products; its
    /// name makes that narrower extraction explicit.
    #[must_use]
    pub fn into_renderer_products(self) -> (SurfaceFrame, SurfaceStyleReport, SurfaceLayoutReport) {
        self.products.into_parts()
    }

    /// Consumes the publication into every independently typed product.
    #[must_use]
    pub fn into_complete_products(
        self,
    ) -> (
        SurfaceInputContext,
        SurfaceFrame,
        SurfaceStyleReport,
        SurfaceLayoutReport,
        Arc<SemanticPublication>,
    ) {
        let Self {
            input_context,
            products,
            semantics,
        } = self;
        let (frame, style_report, layout_report) = products.into_parts();
        (
            input_context,
            frame,
            style_report,
            layout_report,
            semantics,
        )
    }
}
