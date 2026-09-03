use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bymsdfgen_core::{
    Bitmap, Contour, DistanceMapping, EdgeSegment, MsdfGeneratorConfig, Projection, Range,
    SdfTransformation, Shape, Vector2, coloring::edge_coloring_simple, generate_msdf,
};
use runenui_core::{Color, LogicalPoint, LogicalRect, LogicalTransform, ResourceRef, SceneOpacity};
use runenui_runtime::RasterScale;
use runenui_text::{ShapedTextResource, TextGlyph};
use skrifa::raw::TableProvider;
use skrifa::{
    FontRef, MetadataProvider,
    color::ColorGlyphFormat,
    instance::{LocationRef, NormalizedCoord, Size},
    outline::{DrawSettings, OutlinePen},
};

use crate::scene_subset::SupportedFillRect;

const SHAPED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SHAPED_VERTEX_STRIDE: u64 = 36;
const FIELD_RANGE: f64 = 4.0;
const FIELD_BORDER: f64 = 4.0;
const ATLAS_EDGE: u32 = 2048;
const ATLAS_GUTTER: u32 = 1;
const SHAPED_SHADER: &str = r"
@group(0) @binding(0)
var field_texture: texture_2d<f32>;
@group(0) @binding(1)
var field_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) opacity: f32,
    @location(3) foreground: vec4<f32>,
}
struct VertexOutput {
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) foreground: vec4<f32>,
}
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.uv = input.uv;
    output.opacity = input.opacity;
    output.foreground = input.foreground;
    return output;
}
fn median(a: f32, b: f32, c: f32) -> f32 {
    return max(min(a, b), min(max(a, b), c));
}
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let sample = textureSample(field_texture, field_sampler, input.uv).rgb;
    let signed_distance = median(sample.r, sample.g, sample.b) - 0.5;
    let texture_size = vec2<f32>(textureDimensions(field_texture, 0));
    let unit_range = vec2<f32>(4.0, 4.0) / texture_size;
    let screen_texture_size = 1.0 / max(fwidth(input.uv), vec2<f32>(0.000001, 0.000001));
    let screen_pixel_range = max(0.5 * dot(unit_range, screen_texture_size), 1.0);
    let coverage = clamp(screen_pixel_range * signed_distance + 0.5, 0.0, 1.0);
    return vec4<f32>(input.foreground.rgb, input.foreground.a * input.opacity * coverage);
}
";

