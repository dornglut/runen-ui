//! File: domain/ui/ui_layout/src/contracts.rs
//! Crate: ui_layout

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiLayoutRole {
    Panel,
    Row,
    Column,
    Stack,
    Split,
    Scroll,
    List,
    Table,
    Tree,
    VirtualList,
    VirtualTable,
}

impl UiLayoutRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Panel => "panel",
            Self::Row => "row",
            Self::Column => "column",
            Self::Stack => "stack",
            Self::Split => "split",
            Self::Scroll => "scroll",
            Self::List => "list",
            Self::Table => "table",
            Self::Tree => "tree",
            Self::VirtualList => "virtual-list",
            Self::VirtualTable => "virtual-table",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiContainerKind {
    Panel,
    Viewport,
    Section,
    Group,
    Collection,
    SplitPane,
    ScrollRegion,
}

impl UiContainerKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Panel => "panel",
            Self::Viewport => "viewport",
            Self::Section => "section",
            Self::Group => "group",
            Self::Collection => "collection",
            Self::SplitPane => "split-pane",
            Self::ScrollRegion => "scroll-region",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiSizeConstraintKind {
    MinSize,
    MaxSize,
    PreferredSize,
    FillWidth,
    FillHeight,
    IntrinsicSize,
}

impl UiSizeConstraintKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MinSize => "min-size",
            Self::MaxSize => "max-size",
            Self::PreferredSize => "preferred-size",
            Self::FillWidth => "fill-width",
            Self::FillHeight => "fill-height",
            Self::IntrinsicSize => "intrinsic-size",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiScrollRequirement {
    Scrollable,
    ScrollOwner,
    AxisX,
    AxisY,
    PositionHostOwned,
}

impl UiScrollRequirement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scrollable => "scrollable",
            Self::ScrollOwner => "scroll-owner",
            Self::AxisX => "scroll-axis-x",
            Self::AxisY => "scroll-axis-y",
            Self::PositionHostOwned => "scroll-position-host-owned",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiContentState {
    Empty,
    Loading,
    Error,
    Overflow,
    Ready,
}

impl UiContentState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Loading => "loading",
            Self::Error => "error",
            Self::Overflow => "overflow",
            Self::Ready => "ready",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiItemIdentityRequirement {
    pub identity_id: String,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl UiItemIdentityRequirement {
    pub fn new(identity_id: impl Into<String>) -> Self {
        Self {
            identity_id: identity_id.into(),
            required: true,
            notes: String::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSelectionIdentityRequirement {
    pub identity_id: String,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl UiSelectionIdentityRequirement {
    pub fn new(identity_id: impl Into<String>) -> Self {
        Self {
            identity_id: identity_id.into(),
            required: true,
            notes: String::new(),
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiVirtualizationRequirement {
    Ready,
    EstimatedItemSize,
    StableItemIdentity,
    WindowedRendering,
    OverscanBudget,
}

impl UiVirtualizationRequirement {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "virtualization-ready",
            Self::EstimatedItemSize => "estimated-item-size",
            Self::StableItemIdentity => "stable-item-identity",
            Self::WindowedRendering => "windowed-rendering",
            Self::OverscanBudget => "overscan-budget",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiLargeContentBudget {
    pub budget_id: String,
    #[serde(default)]
    pub estimated_item_count: Option<u32>,
    #[serde(default)]
    pub overscan_budget_items: Option<u32>,
}

impl UiLargeContentBudget {
    pub fn new(budget_id: impl Into<String>) -> Self {
        Self {
            budget_id: budget_id.into(),
            estimated_item_count: None,
            overscan_budget_items: None,
        }
    }

    pub fn with_estimated_item_count(mut self, estimated_item_count: u32) -> Self {
        self.estimated_item_count = Some(estimated_item_count);
        self
    }

    pub fn with_overscan_budget_items(mut self, overscan_budget_items: u32) -> Self {
        self.overscan_budget_items = Some(overscan_budget_items);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiLayoutDiagnosticKind {
    MissingItemIdentity,
    MissingSelectionIdentity,
    MissingScrollOwner,
    MissingLargeContentBudget,
    ExpectedFailure,
}

impl UiLayoutDiagnosticKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingItemIdentity => "missing-item-identity",
            Self::MissingSelectionIdentity => "missing-selection-identity",
            Self::MissingScrollOwner => "missing-scroll-owner",
            Self::MissingLargeContentBudget => "missing-large-content-budget",
            Self::ExpectedFailure => "expected-failure",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiLayoutDiagnostic {
    pub diagnostic_id: String,
    pub kind: UiLayoutDiagnosticKind,
    pub message: String,
}

impl UiLayoutDiagnostic {
    pub fn new(
        diagnostic_id: impl Into<String>,
        kind: UiLayoutDiagnosticKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            diagnostic_id: diagnostic_id.into(),
            kind,
            message: message.into(),
        }
    }
}

fn default_required() -> bool {
    true
}
