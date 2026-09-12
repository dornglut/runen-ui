#![allow(refining_impl_trait)]

use core::{future::Future, pin::pin, task::Poll};
use std::{
    cell::Cell,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    task::Context,
};

use runenui_core::{
    Brush, Color, ContributionClip, EdgeInsets, Element, FontFamily, FontFamilyName,
    GenericFontFamily, LogicalLength, LogicalRect, LogicalSize, LogicalTransform, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionItem, SceneOpacity, SceneShape,
    StyleEnvironment, Typography, UiApp, View, Widget, WidgetMeasure, WidgetMeasureInput, children,
    column, text,
};
use runenui_render_wgpu::{
    BackendSelection, OffscreenPublicationReadback, Renderer, RendererInitError, RendererOptions,
    ResourceCacheOutcome, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{AppRuntime, RasterScale, SurfaceBuildContext, SurfacePublication};

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));
const ARABIC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/RunenUIFixtureArabic-Regular.ttf"
));
const DEVANAGARI: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/RunenUIFixtureDevanagari-Regular.ttf"
));

const BACKGROUND: Color = Color::rgb(22, 26, 34);
const FOREGROUND: Color = Color::rgb(238, 240, 246);
const SAMPLE: &str = "AV  O8  سلام  क्षि";
const WRAP_SAMPLE: &str =
    "Responsive production text AV O8 سلام क्षि wraps through the runtime-owned layout path.";

struct VisualApp;

impl UiApp for VisualApp {
    type State = String;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(label: &Self::State) -> Element<Self::Action> {
        column(children![
            text(label.clone())
                .id("m8d.label")
                .typography(typography(16)),
            text(SAMPLE).id("m8d.size16").typography(typography(16)),
            text(SAMPLE).id("m8d.size24").typography(typography(24)),
            text(SAMPLE).id("m8d.size32").typography(typography(32)),
            text(SAMPLE).id("m8d.size48").typography(typography(48)),
            text(WRAP_SAMPLE).id("m8d.wrap").typography(typography(24)),
            Element::new(CompositionProof),
        ])
        .background(BACKGROUND)
        .foreground(FOREGROUND)
        .padding(EdgeInsets::all(LogicalLength::from(12_u8)))
        .gap(6_u8)
        .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn typography(size: u16) -> Typography {
    Typography::new(
        FontFamily::generic(GenericFontFamily::SansSerif),
        LogicalLength::from(size),
    )
}

#[derive(Debug)]
struct CompositionProof;

impl Widget<()> for CompositionProof {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(LogicalLength::from(120_u8), LogicalLength::from(42_u8))
    }

    fn paint(&self, (): &Self::State, _: PaintContributionContext) -> PaintContribution {
        let rect = LogicalRect::try_new(0.0, 0.0, 104.0, 34.0)
            .unwrap_or_else(|_| unreachable!("visual fixture rect is valid"));
        let clip_rect = LogicalRect::try_new(12.0, 4.0, 72.0, 25.0)
            .unwrap_or_else(|_| unreachable!("visual fixture clip is valid"));
        let transform = LogicalTransform::translation(8.0, 3.0)
            .unwrap_or_else(|_| unreachable!("visual fixture transform is finite"));
        let opacity = SceneOpacity::new(0.68)
            .unwrap_or_else(|_| unreachable!("visual fixture opacity is valid"));
        PaintContribution::single(
            PaintContributionItem::fill(
                SceneShape::rect(rect),
                Brush::solid(Color::rgb(88, 146, 224)),
            )
            .with_transform(transform)
            .with_clip(ContributionClip::identity(SceneShape::rect(clip_rect)))
            .with_opacity(opacity),
        )
    }
}

fn visual_runtime(label: &str) -> AppRuntime<VisualApp> {
    let mut runtime = AppRuntime::<VisualApp>::mount(label.to_owned());
    for bytes in [CANTARELL, ARABIC, DEVANAGARI] {
        assert!(runtime.register_text_font_bytes(bytes.to_vec()).is_ok());
    }
    let families = [
        FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!()),
        FontFamilyName::new("RunenUI Fixture Arabic").unwrap_or_else(|_| unreachable!()),
        FontFamilyName::new("RunenUI Fixture Devanagari").unwrap_or_else(|_| unreachable!()),
    ];
    assert!(
        runtime
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &families)
            .is_ok()
    );
    runtime
}

fn publish(
    runtime: &mut AppRuntime<VisualApp>,
    size: LogicalSize,
    scale: RasterScale,
) -> SurfacePublication {
    let styles = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::tight(&styles, size).with_raster_scale(scale))
        .unwrap_or_else(|_| unreachable!("controlled visual publication is admitted"))
}

