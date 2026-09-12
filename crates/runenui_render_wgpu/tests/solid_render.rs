#![allow(refining_impl_trait)]

use core::{error::Error, future::Future, pin::pin, task::Poll};
use std::{
    sync::Arc,
    task::{Context, Wake, Waker},
    thread,
};

use runenui_core::{
    Brush, Color, ContributionClip, DropShadow, Element, GradientStop, GradientStops,
    ImageDescriptor, ImageIntrinsicSize, ImageMapping, ImagePaintDescriptor, LinearGradient,
    LogicalLength, LogicalPoint, LogicalRect, LogicalSize, LogicalTransform, NoHostProtocol,
    PaintContribution, PaintContributionContext, PaintContributionEntry, PaintContributionGroup,
    PaintContributionItem, PathFillRule, PathVerb, RadialGradient, ResourceKind, ResourceRef,
    SceneLayer, SceneOpacity, ScenePath, SceneShape, StrokeCap, StrokeJoin, StrokeStyle,
    StyleEnvironment, UiApp, UnitInterval, Widget, WidgetMeasure,
};
use runenui_render_wgpu::{
    BackendSelection, ImagePayload, PublicationUpdateMode, Renderer, RendererInitError,
    RendererOptions, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PaintPublication, RasterScale, SurfaceBuildContext,
};

const SURFACE_WIDTH: u16 = 64;
const SURFACE_HEIGHT: u16 = 48;

#[derive(Clone, Debug)]
struct SceneFixture {
    items: Vec<PaintContributionItem>,
}

impl Widget<Vec<PaintContributionItem>> for SceneFixture {
    type State = Vec<PaintContributionItem>;

    fn create_state(&self) -> Self::State {
        self.items.clone()
    }

    fn measure(&self, _: &Self::State, _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::from(SURFACE_WIDTH),
            LogicalLength::from(SURFACE_HEIGHT),
        )
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

#[derive(Clone, Debug)]
struct GroupSceneFixture {
    entries: Vec<PaintContributionEntry>,
}

impl Widget<Vec<PaintContributionEntry>> for GroupSceneFixture {
    type State = Vec<PaintContributionEntry>;

    fn create_state(&self) -> Self::State {
        self.entries.clone()
    }

    fn measure(&self, _: &Self::State, _: runenui_core::WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::measured(
            LogicalLength::from(SURFACE_WIDTH),
            LogicalLength::from(SURFACE_HEIGHT),
        )
    }

    fn paint(&self, entries: &Self::State, _: PaintContributionContext) -> PaintContribution {
        PaintContribution::from_entries(entries.clone())
    }
}

struct GroupFixtureApp;

impl UiApp for GroupFixtureApp {
    type State = Vec<PaintContributionEntry>;
    type Action = Vec<PaintContributionEntry>;
    type HostProtocol = NoHostProtocol;

    fn root(entries: &Self::State) -> Element<Self::Action> {
        Element::new(GroupSceneFixture {
            entries: entries.clone(),
        })
    }

    fn update(entries: &mut Self::State, replacement: Self::Action) {
        *entries = replacement;
    }
}

struct NoResources;

impl ResourceProvider for NoResources {
    fn load(
        &self,
        _: &ResourceRef,
        _: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        Err(ResourceProviderError::new(
            ResourceProviderErrorKind::Malformed,
            "solid-render proof unexpectedly requested a resource",
        ))
    }
}

struct SingleImageProvider {
    resource: ResourceRef,
    payload: ImagePayload,
}

impl SingleImageProvider {
    fn new(resource: ResourceRef) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            resource,
            payload: ImagePayload::new(1, 1, vec![0xFF, 0x00, 0x00, 0xFF])?,
        })
    }
}

impl ResourceProvider for SingleImageProvider {
    fn load(
        &self,
        resource: &ResourceRef,
        request: ResourceRequest,
    ) -> Result<ResourcePayload, ResourceProviderError> {
        if resource != &self.resource || request != ResourceRequest::Image {
            return Err(ResourceProviderError::new(
                ResourceProviderErrorKind::Malformed,
                "atomic-group proof requested an unexpected resource",
            ));
        }
        Ok(ResourcePayload::Image(self.payload.clone()))
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
    LogicalRect::try_new(x, y, width, height)
        .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
}

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("fixture point is finite"))
}

