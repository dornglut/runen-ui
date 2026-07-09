//! Core retained UI node contracts.

use crate::WidgetId;

#[derive(Debug, Clone, PartialEq)]
pub struct UiNode {
    pub id: WidgetId,
    pub kind: UiNodeKind,
    pub children: Vec<UiNode>,
}

impl UiNode {
    pub fn new(id: WidgetId, kind: UiNodeKind) -> Self {
        Self {
            id,
            kind,
            children: Vec::new(),
        }
    }

    pub fn with_children(id: WidgetId, kind: UiNodeKind, children: Vec<UiNode>) -> Self {
        Self { id, kind, children }
    }

    pub fn push_child(&mut self, child: UiNode) {
        self.children.push(child);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiNodeKind {
    Panel(super::PanelNode),
    Popup(super::PopupNode),
    RadialMenu(super::RadialMenuNode),
    OverlayAdornment(super::OverlayAdornmentNode),
    Label(super::LabelNode),
    Button(super::ButtonNode),
    TextInput(super::TextInputNode),
    Toggle(super::ToggleNode),
    NumericInput(super::NumericInputNode),
    Tabs(super::TabsNode),
    Select(super::SelectNode),
    Table(super::TableNode),
    Tree(super::TreeNode),
    Spacer(super::SpacerNode),
    Divider(super::DividerNode),
    Image(super::ImageNode),
    ProductSurface(super::ProductSurfaceNode),
    GraphCanvas(super::GraphCanvasNode),
    ViewportSurfaceEmbed(super::ViewportSurfaceEmbedNode),
    Scroll(super::ScrollNode),
    Stack(super::StackNode),
    Split(super::SplitNode),
}
