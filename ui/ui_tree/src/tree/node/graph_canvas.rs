//! Graph canvas node contracts.

use ui_math::UiSize;
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphCanvasNode {
    pub canvas: ui_graph_editor::GraphCanvasViewModel,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
    pub text_style: TextStyle,
    pub focusable: bool,
    pub capture_pointer_drag: bool,
    pub owns_wheel_zoom: bool,
    pub clip: bool,
}

impl GraphCanvasNode {
    pub fn new(canvas: ui_graph_editor::GraphCanvasViewModel, theme: ThemeTokens) -> Self {
        Self {
            canvas,
            min_size: UiSize::new(160.0, 120.0),
            theme,
            text_style: TextStyle::default(),
            focusable: true,
            capture_pointer_drag: true,
            owns_wheel_zoom: true,
            clip: true,
        }
    }

    pub fn with_min_size(mut self, min_size: UiSize) -> Self {
        self.min_size = min_size;
        self
    }
}