fn path(verbs: Vec<PathVerb>) -> SceneShape {
    SceneShape::path(
        ScenePath::new(verbs, PathFillRule::NonZero)
            .unwrap_or_else(|_| unreachable!("fixture path is structurally valid")),
    )
}

fn path_rect(rect: LogicalRect) -> SceneShape {
    path(vec![
        PathVerb::MoveTo(point(rect.x(), rect.y())),
        PathVerb::LineTo(point(rect.max_x(), rect.y())),
        PathVerb::LineTo(point(rect.max_x(), rect.max_y())),
        PathVerb::LineTo(point(rect.x(), rect.max_y())),
        PathVerb::Close,
    ])
}

fn gradient_stops(entries: &[(f32, Color)]) -> GradientStops {
    GradientStops::new(
        entries
            .iter()
            .map(|(offset, color)| {
                GradientStop::new(
                    UnitInterval::new(*offset)
                        .unwrap_or_else(|_| unreachable!("fixture stop offset is valid")),
                    *color,
                )
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| unreachable!("fixture gradient stops are valid"))
}

fn image_item(resource: ResourceRef, destination: LogicalRect) -> PaintContributionItem {
    let descriptor = ImageDescriptor::new(
        resource,
        ImageIntrinsicSize::new(1, 1)
            .unwrap_or_else(|| unreachable!("fixture image extent is non-zero")),
    )
    .unwrap_or_else(|_| unreachable!("fixture resource has image kind"));
    let paint = ImagePaintDescriptor::new(descriptor, destination, ImageMapping::default())
        .unwrap_or_else(|_| unreachable!("fixture image mapping is valid"));
    PaintContributionItem::image(paint)
}

fn publication(items: Vec<PaintContributionItem>) -> PaintPublication {
    let mut runtime = AppRuntime::<FixtureApp>::mount(items);
    let environment = StyleEnvironment::default();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::tight(logical_size))
        .with_raster_scale(
            RasterScale::new(1.0).unwrap_or_else(|_| unreachable!("fixture raster scale is valid")),
        );
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("fixture publication is admitted"))
        .paint_publication()
        .clone()
}

fn grouped_publication(entries: Vec<PaintContributionEntry>) -> PaintPublication {
    let mut runtime = AppRuntime::<GroupFixtureApp>::mount(entries);
    let environment = StyleEnvironment::default();
    let logical_size = LogicalSize::try_new(f32::from(SURFACE_WIDTH), f32::from(SURFACE_HEIGHT))
        .unwrap_or_else(|_| unreachable!("fixture surface extent is valid"));
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::tight(logical_size))
        .with_raster_scale(
            RasterScale::new(1.0).unwrap_or_else(|_| unreachable!("fixture raster scale is valid")),
        );
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("grouped fixture publication is admitted"))
        .paint_publication()
        .clone()
}

fn atomic_group_publication(image_resource: ResourceRef, half: SceneOpacity) -> PaintPublication {
    let contracted = PaintContributionGroup::new(vec![
        PaintContributionItem::fill(
            SceneShape::rect(rect(0.0, 0.0, 32.0, 20.0)),
            Brush::solid(Color::rgb(0xFF, 0x00, 0x00)),
        )
        .with_layer(SceneLayer::new(-1))
        .into(),
        PaintContributionItem::fill(
            SceneShape::rect(rect(12.0, 0.0, 24.0, 20.0)),
            Brush::solid(Color::rgb(0x00, 0x00, 0xFF)),
        )
        .with_layer(SceneLayer::new(1))
        .into(),
    ])
    .with_clip(ContributionClip::identity(SceneShape::rect(rect(
        4.0, 0.0, 28.0, 20.0,
    ))))
    .with_opacity(half);
    let outside = PaintContributionItem::fill(
        SceneShape::rect(rect(20.0, 4.0, 8.0, 8.0)),
        Brush::solid(Color::rgb(0x00, 0xFF, 0x00)),
    );
    let inner = PaintContributionGroup::new(vec![
        PaintContributionItem::fill(
            SceneShape::rect(rect(24.0, 24.0, 12.0, 12.0)),
            Brush::solid(Color::WHITE),
        )
        .into(),
    ])
    .with_opacity(half);
    let nested = PaintContributionGroup::new(vec![inner.into()]).with_opacity(half);
    let image_group = PaintContributionGroup::new(vec![
        image_item(image_resource, rect(44.0, 24.0, 16.0, 16.0)).into(),
    ])
    .with_opacity(half);
    let off_surface = PaintContributionGroup::new(vec![
        PaintContributionItem::fill(
            SceneShape::rect(rect(80.0, 0.0, 8.0, 8.0)),
            Brush::solid(Color::WHITE),
        )
        .into(),
    ]);
    grouped_publication(vec![
        PaintContributionGroup::new(Vec::new()).into(),
        contracted.into(),
        outside.into(),
        nested.into(),
        image_group.into(),
        off_surface.into(),
    ])
}

