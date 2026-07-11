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
use crate::{
    AxisConstraints, AxisLimit, DeterministicMeasurementProvider, LayoutConstraints, LogicalPoint,
    MeasurementProvider, RuntimeNodeId, RuntimeTreeIndex, TextMeasurementKind,
    TextMeasurementRequest,
};

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

/// Per-axis overflow pressure recorded during surface layout.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutOverflow {
    width: bool,
    height: bool,
}

impl LayoutOverflow {
    const fn new(width: bool, height: bool) -> Self {
        Self { width, height }
    }

    /// Returns whether horizontal layout pressure exceeded a finite maximum.
    #[must_use]
    pub const fn width(&self) -> bool {
        self.width
    }

    /// Returns whether vertical layout pressure exceeded a finite maximum.
    #[must_use]
    pub const fn height(&self) -> bool {
        self.height
    }

    /// Returns whether either axis overflowed.
    #[must_use]
    pub const fn any(&self) -> bool {
        self.width || self.height
    }
}

/// One runtime-node-aligned diagnostic result from surface measurement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceLayoutNode {
    id: RuntimeNodeId,
    outer_constraints: LayoutConstraints,
    content_constraints: LayoutConstraints,
    desired_content_size: LogicalSize,
    desired_outer_size: LogicalSize,
    constrained_outer_size: LogicalSize,
    overflow: LayoutOverflow,
}

impl SurfaceLayoutNode {
    const fn new(
        id: RuntimeNodeId,
        outer_constraints: LayoutConstraints,
        content_constraints: LayoutConstraints,
        desired_content_size: LogicalSize,
        desired_outer_size: LogicalSize,
        constrained_outer_size: LogicalSize,
        overflow: LayoutOverflow,
    ) -> Self {
        Self {
            id,
            outer_constraints,
            content_constraints,
            desired_content_size,
            desired_outer_size,
            constrained_outer_size,
            overflow,
        }
    }

    /// Returns the generated runtime node ID.
    #[must_use]
    pub const fn id(&self) -> RuntimeNodeId {
        self.id
    }

    /// Returns the outer constraints supplied to this node.
    #[must_use]
    pub const fn outer_constraints(&self) -> LayoutConstraints {
        self.outer_constraints
    }

    /// Returns this node's padding-adjusted content-box constraints.
    #[must_use]
    pub const fn content_constraints(&self) -> LayoutConstraints {
        self.content_constraints
    }

    /// Returns the sanitized content size desired before content constraints.
    #[must_use]
    pub const fn desired_content_size(&self) -> LogicalSize {
        self.desired_content_size
    }

    /// Returns the desired outer size after content constraints and box policy.
    #[must_use]
    pub const fn desired_outer_size(&self) -> LogicalSize {
        self.desired_outer_size
    }

    /// Returns the final outer size used by arrangement.
    #[must_use]
    pub const fn constrained_outer_size(&self) -> LogicalSize {
        self.constrained_outer_size
    }

    /// Returns deterministic overflow pressure for this node.
    #[must_use]
    pub const fn overflow(&self) -> LayoutOverflow {
        self.overflow
    }
}

/// Runtime-node-aligned layout diagnostics from one surface publication.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SurfaceLayoutReport {
    nodes: Vec<SurfaceLayoutNode>,
}

impl SurfaceLayoutReport {
    const fn new(nodes: Vec<SurfaceLayoutNode>) -> Self {
        Self { nodes }
    }

    /// Returns measured nodes in the same order as the surface frame.
    #[must_use]
    pub const fn nodes(&self) -> &[SurfaceLayoutNode] {
        self.nodes.as_slice()
    }

    /// Returns whether this report contains no layout nodes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the layout node for the provided runtime node ID.
    #[must_use]
    pub fn node(&self, id: RuntimeNodeId) -> Option<&SurfaceLayoutNode> {
        self.nodes.iter().find(|node| node.id() == id)
    }

    /// Returns the root layout node, when present.
    #[must_use]
    pub fn root(&self) -> Option<&SurfaceLayoutNode> {
        self.node(RuntimeNodeId::ROOT)
    }
}

static DEFAULT_MEASUREMENT_PROVIDER: DeterministicMeasurementProvider =
    DeterministicMeasurementProvider::DEFAULT;

