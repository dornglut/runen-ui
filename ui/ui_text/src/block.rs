//! File: domain/ui/ui_text/src/block.rs
//! Purpose: Text block, run, span, and source-range contracts.

use crate::{TextLayoutPolicy, TextSpanStyle, TextStyle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextBlockId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextRunId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextSpanId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSourceRange {
    pub start_cluster: u32,
    pub end_cluster: u32,
}

impl TextSourceRange {
    pub const fn new(start_cluster: u32, end_cluster: u32) -> Self {
        Self {
            start_cluster,
            end_cluster,
        }
    }
    pub const fn collapsed(cluster: u32) -> Self {
        Self {
            start_cluster: cluster,
            end_cluster: cluster,
        }
    }
    pub fn len(self) -> u32 {
        self.end_cluster.saturating_sub(self.start_cluster)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextSemanticRole {
    Label,
    Heading,
    Body,
    Helper,
    Badge,
    Tooltip,
    MenuItem,
    InspectorLabel,
    InspectorValue,
    TabLabel,
    Proof,
}

impl TextSemanticRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Heading => "heading",
            Self::Body => "body",
            Self::Helper => "helper",
            Self::Badge => "badge",
            Self::Tooltip => "tooltip",
            Self::MenuItem => "menu-item",
            Self::InspectorLabel => "inspector-label",
            Self::InspectorValue => "inspector-value",
            Self::TabLabel => "tab-label",
            Self::Proof => "proof",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    pub span_id: TextSpanId,
    pub source_range: TextSourceRange,
    pub style: TextSpanStyle,
    pub semantic_role: Option<TextSemanticRole>,
}

impl TextSpan {
    pub fn new(span_id: TextSpanId, source_range: TextSourceRange) -> Self {
        Self {
            span_id,
            source_range,
            style: TextSpanStyle::inherit(),
            semantic_role: None,
        }
    }
    pub fn with_style(mut self, style: TextSpanStyle) -> Self {
        self.style = style;
        self
    }
    pub fn with_semantic_role(mut self, role: TextSemanticRole) -> Self {
        self.semantic_role = Some(role);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub run_id: TextRunId,
    pub text: String,
    pub style: TextSpanStyle,
    pub semantic_role: Option<TextSemanticRole>,
    pub source_range: Option<TextSourceRange>,
    pub spans: Vec<TextSpan>,
}

impl TextRun {
    pub fn new(run_id: TextRunId, text: impl Into<String>) -> Self {
        Self {
            run_id,
            text: text.into(),
            style: TextSpanStyle::inherit(),
            semantic_role: None,
            source_range: None,
            spans: Vec::new(),
        }
    }
    pub fn with_style(mut self, style: TextSpanStyle) -> Self {
        self.style = style;
        self
    }
    pub fn with_semantic_role(mut self, role: TextSemanticRole) -> Self {
        self.semantic_role = Some(role);
        self
    }
    pub fn with_source_range(mut self, range: TextSourceRange) -> Self {
        self.source_range = Some(range);
        self
    }
    pub fn with_span(mut self, span: TextSpan) -> Self {
        self.spans.push(span);
        self
    }
    pub fn with_spans(mut self, spans: impl IntoIterator<Item = TextSpan>) -> Self {
        self.spans.extend(spans);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    pub text_block_id: TextBlockId,
    pub runs: Vec<TextRun>,
    pub base_style: TextStyle,
    pub layout: TextLayoutPolicy,
    pub semantic_role: Option<TextSemanticRole>,
    pub accessibility_label_role: Option<String>,
}

impl TextBlock {
    pub fn new(text_block_id: TextBlockId, base_style: TextStyle) -> Self {
        Self {
            text_block_id,
            runs: Vec::new(),
            base_style,
            layout: TextLayoutPolicy::default(),
            semantic_role: None,
            accessibility_label_role: None,
        }
    }
    pub fn label(text: impl Into<String>) -> Self {
        Self::label_with_id(TextBlockId(1), TextRunId(1), text)
    }
    pub fn label_with_id(
        text_block_id: TextBlockId,
        text_run_id: TextRunId,
        text: impl Into<String>,
    ) -> Self {
        Self::new(text_block_id, TextStyle::default())
            .with_run(TextRun::new(text_run_id, text).with_semantic_role(TextSemanticRole::Label))
            .with_semantic_role(TextSemanticRole::Label)
            .with_layout(TextLayoutPolicy::no_wrap_label())
    }
    pub fn body(text: impl Into<String>, max_width: f32) -> Self {
        Self::body_with_id(TextBlockId(1), TextRunId(1), text, max_width)
    }
    pub fn body_with_id(
        text_block_id: TextBlockId,
        text_run_id: TextRunId,
        text: impl Into<String>,
        max_width: f32,
    ) -> Self {
        Self::new(text_block_id, TextStyle::default())
            .with_run(TextRun::new(text_run_id, text).with_semantic_role(TextSemanticRole::Body))
            .with_semantic_role(TextSemanticRole::Body)
            .with_layout(TextLayoutPolicy::wrapping_body(max_width))
    }
    pub fn helper(text: impl Into<String>, max_width: f32, max_lines: u32) -> Self {
        Self::helper_with_id(TextBlockId(1), TextRunId(1), text, max_width, max_lines)
    }
    pub fn helper_with_id(
        text_block_id: TextBlockId,
        text_run_id: TextRunId,
        text: impl Into<String>,
        max_width: f32,
        max_lines: u32,
    ) -> Self {
        Self::new(text_block_id, TextStyle::default())
            .with_run(TextRun::new(text_run_id, text).with_semantic_role(TextSemanticRole::Helper))
            .with_semantic_role(TextSemanticRole::Helper)
            .with_layout(TextLayoutPolicy::helper_text(max_width, max_lines))
    }
    pub fn badge(text: impl Into<String>, max_width: f32) -> Self {
        Self::badge_with_id(TextBlockId(1), TextRunId(1), text, max_width)
    }
    pub fn badge_with_id(
        text_block_id: TextBlockId,
        text_run_id: TextRunId,
        text: impl Into<String>,
        max_width: f32,
    ) -> Self {
        Self::new(text_block_id, TextStyle::default())
            .with_run(TextRun::new(text_run_id, text).with_semantic_role(TextSemanticRole::Badge))
            .with_semantic_role(TextSemanticRole::Badge)
            .with_layout(TextLayoutPolicy::badge(max_width))
    }
    pub fn tab_label(text: impl Into<String>, max_width: f32) -> Self {
        Self::tab_label_with_id(TextBlockId(1), TextRunId(1), text, max_width)
    }
    pub fn tab_label_with_id(
        text_block_id: TextBlockId,
        text_run_id: TextRunId,
        text: impl Into<String>,
        max_width: f32,
    ) -> Self {
        Self::new(text_block_id, TextStyle::default())
            .with_run(
                TextRun::new(text_run_id, text).with_semantic_role(TextSemanticRole::TabLabel),
            )
            .with_semantic_role(TextSemanticRole::TabLabel)
            .with_layout(TextLayoutPolicy::tab_label(max_width))
    }
    pub fn inspector_row_value(text: impl Into<String>, max_width: f32) -> Self {
        Self::inspector_row_value_with_id(TextBlockId(1), TextRunId(1), text, max_width)
    }
    pub fn inspector_row_value_with_id(
        text_block_id: TextBlockId,
        text_run_id: TextRunId,
        text: impl Into<String>,
        max_width: f32,
    ) -> Self {
        Self::new(text_block_id, TextStyle::default())
            .with_run(
                TextRun::new(text_run_id, text)
                    .with_semantic_role(TextSemanticRole::InspectorValue),
            )
            .with_semantic_role(TextSemanticRole::InspectorValue)
            .with_layout(TextLayoutPolicy::inspector_row_value(max_width))
    }
    pub fn inline_spans(text: impl Into<String>, spans: Vec<TextSpan>) -> Self {
        Self::inline_spans_with_id(TextBlockId(1), TextRunId(1), text, spans)
    }
    pub fn inline_spans_with_id(
        text_block_id: TextBlockId,
        text_run_id: TextRunId,
        text: impl Into<String>,
        spans: Vec<TextSpan>,
    ) -> Self {
        Self::new(text_block_id, TextStyle::default())
            .with_run(TextRun::new(text_run_id, text).with_spans(spans))
            .with_semantic_role(TextSemanticRole::Body)
    }
    pub fn with_run(mut self, run: TextRun) -> Self {
        self.runs.push(run);
        self
    }
    pub fn with_base_style(mut self, style: TextStyle) -> Self {
        self.base_style = style;
        self
    }
    pub fn with_layout(mut self, layout: TextLayoutPolicy) -> Self {
        self.layout = layout;
        self
    }
    pub fn with_semantic_role(mut self, role: TextSemanticRole) -> Self {
        self.semantic_role = Some(role);
        self
    }
    pub fn with_accessibility_label_role(mut self, role: impl Into<String>) -> Self {
        self.accessibility_label_role = Some(role.into());
        self
    }
}