fn assert_atomic_group_pixels(readback: &runenui_render_wgpu::OffscreenReadback) {
    assert_eq!(
        pixel(readback, 2, 8),
        [0, 0, 0, 0],
        "group clip constrains the complete composed result"
    );

    let half_red = pixel(readback, 8, 8);
    assert!(half_red[0] >= 186 && half_red[0] <= 189);
    assert_eq!([half_red[1], half_red[2]], [0, 0]);
    assert!(half_red[3].abs_diff(128) <= 1);

    let half_blue = pixel(readback, 16, 8);
    assert_eq!([half_blue[0], half_blue[1]], [0, 0]);
    assert!(half_blue[2] >= 186 && half_blue[2] <= 189);
    assert!(
        half_blue[3].abs_diff(128) <= 1,
        "overlapping opaque children receive group opacity once, not per child"
    );

    assert_eq!(
        pixel(readback, 22, 8),
        [0x00, 0xFF, 0x00, 0xFF],
        "first-member group contraction keeps the outside layer-zero item above the isolated +1 child"
    );

    let nested_pixel = pixel(readback, 28, 28);
    for channel in 0..3 {
        assert!(
            nested_pixel[channel] >= 135 && nested_pixel[channel] <= 139,
            "nested half-opacity group channel {channel} is not quarter-premultiplied: {nested_pixel:?}"
        );
    }
    assert!(nested_pixel[3].abs_diff(64) <= 1);

    let image_pixel = pixel(readback, 50, 30);
    assert!(image_pixel[0] >= 186 && image_pixel[0] <= 189);
    assert_eq!([image_pixel[1], image_pixel[2]], [0, 0]);
    assert!(image_pixel[3].abs_diff(128) <= 1);
    assert_eq!(pixel(readback, 62, 2), [0, 0, 0, 0]);
}

fn shadowed_group_publication() -> Result<PaintPublication, Box<dyn Error>> {
    let shadow = DropShadow::new(
        2.0,
        2.0,
        LogicalLength::new(1.0)?,
        0.0,
        Color::rgba(0x00, 0x00, 0x00, 0x80),
    )?;
    Ok(grouped_publication(vec![
        PaintContributionGroup::new(vec![
            PaintContributionItem::fill(
                SceneShape::rect(rect(4.0, 4.0, 8.0, 8.0)),
                Brush::solid(Color::WHITE),
            )
            .into(),
        ])
        .with_shadows(vec![shadow])
        .into(),
    ]))
}

#[test]
fn real_gpu_generic_solids_preserve_shape_transform_clip_and_degenerate_semantics()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let green = Color::rgb(0x35, 0xB8, 0x68);
    let red = Color::rgb(0xD9, 0x4E, 0x49);
    let blue = Color::rgb(0x45, 0x79, 0xD8);

    let triangle = path(vec![
        PathVerb::MoveTo(point(4.0, 4.0)),
        PathVerb::LineTo(point(28.0, 4.0)),
        PathVerb::LineTo(point(16.0, 28.0)),
        PathVerb::Close,
    ]);
    let clipped_triangle = PaintContributionItem::fill(triangle, Brush::solid(green)).with_clip(
        ContributionClip::identity(SceneShape::rect(rect(10.0, 0.0, 12.0, 32.0))),
    );
    let transformed_ellipse = PaintContributionItem::fill(
        SceneShape::ellipse(rect(0.0, 0.0, 12.0, 10.0)),
        Brush::solid(red),
    )
    .with_transform(LogicalTransform::translation(34.0, 4.0)?);
    let round_line = PaintContributionItem::stroke(
        path(vec![
            PathVerb::MoveTo(point(34.0, 24.0)),
            PathVerb::LineTo(point(50.0, 24.0)),
        ]),
        Brush::solid(blue),
        StrokeStyle::new(LogicalLength::new(4.0)?).with_cap(StrokeCap::Round),
    );
    let degenerate = PaintContributionItem::stroke(
        SceneShape::rect(rect(56.0, 8.0, 0.0, 20.0)),
        Brush::solid(Color::WHITE),
        StrokeStyle::new(LogicalLength::new(8.0)?).with_join(StrokeJoin::Round),
    );
    let publication = publication(vec![
        clipped_triangle,
        transformed_ellipse,
        round_line,
        degenerate,
    ]);

    let output = renderer.render_offscreen_publication(&publication, &provider)?;
    let readback = output.readback();
    assert_eq!(pixel(readback, 16, 8), [0x35, 0xB8, 0x68, 0xFF]);
    assert_eq!(
        pixel(readback, 8, 8),
        [0, 0, 0, 0],
        "the rectangular item clip excludes otherwise-covered path fill"
    );
    assert_eq!(pixel(readback, 40, 9), [0xD9, 0x4E, 0x49, 0xFF]);
    assert_eq!(pixel(readback, 42, 24), [0x45, 0x79, 0xD8, 0xFF]);
    assert_eq!(
        pixel(readback, 56, 18),
        [0, 0, 0, 0],
        "zero-extent rectangle stroke remains empty"
    );
    Ok(())
}

