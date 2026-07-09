//! File: domain/ui/ui_text/src/font.rs
//! Purpose: MSDF font id and glyph metrics contracts.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FontId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphMetrics {
    pub advance: f32,
    pub plane_left: f32,
    pub plane_top: f32,
    pub plane_right: f32,
    pub plane_bottom: f32,
    pub atlas_left: f32,
    pub atlas_top: f32,
    pub atlas_right: f32,
    pub atlas_bottom: f32,
}

#[derive(Debug, Clone)]
pub struct FontFaceMetrics {
    pub ascender: f32,
    pub descender: f32,
    pub line_height: f32,
    pub base_size: f32,
}
