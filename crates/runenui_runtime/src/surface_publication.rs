//! Context-bearing public surface publication product.

use runenui_core::SurfaceInputContext;

use crate::{SurfaceFrame, SurfaceLayoutReport, SurfaceStyleReport};

/// One runtime-issued displayed surface publication.
///
/// Equality preserves the renderer-facing publication contract that existed
/// before input contexts were attached: it compares the frame, style report,
/// and layout report, but not the runtime-issued input context. Compare
/// [`Self::input_context`] explicitly when exact displayed-snapshot identity is
/// part of the assertion.
#[derive(Clone, Debug)]
pub struct SurfacePublication {
    input_context: SurfaceInputContext,
    products: crate::surface::SurfacePublication,
}

impl PartialEq for SurfacePublication {
    fn eq(&self, other: &Self) -> bool {
        self.products == other.products
    }
}

impl SurfacePublication {
    pub(crate) const fn new(
        input_context: SurfaceInputContext,
        products: crate::surface::SurfacePublication,
    ) -> Self {
        Self {
            input_context,
            products,
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

    /// Consumes the publication into the existing renderer-facing products.
    #[must_use]
    pub fn into_parts(self) -> (SurfaceFrame, SurfaceStyleReport, SurfaceLayoutReport) {
        self.products.into_parts()
    }

    /// Consumes the publication into its context and renderer-facing products.
    #[must_use]
    pub fn into_context_and_parts(
        self,
    ) -> (
        SurfaceInputContext,
        SurfaceFrame,
        SurfaceStyleReport,
        SurfaceLayoutReport,
    ) {
        let Self {
            input_context,
            products,
        } = self;
        let (frame, style_report, layout_report) = products.into_parts();
        (input_context, frame, style_report, layout_report)
    }
}