/// Explicit inputs used to publish one surface snapshot.
#[derive(Clone, Copy)]
pub struct SurfaceBuildContext<'a> {
    style_tokens: &'a StyleTokens,
    root_constraints: LayoutConstraints,
    measurement_provider: &'a dyn MeasurementProvider,
}

impl<'a> SurfaceBuildContext<'a> {
    /// Creates a build context with explicit root constraints and the
    /// deterministic headless measurement provider.
    #[must_use]
    pub fn new(style_tokens: &'a StyleTokens, root_constraints: LayoutConstraints) -> Self {
        Self {
            style_tokens,
            root_constraints,
            measurement_provider: &DEFAULT_MEASUREMENT_PROVIDER,
        }
    }

    /// Creates a build context with tight root constraints.
    #[must_use]
    pub fn tight(style_tokens: &'a StyleTokens, size: LogicalSize) -> Self {
        Self::new(style_tokens, LayoutConstraints::tight(size))
    }

    /// Replaces the root constraints for this publication.
    #[must_use]
    pub const fn with_root_constraints(mut self, root_constraints: LayoutConstraints) -> Self {
        self.root_constraints = root_constraints;
        self
    }

    /// Replaces the measurement provider for this publication.
    #[must_use]
    pub fn with_measurement_provider(
        mut self,
        measurement_provider: &'a dyn MeasurementProvider,
    ) -> Self {
        self.measurement_provider = measurement_provider;
        self
    }

    /// Returns the explicit style-token input.
    #[must_use]
    pub const fn style_tokens(&self) -> &'a StyleTokens {
        self.style_tokens
    }

    /// Returns the explicit root layout constraints.
    #[must_use]
    pub const fn root_constraints(&self) -> LayoutConstraints {
        self.root_constraints
    }

    /// Returns the borrowed measurement provider for this publication.
    #[must_use]
    pub const fn measurement_provider(&self) -> &'a dyn MeasurementProvider {
        self.measurement_provider
    }
}

/// Aligned renderer-facing and diagnostic products from one surface preparation.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePublication {
    frame: SurfaceFrame,
    style_report: SurfaceStyleReport,
    layout_report: SurfaceLayoutReport,
}

impl SurfacePublication {
    const fn new(
        frame: SurfaceFrame,
        style_report: SurfaceStyleReport,
        layout_report: SurfaceLayoutReport,
    ) -> Self {
        Self {
            frame,
            style_report,
            layout_report,
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

    /// Returns layout diagnostics aligned to the frame nodes.
    #[must_use]
    pub const fn layout_report(&self) -> &SurfaceLayoutReport {
        &self.layout_report
    }

    /// Consumes the publication and returns its aligned products.
    #[must_use]
    pub fn into_parts(self) -> (SurfaceFrame, SurfaceStyleReport, SurfaceLayoutReport) {
        (self.frame, self.style_report, self.layout_report)
    }
}

/// Resolves style once per node and publishes aligned frame and diagnostic products.
///
/// The row/column layout consumes concrete computed padding for intrinsic outer
/// sizing, container content origins, root child placement, and hit testing.
#[must_use]
pub fn publish_surface<Action>(
    root: &Element<Action>,
    context: &SurfaceBuildContext<'_>,
) -> SurfacePublication {
    let resolved_tree = ResolvedSurfaceTree::new(root, context.style_tokens());
    let (frame, layout_report) = layout_resolved_surface(
        &resolved_tree,
        context.root_constraints(),
        context.measurement_provider(),
    );
    let style_report = build_surface_style_report(&resolved_tree);

    SurfacePublication::new(frame, style_report, layout_report)
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
    root_constraints: LayoutConstraints,
    measurement_provider: &dyn MeasurementProvider,
) -> (SurfaceFrame, SurfaceLayoutReport) {
    let mut measured_layout = MeasuredSurfaceLayout::new(resolved_tree.nodes().len());
    let mut frame_size = root_constraints.constrain(LogicalSize::new(0.0, 0.0));

    if let Some(root) = resolved_tree.node(RuntimeNodeId::ROOT) {
        let measurer = SurfaceMeasurer::new(measurement_provider);
        if let Some(size) =
            measurer.measure_node(resolved_tree, &mut measured_layout, root, root_constraints)
        {
            frame_size = size;
        }
    }

    let frame_nodes = {
        let mut arranger = SurfaceArrangementBuilder::new(&measured_layout);
        if let Some(root) = resolved_tree.node(RuntimeNodeId::ROOT) {
            arranger.push_node(resolved_tree, root, LogicalPoint::new(0.0, 0.0));
        }
        arranger.into_nodes()
    };
    let frame = SurfaceFrame::new(frame_size, frame_nodes);

    (frame, measured_layout.into_report())
}

struct MeasuredSurfaceLayout {
    nodes: Vec<Option<SurfaceLayoutNode>>,
}

impl MeasuredSurfaceLayout {
    fn new(node_count: usize) -> Self {
        Self {
            nodes: vec![None; node_count],
        }
    }

    fn node(&self, id: RuntimeNodeId) -> Option<&SurfaceLayoutNode> {
        self.nodes.get(id.as_usize()).and_then(Option::as_ref)
    }

    fn record(&mut self, node: SurfaceLayoutNode) -> Option<LogicalSize> {
        let slot = self.nodes.get_mut(node.id().as_usize())?;
        let result = slot.get_or_insert(node);
        Some(result.constrained_outer_size())
    }

    fn into_report(self) -> SurfaceLayoutReport {
        SurfaceLayoutReport::new(self.nodes.into_iter().flatten().collect())
    }
}

struct SurfaceMeasurer<'a> {
    measurement_provider: &'a dyn MeasurementProvider,
    button_policy: ButtonLayoutPolicy,
}

impl<'a> SurfaceMeasurer<'a> {
    fn new(measurement_provider: &'a dyn MeasurementProvider) -> Self {
        Self {
            measurement_provider,
            button_policy: ButtonLayoutPolicy::default(),
        }
    }

