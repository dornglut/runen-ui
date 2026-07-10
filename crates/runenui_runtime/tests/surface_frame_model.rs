use runenui_core::ElementId;
use runenui_runtime::prelude::{
    LogicalPoint, LogicalRect, LogicalSize, RuntimeNodeId, SurfaceFrame, SurfaceNode,
    SurfaceNodeKind,
};

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON,
        "expected {expected}, got {actual}",
    );
}

#[test]
fn logical_size_exposes_dimensions() {
    let size = LogicalSize::new(320.0, 240.0);

    assert_f32_eq(size.width(), 320.0);
    assert_f32_eq(size.height(), 240.0);
}

#[test]
fn logical_rect_exposes_origin_size_and_edges() {
    let rect = LogicalRect::from_xywh(8.0, 16.0, 128.0, 32.0);

    assert_eq!(rect.origin(), LogicalPoint::new(8.0, 16.0));
    assert_eq!(rect.size(), LogicalSize::new(128.0, 32.0));
    assert_f32_eq(rect.x(), 8.0);
    assert_f32_eq(rect.y(), 16.0);
    assert_f32_eq(rect.width(), 128.0);
    assert_f32_eq(rect.height(), 32.0);
}

#[test]
fn surface_node_kind_constructors_create_owned_semantic_kinds() {
    assert_eq!(SurfaceNodeKind::container(), SurfaceNodeKind::Container);
    assert_eq!(
        SurfaceNodeKind::text("Counter"),
        SurfaceNodeKind::Text {
            content: "Counter".to_string(),
        }
    );
    assert_eq!(
        SurfaceNodeKind::button("Increment", true),
        SurfaceNodeKind::Button {
            label: "Increment".to_string(),
            enabled: true,
        }
    );
}

#[test]
fn surface_node_carries_runtime_identity_authored_identity_bounds_and_kind() {
    let node = SurfaceNode::new(
        RuntimeNodeId::from_index(2),
        Some(RuntimeNodeId::ROOT),
        Some(ElementId::new("counter.increment")),
        LogicalRect::from_xywh(10.0, 20.0, 80.0, 24.0),
        SurfaceNodeKind::button("+", true),
    );

    assert_eq!(node.id(), RuntimeNodeId::from_index(2));
    assert_eq!(node.parent(), Some(RuntimeNodeId::ROOT));
    assert_eq!(
        node.authored_id().map(ElementId::as_str),
        Some("counter.increment")
    );
    assert_eq!(
        node.bounds(),
        LogicalRect::from_xywh(10.0, 20.0, 80.0, 24.0)
    );
    assert_eq!(
        node.kind(),
        &SurfaceNodeKind::Button {
            label: "+".to_string(),
            enabled: true,
        }
    );
}

#[test]
fn surface_frame_exposes_size_ordered_nodes_root_and_lookup() {
    let root = SurfaceNode::new(
        RuntimeNodeId::ROOT,
        None,
        Some(ElementId::new("counter.root")),
        LogicalRect::from_xywh(0.0, 0.0, 320.0, 240.0),
        SurfaceNodeKind::container(),
    );
    let label = SurfaceNode::new(
        RuntimeNodeId::from_index(1),
        Some(RuntimeNodeId::ROOT),
        Some(ElementId::new("counter.value")),
        LogicalRect::from_xywh(8.0, 8.0, 120.0, 24.0),
        SurfaceNodeKind::text("0"),
    );
    let button = SurfaceNode::new(
        RuntimeNodeId::from_index(2),
        Some(RuntimeNodeId::ROOT),
        Some(ElementId::new("counter.increment")),
        LogicalRect::from_xywh(8.0, 40.0, 80.0, 24.0),
        SurfaceNodeKind::button("+", true),
    );
    let frame = SurfaceFrame::new(
        LogicalSize::new(320.0, 240.0),
        vec![root.clone(), label.clone(), button.clone()],
    );

    assert_eq!(frame.size(), LogicalSize::new(320.0, 240.0));
    assert_eq!(frame.nodes(), &[root.clone(), label, button.clone()]);
    assert_eq!(frame.root(), Some(&root));
    assert_eq!(frame.node(RuntimeNodeId::from_index(2)), Some(&button));
    assert_eq!(frame.node(RuntimeNodeId::from_index(99)), None);
    assert!(!frame.is_empty());
}

#[test]
fn empty_surface_frame_contains_no_nodes() {
    let frame = SurfaceFrame::empty(LogicalSize::new(640.0, 480.0));

    assert_eq!(frame.size(), LogicalSize::new(640.0, 480.0));
    assert!(frame.nodes().is_empty());
    assert!(frame.is_empty());
    assert_eq!(frame.root(), None);
}
