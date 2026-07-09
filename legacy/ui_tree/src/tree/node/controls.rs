//! Control node contracts.

use ui_layout::LayoutConstraints;
use ui_math::{UiInsets, UiSize};
use ui_text::{TextDirectionPolicy, TextLayoutPolicy, TextStyle, TextVerticalAlign};
use ui_theme::{ThemeTokens, UiColor};

use crate::WidgetId;

#[derive(Debug, Clone, PartialEq)]
pub struct LabelNode {
    pub text: String,
    pub text_style: TextStyle,
    pub text_layout: TextLayoutPolicy,
    pub constraints: LayoutConstraints,
}

impl LabelNode {
    pub fn new(text: impl Into<String>, text_style: TextStyle) -> Self {
        Self {
            text: text.into(),
            text_style,
            text_layout: TextLayoutPolicy {
                vertical_align: TextVerticalAlign::Center,
                max_lines: Some(1),
                text_direction: TextDirectionPolicy::Ltr,
                ..TextLayoutPolicy::no_wrap_label()
            },
            constraints: LayoutConstraints::loose(UiSize::new(f32::MAX, f32::MAX)),
        }
    }

    pub fn with_text_layout(mut self, text_layout: TextLayoutPolicy) -> Self {
        self.text_layout = text_layout;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonNode {
    pub label: String,
    pub accessible_label: Option<String>,
    pub text_style: TextStyle,
    pub text_layout: TextLayoutPolicy,
    pub padding: UiInsets,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
    pub enabled: bool,
    pub selected: bool,
    pub selected_fill: Option<UiColor>,
    pub selected_border: Option<UiColor>,
    pub corner_radius: Option<f32>,
    pub fill_width: bool,
    pub reveal_on_hover_anchor: Option<WidgetId>,
}

impl ButtonNode {
    pub fn new(label: impl Into<String>, text_style: TextStyle, theme: ThemeTokens) -> Self {
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let padding = UiInsets::new(
            theme.spacing.sm,
            theme.spacing.xs,
            theme.spacing.sm,
            theme.spacing.xs,
        );
        Self {
            label: label.into(),
            accessible_label: None,
            text_style,
            text_layout: TextLayoutPolicy::button_label(),
            padding,
            min_size: UiSize::new(
                (theme.spacing.xl * 2.0).max(32.0),
                (line_height + padding.vertical()).max(theme.spacing.lg + theme.spacing.sm),
            ),
            theme,
            enabled: true,
            selected: false,
            selected_fill: None,
            selected_border: None,
            corner_radius: None,
            fill_width: false,
            reveal_on_hover_anchor: None,
        }
    }

    pub fn with_text_layout(mut self, text_layout: TextLayoutPolicy) -> Self {
        self.text_layout = text_layout;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextInputNode {
    pub value: String,
    pub placeholder: String,
    pub text_style: TextStyle,
    pub padding: UiInsets,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
    pub editable: bool,
    pub fill_width: bool,
}

impl TextInputNode {
    pub fn new(
        value: impl Into<String>,
        placeholder: impl Into<String>,
        text_style: TextStyle,
        theme: ThemeTokens,
    ) -> Self {
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let padding = UiInsets::new(
            theme.spacing.sm,
            theme.spacing.xs,
            theme.spacing.sm,
            theme.spacing.xs,
        );
        Self {
            value: value.into(),
            placeholder: placeholder.into(),
            text_style,
            padding,
            min_size: UiSize::new(
                (theme.spacing.xl * 3.0).max(72.0),
                line_height + padding.vertical(),
            ),
            theme,
            editable: true,
            fill_width: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToggleNode {
    pub label: String,
    pub text_style: TextStyle,
    pub padding: UiInsets,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
    pub checked: bool,
    pub enabled: bool,
}

impl ToggleNode {
    pub fn new(
        label: impl Into<String>,
        checked: bool,
        text_style: TextStyle,
        theme: ThemeTokens,
    ) -> Self {
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let padding = UiInsets::new(
            theme.spacing.sm,
            theme.spacing.xs,
            theme.spacing.sm,
            theme.spacing.xs,
        );
        let min_toggle = (line_height + padding.vertical()).max(18.0);
        Self {
            label: label.into(),
            text_style,
            padding,
            min_size: UiSize::new((min_toggle * 2.0).max(48.0), min_toggle),
            theme,
            checked,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericInputNode {
    pub value: f64,
    pub step: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub precision: u8,
    pub text_style: TextStyle,
    pub padding: UiInsets,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
    pub enabled: bool,
}

impl NumericInputNode {
    pub fn new(
        value: f64,
        step: f64,
        min: Option<f64>,
        max: Option<f64>,
        precision: u8,
        text_style: TextStyle,
        theme: ThemeTokens,
    ) -> Self {
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let padding = UiInsets::new(
            theme.spacing.sm,
            theme.spacing.xs,
            theme.spacing.sm,
            theme.spacing.xs,
        );
        Self {
            value,
            step: step.abs().max(0.000_001),
            min,
            max,
            precision,
            text_style,
            padding,
            min_size: UiSize::new(
                (theme.spacing.xl * 2.5).max(64.0),
                line_height + padding.vertical(),
            ),
            theme,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TabsNode {
    pub labels: Vec<String>,
    pub selected_index: usize,
    pub text_style: TextStyle,
    pub padding: UiInsets,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
}

impl TabsNode {
    pub fn new(
        labels: impl IntoIterator<Item = impl Into<String>>,
        selected_index: usize,
        text_style: TextStyle,
        theme: ThemeTokens,
    ) -> Self {
        let labels = labels.into_iter().map(Into::into).collect::<Vec<_>>();
        let clamped_index = if labels.is_empty() {
            0
        } else {
            selected_index.min(labels.len() - 1)
        };
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let padding = UiInsets::new(
            theme.spacing.sm,
            theme.spacing.xs,
            theme.spacing.sm,
            theme.spacing.xs,
        );
        let width_estimate = labels
            .iter()
            .map(|label| {
                label.chars().count() as f32 * text_style.font_size * 0.65 + padding.horizontal()
            })
            .sum::<f32>()
            .max((theme.spacing.xl * 3.0).max(72.0));
        Self {
            labels,
            selected_index: clamped_index,
            text_style,
            padding,
            min_size: UiSize::new(width_estimate, line_height + padding.vertical()),
            theme,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectNode {
    pub options: Vec<String>,
    pub selected_index: Option<usize>,
    pub placeholder: String,
    pub text_style: TextStyle,
    pub padding: UiInsets,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
    pub enabled: bool,
}

impl SelectNode {
    pub fn new(
        options: impl IntoIterator<Item = impl Into<String>>,
        selected_index: Option<usize>,
        placeholder: impl Into<String>,
        text_style: TextStyle,
        theme: ThemeTokens,
    ) -> Self {
        let options = options.into_iter().map(Into::into).collect::<Vec<_>>();
        let selected_index = selected_index.filter(|index| *index < options.len());
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let padding = UiInsets::new(
            theme.spacing.sm,
            theme.spacing.xs,
            theme.spacing.sm,
            theme.spacing.xs,
        );
        let width_estimate = options
            .iter()
            .map(|option| option.chars().count() as f32 * text_style.font_size * 0.6)
            .fold(0.0, f32::max)
            + padding.horizontal()
            + line_height;
        Self {
            options,
            selected_index,
            placeholder: placeholder.into(),
            text_style,
            padding,
            min_size: UiSize::new(width_estimate.max(96.0), line_height + padding.vertical()),
            theme,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    pub label: String,
    pub min_width: f32,
}

impl TableColumn {
    pub fn new(label: impl Into<String>, min_width: f32) -> Self {
        Self {
            label: label.into(),
            min_width: min_width.max(24.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableRow {
    pub cells: Vec<String>,
    pub selected: bool,
    pub enabled: bool,
}

impl TableRow {
    pub fn new(cells: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            cells: cells.into_iter().map(Into::into).collect(),
            selected: false,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableNode {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
    pub text_style: TextStyle,
    pub header_text_style: TextStyle,
    pub row_height: f32,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
}

impl TableNode {
    pub fn new(
        columns: impl IntoIterator<Item = TableColumn>,
        rows: impl IntoIterator<Item = TableRow>,
        text_style: TextStyle,
        header_text_style: TextStyle,
        theme: ThemeTokens,
    ) -> Self {
        let columns = columns.into_iter().collect::<Vec<_>>();
        let rows = rows.into_iter().collect::<Vec<_>>();
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let row_height = (line_height + theme.spacing.xs * 2.0).max(18.0);
        let width = columns
            .iter()
            .map(|column| column.min_width)
            .sum::<f32>()
            .max(96.0);
        let height = row_height * (rows.len() as f32 + 1.0);
        Self {
            columns,
            rows,
            text_style,
            header_text_style,
            row_height,
            min_size: UiSize::new(width, height),
            theme,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow {
    pub label: String,
    pub depth: usize,
    pub expanded: bool,
    pub selected: bool,
    pub has_children: bool,
    pub enabled: bool,
}

impl TreeRow {
    pub fn new(label: impl Into<String>, depth: usize, has_children: bool) -> Self {
        Self {
            label: label.into(),
            depth,
            expanded: true,
            selected: false,
            has_children,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub rows: Vec<TreeRow>,
    pub text_style: TextStyle,
    pub row_height: f32,
    pub indent_width: f32,
    pub min_size: UiSize,
    pub theme: ThemeTokens,
}

impl TreeNode {
    pub fn new(
        rows: impl IntoIterator<Item = TreeRow>,
        text_style: TextStyle,
        theme: ThemeTokens,
    ) -> Self {
        let rows = rows.into_iter().collect::<Vec<_>>();
        let line_height = text_style.line_height_or_default(text_style.font_size * 1.2);
        let row_height = (line_height + theme.spacing.xs * 1.5).max(18.0);
        let indent_width = theme.spacing.lg.max(16.0);
        let width = rows
            .iter()
            .map(|row| {
                row.depth as f32 * indent_width
                    + row.label.chars().count() as f32 * text_style.font_size * 0.6
                    + theme.spacing.xl
            })
            .fold(96.0, f32::max);
        Self {
            rows,
            text_style,
            row_height,
            indent_width,
            min_size: UiSize::new(width, row_height),
            theme,
        }
    }
}
