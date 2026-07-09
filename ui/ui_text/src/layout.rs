//! File: domain/ui/ui_text/src/layout.rs
//! Purpose: Renderer-independent text block layout request/result contracts.

use crate::{FontAtlasSource, TextBlock, TextBlockLayoutResult};

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlockLayoutRequest {
    pub block: TextBlock,
}

impl TextBlockLayoutRequest {
    pub fn new(block: TextBlock) -> Self {
        Self { block }
    }
}

pub trait TextLayouter: Send + Sync {
    fn layout(
        &self,
        atlas_source: &dyn FontAtlasSource,
        request: TextBlockLayoutRequest,
    ) -> TextBlockLayoutResult;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AtlasTextLayouter;

impl TextLayouter for AtlasTextLayouter {
    fn layout(
        &self,
        atlas_source: &dyn FontAtlasSource,
        request: TextBlockLayoutRequest,
    ) -> TextBlockLayoutResult {
        crate::proof_layout::layout_text_block(atlas_source, request)
    }
}
