#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    cell::Cell,
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use ab_glyph::{Font, FontArc, point};
use runenui_core::{
    Color, ContributionClip, Element, LogicalLength, LogicalPoint, LogicalRect, LogicalSize,
    LogicalTransform, NoHostProtocol, PaintContribution, PaintContributionContext,
    PaintContributionItem, ResourceKind, ResourceRef, SceneShape, StyleEnvironment, UiApp, Widget,
    WidgetMeasure, WidgetUpdateContext,
};
use runenui_render_wgpu::{
    BackendSelection, ImagePayload, PublicationRenderError, PublicationStageResult,
    PublicationUpdateMode, Renderer, RendererInitError, RendererOptions, ResourceCacheOutcome,
    ResourcePayload, ResourceProvider, ResourceProviderError, ResourceProviderErrorKind,
    ResourceRequest, ResourceResolveError, ShapedRunRaster,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SurfaceBuildContext,
};

const SURFACE_WIDTH: u16 = 32;
const SURFACE_HEIGHT: u16 = 24;
const FONT_BYTES: &[u8] = include_bytes!("fixtures/Cantarell-Regular.ttf");

#[derive(Clone, Debug)]
struct SceneFixture {
    items: Vec<PaintContributionItem>,
}

impl Widget<Vec<PaintContributionItem>> for SceneFixture {
    type State = Vec<PaintContributionItem>;

    fn create_state(&self) -> Self::State {
        self.items.clone()
    }

    fn update(
        &self,
        state: &mut Self::State,
        _: &mut WidgetUpdateContext<Vec<PaintContributionItem>>,
    ) {
        state.clone_from(&self.items);
    }

    fn measure(&self, _: &Self::State) -> WidgetMeasure {
        WidgetMeasure::Fixed {
            width: LogicalLength::from(SURFACE_WIDTH),
            height: LogicalLength::from(SURFACE_HEIGHT),
        }
    }

    fn paint(&self, items: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::new(items.clone())
    }
}

struct FixtureApp;

impl UiApp for FixtureApp {
    type State = Vec<PaintContributionItem>;
    type Action = Vec<PaintContributionItem>;
    type HostProtocol = NoHostProtocol;

    fn root(items: &Self::State) -> Element<Self::Action> {
        Element::new(SceneFixture {
            items: items.clone(),
        })
    }

    fn update(items: &mut Self::State, replacement: Self::Action) {
        *items = replacement;
    }
}

fn publication(items: Vec<PaintContributionItem>, raster_scale: RasterScale) -> PaintPublication {
    let mut runtime = AppRuntime::<FixtureApp>::mount(items);
    let style_environment = StyleEnvironment::default();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
    let context =
        SurfaceBuildContext::new(&style_environment, LayoutConstraints::tight(logical_size))
            .with_raster_scale(raster_scale);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
        .paint_publication()
        .clone()
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
}

fn shaped_item(
    resource: ResourceRef,
    foreground: Color,
) -> Result<PaintContributionItem, Box<dyn Error>> {
    Ok(PaintContributionItem::shaped_text_run(
        resource,
        LogicalPoint::new(8.0, 18.0)?,
        foreground,
    )?)
}

fn shaped_item_at(
    resource: ResourceRef,
    origin: LogicalPoint,
    foreground: Color,
) -> Result<PaintContributionItem, Box<dyn Error>> {
    Ok(PaintContributionItem::shaped_text_run(
        resource, origin, foreground,
    )?)
}

struct GlyphProvider {
    resource: ResourceRef,
    loads: Cell<usize>,
    empty: bool,
    available: Cell<bool>,
}

impl GlyphProvider {
    const fn new(resource: ResourceRef) -> Self {
        Self {
            resource,
            loads: Cell::new(0),
            empty: false,
            available: Cell::new(true),
        }
    }

    const fn empty(resource: ResourceRef) -> Self {
        Self {
            resource,
            loads: Cell::new(0),
            empty: true,
            available: Cell::new(true),
        }
    }

    const fn loads(&self) -> usize {
        self.loads.get()
    }

    fn set_available(&self, available: bool) {
        self.available.set(available);
    }
}