fn shaped_refs(publication: &SurfacePublication) -> Vec<runenui_core::ResourceRef> {
    publication
        .paint_publication()
        .scene()
        .items()
        .iter()
        .filter_map(|item| {
            item.primitive()
                .as_shaped_text_run()
                .map(|run| run.resource_ref().clone())
        })
        .collect()
}

fn authored_height(publication: &SurfacePublication, authored_id: &str) -> f32 {
    publication
        .frame()
        .nodes()
        .iter()
        .find(|node| {
            node.authored_id()
                .is_some_and(|id| id.as_str() == authored_id)
        })
        .unwrap_or_else(|| unreachable!("visual corpus authored node is published"))
        .bounds()
        .height()
}

#[derive(Default)]
struct ExternalImagesOnly {
    loads: Cell<usize>,
}

impl ResourceProvider for ExternalImagesOnly {
    fn load(
        &self,
        _: &runenui_core::ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        self.loads.set(self.loads.get() + 1);
        match request {
            ResourceRequest::Image => Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Missing,
                "M8D visual corpus publishes no external images",
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
            eprintln!("M8D visual evidence skipped: {detail}");
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

struct Panel {
    name: &'static str,
    publication: SurfacePublication,
    readback: OffscreenPublicationReadback,
}

fn render_panel(
    renderer: &mut Renderer,
    name: &'static str,
    publication: SurfacePublication,
) -> Result<Panel, Box<dyn std::error::Error>> {
    let provider = ExternalImagesOnly::default();
    let readback =
        renderer.render_offscreen_publication(publication.paint_publication(), &provider)?;
    assert_eq!(
        provider.loads.get(),
        0,
        "text realization is publication-owned"
    );
    assert!(!shaped_refs(&publication).is_empty());
    assert!(!readback.observation().resource_observations().is_empty());
    assert!(
        readback
            .readback()
            .rgba8_srgb()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| {
                pixel[0] != BACKGROUND.red()
                    || pixel[1] != BACKGROUND.green()
                    || pixel[2] != BACKGROUND.blue()
            }),
        "real renderer output must contain more than the root background"
    );
    Ok(Panel {
        name,
        publication,
        readback,
    })
}

fn evidence_dir() -> Option<PathBuf> {
    std::env::var_os("RUNENUI_M8D_EVIDENCE_DIR").map(PathBuf::from)
}

fn write_evidence(
    directory: &Path,
    renderer: &Renderer,
    panels: &[Panel],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(directory)?;
    let gap = 8_u32;
    let first_row_width = panels[0].readback.readback().extent().width()
        + gap
        + panels[1].readback.readback().extent().width();
    let second_row_width = panels[2].readback.readback().extent().width()
        + gap
        + panels[3].readback.readback().extent().width();
    let width = first_row_width.max(second_row_width);
    let first_row_height = panels[0]
        .readback
        .readback()
        .extent()
        .height()
        .max(panels[1].readback.readback().extent().height());
    let second_row_height = panels[2]
        .readback
        .readback()
        .extent()
        .height()
        .max(panels[3].readback.readback().extent().height());
    let height = first_row_height + gap + second_row_height;
    let mut sheet = vec![0_u8; usize::try_from(width)? * usize::try_from(height)? * 4];
    for pixel in sheet.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&[
            BACKGROUND.red(),
            BACKGROUND.green(),
            BACKGROUND.blue(),
            BACKGROUND.alpha(),
        ]);
    }
    blit(&mut sheet, width, &panels[0].readback, 0, 0)?;
    blit(
        &mut sheet,
        width,
        &panels[1].readback,
        panels[0].readback.readback().extent().width() + gap,
        0,
    )?;
    blit(
        &mut sheet,
        width,
        &panels[2].readback,
        0,
        first_row_height + gap,
    )?;
    blit(
        &mut sheet,
        width,
        &panels[3].readback,
        panels[2].readback.readback().extent().width() + gap,
        first_row_height + gap,
    )?;
    image::save_buffer(
        directory.join("m8d-production-text-contact-sheet.png"),
        &sheet,
        width,
        height,
        image::ColorType::Rgba8,
    )?;

