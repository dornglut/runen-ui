#![allow(refining_impl_trait)]

//! Native offscreen proof for ordinary M8 text leaves nested in a semantic Tree.
use core::{
    error::Error,
    future::Future,
    pin::pin,
    task::{Context, Poll},
};
use runenui_core::{
    Color, FontFamilyName, GenericFontFamily, NoHostProtocol, SemanticRole, StyleEnvironment,
    UiApp, View, tree_item, tree_view,
};
use runenui_render_wgpu::{
    Renderer, RendererInitError, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, LogicalSize, SurfaceBuildContext};
use std::{fs, path::PathBuf};

const FONT: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");
struct NoResources;
impl ResourceProvider for NoResources {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "TreeView uses no external image sources",
        ))
    }
}
struct TreeGpuApp;
impl UiApp for TreeGpuApp {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(expanded: &Self::State) -> impl View<Self::Action> {
        let child = tree_item("child", "Leaf", Vec::new())
            .foreground(Color::WHITE)
            .on_request(|_| ())
            .id("tree.gpu.child");
        let parent = tree_item("parent", "Folder", vec![child])
            .expanded(*expanded)
            .foreground(Color::WHITE)
            .on_request(|_| ())
            .id("tree.gpu.parent");
        tree_view(vec![parent]).id("tree.gpu")
    }
    fn update(_: &mut Self::State, (): Self::Action) {}
}
fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut ctx = Context::from_waker(waker);
    let mut future = pin!(future);
    loop {
        match Future::poll(future.as_mut(), &mut ctx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
fn publication(expanded: bool) -> Result<runenui_runtime::SurfacePublication, Box<dyn Error>> {
    let mut app = AppRuntime::<TreeGpuApp>::mount(expanded);
    app.register_text_font_bytes(FONT.to_vec())?;
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")?],
    )?;
    app.publish_surface(&SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(400.0, 180.0)?,
    ))
    .map_err(|error| std::io::Error::other(format!("TreeView surface: {error:?}")))
    .map_err(Into::into)
}
#[test]
fn nested_tree_item_labels_use_retained_m8_text_and_native_wgpu() -> Result<(), Box<dyn Error>> {
    let required = std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR").is_some()
        || std::env::var_os("RUNENUI_M11TREE_EVIDENCE_DIR").is_some();
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) if !required => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let open = publication(true)?;
    let closed = publication(false)?;
    for (source, expected_items) in [(&open, 2), (&closed, 1)] {
        let semantic = source.semantic_publication().snapshot();
        let tree = semantic
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::Tree)
            .ok_or_else(|| std::io::Error::other("Tree semantic role absent"))?;
        assert_eq!(tree.children().len(), 1);
        let count = semantic
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::TreeItem)
            .count();
        assert_eq!(count, expected_items);
        let runs = source
            .paint_scene()
            .items()
            .iter()
            .filter(|item| item.primitive().as_shaped_text_run().is_some())
            .count();
        assert_eq!(
            runs, expected_items,
            "each mounted visible TreeItem owns one ordinary text leaf, not a renderer primitive"
        );
    }
    let a = renderer.render_offscreen_publication(open.paint_publication(), &NoResources)?;
    let b = renderer.render_offscreen_publication(closed.paint_publication(), &NoResources)?;
    assert_ne!(
        a.readback().rgba8_srgb(),
        b.readback().rgba8_srgb(),
        "expanded child label must visibly change actual wgpu pixels"
    );
    let directory = std::env::var_os("RUNENUI_M11TREE_EVIDENCE_DIR")
        .or_else(|| std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR"));
    if let Some(path) = directory.map(PathBuf::from) {
        fs::create_dir_all(&path)?;
        for (name, result) in [("m11-tree-open.png", &a), ("m11-tree-closed.png", &b)] {
            let extent = result.readback().extent();
            image::save_buffer(
                path.join(name),
                result.readback().rgba8_srgb(),
                extent.width(),
                extent.height(),
                image::ColorType::Rgba8,
            )?;
        }
    }
    Ok(())
}