impl ResourceProvider for GlyphProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        let ResourceRequest::ShapedTextRun { raster_scale } = request else {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "shaped fixture received a non-shaped request",
            ));
        };
        if resource != &self.resource {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "shaped fixture received an unexpected complete resource reference",
            ));
        }
        if !self.available.get() {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Unavailable,
                "fixture provider intentionally unavailable",
            ));
        }
        let raster: Result<ShapedRunRaster, Box<dyn Error>> = if self.empty {
            ShapedRunRaster::new(
                LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!()),
                0,
                0,
                raster_scale,
                Vec::new(),
            )
            .map_err(|error| Box::new(error) as Box<dyn Error>)
        } else {
            rasterize_a(raster_scale)
        };
        raster.map(ResourcePayload::ShapedTextRun).map_err(|error| {
            ResourceProviderError::new(ResourceProviderErrorKind::Malformed, error.to_string())
        })
    }
}

struct ControlledShapedProvider {
    resource: ResourceRef,
    coverage: u8,
    loads: Cell<usize>,
}

impl ResourceProvider for ControlledShapedProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        let ResourceRequest::ShapedTextRun { raster_scale } = request else {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "controlled shaped fixture received a non-shaped request",
            ));
        };
        if resource != &self.resource {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "controlled shaped fixture received an unexpected resource",
            ));
        }
        let raster = ShapedRunRaster::new(
            LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!()),
            1,
            1,
            raster_scale,
            vec![self.coverage],
        )
        .unwrap_or_else(|_| unreachable!("controlled shaped fixture is valid"));
        Ok(ResourcePayload::ShapedTextRun(raster))
    }
}

struct MixedProvider {
    image: ResourceRef,
    shaped: ResourceRef,
}

