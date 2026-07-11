//! Renderer-facing surface-frame data model.
//!
//! Surface frames are host-neutral snapshots that later renderer stages can
//! consume. This module owns explicit surface build inputs, a small row/column
//! layout pass, concrete computed-style delivery, and bounds hit testing.

use runenui_core::{
    Axis, ComputedStyle, EdgeInsets, Element, ElementId, ElementKind, StyleResolution, StyleTokens,
    resolve_style,
};

use crate::style_debug::{SurfaceStyleNode, SurfaceStyleReport};
use crate::{LogicalPoint, RuntimeNodeId, RuntimeTreeIndex};

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

    /// Returns the right edge.
    #[must_use]
    pub fn max_x(&self) -> f32 {
        self.x() + self.width()
    }

    /// Returns the bottom edge.
    #[must_use]
    pub fn max_y(&self) -> f32 {
        self.y() + self.height()
    }

    /// Returns whether the point is inside this rectangle.
    ///
    /// Containment is left/top inclusive and right/bottom exclusive. This makes
    /// adjacent bounds deterministic during hit testing.
    #[must_use]
    pub fn contains(&self, point: LogicalPoint) -> bool {
        (self.x()..self.max_x()).contains(&point.x())
            && (self.y()..self.max_y()).contains(&point.y())
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
    computed_style: ComputedStyle,
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
        computed_style: ComputedStyle,
    ) -> Self {
        Self {
            id,
            parent,
            authored_id,
            bounds,
            kind,
            computed_style,
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

    /// Returns the resolved logical outer bounds.
    #[must_use]
    pub const fn bounds(&self) -> LogicalRect {
        self.bounds
    }

    /// Returns the renderer-facing surface node kind.
    #[must_use]
    pub const fn kind(&self) -> &SurfaceNodeKind {
        &self.kind
    }

    /// Returns the concrete resolved style consumed by layout and renderers.
    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.computed_style
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

    /// Returns the topmost surface node containing the provided point.
    ///
    /// Nodes are checked in reverse surface order so later/deeper nodes win over
    /// parent containers whose bounds also contain the point.
    #[must_use]
    pub fn hit_test(&self, point: LogicalPoint) -> Option<&SurfaceNode> {
        self.nodes
            .iter()
            .rev()
            .find(|node| node.bounds().contains(point))
    }

    /// Returns the runtime node ID for the topmost node containing the point.
    #[must_use]
    pub fn hit_test_id(&self, point: LogicalPoint) -> Option<RuntimeNodeId> {
        self.hit_test(point).map(SurfaceNode::id)
    }
}

/// Intrinsic metrics used by the simple row/column surface layout pass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceLayoutMetrics {
    text_char_width: f32,
    text_height: f32,
    button_char_width: f32,
    button_height: f32,
    min_button_width: f32,
}

impl Default for SurfaceLayoutMetrics {
    fn default() -> Self {
        Self::new(8.0, 20.0, 8.0, 32.0, 64.0)
    }
}

impl SurfaceLayoutMetrics {
    /// Creates intrinsic metrics for the simple surface layout pass.
    #[must_use]
    pub const fn new(
        text_char_width: f32,
        text_height: f32,
        button_char_width: f32,
        button_height: f32,
        min_button_width: f32,
    ) -> Self {
        Self {
            text_char_width,
            text_height,
            button_char_width,
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

    /// Returns the minimum outer button height.
    #[must_use]
    pub const fn button_height(&self) -> f32 {
        self.button_height
    }

    /// Returns the minimum outer button width.
    #[must_use]
    pub const fn min_button_width(&self) -> f32 {
        self.min_button_width
    }
}

/// Explicit inputs used to publish one surface snapshot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceBuildContext<'a> {
    style_tokens: &'a StyleTokens,
    layout_metrics: SurfaceLayoutMetrics,
}

impl<'a> SurfaceBuildContext<'a> {
    /// Creates a build context with default placeholder layout metrics.
    #[must_use]
    pub fn new(style_tokens: &'a StyleTokens) -> Self {
        Self {
            style_tokens,
            layout_metrics: SurfaceLayoutMetrics::default(),
        }
    }

    /// Replaces the placeholder layout metrics for this publication.
    #[must_use]
    pub const fn with_layout_metrics(mut self, layout_metrics: SurfaceLayoutMetrics) -> Self {
        self.layout_metrics = layout_metrics;
        self
    }

    /// Returns the explicit style-token input.
    #[must_use]
    pub const fn style_tokens(&self) -> &'a StyleTokens {
        self.style_tokens
    }

