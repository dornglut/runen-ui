//! File: domain/ui/ui_text/src/atlas.rs
//! Purpose: MSDF atlas contracts.

use std::collections::HashMap;

use crate::{FontFaceMetrics, FontId, GlyphMetrics};

#[derive(Debug, Clone)]
pub struct MsdfFontAtlas {
    pub font_id: FontId,
    pub texture_width: u32,
    pub texture_height: u32,
    pub metrics: FontFaceMetrics,
    pub glyphs: HashMap<char, GlyphMetrics>,
}

impl Default for MsdfFontAtlas {
    fn default() -> Self {
        Self {
            font_id: FontId(0),
            texture_width: 0,
            texture_height: 0,
            metrics: FontFaceMetrics {
                ascender: 0.0,
                descender: 0.0,
                line_height: 1.0,
                base_size: 1.0,
            },
            glyphs: HashMap::new(),
        }
    }
}

pub trait FontAtlasSource: Send + Sync {
    fn atlas(&self, font_id: FontId) -> Option<&MsdfFontAtlas>;
}