impl ResourceProvider for MixedProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        match (resource, request) {
            (resource, ResourceRequest::Image) if resource == &self.image => {
                Ok(ResourcePayload::Image(
                    ImagePayload::new(1, 1, vec![0xFF, 0x20, 0x10, 0xFF])
                        .unwrap_or_else(|_| unreachable!("mixed image fixture is valid")),
                ))
            }
            (resource, ResourceRequest::ShapedTextRun { raster_scale })
                if resource == &self.shaped =>
            {
                Ok(ResourcePayload::ShapedTextRun(
                    ShapedRunRaster::new(
                        LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!()),
                        1,
                        1,
                        raster_scale,
                        vec![u8::MAX],
                    )
                    .unwrap_or_else(|_| unreachable!("mixed shaped fixture is valid")),
                ))
            }
            _ => Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "mixed fixture received an unexpected complete resource request",
            )),
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the bundled font's finite pixel bounds and clamped coverage are narrowed at the test raster buffer boundary"
)]
fn rasterize_a(raster_scale: RasterScale) -> Result<ShapedRunRaster, Box<dyn Error>> {
    let font = FontArc::try_from_vec(FONT_BYTES.to_vec())?;
    let px_scale = 24.0 * raster_scale.get();
    let glyph = font
        .outline_glyph(
            font.glyph_id('A')
                .with_scale_and_position(px_scale, point(0.0, 0.0)),
        )
        .ok_or("fixture glyph has no outline")?;
    let bounds = glyph.px_bounds();
    let width = bounds.width().ceil() as u32;
    let height = bounds.height().ceil() as u32;
    let mut alpha = vec![0_u8; (width as usize).saturating_mul(height as usize)];
    glyph.draw(|x, y, coverage| {
        let index = y as usize * width as usize + x as usize;
        if let Some(pixel) = alpha.get_mut(index) {
            *pixel = (coverage * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    });
    Ok(ShapedRunRaster::new(
        LogicalPoint::new(
            bounds.min.x / raster_scale.get(),
            bounds.min.y / raster_scale.get(),
        )?,
        width,
        height,
        raster_scale,
        alpha,
    )?)
}

fn renderer_or_adapterless() -> Result<Option<Renderer>, Box<dyn Error>> {
    match block_on(Renderer::request(RendererOptions::new())) {
        Ok(renderer) => Ok(Some(renderer)),
        Err(RendererInitError::AdapterUnavailable {
            requested,
            compatible_surface_required,
            detail,
        }) => {
            eprintln!(
                "native wgpu shaped-text proof unavailable under {requested:?}; structured adapter failure: {detail}"
            );
            assert_eq!(requested, BackendSelection::AllNative);
            assert!(!compatible_surface_required);
            assert!(!detail.is_empty());
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "this real-GPU corpus keeps scale, cache, foreground, observation, and recovery assertions together as one proof record"
)]
fn shaped_text_realization_is_scale_qualified_and_foreground_is_scene_owned()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
    let provider = GlyphProvider::new(resource.clone());
    let white = publication(
        vec![shaped_item(resource.clone(), Color::WHITE)?],
        RasterScale::ONE,
    );
    let first = renderer.render_offscreen_publication(&white, &provider)?;
    assert_eq!(
        first.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(
        first.readback().extent(),
        runenui_render_wgpu::OffscreenExtent::new(32, 24)?
    );
    assert!(
        first
            .readback()
            .rgba8_srgb()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] != 0)
    );
    let observation = first.observation();
    assert_eq!(
        observation.physical_extent(),
        Some(runenui_render_wgpu::OffscreenExtent::new(32, 24)?)
    );
    assert_eq!(
        observation.target_format(),
        Some(wgpu::TextureFormat::Rgba8UnormSrgb)
    );
    assert_eq!(observation.raster_scale(), RasterScale::ONE);
    assert!(observation.target_generation().is_some());
    assert!(observation.adapter_name().is_some());
    assert!(observation.backend().is_some());
    assert_eq!(
        observation.render_result(),
        PublicationStageResult::Succeeded
    );
    assert_eq!(
        observation.readback_result(),
        PublicationStageResult::Succeeded
    );
    assert_eq!(
        observation.present_result(),
        PublicationStageResult::NotAttempted
    );
    assert_eq!(observation.resource_observations().len(), 1);
    assert_eq!(
        observation.resource_observations()[0].cache_outcome(),
        ResourceCacheOutcome::Realized
    );
    assert_eq!(provider.loads(), 1);

    let current = renderer.render_offscreen_publication(&white, &provider)?;
    assert_eq!(
        current.update_plan().mode(),
        PublicationUpdateMode::AlreadyCurrent
    );
    assert_eq!(
        provider.loads(),
        1,
        "same complete ref and exact scale reuse the realization"
    );
    assert_eq!(
        current.observation().resource_observations()[0].cache_outcome(),
        ResourceCacheOutcome::Reused
    );
    assert_eq!(
        current.readback().rgba8_srgb(),
        first.readback().rgba8_srgb()
    );

    let red = publication(
        vec![shaped_item(resource.clone(), Color::rgb(0xFF, 0x20, 0x10))?],
        RasterScale::ONE,
    );
    let red_output = renderer.render_offscreen_publication(&red, &provider)?;
    assert_eq!(
        provider.loads(),
        1,
        "foreground is not part of resource identity"
    );
    assert_ne!(
        red_output.readback().rgba8_srgb(),
        first.readback().rgba8_srgb()
    );

    let two = RasterScale::new(2.0)?;
    let scaled = publication(vec![shaped_item(resource, Color::WHITE)?], two);
    let scaled_output = renderer.render_offscreen_publication(&scaled, &provider)?;
    assert_eq!(
        scaled_output.readback().extent(),
        runenui_render_wgpu::OffscreenExtent::new(64, 48)?
    );
    assert_eq!(
        provider.loads(),
        2,
        "same ref at another exact scale re-realizes"
    );
    assert!(renderer.discard_resource_cache());
    provider.set_available(false);
    let failed = renderer.render_offscreen_publication(&scaled, &provider);
    assert!(matches!(
        failed,
        Err(PublicationRenderError::Resource {
            item_index: 0,
            error: ResourceResolveError::Provider(ref error),
        }) if error.kind() == ResourceProviderErrorKind::Unavailable
    ));
    provider.set_available(true);
    let rebuilt = renderer.render_offscreen_publication(&scaled, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(
        provider.loads(),
        4,
        "cache loss reloads the exact scale lineage"
    );
    Ok(())
}

#[test]
fn empty_shaped_coverage_is_valid_and_never_creates_zero_texture() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
    let provider = GlyphProvider::empty(resource.clone());
    let publication = publication(vec![shaped_item(resource, Color::WHITE)?], RasterScale::ONE);
    let output = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(provider.loads(), 1);
    assert!(
        output
            .readback()
            .rgba8_srgb()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel == &[0, 0, 0, 0])
    );
    Ok(())
}