    /// Returns the explicit layout metrics input.
    #[must_use]
    pub const fn layout_metrics(&self) -> SurfaceLayoutMetrics {
        self.layout_metrics
    }
}

/// Aligned renderer-facing and diagnostic products from one surface preparation.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePublication {
    frame: SurfaceFrame,
    style_report: SurfaceStyleReport,
}

impl SurfacePublication {
    const fn new(frame: SurfaceFrame, style_report: SurfaceStyleReport) -> Self {
        Self {
            frame,
            style_report,
        }
    }

    /// Returns the renderer-facing surface frame.
    #[must_use]
    pub const fn frame(&self) -> &SurfaceFrame {
        &self.frame
    }

    /// Returns style provenance and diagnostics aligned to the frame nodes.
    #[must_use]
    pub const fn style_report(&self) -> &SurfaceStyleReport {
        &self.style_report
    }

    /// Consumes the publication and returns its aligned products.
    #[must_use]
    pub fn into_parts(self) -> (SurfaceFrame, SurfaceStyleReport) {
        (self.frame, self.style_report)
    }
}

/// Resolves style once per node and publishes aligned frame and diagnostic products.
///
/// The row/column layout consumes concrete computed padding for intrinsic outer
/// sizing, container content origins, root child placement, and hit testing.
#[must_use]
pub fn publish_surface<Action>(
    root: &Element<Action>,
    size: LogicalSize,
    context: &SurfaceBuildContext<'_>,
) -> SurfacePublication {
    let resolved_tree = ResolvedSurfaceTree::new(root, context.style_tokens());
    let frame = layout_resolved_surface(&resolved_tree, size, context.layout_metrics());
    let style_report = build_surface_style_report(&resolved_tree);

    SurfacePublication::new(frame, style_report)
}

struct ResolvedSurfaceTree<'a, Action> {
    nodes: Vec<ResolvedSurfaceNode<'a, Action>>,
}

impl<'a, Action> ResolvedSurfaceTree<'a, Action> {
    fn new(root: &'a Element<Action>, tokens: &StyleTokens) -> Self {
        let index = RuntimeTreeIndex::new(root);
        let mut children_by_parent: Vec<Vec<RuntimeNodeId>> =
            (0..index.nodes().len()).map(|_| Vec::new()).collect();

        for node in index.nodes() {
            if let Some(parent) = node.parent()
                && let Some(children) = children_by_parent.get_mut(parent.as_usize())
            {
                children.push(node.id());
            }
        }

        let nodes = index
            .nodes()
            .iter()
            .zip(children_by_parent)
            .map(|(node, children)| ResolvedSurfaceNode {
                id: node.id(),
                parent: node.parent(),
                element: node.element(),
                children,
                resolution: resolve_style(node.element().visual_style(), tokens),
            })
            .collect();

        Self { nodes }
    }

    const fn nodes(&self) -> &[ResolvedSurfaceNode<'a, Action>] {
        self.nodes.as_slice()
    }

    fn node(&self, id: RuntimeNodeId) -> Option<&ResolvedSurfaceNode<'a, Action>> {
        self.nodes.get(id.as_usize())
    }
}

struct ResolvedSurfaceNode<'a, Action> {
    id: RuntimeNodeId,
    parent: Option<RuntimeNodeId>,
    element: &'a Element<Action>,
    children: Vec<RuntimeNodeId>,
    resolution: StyleResolution,
}

impl<Action> ResolvedSurfaceNode<'_, Action> {
    const fn id(&self) -> RuntimeNodeId {
        self.id
    }

    const fn parent(&self) -> Option<RuntimeNodeId> {
        self.parent
    }

    const fn element(&self) -> &Element<Action> {
        self.element
    }

    const fn children(&self) -> &[RuntimeNodeId] {
        self.children.as_slice()
    }

    const fn resolution(&self) -> &StyleResolution {
        &self.resolution
    }
}

fn build_surface_style_report<Action>(
    resolved_tree: &ResolvedSurfaceTree<'_, Action>,
) -> SurfaceStyleReport {
    let nodes = resolved_tree
        .nodes()
        .iter()
        .map(|node| {
            SurfaceStyleNode::new(
                node.id(),
                node.element().element_id().cloned(),
                node.resolution().clone(),
            )
        })
        .collect();

    SurfaceStyleReport::new(nodes)
}

