//! File: domain/ui/ui_graph_editor/src/lib.rs
//! Purpose: Backend-neutral graph editor interaction contracts.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GraphCanvasId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GraphNodeKey(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GraphPortKey(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GraphEdgeKey(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GraphSelectionKey(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPoint {
    pub x: i32,
    pub y: i32,
}

impl GraphPoint {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GraphVector {
    pub x: i32,
    pub y: i32,
}

impl GraphVector {
    pub const ZERO: Self = Self { x: 0, y: 0 };

    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl GraphRect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn contains(self, point: GraphPoint) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphPortDirection {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNodeView {
    pub node: GraphNodeKey,
    pub title: String,
    pub rect: GraphRect,
    pub selected: bool,
}

impl GraphNodeView {
    pub fn new(node: GraphNodeKey, title: impl Into<String>, rect: GraphRect) -> Self {
        Self {
            node,
            title: title.into(),
            rect,
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPortView {
    pub port: GraphPortKey,
    pub node: GraphNodeKey,
    pub label: String,
    pub direction: GraphPortDirection,
    pub rect: GraphRect,
}

impl GraphPortView {
    pub fn new(
        port: GraphPortKey,
        node: GraphNodeKey,
        label: impl Into<String>,
        direction: GraphPortDirection,
        rect: GraphRect,
    ) -> Self {
        Self {
            port,
            node,
            label: label.into(),
            direction,
            rect,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdgeView {
    pub edge: GraphEdgeKey,
    pub from_port: GraphPortKey,
    pub to_port: GraphPortKey,
    pub from: GraphPoint,
    pub to: GraphPoint,
    pub hit_rect: GraphRect,
    pub selected: bool,
}

impl GraphEdgeView {
    pub fn new(
        edge: GraphEdgeKey,
        from_port: GraphPortKey,
        to_port: GraphPortKey,
        from: GraphPoint,
        to: GraphPoint,
        hit_rect: GraphRect,
    ) -> Self {
        Self {
            edge,
            from_port,
            to_port,
            from,
            to,
            hit_rect,
            selected: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphOverlayAnchor {
    Node(GraphNodeKey),
    Port(GraphPortKey),
    Edge(GraphEdgeKey),
    Point(GraphPoint),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphOverlaySeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphOverlayView {
    pub anchor: GraphOverlayAnchor,
    pub label: String,
    pub rect: GraphRect,
    pub severity: GraphOverlaySeverity,
    pub active: bool,
}

impl GraphOverlayView {
    pub fn new(
        anchor: GraphOverlayAnchor,
        label: impl Into<String>,
        rect: GraphRect,
        severity: GraphOverlaySeverity,
    ) -> Self {
        Self {
            anchor,
            label: label.into(),
            rect,
            severity,
            active: false,
        }
    }

    pub const fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNodeBounds {
    pub node: GraphNodeKey,
    pub rect: GraphRect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPortBounds {
    pub port: GraphPortKey,
    pub node: GraphNodeKey,
    pub rect: GraphRect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdgeBounds {
    pub edge: GraphEdgeKey,
    pub rect: GraphRect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphSelectionBounds {
    pub selection: GraphSelectionKey,
    pub rect: GraphRect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphHitTarget {
    Empty,
    Background,
    NodeBody(GraphNodeKey),
    Port(GraphPortKey),
    Edge(GraphEdgeKey),
    Selection(GraphSelectionKey),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphHitTestScene {
    pub canvas_rect: GraphRect,
    pub nodes: Vec<GraphNodeBounds>,
    pub ports: Vec<GraphPortBounds>,
    pub edges: Vec<GraphEdgeBounds>,
    pub selections: Vec<GraphSelectionBounds>,
}

impl Default for GraphHitTestScene {
    fn default() -> Self {
        Self {
            canvas_rect: GraphRect::new(0, 0, 0, 0),
            nodes: Vec::new(),
            ports: Vec::new(),
            edges: Vec::new(),
            selections: Vec::new(),
        }
    }
}

impl GraphHitTestScene {
    pub fn with_canvas_rect(mut self, canvas_rect: GraphRect) -> Self {
        self.canvas_rect = canvas_rect;
        self
    }

    pub fn hit_test(&self, point: GraphPoint) -> GraphHitTarget {
        if !self.canvas_rect.contains(point) {
            return GraphHitTarget::Empty;
        }
        for port in self.ports.iter().rev() {
            if port.rect.contains(point) {
                return GraphHitTarget::Port(port.port);
            }
        }
        for node in self.nodes.iter().rev() {
            if node.rect.contains(point) {
                return GraphHitTarget::NodeBody(node.node);
            }
        }
        for selection in self.selections.iter().rev() {
            if selection.rect.contains(point) {
                return GraphHitTarget::Selection(selection.selection);
            }
        }
        for edge in self.edges.iter().rev() {
            if edge.rect.contains(point) {
                return GraphHitTarget::Edge(edge.edge);
            }
        }
        GraphHitTarget::Background
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphViewport {
    pub pan_x: i32,
    pub pan_y: i32,
    pub zoom_milli: u32,
}

impl Default for GraphViewport {
    fn default() -> Self {
        Self {
            pan_x: 0,
            pan_y: 0,
            zoom_milli: 1000,
        }
    }
}

impl GraphViewport {
    pub const MIN_ZOOM_MILLI: u32 = 250;
    pub const MAX_ZOOM_MILLI: u32 = 4000;

    pub fn pan_by(mut self, delta: GraphVector) -> Self {
        self.pan_x = self.pan_x.saturating_add(delta.x);
        self.pan_y = self.pan_y.saturating_add(delta.y);
        self
    }

    pub fn zoom_by_wheel_delta(mut self, wheel_delta_y: i32) -> Self {
        let step = (wheel_delta_y.abs().max(1) as u32).saturating_mul(50);
        self.zoom_milli = if wheel_delta_y > 0 {
            self.zoom_milli.saturating_add(step)
        } else {
            self.zoom_milli.saturating_sub(step)
        }
        .clamp(Self::MIN_ZOOM_MILLI, Self::MAX_ZOOM_MILLI);
        self
    }

    pub fn screen_to_graph_point(self, point: GraphPoint) -> GraphPoint {
        let zoom = self.zoom_milli.max(1) as i64;
        GraphPoint::new(
            (((point.x - self.pan_x) as i64 * 1000) / zoom) as i32,
            (((point.y - self.pan_y) as i64 * 1000) / zoom) as i32,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GraphSelection {
    pub nodes: BTreeSet<GraphNodeKey>,
    pub edges: BTreeSet<GraphEdgeKey>,
}

impl GraphSelection {
    pub fn select_node(node: GraphNodeKey) -> Self {
        Self {
            nodes: BTreeSet::from([node]),
            edges: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphCanvasViewModel {
    pub canvas_id: GraphCanvasId,
    pub viewport: GraphViewport,
    pub nodes: Vec<GraphNodeView>,
    pub ports: Vec<GraphPortView>,
    pub edges: Vec<GraphEdgeView>,
    pub overlays: Vec<GraphOverlayView>,
    pub selection: GraphSelection,
    pub hit_test_scene: GraphHitTestScene,
}

impl Default for GraphCanvasViewModel {
    fn default() -> Self {
        Self {
            canvas_id: GraphCanvasId(0),
            viewport: GraphViewport::default(),
            nodes: Vec::new(),
            ports: Vec::new(),
            edges: Vec::new(),
            overlays: Vec::new(),
            selection: GraphSelection::default(),
            hit_test_scene: GraphHitTestScene::default(),
        }
    }
}

impl GraphCanvasViewModel {
    pub fn new(canvas_id: GraphCanvasId) -> Self {
        Self {
            canvas_id,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GraphEditorViewModel {
    pub viewport: GraphViewport,
    pub selection: GraphSelection,
    pub hit_test_scene: GraphHitTestScene,
    pub canvas: GraphCanvasViewModel,
    pub can_undo: bool,
    pub can_redo: bool,
    pub active_edit_group: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GraphInputModifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphCanvasAction {
    SelectNode {
        node: GraphNodeKey,
        additive: bool,
    },
    SelectEdge {
        edge: GraphEdgeKey,
        additive: bool,
    },
    ClearSelection,
    Pan {
        phase: GraphGesturePhase,
        delta: GraphVector,
    },
    Zoom {
        anchor: GraphPoint,
        previous_zoom_milli: u32,
        zoom_milli: u32,
    },
    BeginNodeDrag {
        node: GraphNodeKey,
        start: GraphPoint,
    },
    UpdateNodeDrag {
        node: GraphNodeKey,
        delta: GraphVector,
    },
    EndNodeDrag {
        node: GraphNodeKey,
        delta: GraphVector,
    },
    BeginConnection {
        from: GraphPortKey,
        start: GraphPoint,
    },
    UpdateConnection {
        from: GraphPortKey,
        current: GraphPoint,
        hover: Option<GraphPortKey>,
    },
    EndConnection {
        from: GraphPortKey,
        to: Option<GraphPortKey>,
    },
    BeginMarquee {
        start: GraphPoint,
    },
    UpdateMarquee {
        rect: GraphRect,
    },
    EndMarquee {
        rect: GraphRect,
    },
    KeyboardDeleteSelection,
    KeyboardShortcut(GraphShortcutAction),
    CancelGesture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphEditorAction {
    Pan {
        delta_x: i32,
        delta_y: i32,
    },
    SetZoom {
        zoom_milli: u32,
    },
    SelectNode {
        node: GraphNodeKey,
    },
    SelectEdge {
        edge: GraphEdgeKey,
    },
    ClearSelection,
    AddNode {
        descriptor_key: String,
    },
    DeleteSelection,
    ConnectPorts {
        from: GraphPortKey,
        to: GraphPortKey,
    },
    DisconnectEdge {
        edge: GraphEdgeKey,
    },
    SetNodeValue {
        node: GraphNodeKey,
        key: String,
        value: String,
    },
    PersistLayout,
    Undo,
    Redo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphShortcutAction {
    AddNode,
    DeleteSelection,
    Undo,
    Redo,
    BuildPreview,
    FocusPreview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphGesturePhase {
    Begin,
    Update,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphGestureIntent {
    PanViewport {
        phase: GraphGesturePhase,
        delta_x: i32,
        delta_y: i32,
    },
    DragNode {
        phase: GraphGesturePhase,
        node: GraphNodeKey,
        delta_x: i32,
        delta_y: i32,
    },
    ConnectPorts {
        phase: GraphGesturePhase,
        from: GraphPortKey,
        hover: Option<GraphPortKey>,
    },
    BoxSelect {
        phase: GraphGesturePhase,
        rect: GraphRect,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNodeDragState {
    pub node: GraphNodeKey,
    pub start: GraphPoint,
    pub current: GraphPoint,
}

impl GraphNodeDragState {
    pub fn delta(self) -> GraphVector {
        GraphVector::new(self.current.x - self.start.x, self.current.y - self.start.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphConnectionDragState {
    pub from: GraphPortKey,
    pub start: GraphPoint,
    pub current: GraphPoint,
    pub hover: Option<GraphPortKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphPanState {
    pub start: GraphPoint,
    pub current: GraphPoint,
}

impl GraphPanState {
    pub fn delta(self) -> GraphVector {
        GraphVector::new(self.current.x - self.start.x, self.current.y - self.start.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphMarqueeSelectionState {
    pub start: GraphPoint,
    pub current: GraphPoint,
}

impl GraphMarqueeSelectionState {
    pub fn rect(self) -> GraphRect {
        let x1 = self.start.x.min(self.current.x);
        let y1 = self.start.y.min(self.current.y);
        let x2 = self.start.x.max(self.current.x);
        let y2 = self.start.y.max(self.current.y);
        GraphRect::new(x1, y1, x2 - x1, y2 - y1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphActiveGesture {
    NodeDrag(GraphNodeDragState),
    ConnectionPreview(GraphConnectionDragState),
    Pan(GraphPanState),
    Marquee(GraphMarqueeSelectionState),
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GraphCanvasGestureState {
    pub active: Option<GraphActiveGesture>,
}

impl GraphCanvasGestureState {
    pub fn begin_pointer(
        &mut self,
        hit: GraphHitTarget,
        point: GraphPoint,
        modifiers: GraphInputModifiers,
    ) -> Option<GraphCanvasAction> {
        self.active = None;
        match hit {
            GraphHitTarget::NodeBody(node) => {
                self.active = Some(GraphActiveGesture::NodeDrag(GraphNodeDragState {
                    node,
                    start: point,
                    current: point,
                }));
                Some(GraphCanvasAction::BeginNodeDrag { node, start: point })
            }
            GraphHitTarget::Port(from) => {
                self.active = Some(GraphActiveGesture::ConnectionPreview(
                    GraphConnectionDragState {
                        from,
                        start: point,
                        current: point,
                        hover: None,
                    },
                ));
                Some(GraphCanvasAction::BeginConnection { from, start: point })
            }
            GraphHitTarget::Background | GraphHitTarget::Empty => {
                if modifiers.shift {
                    self.active = Some(GraphActiveGesture::Marquee(GraphMarqueeSelectionState {
                        start: point,
                        current: point,
                    }));
                    Some(GraphCanvasAction::BeginMarquee { start: point })
                } else {
                    self.active = Some(GraphActiveGesture::Pan(GraphPanState {
                        start: point,
                        current: point,
                    }));
                    Some(GraphCanvasAction::Pan {
                        phase: GraphGesturePhase::Begin,
                        delta: GraphVector::ZERO,
                    })
                }
            }
            GraphHitTarget::Edge(_) | GraphHitTarget::Selection(_) => None,
        }
    }

    pub fn update_pointer(
        &mut self,
        point: GraphPoint,
        hit: GraphHitTarget,
    ) -> Option<GraphCanvasAction> {
        match self.active.as_mut()? {
            GraphActiveGesture::NodeDrag(drag) => {
                drag.current = point;
                Some(GraphCanvasAction::UpdateNodeDrag {
                    node: drag.node,
                    delta: drag.delta(),
                })
            }
            GraphActiveGesture::ConnectionPreview(connection) => {
                connection.current = point;
                connection.hover = match hit {
                    GraphHitTarget::Port(port) => Some(port),
                    _ => None,
                };
                Some(GraphCanvasAction::UpdateConnection {
                    from: connection.from,
                    current: point,
                    hover: connection.hover,
                })
            }
            GraphActiveGesture::Pan(pan) => {
                let previous = pan.current;
                pan.current = point;
                Some(GraphCanvasAction::Pan {
                    phase: GraphGesturePhase::Update,
                    delta: GraphVector::new(point.x - previous.x, point.y - previous.y),
                })
            }
            GraphActiveGesture::Marquee(marquee) => {
                marquee.current = point;
                Some(GraphCanvasAction::UpdateMarquee {
                    rect: marquee.rect(),
                })
            }
        }
    }

    pub fn end_pointer(
        &mut self,
        point: GraphPoint,
        hit: GraphHitTarget,
    ) -> Option<GraphCanvasAction> {
        let active = self.active.take()?;
        match active {
            GraphActiveGesture::NodeDrag(mut drag) => {
                drag.current = point;
                Some(GraphCanvasAction::EndNodeDrag {
                    node: drag.node,
                    delta: drag.delta(),
                })
            }
            GraphActiveGesture::ConnectionPreview(mut connection) => {
                connection.current = point;
                connection.hover = match hit {
                    GraphHitTarget::Port(port) => Some(port),
                    _ => None,
                };
                Some(GraphCanvasAction::EndConnection {
                    from: connection.from,
                    to: connection.hover,
                })
            }
            GraphActiveGesture::Pan(mut pan) => {
                pan.current = point;
                Some(GraphCanvasAction::Pan {
                    phase: GraphGesturePhase::End,
                    delta: pan.delta(),
                })
            }
            GraphActiveGesture::Marquee(mut marquee) => {
                marquee.current = point;
                Some(GraphCanvasAction::EndMarquee {
                    rect: marquee.rect(),
                })
            }
        }
    }

    pub fn cancel(&mut self) -> Option<GraphCanvasAction> {
        self.active.take().map(|_| GraphCanvasAction::CancelGesture)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphLayoutTransaction {
    pub edit_group: u64,
    pub actions: Vec<GraphEditorAction>,
}

impl GraphLayoutTransaction {
    pub fn new(edit_group: u64, actions: impl IntoIterator<Item = GraphEditorAction>) -> Self {
        Self {
            edit_group,
            actions: actions.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphShortcut {
    pub chord: String,
    pub action: GraphShortcutAction,
}

impl GraphShortcut {
    pub fn new(chord: impl Into<String>, action: GraphShortcutAction) -> Self {
        Self {
            chord: chord.into(),
            action,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_uses_stable_node_keys() {
        let selection = GraphSelection::select_node(GraphNodeKey(7));

        assert!(selection.nodes.contains(&GraphNodeKey(7)));
        assert!(selection.edges.is_empty());
    }

    #[test]
    fn editor_action_is_backend_neutral() {
        let action = GraphEditorAction::ConnectPorts {
            from: GraphPortKey(1),
            to: GraphPortKey(2),
        };

        assert_eq!(
            action,
            GraphEditorAction::ConnectPorts {
                from: GraphPortKey(1),
                to: GraphPortKey(2)
            }
        );
    }

    #[test]
    fn graph_canvas_hit_testing() {
        let scene = GraphHitTestScene {
            canvas_rect: GraphRect::new(0, 0, 320, 240),
            nodes: vec![GraphNodeBounds {
                node: GraphNodeKey(1),
                rect: GraphRect::new(20, 20, 120, 80),
            }],
            ports: vec![GraphPortBounds {
                node: GraphNodeKey(1),
                port: GraphPortKey(9),
                rect: GraphRect::new(28, 28, 12, 12),
            }],
            edges: vec![GraphEdgeBounds {
                edge: GraphEdgeKey(3),
                rect: GraphRect::new(160, 80, 90, 12),
            }],
            selections: vec![GraphSelectionBounds {
                selection: GraphSelectionKey(4),
                rect: GraphRect::new(14, 14, 132, 92),
            }],
        };

        assert_eq!(
            scene.hit_test(GraphPoint::new(30, 30)),
            GraphHitTarget::Port(GraphPortKey(9))
        );
        assert_eq!(
            scene.hit_test(GraphPoint::new(145, 20)),
            GraphHitTarget::Selection(GraphSelectionKey(4))
        );
        assert_eq!(
            scene.hit_test(GraphPoint::new(60, 40)),
            GraphHitTarget::NodeBody(GraphNodeKey(1))
        );
        assert_eq!(
            scene.hit_test(GraphPoint::new(170, 85)),
            GraphHitTarget::Edge(GraphEdgeKey(3))
        );
        assert_eq!(
            scene.hit_test(GraphPoint::new(300, 200)),
            GraphHitTarget::Background
        );
        assert_eq!(
            scene.hit_test(GraphPoint::new(400, 400)),
            GraphHitTarget::Empty
        );
    }

    #[test]
    fn graph_canvas_gesture_state_forms_node_drag() {
        let mut state = GraphCanvasGestureState::default();

        assert_eq!(
            state.begin_pointer(
                GraphHitTarget::NodeBody(GraphNodeKey(7)),
                GraphPoint::new(10, 10),
                GraphInputModifiers::default(),
            ),
            Some(GraphCanvasAction::BeginNodeDrag {
                node: GraphNodeKey(7),
                start: GraphPoint::new(10, 10),
            })
        );
        assert_eq!(
            state.update_pointer(GraphPoint::new(15, 18), GraphHitTarget::Background),
            Some(GraphCanvasAction::UpdateNodeDrag {
                node: GraphNodeKey(7),
                delta: GraphVector::new(5, 8),
            })
        );
        assert_eq!(
            state.end_pointer(GraphPoint::new(20, 25), GraphHitTarget::Background),
            Some(GraphCanvasAction::EndNodeDrag {
                node: GraphNodeKey(7),
                delta: GraphVector::new(10, 15),
            })
        );
        assert!(state.active.is_none());
    }

    #[test]
    fn graph_canvas_pan_separates_incremental_updates_from_final_commit() {
        let mut state = GraphCanvasGestureState::default();

        assert_eq!(
            state.begin_pointer(
                GraphHitTarget::Background,
                GraphPoint::new(10, 10),
                GraphInputModifiers::default(),
            ),
            Some(GraphCanvasAction::Pan {
                phase: GraphGesturePhase::Begin,
                delta: GraphVector::ZERO,
            })
        );
        assert_eq!(
            state.update_pointer(GraphPoint::new(14, 13), GraphHitTarget::Background),
            Some(GraphCanvasAction::Pan {
                phase: GraphGesturePhase::Update,
                delta: GraphVector::new(4, 3),
            })
        );
        assert_eq!(
            state.update_pointer(GraphPoint::new(16, 15), GraphHitTarget::Background),
            Some(GraphCanvasAction::Pan {
                phase: GraphGesturePhase::Update,
                delta: GraphVector::new(2, 2),
            })
        );
        assert_eq!(
            state.end_pointer(GraphPoint::new(18, 19), GraphHitTarget::Background),
            Some(GraphCanvasAction::Pan {
                phase: GraphGesturePhase::End,
                delta: GraphVector::new(8, 9),
            })
        );
    }

    #[test]
    fn graph_canvas_connection_preview_lifecycle() {
        let mut state = GraphCanvasGestureState::default();

        assert_eq!(
            state.begin_pointer(
                GraphHitTarget::Port(GraphPortKey(1)),
                GraphPoint::new(2, 3),
                GraphInputModifiers::default(),
            ),
            Some(GraphCanvasAction::BeginConnection {
                from: GraphPortKey(1),
                start: GraphPoint::new(2, 3),
            })
        );
        assert_eq!(
            state.update_pointer(
                GraphPoint::new(40, 50),
                GraphHitTarget::Port(GraphPortKey(2))
            ),
            Some(GraphCanvasAction::UpdateConnection {
                from: GraphPortKey(1),
                current: GraphPoint::new(40, 50),
                hover: Some(GraphPortKey(2)),
            })
        );
        assert_eq!(
            state.end_pointer(
                GraphPoint::new(42, 52),
                GraphHitTarget::Port(GraphPortKey(2))
            ),
            Some(GraphCanvasAction::EndConnection {
                from: GraphPortKey(1),
                to: Some(GraphPortKey(2)),
            })
        );
    }

    #[test]
    fn graph_canvas_marquee_selection_lifecycle() {
        let mut state = GraphCanvasGestureState::default();
        let modifiers = GraphInputModifiers {
            shift: true,
            ..Default::default()
        };

        assert_eq!(
            state.begin_pointer(
                GraphHitTarget::Background,
                GraphPoint::new(80, 90),
                modifiers,
            ),
            Some(GraphCanvasAction::BeginMarquee {
                start: GraphPoint::new(80, 90),
            })
        );
        assert_eq!(
            state.update_pointer(GraphPoint::new(20, 30), GraphHitTarget::Background),
            Some(GraphCanvasAction::UpdateMarquee {
                rect: GraphRect::new(20, 30, 60, 60),
            })
        );
        assert_eq!(
            state.end_pointer(GraphPoint::new(10, 40), GraphHitTarget::Background),
            Some(GraphCanvasAction::EndMarquee {
                rect: GraphRect::new(10, 40, 70, 50),
            })
        );
    }

    #[test]
    fn layout_transaction_groups_source_edits() {
        let transaction = GraphLayoutTransaction::new(
            55,
            [
                GraphEditorAction::Pan {
                    delta_x: 2,
                    delta_y: -1,
                },
                GraphEditorAction::PersistLayout,
            ],
        );

        assert_eq!(transaction.edit_group, 55);
        assert_eq!(transaction.actions.len(), 2);
    }

    #[test]
    fn gestures_are_backend_neutral_intents() {
        let gesture = GraphGestureIntent::ConnectPorts {
            phase: GraphGesturePhase::Update,
            from: GraphPortKey(1),
            hover: Some(GraphPortKey(2)),
        };

        assert_eq!(
            gesture,
            GraphGestureIntent::ConnectPorts {
                phase: GraphGesturePhase::Update,
                from: GraphPortKey(1),
                hover: Some(GraphPortKey(2))
            }
        );
    }

    #[test]
    fn graph_canvas_has_no_material_semantics() {
        let manifests = [
            include_str!("../Cargo.toml"),
            include_str!("../../ui_tree/Cargo.toml"),
            include_str!("../../ui_runtime/Cargo.toml"),
            include_str!("../../ui_render_data/Cargo.toml"),
        ];
        for manifest in manifests {
            assert!(!manifest.contains(&format!("{}{}", "material", "_graph")));
        }
        let sources = [
            include_str!("lib.rs"),
            concat!(
                include_str!("../../ui_tree/src/tree/node/mod.rs"),
                include_str!("../../ui_tree/src/tree/node/core.rs"),
                include_str!("../../ui_tree/src/tree/node/controls.rs"),
                include_str!("../../ui_tree/src/tree/node/containers.rs"),
                include_str!("../../ui_tree/src/tree/node/overlay.rs"),
                include_str!("../../ui_tree/src/tree/node/surface.rs"),
                include_str!("../../ui_tree/src/tree/node/graph_canvas.rs")
            ),
            concat!(
                include_str!("../../ui_runtime/src/input/pointer/mod.rs"),
                include_str!("../../ui_runtime/src/input/pointer/dispatch.rs"),
                include_str!("../../ui_runtime/src/input/pointer/hover.rs"),
                include_str!("../../ui_runtime/src/input/pointer/press.rs"),
                include_str!("../../ui_runtime/src/input/pointer/scroll.rs"),
                include_str!("../../ui_runtime/src/input/pointer/scrollbar.rs"),
                include_str!("../../ui_runtime/src/input/pointer/middle_pan.rs"),
                include_str!("../../ui_runtime/src/input/pointer/graph_canvas.rs"),
                include_str!("../../ui_runtime/src/input/pointer/popup.rs"),
                include_str!("../../ui_runtime/src/input/pointer/numeric.rs"),
                include_str!("../../ui_runtime/src/input/pointer/helpers.rs")
            ),
            concat!(
                include_str!("../../ui_runtime/src/runtime/ui_runtime/mod.rs"),
                include_str!("../../ui_runtime/src/runtime/ui_runtime/entry.rs"),
                include_str!("../../ui_runtime/src/runtime/ui_runtime/focus.rs"),
                include_str!("../../ui_runtime/src/runtime/ui_runtime/graph_canvas.rs"),
                include_str!("../../ui_runtime/src/runtime/ui_runtime/helpers.rs"),
                include_str!("../../ui_runtime/src/runtime/ui_runtime/popup.rs"),
                include_str!("../../ui_runtime/src/runtime/ui_runtime/scroll_metrics.rs")
            ),
            include_str!("../../ui_runtime/src/output/build_ui_frame.rs"),
            include_str!("../../ui_render_data/src/primitives/graph_canvas.rs"),
        ];
        for forbidden in [
            format!("{}{}", "Material", "Graph"),
            format!("{}{}", "Material", "NodeCatalog"),
            format!("{}{}", "lower_", "material"),
            format!("{}{}", "material", "_graph::"),
        ] {
            for source in sources {
                assert!(
                    !source.contains(forbidden.as_str()),
                    "generic graph canvas substrate must not contain {forbidden}"
                );
            }
        }
    }
}
