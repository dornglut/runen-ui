//! File: domain/ui/ui_widgets/src/vector_input.rs
//! Purpose: Vector numeric input helper constructors.

use crate::{NumericInputConfig, UiNode, WidgetId, numeric_input};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn vector3_input(
    id: WidgetId,
    component_ids: [WidgetId; 3],
    values: [f64; 3],
    step: f64,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    crate::stack::hstack(
        id,
        theme.spacing.xs,
        component_ids
            .into_iter()
            .zip(values)
            .map(|(component_id, value)| {
                numeric_input(
                    component_id,
                    NumericInputConfig::new(value, step, None, None, 3),
                    text_style.clone(),
                    theme.clone(),
                )
            })
            .collect(),
    )
}
