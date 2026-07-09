//! File: domain/ui/ui_widgets/src/scenario.rs
//! Purpose: Generic primitive widget scenario adapters.

use crate::{
    NumericInputConfig, TableColumn, TableRow, TreeRow, UiNode, VisibleWidgetScanRequirement,
    VisibleWidgetState, WidgetId, button, numeric_input, select, table, tabs, text_input, toggle,
    tree,
};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrimitiveWidgetScenarioKind {
    Button,
    TextInput,
    Toggle,
    NumericInput,
    Tabs,
    Select,
    Table,
    Tree,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveWidgetScenario {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: PrimitiveWidgetScenarioKind,
    pub root: UiNode,
    pub scan_requirement: VisibleWidgetScanRequirement,
}

impl PrimitiveWidgetScenario {
    fn new(
        id: &'static str,
        label: &'static str,
        kind: PrimitiveWidgetScenarioKind,
        root: UiNode,
    ) -> Self {
        Self {
            id,
            label,
            kind,
            root,
            scan_requirement: VisibleWidgetScanRequirement::strict_interactive([
                VisibleWidgetState::Default,
                VisibleWidgetState::Focused,
            ]),
        }
    }
}

pub fn primitive_widget_scenarios() -> Vec<PrimitiveWidgetScenario> {
    let style = TextStyle::default();
    let theme = ThemeTokens::default();

    vec![
        PrimitiveWidgetScenario::new(
            "ui.primitive.button.default",
            "Button / Default",
            PrimitiveWidgetScenarioKind::Button,
            button(WidgetId(10_001), "Apply", style.clone(), theme.clone()),
        ),
        PrimitiveWidgetScenario::new(
            "ui.primitive.text_input.default",
            "Text Input / Default",
            PrimitiveWidgetScenarioKind::TextInput,
            text_input(
                WidgetId(10_002),
                "Player",
                "Name",
                style.clone(),
                theme.clone(),
            ),
        ),
        PrimitiveWidgetScenario::new(
            "ui.primitive.toggle.checked",
            "Toggle / Checked",
            PrimitiveWidgetScenarioKind::Toggle,
            toggle(WidgetId(10_003), "Snap", true, style.clone(), theme.clone()),
        ),
        PrimitiveWidgetScenario::new(
            "ui.primitive.numeric_input.default",
            "Numeric Input / Default",
            PrimitiveWidgetScenarioKind::NumericInput,
            numeric_input(
                WidgetId(10_004),
                NumericInputConfig::new(1.0, 0.1, Some(0.0), Some(2.0), 2),
                style.clone(),
                theme.clone(),
            ),
        ),
        PrimitiveWidgetScenario::new(
            "ui.primitive.tabs.default",
            "Tabs / Default",
            PrimitiveWidgetScenarioKind::Tabs,
            tabs(
                WidgetId(10_005),
                ["Layout", "Theme"],
                0,
                style.clone(),
                theme.clone(),
            ),
        ),
        PrimitiveWidgetScenario::new(
            "ui.primitive.select.default",
            "Select / Default",
            PrimitiveWidgetScenarioKind::Select,
            select(
                WidgetId(10_006),
                ["Scene", "Material"],
                Some(0),
                "Choose",
                style.clone(),
                theme.clone(),
            ),
        ),
        PrimitiveWidgetScenario::new(
            "ui.primitive.table.default",
            "Table / Default",
            PrimitiveWidgetScenarioKind::Table,
            table(
                WidgetId(10_007),
                [
                    TableColumn::new("Name", 96.0),
                    TableColumn::new("Kind", 72.0),
                ],
                [TableRow::new(["Cube", "Mesh"])],
                style.clone(),
                style.clone(),
                theme.clone(),
            ),
        ),
        PrimitiveWidgetScenario::new(
            "ui.primitive.tree.default",
            "Tree / Default",
            PrimitiveWidgetScenarioKind::Tree,
            tree(
                WidgetId(10_008),
                [
                    TreeRow::new("Scene", 0, true),
                    TreeRow::new("Cube", 1, false),
                ],
                style,
                theme,
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitive_widget_scenarios_have_executable_roots() {
        let scenarios = primitive_widget_scenarios();

        assert_eq!(scenarios.len(), 8);
        assert!(
            scenarios
                .iter()
                .all(|scenario| !scenario.id.trim().is_empty())
        );
        assert!(
            scenarios
                .iter()
                .all(|scenario| !scenario.label.trim().is_empty())
        );
        assert!(
            scenarios
                .iter()
                .all(|scenario| scenario.root.id.0 >= 10_001)
        );
        assert!(
            scenarios
                .iter()
                .all(|scenario| scenario.scan_requirement.require_layout_bounds)
        );
    }
}
