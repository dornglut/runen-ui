#![allow(refining_impl_trait)]

//! A select-only `ComboBox` must render its ordinary `Option` text while M5
//! projects a singular current active descendant without focus movement.
use core::{
    error::Error,
    future::Future,
    pin::pin,
    task::{Context, Poll},
};
use runenui_core::{
    Color, ElementId, FontFamilyName, GenericFontFamily, NoHostProtocol, SemanticEditableMode,
    SemanticRelationshipKind, SemanticRole, StyleEnvironment, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, UiApp, View,
    combo_box, editable_combo_box, option_item, text_field,
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
            "ComboBox needs no external image resources",
        ))
    }
}
struct ComboGpuApp;
impl UiApp for ComboGpuApp {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(open: &Self::State) -> impl View<Self::Action> {
        combo_box(
            "Choose",
            ElementId::new("gpu.combo.popup").unwrap_or_else(|_| unreachable!()),
            [
                option_item("Alpha", true)
                    .id("gpu.combo.alpha")
                    .foreground(Color::WHITE)
                    .on_activate(|| ()),
                option_item("Bravo", false)
                    .id("gpu.combo.bravo")
                    .foreground(Color::WHITE)
                    .on_activate(|| ()),
            ],
            |_| (),
        )
        .id("gpu.combo.owner")
        .foreground(Color::WHITE)
        .open(*open)
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
fn publication(open: bool) -> Result<runenui_runtime::SurfacePublication, Box<dyn Error>> {
    let mut app = AppRuntime::<ComboGpuApp>::mount(open);
    // Reconcile initial authored popup content and current option identity
    // before publishing; there is no fake semantic target or test-only hook.
    assert!(
        app.pump(runenui_runtime::PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .is_quiescent()
    );
    app.register_text_font_bytes(FONT.to_vec())?;
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")?],
    )?;
    app.publish_surface(&SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(400.0, 200.0)?,
    ))
    .map_err(|e| std::io::Error::other(format!("ComboBox surface: {e:?}")))
    .map_err(Into::into)
}
#[test]
fn select_only_combo_popup_projects_real_m8_options_and_real_wgpu_pixels()
-> Result<(), Box<dyn Error>> {
    let required = std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR").is_some()
        || std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR").is_some();
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) if !required => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let open = publication(true)?;
    let closed = publication(false)?;
    for (source, expected_options, expected_runs) in [(&open, 2, 3), (&closed, 0, 1)] {
        let snapshot = source.semantic_publication().snapshot();
        let combo = snapshot
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::ComboBox)
            .ok_or_else(|| std::io::Error::other("ComboBox primary semantic role absent"))?;
        let count = snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::Option)
            .count();
        assert_eq!(count, expected_options);
        let controls = combo
            .relationships()
            .iter()
            .find(|link| link.kind() == SemanticRelationshipKind::Controls);
        assert_eq!(controls.is_some(), expected_options != 0);
        if let Some(controls) = controls {
            assert_eq!(
                snapshot
                    .nodes()
                    .iter()
                    .find(|node| node.id() == controls.target())
                    .map(runenui_runtime::SemanticNode::role),
                Some(SemanticRole::ListBox),
                "M5 Controls must identify the actual popup ListBox, not the Popover Group"
            );
        }
        assert_eq!(
            combo
                .relationships()
                .iter()
                .filter(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
                .count(),
            usize::from(expected_options != 0)
        );
        let runs = source
            .paint_scene()
            .items()
            .iter()
            .filter(|item| item.primitive().as_shaped_text_run().is_some())
            .count();
        assert_eq!(
            runs, expected_runs,
            "ordinary ComboBox Option text uses retained M8"
        );
    }
    let a = renderer.render_offscreen_publication(open.paint_publication(), &NoResources)?;
    let b = renderer.render_offscreen_publication(closed.paint_publication(), &NoResources)?;
    assert_ne!(
        a.readback().rgba8_srgb(),
        b.readback().rgba8_srgb(),
        "opening ComboBox adds actual native-wgpu Option pixels"
    );
    let dir = std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR")
        .or_else(|| std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR"));
    if let Some(path) = dir.map(PathBuf::from) {
        fs::create_dir_all(&path)?;
        for (name, result) in [("m11-combo-open.png", &a), ("m11-combo-closed.png", &b)] {
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

struct EditableComboGpuApp;
impl UiApp for EditableComboGpuApp {
    type State = bool;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(open: &Self::State) -> impl View<Self::Action> {
        let value = "search";
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(1331), TextDocumentRevision::new(1));
        let caret = TextPosition::new(snapshot, value, value.len(), TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("valid input text"));
        let editor = text_field(
            snapshot,
            value,
            TextSelection::collapsed(caret),
            SemanticEditableMode::SingleLine,
            |_| (),
        )
        .unwrap_or_else(|_| unreachable!("single-line editor"))
        .id("gpu.editable.owner")
        .foreground(Color::WHITE);
        editable_combo_box(
            editor,
            "Search",
            ElementId::new("gpu.editable.popup").unwrap_or_else(|_| unreachable!()),
            [
                option_item("Alpha", false)
                    .id("gpu.editable.alpha")
                    .foreground(Color::WHITE)
                    .on_activate(|| ()),
                option_item("Bravo", false)
                    .id("gpu.editable.bravo")
                    .foreground(Color::WHITE)
                    .on_activate(|| ()),
            ],
            |_| (),
        )
        .unwrap_or_else(|_| unreachable!("compatible M10 editor"))
        .open(*open)
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn editable_publication(open: bool) -> Result<runenui_runtime::SurfacePublication, Box<dyn Error>> {
    let mut app = AppRuntime::<EditableComboGpuApp>::mount(open);
    assert!(
        app.pump(runenui_runtime::PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .is_quiescent()
    );
    app.register_text_font_bytes(FONT.to_vec())?;
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")?],
    )?;
    app.publish_surface(&SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(400.0, 200.0)?,
    ))
    .map_err(|error| std::io::Error::other(format!("editable ComboBox surface: {error:?}")))
    .map_err(Into::into)
}

#[test]
fn editable_combo_shares_m10_text_pixels_and_projects_current_m5_popup()
-> Result<(), Box<dyn Error>> {
    let required = std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR").is_some()
        || std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR").is_some();
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) if !required => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let open = editable_publication(true)?;
    let closed = editable_publication(false)?;
    for (publication, count) in [(&open, 2), (&closed, 0)] {
        let snapshot = publication.semantic_publication().snapshot();
        let editable_combo = snapshot
            .nodes()
            .iter()
            .find(|node| node.role() == SemanticRole::ComboBox)
            .ok_or_else(|| std::io::Error::other("editable ComboBox primary semantics missing"))?;
        assert!(
            editable_combo.editable().is_some(),
            "the single M10 editor is the ComboBox semantic primary"
        );
        assert_eq!(
            editable_combo.editable_mode(),
            Some(SemanticEditableMode::SingleLine)
        );
        assert!(
            !snapshot
                .nodes()
                .iter()
                .any(|node| node.role() == SemanticRole::EditableText),
            "must never mount another primary text editor"
        );
        assert_eq!(
            snapshot
                .nodes()
                .iter()
                .filter(|node| node.role() == SemanticRole::Option)
                .count(),
            count
        );
        assert_eq!(
            editable_combo
                .relationships()
                .iter()
                .filter(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
                .count(),
            usize::from(count > 0)
        );
        let expected_text_runs = if count == 0 { 1 } else { 3 };
        assert_eq!(
            publication
                .paint_scene()
                .items()
                .iter()
                .filter(|item| item.primitive().as_shaped_text_run().is_some())
                .count(),
            expected_text_runs,
            "same M8 source shapes editor text and ordinary popup Option labels"
        );
    }
    let a = renderer.render_offscreen_publication(open.paint_publication(), &NoResources)?;
    let b = renderer.render_offscreen_publication(closed.paint_publication(), &NoResources)?;
    assert_ne!(
        a.readback().rgba8_srgb(),
        b.readback().rgba8_srgb(),
        "editable popup changes real native-wgpu pixels without a second editor"
    );
    let dir = std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR")
        .or_else(|| std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR"));
    if let Some(path) = dir.map(PathBuf::from) {
        fs::create_dir_all(&path)?;
        for (name, result) in [
            ("m11-combo-editable-open.png", &a),
            ("m11-combo-editable-closed.png", &b),
        ] {
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