#[test]
fn real_gpu_self_overlap_is_one_source_and_rebuilds_after_target_loss() -> Result<(), Box<dyn Error>>
{
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let color = Color::rgba(0xC3, 0x4A, 0x42, 0x80);
    let crossing = path(vec![
        PathVerb::MoveTo(point(8.0, 8.0)),
        PathVerb::LineTo(point(32.0, 32.0)),
        PathVerb::MoveTo(point(32.0, 8.0)),
        PathVerb::LineTo(point(8.0, 32.0)),
    ]);
    let publication = publication(vec![PaintContributionItem::stroke(
        crossing,
        Brush::solid(color),
        StrokeStyle::new(LogicalLength::new(6.0)?).with_cap(StrokeCap::Butt),
    )]);

    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    let arm = pixel(first.readback(), 11, 11);
    let overlap = pixel(first.readback(), 20, 20);
    assert_eq!(
        overlap[3], arm[3],
        "tessellation overlap must not multiply one item's alpha"
    );
    assert!(
        overlap[3].abs_diff(0x80) <= 1,
        "one 0x80-alpha logical source must stay near 0x80, got {}",
        overlap[3]
    );
    for channel in 0..3 {
        assert!(
            overlap[channel].abs_diff(arm[channel]) <= 1,
            "overlap and single-coverage samples must share one source result"
        );
    }

    let first_generation = first.target_generation();
    let first_pixels = first.readback().rgba8_srgb().to_vec();
    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_ne!(rebuilt.target_generation(), first_generation);
    assert_eq!(rebuilt.readback().rgba8_srgb(), first_pixels);
    Ok(())
}

