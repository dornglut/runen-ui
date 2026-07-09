//! File: domain/ui/ui_text/src/style.rs
//! Purpose: Renderer-neutral text style contracts.

use crate::FontId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFontWeight {
    Thin,
    Light,
    Regular,
    Medium,
    Semibold,
    Bold,
    Heavy,
}

impl TextFontWeight {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Thin => "thin",
            Self::Light => "light",
            Self::Regular => "regular",
            Self::Medium => "medium",
            Self::Semibold => "semibold",
            Self::Bold => "bold",
            Self::Heavy => "heavy",
        }
    }
}

impl Default for TextFontWeight {
    fn default() -> Self {
        Self::Regular
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFontStyle {
    Normal,
    Italic,
}

impl TextFontStyle {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
        }
    }
}

impl Default for TextFontStyle {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDecoration {
    pub underline: bool,
    pub strikethrough: bool,
}

impl TextDecoration {
    pub const NONE: Self = Self {
        underline: false,
        strikethrough: false,
    };

    pub const fn underline() -> Self {
        Self {
            underline: true,
            strikethrough: false,
        }
    }

    pub const fn strikethrough() -> Self {
        Self {
            underline: false,
            strikethrough: true,
        }
    }
}

impl Default for TextDecoration {
    fn default() -> Self {
        Self::NONE
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextLineHeightPolicy {
    FontDefault,
    Multiplier(f32),
    Absolute(f32),
}

impl TextLineHeightPolicy {
    pub fn resolve(self, font_default: f32, font_size: f32) -> f32 {
        match self {
            Self::FontDefault => font_default,
            Self::Multiplier(value) => (font_size * value).max(font_default),
            Self::Absolute(value) => value.max(font_default),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FontDefault => "font-default",
            Self::Multiplier(_) => "multiplier",
            Self::Absolute(_) => "absolute",
        }
    }
}

impl Default for TextLineHeightPolicy {
    fn default() -> Self {
        Self::FontDefault
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextLetterSpacing {
    Normal,
    Em(f32),
    Absolute(f32),
}

impl TextLetterSpacing {
    pub fn resolve(self, font_size: f32) -> f32 {
        match self {
            Self::Normal => 0.0,
            Self::Em(value) => value * font_size,
            Self::Absolute(value) => value,
        }
    }
}

impl Default for TextLetterSpacing {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_id: FontId,
    pub font_size: f32,
    pub color: [f32; 4],
    pub line_height: TextLineHeightPolicy,
    pub font_weight: TextFontWeight,
    pub font_style: TextFontStyle,
    pub decoration: TextDecoration,
    pub letter_spacing: TextLetterSpacing,
}

impl TextStyle {
    pub fn line_height_or_default(&self, default: f32) -> f32 {
        self.line_height.resolve(default, self.font_size)
    }

    pub fn merged_with(&self, span: &TextSpanStyle) -> Self {
        Self {
            font_id: span.font_id.unwrap_or(self.font_id),
            font_size: span.font_size.unwrap_or(self.font_size),
            color: span.color.unwrap_or(self.color),
            line_height: span.line_height.unwrap_or(self.line_height),
            font_weight: span.font_weight.unwrap_or(self.font_weight),
            font_style: span.font_style.unwrap_or(self.font_style),
            decoration: span.decoration.unwrap_or(self.decoration),
            letter_spacing: span.letter_spacing.unwrap_or(self.letter_spacing),
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_id: FontId(0),
            font_size: 14.0,
            color: [0.92, 0.93, 0.95, 1.0],
            line_height: TextLineHeightPolicy::FontDefault,
            font_weight: TextFontWeight::Regular,
            font_style: TextFontStyle::Normal,
            decoration: TextDecoration::NONE,
            letter_spacing: TextLetterSpacing::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSpanStyle {
    pub font_id: Option<FontId>,
    pub font_size: Option<f32>,
    pub color: Option<[f32; 4]>,
    pub line_height: Option<TextLineHeightPolicy>,
    pub font_weight: Option<TextFontWeight>,
    pub font_style: Option<TextFontStyle>,
    pub decoration: Option<TextDecoration>,
    pub letter_spacing: Option<TextLetterSpacing>,
}

impl TextSpanStyle {
    pub const fn inherit() -> Self {
        Self {
            font_id: None,
            font_size: None,
            color: None,
            line_height: None,
            font_weight: None,
            font_style: None,
            decoration: None,
            letter_spacing: None,
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_font_weight(mut self, weight: TextFontWeight) -> Self {
        self.font_weight = Some(weight);
        self
    }

    pub fn with_decoration(mut self, decoration: TextDecoration) -> Self {
        self.decoration = Some(decoration);
        self
    }
}

impl Default for TextSpanStyle {
    fn default() -> Self {
        Self::inherit()
    }
}