    fn measure_node<Action>(
        &self,
        resolved_tree: &ResolvedSurfaceTree<'_, Action>,
        measured_layout: &mut MeasuredSurfaceLayout,
        node: &ResolvedSurfaceNode<'_, Action>,
        outer_constraints: LayoutConstraints,
    ) -> Option<LogicalSize> {
        if let Some(measured) = measured_layout.node(node.id()) {
            return Some(measured.constrained_outer_size());
        }

        let padding = resolved_padding(node);
        let content_constraints = content_constraints(outer_constraints, padding);

        let (desired_content_size, desired_outer_size) = match node.element().kind() {
            ElementKind::Text(text) => {
                let desired_content_size = self.measure_text_content(
                    node,
                    text.content(),
                    TextMeasurementKind::Text,
                    content_constraints,
                );
                let constrained_content_size = content_constraints.constrain(desired_content_size);
                (
                    desired_content_size,
                    expand_size_by_padding(constrained_content_size, padding),
                )
            }
            ElementKind::Button(button) => {
                let desired_content_size = self.measure_text_content(
                    node,
                    button.label(),
                    TextMeasurementKind::ButtonLabel,
                    content_constraints,
                );
                let constrained_content_size = content_constraints.constrain(desired_content_size);
                let padded_outer = expand_size_by_padding(constrained_content_size, padding);
                (
                    desired_content_size,
                    self.button_policy.apply_minimum(padded_outer),
                )
            }
            ElementKind::Container(container) => {
                let desired_content_size = self.measure_container_content(
                    resolved_tree,
                    measured_layout,
                    node,
                    container.axis(),
                    content_constraints,
                );
                let constrained_content_size = content_constraints.constrain(desired_content_size);
                (
                    desired_content_size,
                    expand_size_by_padding(constrained_content_size, padding),
                )
            }
        };
        let constrained_outer_size = outer_constraints.constrain(desired_outer_size);
        let overflow = layout_overflow(
            desired_content_size,
            content_constraints,
            desired_outer_size,
            outer_constraints,
        );
        let measured = SurfaceLayoutNode::new(
            node.id(),
            outer_constraints,
            content_constraints,
            desired_content_size,
            desired_outer_size,
            constrained_outer_size,
            overflow,
        );

        measured_layout.record(measured)
    }

    fn measure_text_content<Action>(
        &self,
        node: &ResolvedSurfaceNode<'_, Action>,
        content: &str,
        kind: TextMeasurementKind,
        content_constraints: LayoutConstraints,
    ) -> LogicalSize {
        let request =
            TextMeasurementRequest::new(content, content_constraints, kind).with_node_id(node.id());
        sanitize_size(self.measurement_provider.measure_text(&request).size())
    }