fn layout_resolved_surface<Action>(
    resolved_tree: &ResolvedSurfaceTree<'_, Action>,
    size: LogicalSize,
    metrics: SurfaceLayoutMetrics,
) -> SurfaceFrame {
    let mut builder = SurfaceLayoutBuilder::new(metrics);

    if let Some(root) = resolved_tree.node(RuntimeNodeId::ROOT) {
        builder.push_node(
            resolved_tree,
            root,
            LogicalRect::new(LogicalPoint::new(0.0, 0.0), size),
        );
    }

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
        resolved_tree: &ResolvedSurfaceTree<'_, Action>,
        node: &ResolvedSurfaceNode<'_, Action>,
        bounds: LogicalRect,
    ) {
        self.nodes.push(SurfaceNode::new(
            node.id(),
            node.parent(),
            node.element().element_id().cloned(),
            bounds,
            surface_kind(node.element().kind()),
            node.resolution().computed_style(),
        ));

        if let ElementKind::Container(container) = node.element().kind() {
            self.push_container_children(
                resolved_tree,
                node,
                bounds,
                container.axis(),
                node.children(),
            );
        }
    }

    fn push_container_children<Action>(
        &mut self,
        resolved_tree: &ResolvedSurfaceTree<'_, Action>,
        container_node: &ResolvedSurfaceNode<'_, Action>,
        parent_bounds: LogicalRect,
        axis: Axis,
        children: &[RuntimeNodeId],
    ) {
        let gap = container_node.element().style().gap().value();
        let padding = resolved_padding(container_node);
        let mut cursor_x = parent_bounds.x() + padding.left().value();
        let mut cursor_y = parent_bounds.y() + padding.top().value();

        for child_id in children {
            let Some(child) = resolved_tree.node(*child_id) else {
                continue;
            };
            let child_size = self.measure(resolved_tree, child);
            let child_bounds =
                LogicalRect::from_xywh(cursor_x, cursor_y, child_size.width(), child_size.height());
            self.push_node(resolved_tree, child, child_bounds);

            match axis {
                Axis::Vertical => cursor_y += child_size.height() + gap,
                Axis::Horizontal => cursor_x += child_size.width() + gap,
            }
        }
    }

    fn measure<Action>(
        &self,
        resolved_tree: &ResolvedSurfaceTree<'_, Action>,
        node: &ResolvedSurfaceNode<'_, Action>,
    ) -> LogicalSize {
        let padding = resolved_padding(node);

        match node.element().kind() {
            ElementKind::Text(text) => expand_size_by_padding(
                LogicalSize::new(
                    char_count_as_f32(text.content()) * self.metrics.text_char_width(),
                    self.metrics.text_height(),
                ),
                padding,
            ),
            ElementKind::Button(button) => {
                let label_width =
                    char_count_as_f32(button.label()) * self.metrics.button_char_width();
                let desired = expand_size_by_padding(
                    LogicalSize::new(label_width, self.metrics.text_height()),
                    padding,
                );
                LogicalSize::new(
                    desired.width().max(self.metrics.min_button_width()),
                    desired.height().max(self.metrics.button_height()),
                )
            }
            ElementKind::Container(container) => expand_size_by_padding(
                self.measure_container(resolved_tree, node, container.axis(), node.children()),
                padding,
            ),
        }
    }

    fn measure_container<Action>(
        &self,
        resolved_tree: &ResolvedSurfaceTree<'_, Action>,
        node: &ResolvedSurfaceNode<'_, Action>,
        axis: Axis,
        children: &[RuntimeNodeId],
    ) -> LogicalSize {
        let gap = node.element().style().gap().value();
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        let mut child_count = 0_usize;

        for child_id in children {
            let Some(child) = resolved_tree.node(*child_id) else {
                continue;
            };
            child_count += 1;
            let child_size = self.measure(resolved_tree, child);
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

        if child_count > 1 {
            let total_gap = gap * count_as_f32(child_count - 1);
            match axis {
                Axis::Vertical => height += total_gap,
                Axis::Horizontal => width += total_gap,
            }
        }

        LogicalSize::new(width, height)
    }
}

fn resolved_padding<Action>(node: &ResolvedSurfaceNode<'_, Action>) -> EdgeInsets {
    node.resolution()
        .computed_style()
        .padding()
        .unwrap_or(EdgeInsets::ZERO)
}

fn expand_size_by_padding(size: LogicalSize, padding: EdgeInsets) -> LogicalSize {
    LogicalSize::new(
        size.width() + padding.left().value() + padding.right().value(),
        size.height() + padding.top().value() + padding.bottom().value(),
    )
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