const SHAPED_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 8,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 16,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 20,
        shader_location: 3,
    },
];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct SupportedShapedRun {
    pub(super) item_index: usize,
    pub(super) resource: ResourceRef,
    pub(super) origin: LogicalPoint,
    pub(super) foreground: Color,
    pub(super) opacity: SceneOpacity,
    pub(super) local_to_surface: LogicalTransform,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum QualityTier {
    P16,
    P24,
    P32,
    P48,
}

impl QualityTier {
    const fn pixels_per_em(self) -> f64 {
        match self {
            Self::P16 => 16.0,
            Self::P24 => 24.0,
            Self::P32 => 32.0,
            Self::P48 => 48.0,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ShapedRunCacheKey {
    resource: ResourceRef,
    quality: QualityTier,
}

impl ShapedRunCacheKey {
    const fn new(resource: ResourceRef, quality: QualityTier) -> Self {
        Self { resource, quality }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GlyphRaster {
    glyph_id: u32,
    logical_origin: LogicalPoint,
    width: u32,
    height: u32,
    rgba8: Arc<[u8]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GlyphPlacement {
    page_index: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    logical_origin: LogicalPoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedAtlasPage {
    width: u32,
    height: u32,
    rgba8: Arc<[u8]>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ResolvedShapedRun {
    key: ShapedRunCacheKey,
    pages: Vec<ResolvedAtlasPage>,
    placements: HashMap<u32, GlyphPlacement>,
    occurrences: Arc<[TextGlyph]>,
}

impl ResolvedShapedRun {
    const fn key(&self) -> &ShapedRunCacheKey {
        &self.key
    }

    pub(super) const fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    pub(super) fn resource_key(&self) -> (ResourceRef, QualityTier) {
        (self.key.resource.clone(), self.key.quality)
    }
}

#[derive(Debug)]
struct AtlasPageRealization {
    width: u32,
    height: u32,
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

#[derive(Debug)]
struct ShapedRunRealization {
    pages: Vec<AtlasPageRealization>,
    placements: HashMap<u32, GlyphPlacement>,
    occurrences: Arc<[TextGlyph]>,
}

#[derive(Debug)]
struct ShapedRunTargetPipelines {
    ordinary: wgpu::RenderPipeline,
    clipped: wgpu::RenderPipeline,
}

#[derive(Debug)]
pub(super) struct ShapedRunRenderer {
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    pipelines: HashMap<wgpu::TextureFormat, ShapedRunTargetPipelines>,
    cache: HashMap<ShapedRunCacheKey, ShapedRunRealization>,
}

impl ShapedRunRenderer {
    pub(super) fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("runenui shaped-text MSDF atlas bind-group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("runenui shaped-text MSDF linear sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Self {
            bind_group_layout,
            sampler,
            pipelines: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    pub(super) fn quality(
        resource: &ShapedTextResource,
        raster_scale: RasterScale,
        transform: LogicalTransform,
    ) -> QualityTier {
        quality_tier(resource.font_size(), raster_scale, transform)
    }

    pub(super) fn contains(
        &self,
        resource: &ShapedTextResource,
        raster_scale: RasterScale,
        transform: LogicalTransform,
    ) -> bool {
        let key = ShapedRunCacheKey::new(
            resource.resource_ref().clone(),
            Self::quality(resource, raster_scale, transform),
        );
        self.cache.contains_key(&key)
    }

    pub(super) fn ensure_pipelines(
        &mut self,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), super::super::OffscreenRenderError> {
        if !matches!(
            target_format,
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            return Err(
                super::super::OffscreenRenderError::UnsupportedTargetFormat {
                    format: target_format,
                },
            );
        }
        if !self.pipelines.contains_key(&target_format) {
            let ordinary =
                create_shaped_pipeline(device, target_format, &self.bind_group_layout, false);
            let clipped =
                create_shaped_pipeline(device, target_format, &self.bind_group_layout, true);
            self.pipelines.insert(
                target_format,
                ShapedRunTargetPipelines { ordinary, clipped },
            );
        }
        Ok(())
    }

    pub(super) fn realize(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resolved: Vec<ResolvedShapedRun>,
    ) {
        for resolved in resolved {
            if self.cache.contains_key(resolved.key()) {
                continue;
            }
            let pages = resolved
                .pages
                .iter()
                .map(|page| {
                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("runenui shaped-text MSDF atlas page"),
                        size: wgpu::Extent3d {
                            width: page.width,
                            height: page.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: SHAPED_FORMAT,
                        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    });
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        &page.rgba8,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(page.width * 4),
                            rows_per_image: Some(page.height),
                        },
                        wgpu::Extent3d {
                            width: page.width,
                            height: page.height,
                            depth_or_array_layers: 1,
                        },
                    );
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("runenui shaped-text MSDF atlas page bind group"),
                        layout: &self.bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.sampler),
                            },
                        ],
                    });
                    AtlasPageRealization {
                        width: page.width,
                        height: page.height,
                        _texture: texture,
                        bind_group,
                    }
                })
                .collect();
            self.cache.insert(
                resolved.key,
                ShapedRunRealization {
                    pages,
                    placements: resolved.placements,
                    occurrences: resolved.occurrences,
                },
            );
        }
    }

    pub(super) fn retain(&mut self, live: &HashSet<(ResourceRef, QualityTier)>) {
        self.cache
            .retain(|key, _| live.contains(&(key.resource.clone(), key.quality)));
    }

    pub(super) fn discard_cache(&mut self) -> bool {
        let had_entries = !self.cache.is_empty();
        self.cache.clear();
        had_entries
    }

    pub(super) fn vertex_batches(
        &self,
        shaped_run: &SupportedShapedRun,
        resource: &ShapedTextResource,
        extent: super::super::OffscreenExtent,
        canvas_extent: super::super::RasterCanvasExtent,
        raster_scale: RasterScale,
    ) -> Vec<(usize, Vec<u8>)> {
        let quality = Self::quality(resource, raster_scale, shaped_run.local_to_surface);
        let key = ShapedRunCacheKey::new(shaped_run.resource.clone(), quality);
        let Some(realization) = self.cache.get(&key) else {
            return Vec::new();
        };
        let mut batches = (0..realization.pages.len())
            .map(|_| Vec::new())
            .collect::<Vec<Vec<u8>>>();
        let tier_scale = quality.pixels_per_em();
        for glyph in realization.occurrences.iter() {
            let Some(placement) = realization.placements.get(&glyph.id()) else {
                continue;
            };
            let Some(page) = realization.pages.get(placement.page_index) else {
                continue;
            };
            let Some(bytes) = glyph_vertex_bytes(
                shaped_run,
                *glyph,
                *placement,
                page.width,
                page.height,
                extent,
                canvas_extent,
                raster_scale,
                tier_scale,
            ) else {
                continue;
            };
            batches[placement.page_index].extend_from_slice(&bytes);
        }
        batches
            .into_iter()
            .enumerate()
            .filter(|(_, bytes)| !bytes.is_empty())
            .collect()
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the shaped draw boundary keeps target format, color/stencil attachments, exact logical resource and representation class, page bind group, and prepared occurrence vertices explicit"
    )]
    pub(super) fn draw(
        &self,
        target_format: wgpu::TextureFormat,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        stencil_view: Option<&wgpu::TextureView>,
        resource: &ResourceRef,
        quality: QualityTier,
        page_index: usize,
        vertex_buffer: &wgpu::Buffer,
        vertex_count: u32,
    ) {
        let pipelines = self.pipelines.get(&target_format).unwrap_or_else(|| {
            unreachable!("shaped-text target pipelines are cached before drawing")
        });
        let realization = self
            .cache
            .get(&ShapedRunCacheKey::new(resource.clone(), quality))
            .unwrap_or_else(|| unreachable!("shaped-text resource is realized before drawing"));
        let Some(page) = realization.pages.get(page_index) else {
            return;
        };
        let color_attachment = Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        });
        let stencil_attachment = stencil_view.map(|view| wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: None,
            stencil_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
        });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("runenui ordered shaped-text MSDF atlas pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if stencil_view.is_some() {
            render_pass.set_pipeline(&pipelines.clipped);
            render_pass.set_stencil_reference(super::STENCIL_ALLOWED);
        } else {
            render_pass.set_pipeline(&pipelines.ordinary);
        }
        render_pass.set_bind_group(0, &page.bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_count, 0..1);
    }
}