#[test]
fn real_gpu_gradients_match_core_sampling_and_hard_stop_semantics() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let red = Color::rgb(255, 0, 0);
    let blue = Color::rgb(0, 0, 255);

    let hard_stops = gradient_stops(&[(0.25, red), (0.5, red), (0.5, blue), (0.75, Color::WHITE)]);
    let hard_gradient = LinearGradient::new(point(0.5, 0.5), point(64.5, 0.5), hard_stops)
        .unwrap_or_else(|_| unreachable!("fixture hard-stop gradient is valid"));
    let hard_item = PaintContributionItem::fill(
        SceneShape::rect(rect(0.0, 0.0, 64.0, 12.0)),
        Brush::Linear(hard_gradient.clone()),
    );

    let radial_gradient = RadialGradient::new(
        point(48.5, 28.5),
        LogicalLength::new(8.0)?,
        gradient_stops(&[(0.0, Color::BLACK), (1.0, Color::WHITE)]),
    )
    .unwrap_or_else(|_| unreachable!("fixture radial gradient is valid"));
    let radial_item = PaintContributionItem::fill(
        SceneShape::rect(rect(36.0, 16.0, 24.0, 24.0)),
        Brush::Radial(radial_gradient.clone()),
    );

    let alpha_gradient = LinearGradient::new(
        point(0.5, 20.5),
        point(32.5, 20.5),
        gradient_stops(&[
            (0.0, Color::rgb(255, 0, 0)),
            (1.0, Color::rgba(0, 0, 255, 0)),
        ]),
    )
    .unwrap_or_else(|_| unreachable!("fixture alpha gradient is valid"));
    let alpha_item = PaintContributionItem::fill(
        SceneShape::rect(rect(0.0, 16.0, 32.0, 12.0)),
        Brush::Linear(alpha_gradient),
    );

    let publication = publication(vec![hard_item, alpha_item, radial_item]);
    let output = renderer.render_offscreen_publication(&publication, &provider)?;
    let readback = output.readback();

    assert_eq!(
        pixel(readback, 0, 4),
        color_bytes(red),
        "the first authored stop extends toward decreasing coordinates"
    );
    assert_pixel_near(
        pixel(readback, 8, 4),
        color_bytes(hard_gradient.sample_at(point(8.5, 4.5))),
        1,
    );
    assert_eq!(
        pixel(readback, 32, 4),
        color_bytes(red),
        "the exact hard-stop coordinate keeps the first authored boundary color"
    );
    assert_pixel_near(
        pixel(readback, 33, 4),
        color_bytes(hard_gradient.sample_at(point(33.5, 4.5))),
        1,
    );
    assert_eq!(
        pixel(readback, 63, 4),
        color_bytes(Color::WHITE),
        "the last authored stop extends toward increasing coordinates"
    );

    assert_eq!(pixel(readback, 48, 28), color_bytes(Color::BLACK));
    assert_eq!(pixel(readback, 56, 28), color_bytes(Color::WHITE));
    assert_pixel_near(
        pixel(readback, 52, 28),
        color_bytes(radial_gradient.sample_at(point(52.5, 28.5))),
        1,
    );

    let midpoint = pixel(readback, 16, 20);
    assert!(
        midpoint[3].abs_diff(128) <= 1,
        "premultiplied alpha midpoint must remain half-alpha, got {}",
        midpoint[3]
    );
    assert!(
        midpoint[0] >= 186 && midpoint[0] <= 189,
        "opaque-red contribution over transparent target should store half linear red, got {}",
        midpoint[0]
    );
    assert_eq!(
        midpoint[2], 0,
        "transparent blue must not leak color through premultiplied interpolation"
    );
    Ok(())
}

#[test]
fn real_gpu_gradient_transform_clip_stroke_and_rebuild_are_deterministic()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let stops = gradient_stops(&[(0.0, Color::BLACK), (1.0, Color::WHITE)]);
    let fill_gradient = LinearGradient::new(point(0.5, 0.5), point(24.5, 0.5), stops.clone())
        .unwrap_or_else(|_| unreachable!("fixture transformed gradient is valid"));
    let transformed = PaintContributionItem::fill(
        SceneShape::rect(rect(0.0, 0.0, 24.0, 12.0)),
        Brush::Linear(fill_gradient.clone()),
    )
    .with_transform(LogicalTransform::translation(8.0, 32.0)?)
    .with_clip(ContributionClip::identity(SceneShape::rect(rect(
        16.0, 30.0, 12.0, 16.0,
    ))));

    let stroke_gradient = LinearGradient::new(point(36.5, 40.5), point(60.5, 40.5), stops)
        .unwrap_or_else(|_| unreachable!("fixture stroke gradient is valid"));
    let stroke = PaintContributionItem::stroke(
        path(vec![
            PathVerb::MoveTo(point(36.0, 40.0)),
            PathVerb::LineTo(point(60.0, 40.0)),
        ]),
        Brush::Linear(stroke_gradient.clone()),
        StrokeStyle::new(LogicalLength::new(4.0)?).with_cap(StrokeCap::Butt),
    );
    let publication = publication(vec![transformed, stroke]);

    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        pixel(first.readback(), 12, 36),
        [0, 0, 0, 0],
        "the independent owner-local clip excludes transformed primitive coverage"
    );
    assert_pixel_near(
        pixel(first.readback(), 20, 36),
        color_bytes(fill_gradient.sample_at(point(12.5, 4.5))),
        1,
    );
    assert_pixel_near(
        pixel(first.readback(), 48, 40),
        color_bytes(stroke_gradient.sample_at(point(48.5, 40.5))),
        1,
    );

    let first_generation = first.target_generation();
    let first_pixels = first.readback().rgba8_srgb().to_vec();
    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_ne!(rebuilt.target_generation(), first_generation);
    assert_eq!(rebuilt.readback().rgba8_srgb(), first_pixels);
    Ok(())
}