#[test]
fn shaped_text_preserves_origin_transform_clips_and_order() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
    let provider = GlyphProvider::new(resource.clone());
    let transform = LogicalTransform::try_new(1.0, 0.0, 0.0, 1.0, 3.0, 0.0)?;
    let shaped = shaped_item(resource.clone(), Color::rgb(0x40, 0xA0, 0xFF))?
        .with_transform(transform)
        .with_clip(ContributionClip::identity(SceneShape::rect(rect(
            8.0, 8.0, 10.0, 12.0,
        ))));
    let transformed_publication = publication(
        vec![
            PaintContributionItem::fill_rect(rect(0.0, 0.0, 32.0, 24.0), Color::BLACK),
            shaped,
        ],
        RasterScale::ONE,
    );
    let output = renderer.render_offscreen_publication(&transformed_publication, &provider)?;
    assert!(
        output
            .readback()
            .rgba8_srgb()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[2] > 0)
    );
    assert_eq!(provider.loads(), 1);
    let singular = LogicalTransform::try_new(1.0, 0.0, 0.0, 0.0, 0.0, 0.0)?;
    let singular_publication = publication(
        vec![
            PaintContributionItem::fill_rect(rect(0.0, 0.0, 32.0, 24.0), Color::BLACK),
            shaped_item(resource, Color::WHITE)?.with_transform(singular),
        ],
        RasterScale::ONE,
    );
    let singular_output =
        renderer.render_offscreen_publication(&singular_publication, &provider)?;
    assert!(
        singular_output
            .readback()
            .rgba8_srgb()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| pixel == &[0, 0, 0, 255])
    );
    Ok(())
}

#[test]
fn shaped_coverage_multiplies_foreground_alpha_and_item_opacity() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
    let provider = ControlledShapedProvider {
        resource: resource.clone(),
        coverage: 0x80,
        loads: Cell::new(0),
    };
    let item = shaped_item_at(
        resource,
        LogicalPoint::new(4.0, 4.0)?,
        Color::rgba(0xE0, 0x40, 0x20, 0xC0),
    )?
    .with_opacity(runenui_core::SceneOpacity::new(0.5)?);
    let output = renderer
        .render_offscreen_publication(&publication(vec![item], RasterScale::ONE), &provider)?;
    let effective_alpha = (f64::from(0xC0_u8) / 255.0) * 0.5 * (f64::from(0x80_u8) / 255.0);
    let expected = [
        linear_to_srgb8(srgb8_to_linear(0xE0) * effective_alpha),
        linear_to_srgb8(srgb8_to_linear(0x40) * effective_alpha),
        linear_to_srgb8(srgb8_to_linear(0x20) * effective_alpha),
        alpha_to_u8(effective_alpha),
    ];
    let actual = pixel(output.readback(), 4, 4);
    assert_pixel_within(
        "coverage × foreground alpha × item opacity",
        actual,
        expected,
        1,
    );
    assert_eq!(provider.loads.get(), 1);
    Ok(())
}

