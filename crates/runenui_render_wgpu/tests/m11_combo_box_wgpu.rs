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
    Color, CommandOrigin, ElementId, FontFamilyName, GenericFontFamily, LayoutBound,
    LayoutDimension, LayoutStyle, LogicalLength, NoHostProtocol, OverflowPolicy, SemanticCommand,
    SemanticEditableMode, SemanticRelationshipKind, SemanticRole, StyleEnvironment, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, UiApp, View,
    combo_box, editable_combo_box, option_item, text_field,
};
use runenui_render_wgpu::{
    Renderer, RendererInitError, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};
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

struct LongComboGpuApp;
impl UiApp for LongComboGpuApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        let options = (0..12_u8).map(|index| {
            option_item(format!("Entry {index:02}"), index == 0)
                .id(format!("gpu.scroll.option.{index}"))
                .foreground(Color::WHITE)
                .with_layout(
                    LayoutStyle::default()
                        .with_width(LayoutDimension::Length(LogicalLength::from(160_u16)))
                        .with_height(LayoutDimension::Length(LogicalLength::from(24_u8))),
                )
                .on_activate(|| ())
        });
        combo_box(
            "Scroll",
            ElementId::new("gpu.scroll.list").unwrap_or_else(|_| unreachable!()),
            options,
            |_| (),
        )
        .id("gpu.scroll.owner")
        .foreground(Color::WHITE)
        .open(true)
        .popup_max_height(LogicalLength::from(72_u8))
    }

    fn update(_: &mut Self::State, _: Self::Action) {}
}

fn long_combo_id(app: &AppRuntime<LongComboGpuApp>, authored: &str) -> MountedNodeId {
    let id = ElementId::new(authored).unwrap_or_else(|_| unreachable!());
    app.index().nodes().iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("long popup node is mounted"))
        .id().clone()
}

#[test]
fn long_combo_native_gpu_scroll_clips_paint_to_the_existing_bounded_viewport()
-> Result<(), Box<dyn Error>> {
    let required = std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR").is_some()
        || std::env::var_os("RUNENUI_M11SLIDER_EVIDENCE_DIR").is_some();
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) if !required => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut app = AppRuntime::<LongComboGpuApp>::mount(());
    assert!(app.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).is_quiescent());
    app.register_text_font_bytes(FONT.to_vec())?;
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")?],
    )?;
    let owner = long_combo_id(&app, "gpu.scroll.owner");
    let list = long_combo_id(&app, "gpu.scroll.list");
    let last = long_combo_id(&app, "gpu.scroll.option.11");
    let viewport = app.index().node(&list)
        .and_then(|node| node.parent())
        .ok_or_else(|| std::io::Error::other("missing standard scroll parent"))?
        .clone();
    let scroll = app.index().node(&viewport)
        .ok_or_else(|| std::io::Error::other("missing standard scroll viewport"))?
        .layout().clone();
    assert_eq!(scroll.max_height(), LayoutBound::Length(LogicalLength::from(72_u8)));
    assert_eq!(scroll.overflow().vertical(), OverflowPolicy::Scroll);
    assert!(!app.index().node(&viewport)
        .ok_or_else(|| std::io::Error::other("missing scroll focus node"))?.is_focusable());
    app.submit_command(
        owner.clone(), SemanticCommand::RequestFocus, CommandOrigin::programmatic(),
    ).unwrap_or_else(|_| unreachable!("ComboBox owner is focusable"));
    assert!(app.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).is_quiescent());

    let build = SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(320.0, 260.0)?,
    );
    let before = app.publish_surface(&build)
        .map_err(|e| std::io::Error::other(format!("before scroll: {e:?}")))?;
    let bounds = before.frame().node(&viewport)
        .ok_or_else(|| std::io::Error::other("missing viewport layout"))?.bounds();
    let last_bounds = before.frame().node(&last)
        .ok_or_else(|| std::io::Error::other("missing last Option layout"))?.bounds();
    assert!(bounds.height() > 0.0 && bounds.height() <= 72.0);
    assert!(last_bounds.y() > bounds.y() + bounds.height());
    let image_before = renderer.render_offscreen_publication(before.paint_publication(), &NoResources)?;

    // The public #344 exact-target command proves ordinary scroll/render correlation
    // without fabricating a ComboBox-internal targetable EventContext seam.
    app.submit_command(
        last, SemanticCommand::ScrollIntoView, CommandOrigin::programmatic(),
    ).unwrap_or_else(|_| unreachable!("current exact Option target"));
    assert!(app.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).is_quiescent());
    assert!(app.index().node(&viewport)
        .ok_or_else(|| std::io::Error::other("missing scroll offset"))?
        .interaction().scroll_offset().1 > 0.0);
    let after = app.publish_surface(&build)
        .map_err(|e| std::io::Error::other(format!("after scroll: {e:?}")))?;
    let image_after = renderer.render_offscreen_publication(after.paint_publication(), &NoResources)?;
    assert_eq!(app.focus().focused_node(), Some(&owner));
    let a = image_before.readback().rgba8_srgb();
    let b = image_after.readback().rgba8_srgb();
    assert_eq!(image_before.readback().extent(), image_after.readback().extent());
    let extent = image_before.readback().extent();
    let mut inside_changed = false;
    for y in 0..extent.height() {
        for x in 0..extent.width() {
            let index = (y as usize * extent.width() as usize + x as usize) * 4;
            if a[index..index + 4] == b[index..index + 4] {
                continue;
            }
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            assert!(px >= bounds.x() - 1.0
                && px <= bounds.x() + bounds.width() + 1.0
                && py >= bounds.y() - 1.0
                && py <= bounds.y() + bounds.height() + 1.0,
                "native GPU scroll must not leak changing Option pixels outside the viewport");
            inside_changed = true;
        }
    }
    assert!(inside_changed, "current exact Option reveal must change clipped GPU pixels");
    if let Some(dir) = std::env::var_os("RUNENUI_M11COMBO_EVIDENCE_DIR").map(PathBuf::from) {
        fs::create_dir_all(&dir)?;
        for (name, result) in [
            ("m11-combo-scroll-before.png", &image_before),
            ("m11-combo-scroll-after.png", &image_after),
        ] {
            image::save_buffer(
                dir.join(name),
                result.readback().rgba8_srgb(),
                extent.width(),
                extent.height(),
                image::ColorType::Rgba8,
            )?;
        }
    }
    Ok(())
}
