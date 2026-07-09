//! File: domain/ui/ui_widgets/src/table.rs
//! Purpose: Table widget constructor.

use crate::{TableColumn, TableNode, TableRow, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn table(
    id: WidgetId,
    columns: impl IntoIterator<Item = TableColumn>,
    rows: impl IntoIterator<Item = TableRow>,
    text_style: TextStyle,
    header_text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::Table(TableNode::new(
            columns,
            rows,
            text_style,
            header_text_style,
            theme,
        )),
    )
}
