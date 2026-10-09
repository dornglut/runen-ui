#![allow(refining_impl_trait)]
use core::{
    future::Future,
    pin::pin,
    task::{Context, Poll},
};
use std::{fs, path::PathBuf};
use runenui_core::{
    Color, EditIntent, EditResolution, FontFamilyName, GenericFontFamily, NoHostProtocol,
    SemanticEditableMode, StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextPosition, TextSelection, UiApp, UpdateOutput, View,
};
use runenui_render_wgpu::{
    Renderer, RendererInitError, RendererOptions, ResourcePayload, ResourceProvider,
    ResourceProviderError, ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};

const FONT: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");
const SECRET: &str = "clé-é漢🌐";

#[derive(Clone, Copy)]
struct State {
    masked: bool,
}
enum Action {
    Edit(Box<EditIntent>),
    Toggle,
}
struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> impl View<Action> {
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(999), TextDocumentRevision::new(1));
        let position = TextPosition::new(snapshot, SECRET, SECRET.len(), TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("valid source position"));
        let field = runenui_core::text_field(
            snapshot,
            SECRET,
            TextSelection::collapsed(position),
            SemanticEditableMode::SingleLine,
            |intent| Action::Edit(Box::new(intent)),
        )
        .unwrap_or_else(|_| unreachable!("valid single-line source"));
        let field = if state.masked {
            field
                .password()
                .unwrap_or_else(|_| unreachable!("valid password mode"))
        } else {
            field
        };
        field.id("gpu.password").foreground(Color::WHITE)
    }

    fn update(state: &mut State, action: Action) -> UpdateOutput<Action, NoHostProtocol> {
        match action {
            Action::Toggle => {
                state.masked = !state.masked;
                UpdateOutput::effects(runenui_core::Effects::none())
            }
            Action::Edit(intent) => UpdateOutput::edit(EditResolution::rejected(
                intent.request().clone(),
                TextDocumentSnapshot::new(TextDocumentId::new(999), TextDocumentRevision::new(1)),
            )),
        }
    }
}

struct NoExternalResources;
impl ResourceProvider for NoExternalResources {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "password text publication requires no external image resources",
        ))
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = pin!(future);
    loop {
        match Future::poll(future.as_mut(), &mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn publish(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    let env = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &env,
            LogicalSize::try_new(300.0, 60.0)
                .unwrap_or_else(|_| unreachable!("valid logical extent")),
        ))
        .unwrap_or_else(|_| unreachable!("password surface publishes"))
}

#[test]
fn masked_standard_password_renders_with_real_wgpu_and_reclassifies_without_source_glyphs()
-> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => renderer,
        Err(RendererInitError::AdapterUnavailable { .. }) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let mut runtime = AppRuntime::<App>::mount(State { masked: true });
    runtime.register_text_font_bytes(FONT.to_vec())?;
    runtime.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell")?],
    )?;
    let provider = NoExternalResources;
    let masked = publish(&mut runtime);
    assert!(!format!("{:?}", masked.paint_scene()).contains(SECRET));
    assert_eq!(
        masked.semantic_publication().snapshot().nodes()[0]
            .editable()
            .and_then(|edit| edit.value()),
        None
    );
    let initial = renderer.render_offscreen_publication(masked.paint_publication(), &provider)?;
    let pixels = initial.readback().rgba8_srgb().to_vec();
    let evidence_dir = std::env::var_os("RUNENUI_M11PASSWORD_EVIDENCE_DIR").map(PathBuf::from);
    if let Some(directory) = &evidence_dir {
        fs::create_dir_all(directory)?;
        let extent = initial.readback().extent();
        image::save_buffer(
            directory.join("m11-password-masked.png"),
            &pixels,
            extent.width(),
            extent.height(),
            image::ColorType::Rgba8,
        )?;
    }
    runtime
        .submit_action(Action::Toggle)
        .unwrap_or_else(|_| unreachable!("toggle is accepted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let public = publish(&mut runtime);
    let visible = renderer.render_offscreen_publication(public.paint_publication(), &provider)?;
    assert_ne!(pixels, visible.readback().rgba8_srgb());
    if let Some(directory) = &evidence_dir {
        let extent = visible.readback().extent();
        image::save_buffer(
            directory.join("m11-password-public.png"),
            visible.readback().rgba8_srgb(),
            extent.width(),
            extent.height(),
            image::ColorType::Rgba8,
        )?;
    }
    runtime
        .submit_action(Action::Toggle)
        .unwrap_or_else(|_| unreachable!("toggle is accepted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let masked_again = publish(&mut runtime);
    assert!(!format!("{:?}", masked_again.paint_scene()).contains(SECRET));
    let repeated =
        renderer.render_offscreen_publication(masked_again.paint_publication(), &provider)?;
    assert_eq!(pixels, repeated.readback().rgba8_srgb());
    Ok(())
}