#[test]
fn real_gpu_generic_ellipse_and_path_clips_are_conjunctive_order_invariant_and_rebuildable()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let color = Color::rgb(0x36, 0xB5, 0x6B);
    let ellipse = ContributionClip::new(
        SceneShape::ellipse(rect(4.0, 6.0, 40.0, 28.0)),
        LogicalTransform::translation(4.0, 2.0)?,
    );
    let path_clip = ContributionClip::identity(path_rect(rect(20.0, 10.0, 32.0, 24.0)));
    let item = |clips: [ContributionClip; 2]| {
        clips.into_iter().fold(
            PaintContributionItem::fill(
                SceneShape::rect(rect(
                    0.0,
                    0.0,
                    f32::from(SURFACE_WIDTH),
                    f32::from(SURFACE_HEIGHT),
                )),
                Brush::solid(color),
            ),
            PaintContributionItem::with_clip,
        )
    };
    let forward = publication(vec![item([ellipse.clone(), path_clip.clone()])]);
    let reversed = publication(vec![item([path_clip, ellipse])]);

    let first = renderer.render_offscreen_publication(&forward, &provider)?;
    assert_eq!(pixel(first.readback(), 28, 22), color_bytes(color));
    assert_eq!(
        pixel(first.readback(), 16, 22),
        [0, 0, 0, 0],
        "ellipse-only coverage is removed by the path clip"
    );
    assert_eq!(
        pixel(first.readback(), 50, 12),
        [0, 0, 0, 0],
        "path-only coverage is removed by the transformed ellipse clip"
    );

    let first_pixels = first.readback().rgba8_srgb().to_vec();
    let reversed_output = renderer.render_offscreen_publication(&reversed, &provider)?;
    assert_eq!(
        reversed_output.readback().rgba8_srgb(),
        first_pixels,
        "conjunctive clip coverage cannot depend on authored clip order"
    );

    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&forward, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_eq!(
        rebuilt.readback().rgba8_srgb(),
        first_pixels,
        "generic clip realization reconstructs identically after target loss"
    );
    Ok(())
}

#[test]
fn real_gpu_atomic_groups_preserve_contraction_clips_opacity_resources_nesting_and_rebuild()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let image_resource = ResourceRef::new(ResourceKind::Image);
    let provider = SingleImageProvider::new(image_resource.clone())?;
    let half = SceneOpacity::new(0.5)?;
    let publication = atomic_group_publication(image_resource, half);

    assert_eq!(publication.scene().groups().len(), 5);
    assert_eq!(publication.scene().root_entries().len(), 5);

    let first = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_atomic_group_pixels(first.readback());

    let first_generation = first.target_generation();
    let first_pixels = first.readback().rgba8_srgb().to_vec();
    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_ne!(rebuilt.target_generation(), first_generation);
    assert_eq!(rebuilt.readback().rgba8_srgb(), first_pixels);

    let unchanged = renderer.render_offscreen_publication(&publication, &provider)?;
    assert_eq!(
        unchanged.update_plan().mode(),
        PublicationUpdateMode::AlreadyCurrent
    );
    assert_eq!(unchanged.target_generation(), rebuilt.target_generation());
    assert_eq!(unchanged.readback().rgba8_srgb(), first_pixels);
    Ok(())
}

#[test]
fn real_gpu_ordinary_group_shadow_renders_behind_child_and_rebuilds() -> Result<(), Box<dyn Error>>
{
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let publication = shadowed_group_publication()?;
    let first = renderer.render_offscreen_publication(&publication, &NoResources)?;

    assert_eq!(
        pixel(first.readback(), 8, 8),
        color_bytes(Color::WHITE),
        "child color must remain above its own ordinary shadow"
    );
    let shadow_only = pixel(first.readback(), 13, 8);
    assert_eq!([shadow_only[0], shadow_only[1], shadow_only[2]], [0, 0, 0]);
    assert!(
        shadow_only[3] > 0 && shadow_only[3] < 0x80,
        "finite Gaussian realization must produce partial shadow alpha outside child coverage: {shadow_only:?}"
    );
    assert_eq!(
        pixel(first.readback(), 20, 20),
        [0, 0, 0, 0],
        "ordinary shadow must produce no coverage outside its finite support"
    );

    let first_generation = first.target_generation();
    let first_pixels = first.readback().rgba8_srgb().to_vec();
    assert!(renderer.discard_offscreen_target());
    let rebuilt = renderer.render_offscreen_publication(&publication, &NoResources)?;
    assert_eq!(
        rebuilt.update_plan().mode(),
        PublicationUpdateMode::FullResync
    );
    assert_ne!(rebuilt.target_generation(), first_generation);
    assert_eq!(
        rebuilt.readback().rgba8_srgb(),
        first_pixels,
        "ordinary-shadow realization must reconstruct identically after target loss"
    );
    Ok(())
}