    fn measure_container_content<Action>(
        &self,
        resolved_tree: &ResolvedSurfaceTree<'_, Action>,
        measured_layout: &mut MeasuredSurfaceLayout,
        node: &ResolvedSurfaceNode<'_, Action>,
        axis: Axis,
        content_constraints: LayoutConstraints,
    ) -> LogicalSize {
        let child_constraints = child_constraints(axis, content_constraints);
        let gap = valid_extent(node.element().style().gap().value());
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        let mut measured_child_count = 0_usize;

        for child_id in node.children() {
            let Some(child) = resolved_tree.node(*child_id) else {
                continue;
            };
            let Some(child_size) =
                self.measure_node(resolved_tree, measured_layout, child, child_constraints)
            else {
                continue;
            };

            if measured_child_count > 0 {
                match axis {
                    Axis::Vertical => height = finite_sum(height, gap),
                    Axis::Horizontal => width = finite_sum(width, gap),
                }
            }
            match axis {
                Axis::Vertical => {
                    width = width.max(child_size.width());
                    height = finite_sum(height, child_size.height());
                }
                Axis::Horizontal => {
                    width = finite_sum(width, child_size.width());
                    height = height.max(child_size.height());
                }
            }
            measured_child_count += 1;
        }

        LogicalSize::new(width, height)
    }
}

struct SurfaceArrangementBuilder<'a> {
    measured_layout: &'a MeasuredSurfaceLayout,
    nodes: Vec<SurfaceNode>,
}

