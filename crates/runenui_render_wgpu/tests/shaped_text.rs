#![allow(refining_impl_trait)]

use core::{future::Future, pin::pin, task::Poll};
use std::{cell::Cell, task::Context};

use runenui_core::{
    Element, FontFamilyName, GenericFontFamily, NoHostProtocol, StyleEnvironment, UiApp, View, text,
};
use runenui_render_wgpu::{
    BackendSelection, PublicationStageResult, Renderer, RendererInitError, RendererOptions,
    ResourceCacheOutcome, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, LogicalSize, PaintPublication, RasterScale, SurfaceBuildContext,
};

const FONT_BYTES: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");
const COLR_FONT_BYTES: &[u8] = include_bytes!("fixtures/RunenUIFixtureColr-Regular.ttf");
const SVG_FONT_BYTES: &[u8] = include_bytes!("fixtures/RunenUIFixtureSvg-Regular.ttf");
const BITMAP_FONT_BYTES: &[u8] = include_bytes!("fixtures/RunenUIFixtureBitmap-Regular.ttf");

fn surface_size() -> LogicalSize {
    LogicalSize::try_new(48.0, 32.0).unwrap_or_else(|_| unreachable!())
}

struct TextApp;
impl UiApp for TextApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        text("A").into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn register_font(runtime: &mut AppRuntime<TextApp>) {
    register_font_bytes(runtime, FONT_BYTES);
}

fn register_font_bytes(runtime: &mut AppRuntime<TextApp>, font_bytes: &[u8]) {
    assert!(
        runtime
            .register_text_font_bytes(font_bytes.to_vec())
            .is_ok()
    );
    let family = FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!());
    assert!(
        runtime
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
            .is_ok()
    );
}

fn publish(runtime: &mut AppRuntime<TextApp>, scale: RasterScale) -> PaintPublication {
    let styles = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&styles, LayoutConstraints::tight(surface_size()))
        .with_raster_scale(scale);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("bundled text publication is admitted"))
        .paint_publication()
        .clone()
}

fn text_ref(publication: &PaintPublication) -> runenui_core::ResourceRef {
    publication
        .scene()
        .items()
        .iter()
        .find_map(|item| {
            item.primitive()
                .as_shaped_text_run()
                .map(|run| run.resource_ref().clone())
        })
        .unwrap_or_else(|| unreachable!("text publication contains one shaped run"))
}

#[derive(Default)]
struct ExternalOnlyProvider {
    loads: Cell<usize>,
}
impl ResourceProvider for ExternalOnlyProvider {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        match request {
            ResourceRequest::Image => Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Missing,
                "text-only publication requested no external image",
            )),
        }
    }
}