#[test]
fn real_gpu_shadow_support_ignores_child_alpha_and_preserves_sibling_and_group_order()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };

    let transparent_child = PaintContributionItem::fill(
        SceneShape::rect(rect(16.0, 16.0, 8.0, 8.0)),
        Brush::solid(Color::rgba(0xFF, 0xFF, 0xFF, 0x00)),
    );
    let expanded_red = DropShadow::new(
        0.0,
        0.0,
        LogicalLength::new(0.0)?,
        4.0,
        Color::rgba(0xFF, 0x00, 0x00, 0x80),
    )?;
    let compact_blue = DropShadow::new(
        0.0,
        0.0,
        LogicalLength::new(0.0)?,
        0.0,
        Color::rgba(0x00, 0x00, 0xFF, 0x80),
    )?;
    let sibling_group = PaintContributionGroup::new(vec![transparent_child.into()])
        .with_shadows(vec![expanded_red, compact_blue]);

    let clipped_child = PaintContributionItem::fill(
        SceneShape::rect(rect(40.0, 16.0, 8.0, 8.0)),
        Brush::solid(Color::rgba(0xFF, 0xFF, 0xFF, 0x00)),
    );
    let opaque_green = DropShadow::new(
        0.0,
        0.0,
        LogicalLength::new(0.0)?,
        0.0,
        Color::rgb(0x00, 0xFF, 0x00),
    )?;
    let clipped_half_group = PaintContributionGroup::new(vec![clipped_child.into()])
        .with_shadows(vec![opaque_green])
        .with_clip(ContributionClip::identity(SceneShape::rect(rect(
            44.0, 16.0, 4.0, 8.0,
        ))))
        .with_opacity(SceneOpacity::new(0.5)?);

    let publication = grouped_publication(vec![sibling_group.into(), clipped_half_group.into()]);
    let output = renderer.render_offscreen_publication(&publication, &NoResources)?;
    let readback = output.readback();

    let expanded_only = pixel(readback, 13, 20);
    assert!(expanded_only[0] > 0 && expanded_only[3] > 0);
    assert_eq!([expanded_only[1], expanded_only[2]], [0, 0]);

    let sibling_overlap = pixel(readback, 20, 20);
    assert!(sibling_overlap[0] > 0 && sibling_overlap[2] > 0);
    assert!(
        sibling_overlap[2] > sibling_overlap[0],
        "later authored blue shadow must source-over above earlier red shadow: {sibling_overlap:?}"
    );
    assert!(
        sibling_overlap[3] > expanded_only[3],
        "independent sibling shadows overlap without chaining support: {sibling_overlap:?}"
    );

    assert_eq!(
        pixel(readback, 42, 20),
        [0, 0, 0, 0],
        "group clip must constrain the complete shadow result"
    );
    let clipped_half = pixel(readback, 46, 20);
    assert_eq!([clipped_half[0], clipped_half[2]], [0, 0]);
    assert!(clipped_half[1] >= 186 && clipped_half[1] <= 189);
    assert!(
        clipped_half[3].abs_diff(128) <= 1,
        "group opacity must apply exactly once after shadow clipping: {clipped_half:?}"
    );
    Ok(())
}

fn spread_shadow_group(
    origin_x: u16,
    origin_y: u16,
    cells: impl IntoIterator<Item = (u16, u16)>,
) -> Result<PaintContributionGroup, Box<dyn Error>> {
    let children = cells
        .into_iter()
        .map(|(x, y)| {
            PaintContributionItem::fill(
                SceneShape::rect(rect(
                    f32::from(origin_x + x),
                    f32::from(origin_y + y),
                    1.0,
                    1.0,
                )),
                Brush::solid(Color::rgba(0xFF, 0xFF, 0xFF, 0x00)),
            )
            .into()
        })
        .collect::<Vec<_>>();
    let shadow = DropShadow::new(0.0, 0.0, LogicalLength::new(0.0)?, 2.0, Color::WHITE)?;
    Ok(PaintContributionGroup::new(children).with_shadows(vec![shadow]))
}

