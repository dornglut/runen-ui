//! Renderer-facing surface-frame data model.
//!
//! Surface frames are host-neutral snapshots that later layout and renderer
//! stages can consume. This module defines the surface vocabulary and a small
//! row/column layout pass. It does not perform hit testing or render pixels.

use runenui_core::{Axis, Element, ElementId, ElementKind};

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

/// Intrinsic metrics used by the simple row/column surface layout pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceLayoutMetrics {
    text_char_width: f32,
    text_height: f32,
    button_char_width: f32,
    button_horizontal_padding: f32,
    button_height: f32,
    min_button_width: f32,
}

impl Default for SurfaceLayoutMetrics {
    fn default() -> Self {
        Self::new(8.0, 20.0, 8.0, 12.0, 32.0, 64.0)
    }
}

impl SurfaceLayoutMetrics {
    /// Creates intrinsic metrics for the simple surface layout pass.
    #[must_use]
    pub const fn new(
        text_char_width: f32,
        text_height: f32,
        button_char_width: f32,
        button_horizontal_padding: f32,
        button_height: f32,
        min_button_width: f32,
    ) -> Self {
        Self {
            text_char_width,
            text_height,
            button_char_width,
            button_horizontal_padding,
            button_height,
            min_button_width,
        }
    }

    /// Returns the approximate width used for one text character.
    #[must_use]
    pub const fn text_char_width(&self) -> f32 {
        self.text_char_width
    }

    /// Returns the fixed intrinsic text height.
    #[must_use]
    pub const fn text_height(&self) -> f32 {
        self.text_height
    }

    /// Returns the approximate width used for one button label character.
    #[must_use]
    pub const fn button_char_width(&self) -> f32 {
        self.button_char_width
    }

    /// Returns the horizontal padding applied to each side of a button.
    #[must_use]
    pub const fn button_horizontal_padding(&self) -> f32 {
        self.button_horizontal_padding
    }

    /// Returns the fixed intrinsic button height.
    #[must_use]
    pub const fn button_height(&self) -> f32 {
        self.button_height
    }

    /// Returns the minimum intrinsic button width.
    #[must_use]
    pub const fn min_button_width(&self) -> f32 {
        self.min_button_width
    }
}

/// Lays out an element tree into a surface frame using default intrinsic metrics.
///
/// The root element receives the provided frame size. Row and column containers
/// stack children by axis using their authored gap. Text and button bounds are
/// intrinsic placeholders until the text/layout systems become real.
#[must_use]
pub fn layout_surface<Action>(root: &Element<Action>, size: LogicalSize) -> SurfaceFrame {
    layout_surface_with_metrics(root, size, SurfaceLayoutMetrics::default())
}

/// Lays out an element tree into a surface frame using explicit intrinsic metrics.
#[must_use]
pub fn layout_surface_with_metrics<Action>(
    root: &Element<Action>,
    size: LogicalSize,
    metrics: SurfaceLayoutMetrics,
) -> SurfaceFrame {
    let mut builder = SurfaceLayoutBuilder::new(metrics);
    builder.push_node(
        None,
        root,
        LogicalRect::new(LogicalPoint::new(0.0, 0.0), size),
    );
    SurfaceFrame::new(size, builder.into_nodes())
}

struct SurfaceLayoutBuilder {
    metrics: SurfaceLayoutMetrics,
    nodes: Vec<SurfaceNode>,
}

impl SurfaceLayoutBuilder {
    const fn new(metrics: SurfaceLayoutMetrics) -> Self {
        Self {
            metrics,
            nodes: Vec::new(),
        }
    }

    fn into_nodes(self) -> Vec<SurfaceNode> {
        self.nodes
    }

    fn push_node<Action>(
        &mut self,
        parent: Option<RuntimeNodeId>,
        element: &Element<Action>,
        bounds: LogicalRect,
    ) -> RuntimeNodeId {
        let id = RuntimeNodeId::from_index(self.nodes.len());
        self.nodes.push(SurfaceNode::new(
            id,
            parent,
            element.element_id().cloned(),
            bounds,
            surface_kind(element.kind()),
        ));

        if let ElementKind::Container(container) = element.kind() {
            self.push_container_children(
                id,
                bounds,
                container.axis(),
                container.children(),
                element,
            );
        }

        id
    }

    fn push_container_children<Action>(
        &mut self,
        parent: RuntimeNodeId,
        parent_bounds: LogicalRect,
        axis: Axis,
        children: &[Element<Action>],
        container_element: &Element<Action>,
    ) {
        let gap = container_element.style().gap().value();
        let mut cursor_x = parent_bounds.x();
        let mut cursor_y = parent_bounds.y();

        for child in children {
            let child_size = self.measure(child);
            let child_bounds =
                LogicalRect::from_xywh(cursor_x, cursor_y, child_size.width(), child_size.height());
            self.push_node(Some(parent), child, child_bounds);

            match axis {
                Axis::Vertical => cursor_y += child_size.height() + gap,
                Axis::Horizontal => cursor_x += child_size.width() + gap,
            }
        }
    }

    fn measure<Action>(&self, element: &Element<Action>) -> LogicalSize {
        match element.kind() {
            ElementKind::Text(text) => LogicalSize::new(
                char_count_as_f32(text.content()) * self.metrics.text_char_width(),
                self.metrics.text_height(),
            ),
            ElementKind::Button(button) => {
                let label_width =
                    char_count_as_f32(button.label()) * self.metrics.button_char_width();
                let padded_width = self
                    .metrics
                    .button_horizontal_padding()
                    .mul_add(2.0, label_width);
                LogicalSize::new(
                    padded_width.max(self.metrics.min_button_width()),
                    self.metrics.button_height(),
                )
            }
            ElementKind::Container(container) => {
                self.measure_container(element, container.axis(), container.children())
            }
        }
    }

    fn measure_container<Action>(
        &self,
        element: &Element<Action>,
        axis: Axis,
        children: &[Element<Action>],
    ) -> LogicalSize {
        let gap = element.style().gap().value();
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;

        for child in children {
            let child_size = self.measure(child);
            match axis {
                Axis::Vertical => {
                    width = width.max(child_size.width());
                    height += child_size.height();
                }
                Axis::Horizontal => {
                    width += child_size.width();
                    height = height.max(child_size.height());
                }
            }
        }

        if children.len() > 1 {
            let total_gap = gap * count_as_f32(children.len() - 1);
            match axis {
                Axis::Vertical => height += total_gap,
                Axis::Horizontal => width += total_gap,
            }
        }

        LogicalSize::new(width, height)
    }
}

fn char_count_as_f32(value: &str) -> f32 {
    count_as_f32(value.chars().count())
}

fn count_as_f32(count: usize) -> f32 {
    f32::from(u16::try_from(count).unwrap_or(u16::MAX))
}

fn surface_kind<Action>(kind: &ElementKind<Action>) -> SurfaceNodeKind {
    match kind {
        ElementKind::Container(_) => SurfaceNodeKind::container(),
        ElementKind::Text(text) => SurfaceNodeKind::text(text.content()),
        ElementKind::Button(button) => SurfaceNodeKind::button(button.label(), button.enabled()),
    }
}
