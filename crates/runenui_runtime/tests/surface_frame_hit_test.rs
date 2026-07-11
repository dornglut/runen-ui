use runenui_core::prelude::{ComputedStyle, StyleTokens, button, column, row, text};
use runenui_runtime::prelude::{
    LogicalPoint, LogicalRect, LogicalSize, RuntimeNodeId, SurfaceBuildContext, SurfaceFrame,
    SurfaceNode, SurfaceNodeKind, publish_surface,
};

fn surface_frame<Action>(root: &runenui_core::Element<Action>, size: LogicalSize) -> SurfaceFrame {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens);
    publish_surface(root, size, &context).into_parts().0
}

enum Action {}

fn hit_id(frame: &SurfaceFrame, point: LogicalPoint) -> Result<RuntimeNodeId, &'static str> {
    frame.hit_test_id(point).ok_or("expected hit-test target")
}

#[test]
fn logical_rect_contains_left_and_top_edges_but_excludes_right_and_bottom_edges() {
    let rect = LogicalRect::from_xywh(10.0, 20.0, 30.0, 40.0);

    assert!(rect.contains(LogicalPoint::new(10.0, 20.0)));
    assert!(rect.contains(LogicalPoint::new(39.0, 59.0)));
    assert!(!rect.contains(LogicalPoint::new(40.0, 20.0)));
    assert!(!rect.contains(LogicalPoint::new(10.0, 60.0)));
    assert!(!rect.contains(LogicalPoint::new(9.0, 20.0)));
    assert!(!rect.contains(LogicalPoint::new(10.0, 19.0)));
}

#[test]
fn empty_surface_frame_hit_test_returns_none() {
    let frame = SurfaceFrame::empty(LogicalSize::new(100.0, 100.0));

    assert_eq!(frame.hit_test(LogicalPoint::new(1.0, 1.0)), None);
    assert_eq!(frame.hit_test_id(LogicalPoint::new(1.0, 1.0)), None);
}

#[test]
fn hit_test_prefers_later_nodes_when_bounds_overlap() -> Result<(), &'static str> {
    let first = SurfaceNode::new(
        RuntimeNodeId::from_index(1),
        Some(RuntimeNodeId::ROOT),
        None,
        LogicalRect::from_xywh(0.0, 0.0, 50.0, 50.0),
        SurfaceNodeKind::button("First", true),
        ComputedStyle::EMPTY,
    );
    let second = SurfaceNode::new(
        RuntimeNodeId::from_index(2),
        Some(RuntimeNodeId::ROOT),
        None,
        LogicalRect::from_xywh(10.0, 10.0, 50.0, 50.0),
        SurfaceNodeKind::button("Second", true),
        ComputedStyle::EMPTY,
    );
    let frame = SurfaceFrame::new(
        LogicalSize::new(100.0, 100.0),
        vec![
            SurfaceNode::new(
                RuntimeNodeId::ROOT,
                None,
                None,
                LogicalRect::from_xywh(0.0, 0.0, 100.0, 100.0),
                SurfaceNodeKind::container(),
                ComputedStyle::EMPTY,
            ),
            first,
            second,
        ],
    );

    assert_eq!(
        hit_id(&frame, LogicalPoint::new(15.0, 15.0))?,
        RuntimeNodeId::from_index(2)
    );
    assert_eq!(
        hit_id(&frame, LogicalPoint::new(5.0, 5.0))?,
        RuntimeNodeId::from_index(1)
    );

    Ok(())
}

#[test]
fn published_surface_hit_test_returns_child_nodes_before_parent_containers()
-> Result<(), &'static str> {
    let ui = column((
        row((button::<Action>("A"), button::<Action>("B")))
            .id("button.row")
            .gap(4.0),
        text::<Action>("End").id("end"),
    ))
    .id("root")
    .gap(8.0);
    let frame = surface_frame(&ui, LogicalSize::new(300.0, 200.0));

    assert_eq!(
        hit_id(&frame, LogicalPoint::new(1.0, 1.0))?,
        RuntimeNodeId::from_index(2)
    );
    assert_eq!(
        hit_id(&frame, LogicalPoint::new(69.0, 1.0))?,
        RuntimeNodeId::from_index(3)
    );
    assert_eq!(
        hit_id(&frame, LogicalPoint::new(65.0, 1.0))?,
        RuntimeNodeId::from_index(1)
    );
    assert_eq!(
        hit_id(&frame, LogicalPoint::new(1.0, 41.0))?,
        RuntimeNodeId::from_index(4)
    );
    assert_eq!(
        hit_id(&frame, LogicalPoint::new(250.0, 150.0))?,
        RuntimeNodeId::ROOT
    );
    assert_eq!(frame.hit_test_id(LogicalPoint::new(400.0, 1.0)), None);

    Ok(())
}

#[test]
fn hit_test_reports_disabled_button_node() -> Result<(), &'static str> {
    let ui = column(button::<Action>("Disabled").disabled());
    let frame = surface_frame(&ui, LogicalSize::new(200.0, 100.0));

    assert_eq!(
        hit_id(&frame, LogicalPoint::new(1.0, 1.0))?,
        RuntimeNodeId::from_index(1)
    );

    Ok(())
}
