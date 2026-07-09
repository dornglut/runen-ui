//! File: domain/ui/ui_widgets/src/numeric_input.rs
//! Purpose: Numeric input widget constructor.

use crate::{NumericInputNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericInputConfig {
    pub value: f64,
    pub step: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub precision: u8,
}

impl NumericInputConfig {
    pub const fn new(
        value: f64,
        step: f64,
        min: Option<f64>,
        max: Option<f64>,
        precision: u8,
    ) -> Self {
        Self {
            value,
            step,
            min,
            max,
            precision,
        }
    }
}

pub fn numeric_input(
    id: WidgetId,
    config: NumericInputConfig,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::NumericInput(NumericInputNode::new(
            config.value,
            config.step,
            config.min,
            config.max,
            config.precision,
            text_style,
            theme,
        )),
    )
}
