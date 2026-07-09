//! File: domain/ui/ui_theme/src/theme.rs
//! Purpose: Root theme token container.

use ui_text::{FontId, TextLineHeightPolicy, TextStyle};

use crate::{RadiusScale, SpacingScale, TypographyScale, UiColor};

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeTokens {
    pub background: UiColor,
    pub background_panel: UiColor,
    pub foreground: UiColor,
    pub foreground_muted: UiColor,
    pub status_input: UiColor,
    pub status_error: UiColor,
    pub status_warning: UiColor,
    pub status_info: UiColor,
    pub status_debug: UiColor,
    pub accent: UiColor,
    pub border: UiColor,
    pub border_width: f32,
    pub spacing: SpacingScale,
    pub radius: RadiusScale,
    pub typography: TypographyScale,
}

impl ThemeTokens {
    pub fn body_text_style(&self, font_id: FontId) -> TextStyle {
        TextStyle {
            font_id,
            font_size: self.typography.body,
            color: [
                self.foreground.r,
                self.foreground.g,
                self.foreground.b,
                self.foreground.a,
            ],
            line_height: TextLineHeightPolicy::Absolute((self.typography.body * 1.35).max(1.0)),
            ..TextStyle::default()
        }
    }

    pub fn body_small_text_style(&self, font_id: FontId) -> TextStyle {
        TextStyle {
            font_id,
            font_size: self.typography.body_small,
            color: [
                self.foreground_muted.r,
                self.foreground_muted.g,
                self.foreground_muted.b,
                self.foreground_muted.a,
            ],
            line_height: TextLineHeightPolicy::Absolute(
                (self.typography.body_small * 1.35).max(1.0),
            ),
            ..TextStyle::default()
        }
    }

    pub fn heading_text_style(&self, font_id: FontId) -> TextStyle {
        TextStyle {
            font_id,
            font_size: self.typography.heading,
            color: [
                self.foreground.r,
                self.foreground.g,
                self.foreground.b,
                self.foreground.a,
            ],
            line_height: TextLineHeightPolicy::Absolute((self.typography.heading * 1.25).max(1.0)),
            ..TextStyle::default()
        }
    }

    pub fn monospace_text_style(&self, font_id: FontId) -> TextStyle {
        TextStyle {
            font_id,
            font_size: self.typography.monospace,
            color: [
                self.foreground.r,
                self.foreground.g,
                self.foreground.b,
                self.foreground.a,
            ],
            line_height: TextLineHeightPolicy::Absolute(
                (self.typography.monospace * 1.35).max(1.0),
            ),
            ..TextStyle::default()
        }
    }

    pub fn scaled_by(&self, scale: f32) -> Self {
        let factor = scale.max(0.1);
        Self {
            background: self.background,
            background_panel: self.background_panel,
            foreground: self.foreground,
            foreground_muted: self.foreground_muted,
            status_input: self.status_input,
            status_error: self.status_error,
            status_warning: self.status_warning,
            status_info: self.status_info,
            status_debug: self.status_debug,
            accent: self.accent,
            border: self.border,
            border_width: (self.border_width * factor).clamp(0.5, 4.0),
            spacing: SpacingScale {
                xs: self.spacing.xs * factor,
                sm: self.spacing.sm * factor,
                md: self.spacing.md * factor,
                lg: self.spacing.lg * factor,
                xl: self.spacing.xl * factor,
            },
            radius: RadiusScale {
                sm: self.radius.sm * factor,
                md: self.radius.md * factor,
                lg: self.radius.lg * factor,
            },
            typography: TypographyScale {
                body: self.typography.body * factor,
                body_small: self.typography.body_small * factor,
                heading: self.typography.heading * factor,
                monospace: self.typography.monospace * factor,
            },
        }
    }
}

impl Default for ThemeTokens {
    fn default() -> Self {
        Self {
            background: UiColor::new(0.0, 0.0, 0.0, 1.0),
            background_panel: UiColor::new(0.006, 0.006, 0.007, 1.0),
            foreground: UiColor::new(0.940, 0.940, 0.940, 1.0),
            foreground_muted: UiColor::new(0.760, 0.760, 0.775, 1.0),
            status_input: UiColor::new(0.320, 0.610, 1.000, 1.0),
            status_error: UiColor::new(1.000, 0.300, 0.260, 1.0),
            status_warning: UiColor::new(1.000, 0.790, 0.250, 1.0),
            status_info: UiColor::new(0.940, 0.940, 0.940, 1.0),
            status_debug: UiColor::new(0.620, 0.640, 0.680, 1.0),
            accent: UiColor::new(0.165, 0.455, 0.980, 1.0),
            border: UiColor::new(0.070, 0.070, 0.080, 1.0),
            border_width: 1.0,
            spacing: SpacingScale {
                xs: 2.0,
                sm: 4.0,
                md: 6.0,
                lg: 10.0,
                xl: 14.0,
            },
            radius: RadiusScale {
                sm: 0.0,
                md: 0.0,
                lg: 0.0,
            },
            typography: TypographyScale {
                body: 13.0,
                body_small: 11.0,
                heading: 16.0,
                monospace: 12.0,
            },
        }
    }
}
