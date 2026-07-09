//! File: domain/ui/ui_runtime/src/tree/ui_tree.rs
//! Purpose: Retained UI tree root.

use crate::{UiNode, WidgetId};

#[derive(Debug, Clone, PartialEq)]
pub struct UiTree {
    pub root: UiNode,
}

impl UiTree {
    pub fn new(root: UiNode) -> Self {
        Self { root }
    }

    pub fn root_id(&self) -> WidgetId {
        self.root.id
    }

    pub fn walk(&self) -> impl Iterator<Item = &UiNode> {
        let mut out = Vec::new();
        collect_nodes(&self.root, &mut out);
        out.into_iter()
    }
}

fn collect_nodes<'a>(node: &'a UiNode, out: &mut Vec<&'a UiNode>) {
    out.push(node);

    for child in &node.children {
        collect_nodes(child, out);
    }
}
