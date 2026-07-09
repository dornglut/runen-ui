//! File: domain/ui/ui_render_data/src/primitives/graph_canvas.rs
//! Purpose: Role-tagged graph canvas primitive batches over generic UI primitives.

use crate::UiPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphCanvasPrimitiveRole {
    NodeBox,
    Port,
    Edge,
    Label,
    SelectionOutline,
    ConnectionPreview,
    Overlay,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphCanvasRenderPrimitive {
    pub role: GraphCanvasPrimitiveRole,
    pub primitive: UiPrimitive,
    insertion_order: usize,
}

impl GraphCanvasRenderPrimitive {
    pub fn new(
        role: GraphCanvasPrimitiveRole,
        primitive: impl Into<UiPrimitive>,
        insertion_order: usize,
    ) -> Self {
        Self {
            role,
            primitive: primitive.into(),
            insertion_order,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphCanvasPrimitiveBatch {
    pub primitives: Vec<GraphCanvasRenderPrimitive>,
}

impl GraphCanvasPrimitiveBatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, role: GraphCanvasPrimitiveRole, primitive: impl Into<UiPrimitive>) {
        let insertion_order = self.primitives.len();
        self.primitives.push(GraphCanvasRenderPrimitive::new(
            role,
            primitive,
            insertion_order,
        ));
    }

    pub fn count_role(&self, role: GraphCanvasPrimitiveRole) -> usize {
        self.primitives
            .iter()
            .filter(|primitive| primitive.role == role)
            .count()
    }

    pub fn into_render_primitives(mut self) -> Vec<GraphCanvasRenderPrimitive> {
        self.primitives.sort_by_key(|primitive| {
            (
                graph_canvas_role_order(primitive.role),
                primitive.insertion_order,
            )
        });
        self.primitives
    }

    pub fn into_ui_primitives(self) -> Vec<UiPrimitive> {
        self.into_render_primitives()
            .into_iter()
            .map(|primitive| primitive.primitive)
            .collect()
    }
}

fn graph_canvas_role_order(role: GraphCanvasPrimitiveRole) -> u8 {
    match role {
        GraphCanvasPrimitiveRole::Edge => 0,
        GraphCanvasPrimitiveRole::ConnectionPreview => 1,
        GraphCanvasPrimitiveRole::NodeBox => 2,
        GraphCanvasPrimitiveRole::Port => 3,
        GraphCanvasPrimitiveRole::SelectionOutline => 4,
        GraphCanvasPrimitiveRole::Overlay => 5,
        GraphCanvasPrimitiveRole::Label => 6,
    }
}