impl<'a> SurfaceArrangementBuilder<'a> {
    const fn new(measured_layout: &'a MeasuredSurfaceLayout) -> Self {
        Self {
            measured_layout,
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
        origin: LogicalPoint,
    ) {
        let Some(measured) = self.measured_layout.node(node.id()) else {
            return;
        };
        let bounds = LogicalRect::new(origin, measured.constrained_outer_size());
        self.nodes.push(SurfaceNode::new(
            node.id(),
            node.parent(),
            node.element().element_id().cloned(),
            bounds,
            surface_kind(node.element().kind()),
            node.resolution().computed_style(),
        ));

        if let ElementKind::Container(container) = node.element().kind() {
            self.push_container_children(resolved_tree, node, bounds, container.axis());
        }
    }

    fn push_container_children<Action>(
        &mut self,
        resolved_tree: &ResolvedSurfaceTree<'_, Action>,
        container_node: &ResolvedSurfaceNode<'_, Action>,
        parent_bounds: LogicalRect,
        axis: Axis,
    ) {
        let gap = valid_extent(container_node.element().style().gap().value());
        let padding = resolved_padding(container_node);
        let mut cursor_x = finite_sum(parent_bounds.x(), valid_extent(padding.left().value()));
        let mut cursor_y = finite_sum(parent_bounds.y(), valid_extent(padding.top().value()));
        let mut arranged_child_count = 0_usize;

        for child_id in container_node.children() {
            let Some(child) = resolved_tree.node(*child_id) else {
                continue;
            };
            let Some(measured_child) = self.measured_layout.node(*child_id) else {
                continue;
            };
            let child_size = measured_child.constrained_outer_size();

            if arranged_child_count > 0 {
                match axis {
                    Axis::Vertical => cursor_y = finite_sum(cursor_y, gap),
                    Axis::Horizontal => cursor_x = finite_sum(cursor_x, gap),
                }
            }

            self.push_node(resolved_tree, child, LogicalPoint::new(cursor_x, cursor_y));

            match axis {
                Axis::Vertical => cursor_y = finite_sum(cursor_y, child_size.height()),
                Axis::Horizontal => cursor_x = finite_sum(cursor_x, child_size.width()),
            }
            arranged_child_count += 1;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonLayoutPolicy {
    min_width: f32,
    min_height: f32,
}

impl Default for ButtonLayoutPolicy {
    fn default() -> Self {
        Self {
            min_width: 64.0,
            min_height: 32.0,
        }
    }
}

impl ButtonLayoutPolicy {
    const fn apply_minimum(self, size: LogicalSize) -> LogicalSize {
        LogicalSize::new(
            max_extent(size.width(), self.min_width),
            max_extent(size.height(), self.min_height),
        )
    }
}

const fn max_extent(left: f32, right: f32) -> f32 {
    if left > right { left } else { right }
}

fn resolved_padding<Action>(node: &ResolvedSurfaceNode<'_, Action>) -> EdgeInsets {
    node.resolution()
        .computed_style()
        .padding()
        .unwrap_or(EdgeInsets::ZERO)
}

fn content_constraints(
    outer_constraints: LayoutConstraints,
    padding: EdgeInsets,
) -> LayoutConstraints {
    LayoutConstraints::new(
        content_axis_constraints(outer_constraints.horizontal(), horizontal_padding(padding)),
        content_axis_constraints(outer_constraints.vertical(), vertical_padding(padding)),
    )
}

fn child_constraints(axis: Axis, content_constraints: LayoutConstraints) -> LayoutConstraints {
    match axis {
        Axis::Vertical => LayoutConstraints::new(
            loose_axis(content_constraints.horizontal()),
            AxisConstraints::unbounded(),
        ),
        Axis::Horizontal => LayoutConstraints::new(
            AxisConstraints::unbounded(),
            loose_axis(content_constraints.vertical()),
        ),
    }
}

fn loose_axis(axis: AxisConstraints) -> AxisConstraints {
    match axis.max() {
        AxisLimit::Finite(max) => AxisConstraints::loose(max),
        AxisLimit::Unbounded => AxisConstraints::unbounded(),
    }
}

fn content_axis_constraints(axis: AxisConstraints, padding: f32) -> AxisConstraints {
    let max = match axis.max() {
        AxisLimit::Finite(max) => AxisLimit::Finite(subtract_extent(max, padding)),
        AxisLimit::Unbounded => AxisLimit::Unbounded,
    };

    AxisConstraints::new(subtract_extent(axis.min(), padding), max)
}

fn expand_size_by_padding(size: LogicalSize, padding: EdgeInsets) -> LogicalSize {
    LogicalSize::new(
        finite_sum(size.width(), horizontal_padding(padding)),
        finite_sum(size.height(), vertical_padding(padding)),
    )
}

fn sanitize_size(size: LogicalSize) -> LogicalSize {
    LogicalSize::new(valid_extent(size.width()), valid_extent(size.height()))
}

fn layout_overflow(
    desired_content_size: LogicalSize,
    content_constraints: LayoutConstraints,
    desired_outer_size: LogicalSize,
    outer_constraints: LayoutConstraints,
) -> LayoutOverflow {
    LayoutOverflow::new(
        axis_overflow(
            desired_content_size.width(),
            content_constraints.horizontal(),
            desired_outer_size.width(),
            outer_constraints.horizontal(),
        ),
        axis_overflow(
            desired_content_size.height(),
            content_constraints.vertical(),
            desired_outer_size.height(),
            outer_constraints.vertical(),
        ),
    )
}

fn axis_overflow(
    desired_content: f32,
    content_constraints: AxisConstraints,
    desired_outer: f32,
    outer_constraints: AxisConstraints,
) -> bool {
    exceeds_finite_max(desired_content, content_constraints.max())
        || exceeds_finite_max(desired_outer, outer_constraints.max())
}

fn exceeds_finite_max(desired: f32, maximum: AxisLimit) -> bool {
    matches!(maximum, AxisLimit::Finite(max) if desired > max)
}

fn horizontal_padding(padding: EdgeInsets) -> f32 {
    finite_sum(
        valid_extent(padding.left().value()),
        valid_extent(padding.right().value()),
    )
}

fn vertical_padding(padding: EdgeInsets) -> f32 {
    finite_sum(
        valid_extent(padding.top().value()),
        valid_extent(padding.bottom().value()),
    )
}

fn subtract_extent(value: f32, amount: f32) -> f32 {
    valid_extent(value - amount)
}

fn finite_sum(left: f32, right: f32) -> f32 {
    let sum = valid_extent(left) + valid_extent(right);
    if sum.is_finite() { sum } else { f32::MAX }
}

fn valid_extent(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

fn surface_kind<Action>(kind: &ElementKind<Action>) -> SurfaceNodeKind {
    match kind {
        ElementKind::Container(_) => SurfaceNodeKind::container(),
        ElementKind::Text(text) => SurfaceNodeKind::text(text.content()),
        ElementKind::Button(button) => SurfaceNodeKind::button(button.label(), button.enabled()),
    }
}