pub(super) fn resolve_shaped_run(
    shaped_run: &SupportedShapedRun,
    resource: &ShapedTextResource,
    raster_scale: RasterScale,
    max_texture_dimension_2d: u32,
) -> Result<ResolvedShapedRun, ShapedRunResolveFailure> {
    let quality = quality_tier(
        resource.font_size(),
        raster_scale,
        shaped_run.local_to_surface,
    );
    let glyphs = rasterize_unique_glyphs(resource, quality)?;
    let (pages, placements) = pack_atlas(glyphs, max_texture_dimension_2d)?;
    Ok(ResolvedShapedRun {
        key: ShapedRunCacheKey::new(resource.resource_ref().clone(), quality),
        pages,
        placements,
        occurrences: resource.glyphs().to_vec().into(),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UnsupportedGlyphKind {
    ColrV0,
    ColrV1,
    Bitmap,
    Svg,
    FauxBold,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ShapedRunResolveFailure {
    UnsupportedGlyph {
        glyph_id: u32,
        kind: UnsupportedGlyphKind,
    },
    InvalidFont,
    InvalidOutline {
        glyph_id: u32,
    },
    GlyphExtentExceedsDeviceLimit {
        glyph_id: u32,
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
    },
}

fn quality_tier(
    font_size: f32,
    raster_scale: RasterScale,
    transform: LogicalTransform,
) -> QualityTier {
    let effective_ppem = f64::from(font_size.max(0.0))
        * f64::from(raster_scale.get())
        * max_linear_stretch(transform);
    if effective_ppem <= QualityTier::P16.pixels_per_em() {
        QualityTier::P16
    } else if effective_ppem <= QualityTier::P24.pixels_per_em() {
        QualityTier::P24
    } else if effective_ppem <= QualityTier::P32.pixels_per_em() {
        QualityTier::P32
    } else {
        QualityTier::P48
    }
}

fn max_linear_stretch(transform: LogicalTransform) -> f64 {
    let [m11, m12, m21, m22, _, _] = transform.components().map(f64::from);
    let a = m11.mul_add(m11, m12 * m12);
    let d = m21.mul_add(m21, m22 * m22);
    let b = m11.mul_add(m21, m12 * m22);
    let trace = a + d;
    let discriminant = ((a - d).mul_add(a - d, 4.0 * b * b)).max(0.0).sqrt();
    f64::midpoint(trace, discriminant).sqrt().max(0.0)
}

fn rasterize_unique_glyphs(
    resource: &ShapedTextResource,
    quality: QualityTier,
) -> Result<Vec<GlyphRaster>, ShapedRunResolveFailure> {
    let font = FontRef::from_index(resource.font().bytes(), resource.font().face_index())
        .map_err(|_| ShapedRunResolveFailure::InvalidFont)?;
    let normalized: Vec<NormalizedCoord> = resource
        .font()
        .normalized_coords()
        .iter()
        .copied()
        .map(NormalizedCoord::from_bits)
        .collect();
    let location = LocationRef::new(&normalized);
    let outlines = font.outline_glyphs();
    let colors = font.color_glyphs();
    let bitmaps = font.bitmap_strikes();
    let svg = font.svg().ok();
    let mut seen = HashSet::new();
    let mut rasters = Vec::new();
    for glyph in resource.glyphs() {
        if !seen.insert(glyph.id()) {
            continue;
        }
        if resource.font().faux_bold() {
            return Err(ShapedRunResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind: UnsupportedGlyphKind::FauxBold,
            });
        }
        let glyph_id = skrifa::GlyphId::new(glyph.id());
        if svg
            .as_ref()
            .and_then(|svg| svg.glyph_data(glyph_id).ok().flatten())
            .is_some()
        {
            return Err(ShapedRunResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind: UnsupportedGlyphKind::Svg,
            });
        }
        if let Some(color) = colors.get(glyph_id) {
            let kind = match color.format() {
                ColorGlyphFormat::ColrV0 => UnsupportedGlyphKind::ColrV0,
                ColorGlyphFormat::ColrV1 => UnsupportedGlyphKind::ColrV1,
            };
            return Err(ShapedRunResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind,
            });
        }
        if bitmaps
            .glyph_for_size(Size::new(resource.font_size()), glyph_id)
            .is_some()
        {
            return Err(ShapedRunResolveFailure::UnsupportedGlyph {
                glyph_id: glyph.id(),
                kind: UnsupportedGlyphKind::Bitmap,
            });
        }
        let Some(outline) = outlines.get(glyph_id) else {
            // A glyph without a scalable outline is valid non-painting content until an intrinsic
            // representation above proves that it is unsupported color/bitmap content.
            continue;
        };
        let mut shape = Shape::new();
        let mut pen = ShapePen::new(&mut shape, resource.font().faux_skew());
        outline
            .draw(
                DrawSettings::unhinted(Size::new(resource.font_size()), location),
                &mut pen,
            )
            .map_err(|_| ShapedRunResolveFailure::InvalidOutline {
                glyph_id: glyph.id(),
            })?;
        pen.finish();
        if shape.contours.is_empty() {
            continue;
        }
        if !shape.validate() {
            return Err(ShapedRunResolveFailure::InvalidOutline {
                glyph_id: glyph.id(),
            });
        }
        shape.normalize();
        shape.orient_contours();
        edge_coloring_simple(&mut shape, 3.0, 0);
        rasters.push(generate_msdf_raster(shape, quality, glyph.id())?);
    }
    Ok(rasters)
}

#[derive(Debug)]
struct AtlasPageBuilder {
    cursor_x: u32,
    cursor_y: u32,
    shelf_height: u32,
    used_width: u32,
    used_height: u32,
    glyphs: Vec<(GlyphRaster, u32, u32)>,
}

#[allow(
    clippy::too_many_lines,
    reason = "deterministic shelf placement and occupied-bound page materialization are kept together so the renderer-private packing policy remains auditable"
)]
fn pack_atlas(
    mut glyphs: Vec<GlyphRaster>,
    max_texture_dimension_2d: u32,
) -> Result<(Vec<ResolvedAtlasPage>, HashMap<u32, GlyphPlacement>), ShapedRunResolveFailure> {
    let edge = ATLAS_EDGE.min(max_texture_dimension_2d);
    glyphs.sort_by_key(|glyph| {
        (
            std::cmp::Reverse(glyph.height),
            std::cmp::Reverse(glyph.width),
            glyph.glyph_id,
        )
    });
    let mut pages = Vec::<AtlasPageBuilder>::new();
    let mut placements = HashMap::new();
    for glyph in glyphs {
        let packed_width = glyph.width.saturating_add(ATLAS_GUTTER * 2);
        let packed_height = glyph.height.saturating_add(ATLAS_GUTTER * 2);
        if packed_width > edge || packed_height > edge || edge == 0 {
            return Err(ShapedRunResolveFailure::GlyphExtentExceedsDeviceLimit {
                glyph_id: glyph.glyph_id,
                width: glyph.width,
                height: glyph.height,
                max_texture_dimension_2d,
            });
        }
        let mut page_index = pages.len().checked_sub(1);
        let mut new_shelf = false;
        if let Some(index) = page_index {
            let page = &pages[index];
            let same_shelf_fits = page.cursor_x.saturating_add(packed_width) <= edge
                && page.cursor_y.saturating_add(packed_height) <= edge;
            if !same_shelf_fits {
                new_shelf = true;
                if page
                    .cursor_y
                    .saturating_add(page.shelf_height)
                    .saturating_add(packed_height)
                    > edge
                {
                    page_index = None;
                }
            }
        }
        let index = page_index.unwrap_or_else(|| {
            pages.push(AtlasPageBuilder {
                cursor_x: 0,
                cursor_y: 0,
                shelf_height: 0,
                used_width: 0,
                used_height: 0,
                glyphs: Vec::new(),
            });
            pages.len() - 1
        });
        let page = &mut pages[index];
        if new_shelf {
            page.cursor_x = 0;
            page.cursor_y = page.cursor_y.saturating_add(page.shelf_height);
            page.shelf_height = 0;
        }
        let x = page.cursor_x.saturating_add(ATLAS_GUTTER);
        let y = page.cursor_y.saturating_add(ATLAS_GUTTER);
        page.cursor_x = page.cursor_x.saturating_add(packed_width);
        page.shelf_height = page.shelf_height.max(packed_height);
        page.used_width = page
            .used_width
            .max(x.saturating_add(glyph.width).saturating_add(ATLAS_GUTTER));
        page.used_height = page
            .used_height
            .max(y.saturating_add(glyph.height).saturating_add(ATLAS_GUTTER));
        page.glyphs.push((glyph.clone(), x, y));
        placements.insert(
            glyph.glyph_id,
            GlyphPlacement {
                page_index: index,
                x,
                y,
                width: glyph.width,
                height: glyph.height,
                logical_origin: glyph.logical_origin,
            },
        );
    }
    let resolved_pages = pages
        .into_iter()
        .map(|page| {
            let width = page.used_width.max(1);
            let height = page.used_height.max(1);
            let mut rgba8 = vec![128; width as usize * height as usize * 4];
            for (glyph, x, y) in page.glyphs {
                for row in 0..glyph.height {
                    let destination = ((y + row) * width + x) as usize * 4;
                    let source = (row * glyph.width) as usize * 4;
                    let bytes = glyph.width as usize * 4;
                    rgba8[destination..destination + bytes]
                        .copy_from_slice(&glyph.rgba8[source..source + bytes]);
                }
            }
            for pixel in rgba8.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
            ResolvedAtlasPage {
                width,
                height,
                rgba8: rgba8.into(),
            }
        })
        .collect();
    Ok((resolved_pages, placements))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::needless_pass_by_value,
    reason = "MSDF generation consumes the assembled glyph shape and narrows validated field values at the bitmap boundary"
)]
fn generate_msdf_raster(
    shape: Shape,
    quality: QualityTier,
    glyph_id: u32,
) -> Result<GlyphRaster, ShapedRunResolveFailure> {
    let bounds = shape.get_bounds(0.0);
    let scale = quality.pixels_per_em();
    let origin_x = bounds.l - FIELD_BORDER / scale;
    let origin_y = bounds.b - FIELD_BORDER / scale;
    let width = (((bounds.r - bounds.l) + 2.0 * FIELD_BORDER / scale) * scale)
        .ceil()
        .max(1.0) as usize;
    let height = (((bounds.t - bounds.b) + 2.0 * FIELD_BORDER / scale) * scale)
        .ceil()
        .max(1.0) as usize;
    let width_u32 = u32::try_from(width).map_err(|_| {
        ShapedRunResolveFailure::GlyphExtentExceedsDeviceLimit {
            glyph_id,
            width: u32::MAX,
            height: u32::MAX,
            max_texture_dimension_2d: u32::MAX,
        }
    })?;
    let height_u32 = u32::try_from(height).map_err(|_| {
        ShapedRunResolveFailure::GlyphExtentExceedsDeviceLimit {
            glyph_id,
            width: u32::MAX,
            height: u32::MAX,
            max_texture_dimension_2d: u32::MAX,
        }
    })?;
    let projection = Projection::new(Vector2::splat(scale), Vector2::new(-origin_x, -origin_y));
    let mapping = DistanceMapping::from_range(Range::symmetric(FIELD_RANGE / scale));
    let transformation = SdfTransformation::new(projection, mapping);
    let mut bitmap: Bitmap<f32, 3> = Bitmap::new(width, height);
    generate_msdf(
        &mut bitmap,
        &shape,
        &transformation,
        &MsdfGeneratorConfig::default(),
    );
    let mut rgba8 = Vec::with_capacity(width.saturating_mul(height).saturating_mul(4));
    for y in (0..height).rev() {
        for x in 0..width {
            for channel in bitmap.pixel(x, y) {
                rgba8.push((channel.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            rgba8.push(255);
        }
    }
    let logical_origin = LogicalPoint::new(origin_x as f32, origin_y as f32)
        .map_err(|_| ShapedRunResolveFailure::InvalidOutline { glyph_id })?;
    Ok(GlyphRaster {
        glyph_id,
        logical_origin,
        width: width_u32,
        height: height_u32,
        rgba8: rgba8.into(),
    })
}

struct ShapePen<'a> {
    shape: &'a mut Shape,
    contour: Option<Contour>,
    current: Option<Vector2>,
    start: Option<Vector2>,
    skew: f64,
}

impl<'a> ShapePen<'a> {
    fn new(shape: &'a mut Shape, faux_skew: Option<f32>) -> Self {
        Self {
            shape,
            contour: None,
            current: None,
            start: None,
            skew: faux_skew.map_or(0.0, f64::from).tan(),
        }
    }

    fn point(&self, x: f32, y: f32) -> Vector2 {
        let y = -f64::from(y);
        Vector2::new(self.skew.mul_add(y, f64::from(x)), y)
    }

    fn finish_contour(&mut self) {
        let Some(mut contour) = self.contour.take() else {
            return;
        };
        if let (Some(current), Some(start)) = (self.current, self.start)
            && current != start
        {
            contour.add_edge(EdgeSegment::line(current, start));
        }
        if !contour.is_empty() {
            self.shape.add_contour(contour);
        }
        self.current = None;
        self.start = None;
    }

    fn finish(&mut self) {
        self.finish_contour();
    }
}

impl OutlinePen for ShapePen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.finish_contour();
        let point = self.point(x, y);
        self.contour = Some(Contour::new());
        self.current = Some(point);
        self.start = Some(point);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let next = self.point(x, y);
        if let Some(current) = self.current
            && let Some(contour) = self.contour.as_mut()
        {
            contour.add_edge(EdgeSegment::line(current, next));
        }
        self.current = Some(next);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let control = self.point(cx, cy);
        let next = self.point(x, y);
        if let Some(current) = self.current
            && let Some(contour) = self.contour.as_mut()
        {
            contour.add_edge(EdgeSegment::quadratic(current, control, next));
        }
        self.current = Some(next);
    }

    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        let control0 = self.point(c0x, c0y);
        let control1 = self.point(c1x, c1y);
        let next = self.point(x, y);
        if let Some(current) = self.current
            && let Some(contour) = self.contour.as_mut()
        {
            contour.add_edge(EdgeSegment::cubic(current, control0, control1, next));
        }
        self.current = Some(next);
    }

    fn close(&mut self) {
        self.finish_contour();
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "validated logical geometry is narrowed only at the renderer-local f32 GPU ABI boundary"
)]
#[allow(
    clippy::too_many_arguments,
    reason = "this renderer-private geometry helper makes the exact transform, canvas, scale, atlas, and glyph facts explicit at the GPU boundary"
)]
fn glyph_vertex_bytes(
    shaped_run: &SupportedShapedRun,
    glyph: TextGlyph,
    placement: GlyphPlacement,
    page_width: u32,
    page_height: u32,
    extent: super::super::OffscreenExtent,
    canvas_extent: super::super::RasterCanvasExtent,
    raster_scale: RasterScale,
    tier_scale: f64,
) -> Option<Vec<u8>> {
    let origin = LogicalPoint::new(
        shaped_run.origin.x() + glyph.x() + placement.logical_origin.x(),
        shaped_run.origin.y() + glyph.y() + placement.logical_origin.y(),
    )
    .ok()?;
    let rect = LogicalRect::try_new(
        origin.x(),
        origin.y(),
        f64::from(placement.width) as f32 / tier_scale as f32,
        f64::from(placement.height) as f32 / tier_scale as f32,
    )
    .ok()?;
    let local_to_surface = LogicalTransform::translation(0.0, 0.0)
        .ok()?
        .then(shaped_run.local_to_surface)
        .ok()?;
    let fill = SupportedFillRect {
        rect,
        color: shaped_run.foreground,
        opacity: shaped_run.opacity,
        local_to_surface,
    };
    let polygon = super::super::transformed_fill_polygon(&fill, canvas_extent, raster_scale);
    if polygon.len() < 3 {
        return None;
    }
    let surface_to_local = local_to_surface.inverse()?;
    let [m11, m12, m21, m22, tx, ty] = surface_to_local.components().map(f64::from);
    let actual_scale = f64::from(raster_scale.get());
    let origin_x = f64::from(rect.x());
    let origin_y = f64::from(rect.y());
    let width = f64::from(rect.width());
    let height = f64::from(rect.height());
    let foreground = [
        super::super::srgb8_to_linear_f32(shaped_run.foreground.red()),
        super::super::srgb8_to_linear_f32(shaped_run.foreground.green()),
        super::super::srgb8_to_linear_f32(shaped_run.foreground.blue()),
        f32::from(shaped_run.foreground.alpha()) / 255.0,
    ];
    let page_width = f64::from(page_width);
    let page_height = f64::from(page_height);
    let vertices = polygon
        .into_iter()
        .map(|point| {
            let surface_x = point[0] / actual_scale;
            let surface_y = point[1] / actual_scale;
            let local_x = m11.mul_add(surface_x, m21.mul_add(surface_y, tx));
            let local_y = m12.mul_add(surface_x, m22.mul_add(surface_y, ty));
            let field_x = (local_x - origin_x).clamp(0.0, width);
            let field_y = (local_y - origin_y).clamp(0.0, height);
            let uv = [
                (f64::from(placement.x) + field_x) / page_width,
                (f64::from(placement.y) + field_y) / page_height,
            ];
            (
                super::super::physical_point_to_ndc(point, extent),
                [uv[0] as f32, uv[1] as f32],
                shaped_run.opacity.get(),
                foreground,
            )
        })
        .collect::<Vec<_>>();
    let mut bytes = Vec::with_capacity(
        vertices
            .len()
            .saturating_mul(3 * SHAPED_VERTEX_STRIDE as usize),
    );
    for index in 1..vertices.len() - 1 {
        for (position, uv, opacity, foreground) in
            [vertices[0], vertices[index], vertices[index + 1]]
        {
            push_shaped_vertex(&mut bytes, position, uv, opacity, foreground);
        }
    }
    Some(bytes)
}

