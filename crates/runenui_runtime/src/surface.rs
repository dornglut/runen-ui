//! Renderer-facing surface-frame data model.
//!
//! Surface frames are host-neutral snapshots that later layout and renderer
//! stages can consume. This module defines only the data vocabulary; it does
//! not compute layout, perform hit testing, or render pixels.

use runenui_core::ElementId;

use crate::{LogicalPoint, RuntimeNodeId};

/// Logical size in UI coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalSize {
    width: f32,
    height: f32,
}

impl LogicalSize {
    /// Creates a logical size.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Returns the horizontal extent.
    #[must_use]
    pub const fn width(&self) -> f32 {
        self.width
    }

    /// Returns the vertical extent.
    #[must_use]
    pub const fn height(&self) -> f32 {
        self.height
    }
}

/// Logical rectangle in UI coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalRect {
    origin: LogicalPoint,
    size: LogicalSize,
}

impl LogicalRect {
    /// Creates a logical rectangle from an origin and size.
    #[must_use]
    pub const fn new(origin: LogicalPoint, size: LogicalSize) -> Self {
        Self { origin, size }
    }

    /// Creates a logical rectangle from scalar components.
    #[must_use]
    pub const fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self::new(LogicalPoint::new(x, y), LogicalSize::new(width, height))
    }

    /// Returns the top-left origin.
    #[must_use]
    pub const fn origin(&self) -> LogicalPoint {
        self.origin
    }

    /// Returns the rectangle size.
    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    /// Returns the left edge.
    #[must_use]
    pub const fn x(&self) -> f32 {
        self.origin.x()
    }

    /// Returns the top edge.
    #[must_use]
    pub const fn y(&self) -> f32 {
        self.origin.y()
    }

    /// Returns the rectangle width.
    #[must_use]
    pub const fn width(&self) -> f32 {
        self.size.width()
    }

    /// Returns the rectangle height.
    #[must_use]
    pub const fn height(&self) -> f32 {
        self.size.height()
    }
}

/// Renderer-facing node kind carried by a surface frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceNodeKind {
    /// Non-visual grouping node.
    Container,
    /// Text node with display content.
    Text { content: String },
    /// Button control with display label and enabled state.
    Button { label: String, enabled: bool },
}

impl SurfaceNodeKind {
    /// Creates a container surface node kind.
    #[must_use]
    pub const fn container() -> Self {
        Self::Container
    }

    /// Creates a text surface node kind.
    #[must_use]
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
        }
    }

    /// Creates a button surface node kind.
    #[must_use]
    pub fn button(label: impl Into<String>, enabled: bool) -> Self {
        Self::Button {
            label: label.into(),
            enabled,
        }
    }
}

/// One ordered node in a renderer-facing surface frame.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceNode {
    id: RuntimeNodeId,
    parent: Option<RuntimeNodeId>,
    authored_id: Option<ElementId>,
    bounds: LogicalRect,
    kind: SurfaceNodeKind,
}

impl SurfaceNode {
    /// Creates a surface node.
    #[must_use]
    pub const fn new(
        id: RuntimeNodeId,
        parent: Option<RuntimeNodeId>,
        authored_id: Option<ElementId>,
        bounds: LogicalRect,
        kind: SurfaceNodeKind,
    ) -> Self {
        Self {
            id,
            parent,
            authored_id,
            bounds,
            kind,
        }
    }

    /// Returns the generated runtime node ID.
    #[must_use]
    pub const fn id(&self) -> RuntimeNodeId {
        self.id
    }

    /// Returns the generated runtime parent ID, if present.
    #[must_use]
    pub const fn parent(&self) -> Option<RuntimeNodeId> {
        self.parent
    }

    /// Returns the optional authored element ID.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }

    /// Returns the resolved logical bounds.
    #[must_use]
    pub const fn bounds(&self) -> LogicalRect {
        self.bounds
    }

    /// Returns the renderer-facing surface node kind.
    #[must_use]
    pub const fn kind(&self) -> &SurfaceNodeKind {
        &self.kind
    }
}

/// Renderer-facing surface snapshot for one UI tree.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceFrame {
    size: LogicalSize,
    nodes: Vec<SurfaceNode>,
}

impl SurfaceFrame {
    /// Creates a surface frame from a frame size and ordered nodes.
    #[must_use]
    pub const fn new(size: LogicalSize, nodes: Vec<SurfaceNode>) -> Self {
        Self { size, nodes }
    }

    /// Creates an empty surface frame with no nodes.
    #[must_use]
    pub const fn empty(size: LogicalSize) -> Self {
        Self {
            size,
            nodes: Vec::new(),
        }
    }

    /// Returns the logical frame size.
    #[must_use]
    pub const fn size(&self) -> LogicalSize {
        self.size
    }

    /// Returns the ordered surface nodes.
    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceNode] {
        self.nodes.as_slice()
    }

    /// Returns whether this frame contains no surface nodes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the surface node for the provided runtime node ID.
    #[must_use]
    pub fn node(&self, id: RuntimeNodeId) -> Option<&SurfaceNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    /// Returns the root surface node, when present.
    #[must_use]
    pub fn root(&self) -> Option<&SurfaceNode> {
        self.node(RuntimeNodeId::ROOT)
    }
}
