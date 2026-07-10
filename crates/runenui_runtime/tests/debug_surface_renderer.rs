use runenui_core::prelude::{button, column, text};
use runenui_runtime::prelude::{
    DebugSurfaceRenderer, LogicalSize, SurfaceFrame, layout_surface, render_debug_surface_frame,
};

enum Action {}

#[test]
fn debug_renderer_renders_empty_surface_frame_header() {
    let frame = SurfaceFrame::empty(LogicalSize::new(320.0, 240.0));

    assert_eq!(
        render_debug_surface_frame(&frame),
        "surface size=(320.0,240.0) nodes=0\n"
    );
}

#[test]
fn debug_renderer_renders_layout_nodes() {
    let ui = column((
        text::<Action>("Counter").id("counter.title"),
        button::<Action>("Reset").disabled().id("counter.reset"),
    ))
    .id("counter.root")
    .gap(8.0);
    let frame = layout_surface(&ui, LogicalSize::new(200.0, 100.0));

    let rendered = DebugSurfaceRenderer::new().render(&frame);

    assert!(rendered.contains("surface size=(200.0,100.0) nodes=3"));
    assert!(rendered.contains("node id=0 parent=- authored=counter.root"));
    assert!(rendered.contains("bounds=(0.0,0.0,200.0,100.0) kind=container"));
    assert!(rendered.contains("node id=1 parent=0 authored=counter.title"));
    assert!(rendered.contains("kind=text \"Counter\""));
    assert!(rendered.contains("node id=2 parent=0 authored=counter.reset"));
    assert!(rendered.contains("bounds=(0.0,28.0,64.0,32.0) kind=button \"Reset\" enabled=false"));
}