fn push_shaped_vertex(
    bytes: &mut Vec<u8>,
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    foreground: [f32; 4],
) {
    for component in position
        .into_iter()
        .chain(uv)
        .chain([opacity])
        .chain(foreground)
    {
        bytes.extend_from_slice(&component.to_ne_bytes());
    }
}

fn create_shaped_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    bind_group_layout: &wgpu::BindGroupLayout,
    clipped: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("runenui shaped-text MSDF atlas shader"),
        source: wgpu::ShaderSource::Wgsl(SHAPED_SHADER.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("runenui shaped-text MSDF atlas pipeline layout"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if clipped {
            "runenui clipped shaped-text MSDF atlas pipeline"
        } else {
            "runenui shaped-text MSDF atlas pipeline"
        }),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: SHAPED_VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &SHAPED_ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: clipped.then(super::clipped_fill_stencil_state),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use runenui_core::{
        FontFamilyName, GenericFontFamily, LogicalPoint, LogicalTransform, Typography,
    };
    use runenui_runtime::RasterScale;
    use runenui_text::{FontSourcePolicy, TextConstraints, TextRequest, TextSystem};

    use super::{
        GlyphRaster, QualityTier, max_linear_stretch, pack_atlas, quality_tier,
        rasterize_unique_glyphs,
    };

    const FONT_BYTES: &[u8] = include_bytes!("../../../tests/fixtures/Cantarell-Regular.ttf");

    #[test]
    fn quality_uses_linear_stretch_and_ignores_translation() {
        let transform = LogicalTransform::try_new(3.0, 0.0, 0.0, 2.0, 1000.0, -2000.0)
            .unwrap_or_else(|_| unreachable!());
        assert!((max_linear_stretch(transform) - 3.0).abs() < f64::EPSILON);
        assert_eq!(
            quality_tier(8.0, RasterScale::ONE, transform),
            QualityTier::P24
        );
    }

    #[test]
    fn quality_saturates_at_highest_tier() {
        assert_eq!(
            quality_tier(100.0, RasterScale::ONE, LogicalTransform::IDENTITY),
            QualityTier::P48
        );
    }

    fn shaped_resource(text: &str) -> (TextSystem, runenui_text::TextArtifact) {
        let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
        assert!(system.register_font_bytes(FONT_BYTES.to_vec()).is_ok());
        let family = FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!());
        assert!(
            system
                .set_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
                .is_ok()
        );
        let request = TextRequest::new(text, Typography::default(), TextConstraints::unbounded());
        let artifact = system
            .layout_text(&mut runenui_text::TextLayoutState::new(), &request)
            .unwrap_or_else(|_| unreachable!("controlled bundled text shapes"))
            .into_artifact();
        (system, artifact)
    }

    #[test]
    fn repeated_glyphs_share_one_cpu_field_and_whitespace_is_non_painting() {
        let (_system, repeated) = shaped_resource("AAAA");
        let repeated_resource = repeated.lines()[0].runs()[0].shaped_resource();
        let rasters = rasterize_unique_glyphs(repeated_resource, QualityTier::P16)
            .unwrap_or_else(|_| unreachable!("Cantarell outline realization succeeds"));
        assert_eq!(repeated_resource.glyphs().len(), 4);
        assert_eq!(rasters.len(), 1);
        let (_system, whitespace) = shaped_resource("A A");
        let whitespace_resource = whitespace.lines()[0].runs()[0].shaped_resource();
        let rasters = rasterize_unique_glyphs(whitespace_resource, QualityTier::P16)
            .unwrap_or_else(|_| unreachable!("whitespace is not an unsupported outline"));
        assert_eq!(rasters.len(), 1);
    }

    #[test]
    fn shelf_packing_is_deterministic_and_non_overlapping_across_pages() {
        let point = LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!());
        let field: Arc<[u8]> = Arc::from(vec![255; 8 * 8 * 4]);
        let glyphs = (0..3)
            .map(|glyph_id| GlyphRaster {
                glyph_id,
                logical_origin: point,
                width: 8,
                height: 8,
                rgba8: field.clone(),
            })
            .collect();
        let (pages, placements) = pack_atlas(glyphs, 16)
            .unwrap_or_else(|_| unreachable!("three small fields fit on deterministic pages"));
        assert_eq!(pages.len(), 3);
        for left in placements.values() {
            for right in placements.values() {
                if left.page_index == right.page_index && left.x < right.x {
                    assert!(left.x + left.width + 2 <= right.x);
                }
            }
        }
    }
}