#[test]
fn mixed_publication_preserves_fill_stroke_image_and_shaped_order() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let image = ResourceRef::new(ResourceKind::Image);
    let shaped = ResourceRef::new(ResourceKind::ShapedTextRun);
    let provider = MixedProvider {
        image: image.clone(),
        shaped: shaped.clone(),
    };
    let publication = publication(
        vec![
            PaintContributionItem::fill_rect(rect(0.0, 0.0, 32.0, 24.0), Color::BLACK),
            PaintContributionItem::stroke_rect(
                rect(1.0, 1.0, 6.0, 6.0),
                Color::rgb(0x20, 0xD0, 0x50),
                LogicalLength::from(1_u16),
            ),
            PaintContributionItem::image(image, rect(3.0, 3.0, 6.0, 6.0))?,
            shaped_item_at(
                shaped,
                LogicalPoint::new(5.0, 5.0)?,
                Color::rgb(0x30, 0x70, 0xF0),
            )?,
        ],
        RasterScale::ONE,
    );
    let output = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(pixel(output.readback(), 31, 23), [0, 0, 0, 255]);
    assert_eq!(
        pixel(output.readback(), 0, 0),
        [0x20, 0xD0, 0x50, 0xFF],
        "the stroke is visible before later resource items overlap it"
    );
    assert_eq!(
        pixel(output.readback(), 4, 4),
        [0xFF, 0x20, 0x10, 0xFF],
        "the image follows and covers the earlier stroke"
    );
    assert_eq!(
        pixel(output.readback(), 5, 5),
        [0x30, 0x70, 0xF0, 0xFF],
        "the shaped run follows and covers the earlier image"
    );
    assert_eq!(output.observation().resource_observations().len(), 2);
    Ok(())
}

#[test]
fn shaped_provider_failure_preserves_target_and_observation_state() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
    let provider = GlyphProvider::new(resource.clone());
    let valid = publication(
        vec![shaped_item(resource.clone(), Color::WHITE)?],
        RasterScale::ONE,
    );
    let first = renderer.render_offscreen_publication(&valid, &provider)?;
    let first_generation = first.target_generation();
    let first_pixels = first.readback().rgba8_srgb().to_vec();
    let unavailable = FailingProvider {
        resource: resource.clone(),
    };
    let failed = renderer.render_offscreen_publication(
        &publication(
            vec![shaped_item(resource, Color::WHITE)?],
            RasterScale::new(2.0)?,
        ),
        &unavailable,
    );
    let Err(error) = failed else {
        return Err(std::io::Error::other(
            "the new scale rendered instead of failing during shaped preflight",
        )
        .into());
    };
    assert!(!error.to_string().contains("image"));
    assert!(matches!(
        error,
        PublicationRenderError::Resource {
            item_index: 0,
            error: ResourceResolveError::Provider(ref error),
        } if error.kind() == ResourceProviderErrorKind::Unavailable
    ));
    let failed_observation = renderer
        .last_observation()
        .ok_or_else(|| std::io::Error::other("failed publication did not remain observable"))?;
    assert_eq!(
        failed_observation.update_mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(
        failed_observation.render_result(),
        PublicationStageResult::NotAttempted
    );
    assert_eq!(
        failed_observation.readback_result(),
        PublicationStageResult::NotAttempted
    );
    assert_eq!(
        failed_observation.resource_observations()[0].cache_outcome(),
        ResourceCacheOutcome::Failed
    );
    assert_eq!(
        failed_observation.target_generation(),
        None,
        "the failed publication has a different physical extent and did not replace A's target"
    );

    let retained = renderer.render_offscreen_publication(&valid, &provider)?;
    assert_eq!(
        retained.update_plan().mode(),
        PublicationUpdateMode::AlreadyCurrent
    );
    assert_eq!(retained.target_generation(), first_generation);
    assert_eq!(retained.readback().rgba8_srgb(), first_pixels.as_slice());
    assert_eq!(
        provider.loads(),
        1,
        "failed preflight did not reload or mutate A"
    );
    Ok(())
}

#[test]
fn shaped_text_matches_checked_in_golden() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let resource = ResourceRef::new(ResourceKind::ShapedTextRun);
    let provider = GlyphProvider::new(resource.clone());
    let output = renderer.render_offscreen_publication(
        &publication(vec![shaped_item(resource, Color::WHITE)?], RasterScale::ONE),
        &provider,
    )?;
    let golden =
        image::load_from_memory(include_bytes!("fixtures/golden/shaped_text_1x.png"))?.to_rgba8();
    compare_exact_golden(
        &renderer,
        &output,
        &golden,
        RasterScale::ONE,
        "shaped_text_1x",
    );
    Ok(())
}