    let mut manifest = String::new();
    manifest.push_str("M8D real-wgpu production text evidence\n\n");
    manifest.push_str("Panel order: narrow 1x | narrow 2x; wide 1x | wide 2x.\n");
    manifest.push_str(
        "Every panel uses ordinary runtime layout/text/publication and renderer SDF/MSDF realization.\n",
    );
    manifest.push_str(
        "Text corpus includes Latin AV/O8, Arabic سلام, Devanagari क्षि, and 16/24/32/48 logical-pixel sizes.\n",
    );
    manifest.push_str(
        "The blue sample exercises accepted transform + clip + opacity/color composition.\n",
    );
    manifest.push_str(
        "The CI evidence command also runs shaped_text.rs, whose intrinsic COLR/SVG/bitmap fixtures must remain explicit diagnostics.\n\n",
    );
    writeln!(
        manifest,
        "adapter={:?}",
        renderer.diagnostics().adapter_info()
    )?;
    for panel in panels {
        let extent = panel.readback.readback().extent();
        writeln!(
            manifest,
            "{}: physical={}x{} shaped_refs={} resource_observations={}",
            panel.name,
            extent.width(),
            extent.height(),
            shaped_refs(&panel.publication).len(),
            panel.readback.observation().resource_observations().len(),
        )?;
    }
    fs::write(directory.join("m8d-production-text-evidence.txt"), manifest)?;
    Ok(())
}

fn blit(
    sheet: &mut [u8],
    sheet_width: u32,
    panel: &OffscreenPublicationReadback,
    x: u32,
    y: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let extent = panel.readback().extent();
    let source = panel.readback().rgba8_srgb();
    let sheet_width = usize::try_from(sheet_width)?;
    let panel_width = usize::try_from(extent.width())?;
    let x = usize::try_from(x)?;
    let y = usize::try_from(y)?;
    for row in 0..usize::try_from(extent.height())? {
        let source_start = row * panel_width * 4;
        let source_end = source_start + panel_width * 4;
        let target_start = ((y + row) * sheet_width + x) * 4;
        let target_end = target_start + panel_width * 4;
        sheet[target_start..target_end].copy_from_slice(&source[source_start..source_end]);
    }
    Ok(())
}

#[test]
fn real_wgpu_m8d_contact_sheet_covers_responsive_multiscript_text()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(mut renderer) = renderer_or_skip()? else {
        return Ok(());
    };
    let narrow = LogicalSize::try_new(280.0, 520.0)?;
    let wide = LogicalSize::try_new(520.0, 520.0)?;
    let one = RasterScale::ONE;
    let two = RasterScale::new(2.0)?;

    let mut narrow_runtime = visual_runtime("narrow production corpus");
    let narrow_one_publication = publish(&mut narrow_runtime, narrow, one);
    let narrow_one = render_panel(&mut renderer, "narrow 1x", narrow_one_publication)?;
    let narrow_refs = shaped_refs(&narrow_one.publication);
    assert!(renderer.discard_resource_cache());
    let provider = ExternalImagesOnly::default();
    let retry = renderer
        .render_offscreen_publication(narrow_one.publication.paint_publication(), &provider)?;
    assert_eq!(provider.loads.get(), 0);
    assert!(
        retry
            .observation()
            .resource_observations()
            .iter()
            .any(|observation| observation.cache_outcome() == ResourceCacheOutcome::Realized)
    );
    let narrow_two_publication = publish(&mut narrow_runtime, narrow, two);
    let narrow_two = render_panel(&mut renderer, "narrow 2x", narrow_two_publication)?;
    assert_eq!(shaped_refs(&narrow_two.publication), narrow_refs);
    assert_eq!(
        narrow_two.publication.frame(),
        narrow_one.publication.frame(),
        "raster scale must not change logical layout"
    );

    let mut wide_runtime = visual_runtime("wide production corpus");
    let wide_one_publication = publish(&mut wide_runtime, wide, one);
    let wide_one = render_panel(&mut renderer, "wide 1x", wide_one_publication)?;
    let wide_two_publication = publish(&mut wide_runtime, wide, two);
    let wide_two = render_panel(&mut renderer, "wide 2x", wide_two_publication)?;
    assert_eq!(
        shaped_refs(&wide_two.publication),
        shaped_refs(&wide_one.publication)
    );
    assert_eq!(wide_two.publication.frame(), wide_one.publication.frame());
    assert!(
        authored_height(&narrow_one.publication, "m8d.wrap")
            > authored_height(&wide_one.publication, "m8d.wrap"),
        "the exact wrap node must become taller under the narrower production constraint"
    );

    let panels = [narrow_one, narrow_two, wide_one, wide_two];
    if let Some(directory) = evidence_dir() {
        write_evidence(&directory, &renderer, &panels)?;
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