fn renderer_or_skip() -> Result<Option<Renderer>, Box<dyn std::error::Error>> {
    match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => Ok(Some(renderer)),
        Err(RendererInitError::AdapterUnavailable {
            requested,
            compatible_surface_required,
            detail,
        }) => {
            assert_eq!(requested, BackendSelection::AllNative);
            assert!(!compatible_surface_required);
            assert!(!detail.is_empty());
            eprintln!("production shaped-text proof skipped: {detail}");
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn alpha_pixels(publication: &runenui_render_wgpu::OffscreenPublicationReadback) -> usize {
    publication
        .readback()
        .rgba8_srgb()
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[3] != 0)
        .count()
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

#[test]
fn production_msdf_uses_one_logical_ref_at_multiple_renderer_realizations()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let mut runtime = AppRuntime::<TextApp>::mount(());
    register_font(&mut runtime);
    let low = publish(&mut runtime, RasterScale::new(0.75)?);
    let one = publish(&mut runtime, RasterScale::ONE);
    let two = publish(&mut runtime, RasterScale::new(2.0)?);
    assert_eq!(text_ref(&low), text_ref(&one));
    assert_eq!(text_ref(&one), text_ref(&two));

    let provider = ExternalOnlyProvider::default();
    let first = renderer.render_offscreen_publication(&low, &provider)?;
    let same_tier = renderer.render_offscreen_publication(&one, &provider)?;
    let second = renderer.render_offscreen_publication(&two, &provider)?;
    assert_eq!(
        provider.loads.get(),
        0,
        "shaped text is not caller-provider authority"
    );
    assert!(alpha_pixels(&first) > 0);
    assert!(alpha_pixels(&second) > 0);
    assert_eq!(
        first.observation().resource_observations()[0].resource(),
        &text_ref(&one)
    );
    assert_eq!(
        second.observation().resource_observations()[0].resource(),
        &text_ref(&two)
    );
    assert_eq!(
        first.observation().resource_observations()[0].cache_outcome(),
        ResourceCacheOutcome::Realized
    );
    assert_eq!(
        second.observation().resource_observations()[0].cache_outcome(),
        ResourceCacheOutcome::Realized
    );
    assert_eq!(
        same_tier.observation().resource_observations()[0].cache_outcome(),
        ResourceCacheOutcome::Reused
    );
    Ok(())
}

#[test]
fn retained_publication_retries_after_renderer_realization_loss()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let mut runtime = AppRuntime::<TextApp>::mount(());
    register_font(&mut runtime);
    let publication = publish(&mut runtime, RasterScale::ONE);
    let expected_ref = text_ref(&publication);
    drop(runtime);
    let provider = ExternalOnlyProvider::default();
    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    assert!(renderer.discard_resource_cache());
    let retry = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(provider.loads.get(), 0);
    assert_eq!(text_ref(&publication), expected_ref);
    assert!(alpha_pixels(&first) > 0);
    assert!(alpha_pixels(&retry) > 0);
    assert_eq!(
        retry.observation().render_result(),
        PublicationStageResult::Succeeded
    );
    assert_eq!(
        retry.observation().resource_observations()[0].cache_outcome(),
        ResourceCacheOutcome::Realized
    );
    let Some(mut fresh_renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let fresh = fresh_renderer.render_offscreen_publication(&publication, &provider)?;
    assert!(alpha_pixels(&fresh) > 0);
    assert_eq!(provider.loads.get(), 0);
    Ok(())
}

#[test]
fn production_msdf_pixel_evidence_is_stable_and_small_size_is_covered()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let mut runtime = AppRuntime::<TextApp>::mount(());
    register_font(&mut runtime);
    let small = publish(&mut runtime, RasterScale::new(0.5)?);
    let provider = ExternalOnlyProvider::default();
    let readback = renderer.render_offscreen_publication(&small, &provider)?;
    let pixels = readback.readback().rgba8_srgb();
    assert!(alpha_pixels(&readback) > 0);
    assert!(
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] > 0 && pixel[3] < 255)
    );
    eprintln!(
        "production MSDF small-size golden: hash={:016x}, alpha_pixels={}",
        fnv1a(pixels),
        alpha_pixels(&readback)
    );
    assert_eq!(fnv1a(pixels), 0x8a54_f9b6_5ca8_085f);
    Ok(())
}

#[test]
fn unsupported_glyph_diagnostic_contract_is_explicit() {
    use runenui_render_wgpu::UnsupportedShapedGlyphKind;
    assert_ne!(
        format!("{:?}", UnsupportedShapedGlyphKind::ColrV0),
        format!("{:?}", UnsupportedShapedGlyphKind::Bitmap)
    );
    assert!(
        format!(
            "{}",
            runenui_render_wgpu::PublicationRenderError::UnsupportedShapedGlyph {
                item_index: 0,
                glyph_id: 7,
                kind: UnsupportedShapedGlyphKind::ColrV1
            }
        )
        .contains("outline MSDF")
    );
}

#[test]
fn production_intrinsic_glyph_formats_return_structured_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    for (font_bytes, expected) in [
        (
            COLR_FONT_BYTES,
            runenui_render_wgpu::UnsupportedShapedGlyphKind::ColrV0,
        ),
        (
            SVG_FONT_BYTES,
            runenui_render_wgpu::UnsupportedShapedGlyphKind::Svg,
        ),
        (
            BITMAP_FONT_BYTES,
            runenui_render_wgpu::UnsupportedShapedGlyphKind::Bitmap,
        ),
    ] {
        let mut runtime = AppRuntime::<TextApp>::mount(());
        register_font_bytes(&mut runtime, font_bytes);
        let publication = publish(&mut runtime, RasterScale::ONE);
        let provider = ExternalOnlyProvider::default();
        let result = renderer.render_offscreen_publication(&publication, &provider);
        match result {
            Err(runenui_render_wgpu::PublicationRenderError::UnsupportedShapedGlyph {
                item_index,
                glyph_id,
                kind,
            }) => {
                assert_eq!(item_index, 0);
                assert_eq!(
                    glyph_id, 36,
                    "fixture A must be classified on the real glyph"
                );
                assert_eq!(kind, expected);
            }
            Ok(_) => return Err("intrinsic glyph format silently reached monochrome MSDF".into()),
            Err(error) => return Err(format!("unexpected intrinsic glyph result: {error}").into()),
        }
        assert_eq!(provider.loads.get(), 0);
    }
    Ok(())
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