#[test]
fn real_gpu_euclidean_spread_rejects_square_corners_and_is_quarter_turn_invariant()
-> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let l_source = [
        (0_u16, 0_u16),
        (0, 1),
        (0, 2),
        (0, 3),
        (1, 3),
        (2, 3),
        (3, 3),
        (3, 2),
    ];

    let point = spread_shadow_group(4, 4, [(0, 0)])?;
    let source = spread_shadow_group(16, 8, l_source.iter().copied())?;
    let rotated = spread_shadow_group(40, 8, l_source.iter().copied().map(|(x, y)| (3 - y, x)))?;
    let publication = grouped_publication(vec![point.into(), source.into(), rotated.into()]);
    let output = renderer.render_offscreen_publication(&publication, &NoResources)?;
    let readback = output.readback();

    assert_eq!(pixel(readback, 4, 4)[3], u8::MAX);
    assert_eq!(pixel(readback, 2, 4)[3], u8::MAX);
    assert_eq!(pixel(readback, 4, 2)[3], u8::MAX);
    assert_eq!(pixel(readback, 3, 3)[3], u8::MAX);
    assert_eq!(
        pixel(readback, 2, 2)[3],
        0,
        "radius-2 Euclidean spread must reject the distance-sqrt(8) corner that a square kernel would include"
    );

    for y in 0..8 {
        for x in 0..8 {
            let original = pixel(readback, 14 + x, 6 + y)[3];
            let quarter_turned = pixel(readback, 38 + (7 - y), 6 + x)[3];
            assert_eq!(
                quarter_turned, original,
                "quarter-turn spread mismatch at local ({x}, {y})"
            );
        }
    }
    Ok(())
}

#[test]
fn real_gpu_empty_and_singular_generic_clips_erase_coverage() -> Result<(), Box<dyn Error>> {
    let Some(mut renderer) = renderer_or_adapterless()? else {
        return Ok(());
    };
    let provider = NoResources;
    let full_item = || {
        PaintContributionItem::fill(
            SceneShape::rect(rect(
                0.0,
                0.0,
                f32::from(SURFACE_WIDTH),
                f32::from(SURFACE_HEIGHT),
            )),
            Brush::solid(Color::WHITE),
        )
    };
    let empty = ContributionClip::identity(SceneShape::ellipse(rect(8.0, 8.0, 0.0, 20.0)));
    let singular = ContributionClip::new(
        path_rect(rect(8.0, 8.0, 32.0, 24.0)),
        LogicalTransform::try_new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0)?,
    );
    let output = renderer.render_offscreen_publication(
        &publication(vec![
            full_item().with_clip(empty),
            full_item().with_clip(singular),
        ]),
        &provider,
    )?;

    assert!(
        output
            .readback()
            .rgba8_srgb()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|pixel| *pixel == [0, 0, 0, 0]),
        "empty and singular clips must not fall back to full coverage"
    );
    Ok(())
}

const fn color_bytes(color: Color) -> [u8; 4] {
    [color.red(), color.green(), color.blue(), color.alpha()]
}

fn assert_pixel_near(actual: [u8; 4], expected: [u8; 4], tolerance: u8) {
    for channel in 0..4 {
        assert!(
            actual[channel].abs_diff(expected[channel]) <= tolerance,
            "channel {channel} differs: actual={actual:?}, expected={expected:?}, tolerance={tolerance}"
        );
    }
}

fn pixel(readback: &runenui_render_wgpu::OffscreenReadback, x: u32, y: u32) -> [u8; 4] {
    let index = (y as usize * readback.extent().width() as usize + x as usize) * 4;
    readback.rgba8_srgb()[index..index + 4]
        .try_into()
        .unwrap_or_else(|_| unreachable!("pixel index is in the fixture target"))
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
                "native wgpu solid proof unavailable under {requested:?}; structured adapter failure: {detail}"
            );
            assert_eq!(requested, BackendSelection::AllNative);
            assert!(!compatible_surface_required);
            assert!(!detail.is_empty());
            Ok(None)
        }
        Err(error) => Err(error.into()),
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
