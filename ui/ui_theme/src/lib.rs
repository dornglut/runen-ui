//! File: domain/ui/ui_theme/src/lib.rs
//! Crate: ui_theme

pub mod color;
pub mod radius;
pub mod spacing;
pub mod theme;
pub mod token;
pub mod typography;

pub use color::*;
pub use radius::*;
pub use spacing::*;
pub use theme::*;
pub use token::*;
pub use typography::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_theme_preserves_palette_and_scales_metrics() {
        let theme = ThemeTokens {
            radius: RadiusScale {
                sm: 2.0,
                md: 4.0,
                lg: 8.0,
            },
            ..ThemeTokens::default()
        };
        let scaled = theme.scaled_by(1.5);
        assert_eq!(scaled.background, theme.background);
        assert!(scaled.spacing.md > theme.spacing.md);
        assert!(scaled.radius.md > theme.radius.md);
        assert!(scaled.typography.body > theme.typography.body);
    }
}