#[allow(
    dead_code,
    reason = "the record is emitted as bounded structured debug evidence"
)]
#[derive(Clone, Copy, Debug)]
struct GoldenMismatch {
    x: u32,
    y: u32,
    expected: [u8; 4],
    actual: [u8; 4],
}

#[allow(
    dead_code,
    reason = "the record is emitted as bounded structured debug evidence"
)]
#[derive(Debug)]
struct GoldenComparisonRecord {
    adapter: String,
    backend: wgpu::Backend,
    target_format: wgpu::TextureFormat,
    raster_scale: RasterScale,
    dimensions: (u32, u32),
    mismatch_count: usize,
    first_mismatch: Option<GoldenMismatch>,
    bounded_mismatches: Vec<GoldenMismatch>,
}

fn compare_exact_golden(
    renderer: &Renderer,
    output: &runenui_render_wgpu::OffscreenPublicationReadback,
    golden: &image::RgbaImage,
    raster_scale: RasterScale,
    corpus: &str,
) {
    let extent = output.readback().extent();
    assert_eq!(
        golden.width(),
        extent.width(),
        "{corpus}: golden width differs"
    );
    assert_eq!(
        golden.height(),
        extent.height(),
        "{corpus}: golden height differs"
    );
    let expected = golden.as_raw().as_chunks::<4>().0;
    let actual = output.readback().rgba8_srgb().as_chunks::<4>().0;
    let mut mismatch_count = 0_usize;
    let mut first_mismatch = None;
    let mut bounded_mismatches = Vec::new();
    for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        if expected == actual {
            continue;
        }
        mismatch_count += 1;
        let mismatch = GoldenMismatch {
            x: u32::try_from(index % extent.width() as usize).unwrap_or(u32::MAX),
            y: u32::try_from(index / extent.width() as usize).unwrap_or(u32::MAX),
            expected: *expected,
            actual: *actual,
        };
        if first_mismatch.is_none() {
            first_mismatch = Some(mismatch);
        }
        if bounded_mismatches.len() < 8 {
            bounded_mismatches.push(mismatch);
        }
    }
    let record = GoldenComparisonRecord {
        adapter: renderer.diagnostics().adapter_info().name.clone(),
        backend: renderer.diagnostics().adapter_info().backend,
        target_format: output.readback().format(),
        raster_scale,
        dimensions: (extent.width(), extent.height()),
        mismatch_count,
        first_mismatch,
        bounded_mismatches,
    };
    eprintln!("GOLDEN COMPARISON {corpus}: {record:?}; policy=exact, tolerance=0");
    assert_eq!(
        record.mismatch_count, 0,
        "{corpus}: actual wgpu readback differs from the checked-in golden; bounded diagnostic={record:?}"
    );
}

fn pixel(readback: &runenui_render_wgpu::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
    readback.rgba8_srgb()[index..index + 4]
        .try_into()
        .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
}

fn assert_pixel_within(label: &str, actual: [u8; 4], expected: [u8; 4], tolerance: u8) {
    for (channel, (actual, expected)) in actual.into_iter().zip(expected).enumerate() {
        assert!(
            actual.abs_diff(expected) <= tolerance,
            "{label}: channel {channel} expected {expected}±{tolerance}, got {actual}"
        );
    }
}

fn srgb8_to_linear(value: u8) -> f64 {
    let encoded = f64::from(value) / 255.0;
    if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the normalized diagnostic channel is clamped to one byte before the test-only comparison record"
)]
fn linear_to_srgb8(value: f64) -> u8 {
    let encoded = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055f64.mul_add(value.powf(1.0 / 2.4), -0.055)
    };
    (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the bounded normalized alpha diagnostic is converted to one byte"
)]
fn alpha_to_u8(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

struct FailingProvider {
    resource: ResourceRef,
}

impl ResourceProvider for FailingProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        assert_eq!(resource, &self.resource);
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Unavailable,
            "fixture provider intentionally unavailable",
        ))
    }
}

struct ThreadWake(thread::Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}
