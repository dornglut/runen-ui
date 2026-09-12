//! Disposable raster realization of ADR 0015 neutral support.
//!
//! The symbolic [`NeutralSupport`] tree remains the geometry authority. This module
//! projects it onto renderer-private physical-pixel masks solely to realize ordinary
//! shadows. Source color, image payload alpha, text/MSDF alpha, item/group opacity,
//! target contents, and device cache state never participate in support generation.
//! Off-surface source support is retained through spread/offset/blur and is cropped
//! only after the complete shadow has been realized.

use std::{fmt, sync::Arc};

use runenui_core::{LogicalTransform, SceneShape};
use runenui_runtime::{RasterScale, SceneClip};

use crate::tessellation::{TessellatedGeometry, tessellate_fill, tessellate_stroke};

use super::super::super::super::{OffscreenExtent, RasterCanvasExtent};
use super::support::{NeutralPrimitiveSupport, NeutralSupport};

const DISTANCE_INFINITY: f64 = 1.0e30;
const PEAK_WORKSPACE_BYTES_PER_PIXEL: u64 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MaskLimits {
    max_workspace_bytes: u64,
}

impl MaskLimits {
    pub(super) const fn new(max_workspace_bytes: u64) -> Self {
        Self {
            max_workspace_bytes,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct AlphaMask {
    origin_x: u32,
    origin_y: u32,
    width: u32,
    height: u32,
    alpha: Arc<[u8]>,
}

impl AlphaMask {
    pub(super) const fn origin_x(&self) -> u32 {
        self.origin_x
    }

    pub(super) const fn origin_y(&self) -> u32 {
        self.origin_y
    }

    pub(super) const fn width(&self) -> u32 {
        self.width
    }

    pub(super) const fn height(&self) -> u32 {
        self.height
    }

    pub(super) const fn alpha(&self) -> &Arc<[u8]> {
        &self.alpha
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum MaskError {
    Geometry(String),
    UnresolvedShapedText,
    NonFiniteWorkspace,
    WorkspaceExtentOverflow,
    AllocationExceedsLimit { required_bytes: u64, max_bytes: u64 },
    AllocationFailed,
}

impl fmt::Display for MaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(detail) => write!(formatter, "neutral-support geometry failed: {detail}"),
            Self::UnresolvedShapedText => formatter.write_str(
                "neutral-support shaped text reached raster realization before exact outline resolution",
            ),
            Self::NonFiniteWorkspace => {
                formatter.write_str("neutral-support raster workspace is non-finite")
            }
            Self::WorkspaceExtentOverflow => formatter.write_str(
                "neutral-support raster workspace exceeds the renderer's addressable pixel range",
            ),
            Self::AllocationExceedsLimit {
                required_bytes,
                max_bytes,
            } => write!(
                formatter,
                "neutral-support raster workspace requires {required_bytes} bytes, exceeding renderer allocation limit {max_bytes}"
            ),
            Self::AllocationFailed => formatter
                .write_str("neutral-support raster workspace allocation failed"),
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the disposable visual-shadow boundary keeps exact symbolic source geometry, effect scalars, raster facts, final-canvas clipping, and allocation policy explicit"
)]
pub(super) fn prepare_visual_shadow(
    source: &Arc<NeutralSupport>,
    spread: f64,
    offset_x: f64,
    offset_y: f64,
    blur_square_half_extent: f64,
    raster_scale: RasterScale,
    canvas_extent: RasterCanvasExtent,
    target_extent: OffscreenExtent,
    limits: MaskLimits,
) -> Result<Option<AlphaMask>, MaskError> {
    let scale = f64::from(raster_scale.get());
    let Some(mut source) = rasterize_support(source, scale, limits)? else {
        return Ok(None);
    };
    source = signed_euclidean_spread(source, spread * scale, limits)?;
    if source.is_empty() {
        return Ok(None);
    }
    source.origin_x = offset_x.mul_add(scale, source.origin_x);
    source.origin_y = offset_y.mul_add(scale, source.origin_y);
    let sigma = blur_square_half_extent / 3.0 * scale;
    let blur_radius = blur_square_half_extent * scale;
    let blurred = gaussian_blur(source, sigma, blur_radius, limits)?;
    crop_to_final_canvas(&blurred, canvas_extent, target_extent, limits)
}

fn rasterize_support(
    support: &Arc<NeutralSupport>,
    scale: f64,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    match support.as_ref() {
        NeutralSupport::Empty => Ok(None),
        NeutralSupport::Primitive(primitive) => rasterize_primitive(primitive, scale, limits),
        NeutralSupport::Union(members) => rasterize_support_union(members, scale, limits),
        NeutralSupport::Shadow {
            source,
            spread,
            offset_x,
            offset_y,
            blur_square_half_extent,
        } => {
            let Some(source) = rasterize_support(source, scale, limits)? else {
                return Ok(None);
            };
            let mut shadow = signed_euclidean_spread(source, *spread * scale, limits)?;
            if shadow.is_empty() {
                return Ok(None);
            }
            shadow.origin_x = (*offset_x).mul_add(scale, shadow.origin_x);
            shadow.origin_y = (*offset_y).mul_add(scale, shadow.origin_y);
            square_dilate(shadow, *blur_square_half_extent * scale, limits).map(Some)
        }
        NeutralSupport::Clip { source, clips } => {
            let Some(mut source) = rasterize_support(source, scale, limits)? else {
                return Ok(None);
            };
            for clip in clips.iter() {
                intersect_clip(&mut source, clip, scale)?;
                if source.is_empty() {
                    return Ok(None);
                }
            }
            Ok(Some(source))
        }
    }
}

fn rasterize_support_union(
    members: &[Arc<NeutralSupport>],
    scale: f64,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    let mut union = None;
    for member in members {
        union = merge_masks(union, rasterize_support(member, scale, limits)?, limits)?;
    }
    Ok(union)
}

fn rasterize_primitive(
    primitive: &NeutralPrimitiveSupport,
    scale: f64,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    match primitive {
        NeutralPrimitiveSupport::Fill {
            shape,
            local_to_surface,
        } => geometry_mask(
            &tessellate_fill(shape).map_err(|error| MaskError::Geometry(error.to_string()))?,
            *local_to_surface,
            scale,
            limits,
        ),
        NeutralPrimitiveSupport::Stroke {
            shape,
            style,
            local_to_surface,
        } => geometry_mask(
            &tessellate_stroke(shape, *style)
                .map_err(|error| MaskError::Geometry(error.to_string()))?,
            *local_to_surface,
            scale,
            limits,
        ),
        NeutralPrimitiveSupport::Image {
            destinations,
            local_to_surface,
        } => {
            let mut union = None;
            for destination in destinations.iter() {
                let shape = SceneShape::rect(*destination);
                let geometry = tessellate_fill(&shape)
                    .map_err(|error| MaskError::Geometry(error.to_string()))?;
                union = merge_masks(
                    union,
                    geometry_mask(&geometry, *local_to_surface, scale, limits)?,
                    limits,
                )?;
            }
            Ok(union)
        }
        NeutralPrimitiveSupport::ShapedText { .. } => Err(MaskError::UnresolvedShapedText),
        NeutralPrimitiveSupport::ShapedTextPaths {
            paths,
            local_to_surface,
        } => {
            let mut union = None;
            for path in paths.iter() {
                let shape = SceneShape::path(path.clone());
                let geometry = tessellate_fill(&shape)
                    .map_err(|error| MaskError::Geometry(error.to_string()))?;
                union = merge_masks(
                    union,
                    geometry_mask(&geometry, *local_to_surface, scale, limits)?,
                    limits,
                )?;
            }
            Ok(union)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RasterMask {
    origin_x: f64,
    origin_y: f64,
    width: u32,
    height: u32,
    samples: Vec<u8>,
}

impl RasterMask {
    fn is_empty(&self) -> bool {
        self.samples.iter().all(|sample| *sample == 0)
    }

    fn sample(&self, surface_x: f64, surface_y: f64) -> u8 {
        let local_x = (surface_x - self.origin_x).floor();
        let local_y = (surface_y - self.origin_y).floor();
        if !local_x.is_finite()
            || !local_y.is_finite()
            || local_x < 0.0
            || local_y < 0.0
            || local_x >= f64::from(self.width)
            || local_y >= f64::from(self.height)
        {
            return 0;
        }
        let x = bounded_f64_to_usize(local_x);
        let y = bounded_f64_to_usize(local_y);
        self.samples[y * usize_from_u32(self.width) + x]
    }
}

fn geometry_mask(
    geometry: &TessellatedGeometry,
    transform: LogicalTransform,
    scale: f64,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    if geometry.positions().is_empty() || geometry.indices().is_empty() {
        return Ok(None);
    }
    let transformed = geometry
        .positions()
        .iter()
        .copied()
        .map(|point| transform_physical(point, transform, scale))
        .collect::<Vec<_>>();
    let Some((origin_x, origin_y, width, height)) = point_workspace(&transformed, limits)? else {
        return Ok(None);
    };
    let mut mask = RasterMask {
        origin_x,
        origin_y,
        width,
        height,
        samples: allocate_u8_samples(width, height, limits)?,
    };
    for triangle in geometry.indices().chunks_exact(3) {
        let a = transformed[usize_from_u32(triangle[0])];
        let b = transformed[usize_from_u32(triangle[1])];
        let c = transformed[usize_from_u32(triangle[2])];
        rasterize_triangle(&mut mask, a, b, c);
    }
    Ok((!mask.is_empty()).then_some(mask))
}

fn transform_physical(point: [f32; 2], transform: LogicalTransform, scale: f64) -> [f64; 2] {
    let [m11, m12, m21, m22, tx, ty] = transform.components().map(f64::from);
    let x = f64::from(point[0]);
    let y = f64::from(point[1]);
    [
        m11.mul_add(x, m21.mul_add(y, tx)) * scale,
        m12.mul_add(x, m22.mul_add(y, ty)) * scale,
    ]
}

fn point_workspace(
    points: &[[f64; 2]],
    limits: MaskLimits,
) -> Result<Option<(f64, f64, u32, u32)>, MaskError> {
    let Some(first) = points.first().copied() else {
        return Ok(None);
    };
    let mut min_x = first[0];
    let mut min_y = first[1];
    let mut max_x = first[0];
    let mut max_y = first[1];
    for [x, y] in points.iter().copied().skip(1) {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    workspace_from_bounds(min_x, min_y, max_x, max_y, limits)
}

fn workspace_from_bounds(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    limits: MaskLimits,
) -> Result<Option<(f64, f64, u32, u32)>, MaskError> {
    if ![min_x, min_y, max_x, max_y].into_iter().all(f64::is_finite) {
        return Err(MaskError::NonFiniteWorkspace);
    }
    let origin_x = min_x.floor();
    let origin_y = min_y.floor();
    let end_x = max_x.ceil();
    let end_y = max_y.ceil();
    if end_x <= origin_x || end_y <= origin_y {
        return Ok(None);
    }
    let width = finite_dimension(end_x - origin_x)?;
    let height = finite_dimension(end_y - origin_y)?;
    validate_workspace(width, height, limits)?;
    Ok(Some((origin_x, origin_y, width, height)))
}

fn finite_dimension(value: f64) -> Result<u32, MaskError> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
        return Err(MaskError::WorkspaceExtentOverflow);
    }
    Ok(f64_to_u32(value))
}

fn validate_workspace(width: u32, height: u32, limits: MaskLimits) -> Result<(), MaskError> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let required_bytes = pixels
        .checked_mul(PEAK_WORKSPACE_BYTES_PER_PIXEL)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    if required_bytes > limits.max_workspace_bytes {
        return Err(MaskError::AllocationExceedsLimit {
            required_bytes,
            max_bytes: limits.max_workspace_bytes,
        });
    }
    Ok(())
}

fn sample_count(width: u32, height: u32, limits: MaskLimits) -> Result<usize, MaskError> {
    validate_workspace(width, height, limits)?;
    usize::try_from(u64::from(width) * u64::from(height))
        .map_err(|_| MaskError::WorkspaceExtentOverflow)
}

fn allocate_u8_samples(width: u32, height: u32, limits: MaskLimits) -> Result<Vec<u8>, MaskError> {
    zeroed_vec(sample_count(width, height, limits)?, 0_u8)
}

fn zeroed_vec<T: Clone>(len: usize, value: T) -> Result<Vec<T>, MaskError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| MaskError::AllocationFailed)?;
    values.resize(len, value);
    Ok(values)
}

fn rasterize_triangle(mask: &mut RasterMask, a: [f64; 2], b: [f64; 2], c: [f64; 2]) {
    if triangle_area2(a, b, c) == 0.0 {
        return;
    }
    let min_x = a[0].min(b[0]).min(c[0]).floor().max(mask.origin_x);
    let min_y = a[1].min(b[1]).min(c[1]).floor().max(mask.origin_y);
    let max_x = a[0]
        .max(b[0])
        .max(c[0])
        .ceil()
        .min(mask.origin_x + f64::from(mask.width));
    let max_y = a[1]
        .max(b[1])
        .max(c[1])
        .ceil()
        .min(mask.origin_y + f64::from(mask.height));
    let start_x = bounded_f64_to_usize((min_x - mask.origin_x).max(0.0));
    let start_y = bounded_f64_to_usize((min_y - mask.origin_y).max(0.0));
    let end_x = bounded_f64_to_usize((max_x - mask.origin_x).max(0.0));
    let end_y = bounded_f64_to_usize((max_y - mask.origin_y).max(0.0));
    let width = usize_from_u32(mask.width);
    for y in start_y..end_y {
        let py = mask.origin_y + usize_as_f64(y) + 0.5;
        for x in start_x..end_x {
            let px = mask.origin_x + usize_as_f64(x) + 0.5;
            if point_in_triangle([px, py], a, b, c) {
                mask.samples[y * width + x] = u8::MAX;
            }
        }
    }
}

fn point_in_triangle(point: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let ab = edge_sign(point, a, b);
    let bc = edge_sign(point, b, c);
    let ca = edge_sign(point, c, a);
    let has_negative = ab < 0.0 || bc < 0.0 || ca < 0.0;
    let has_positive = ab > 0.0 || bc > 0.0 || ca > 0.0;
    !(has_negative && has_positive)
}

fn triangle_area2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[1] - a[1]).mul_add(-(c[0] - a[0]), (b[0] - a[0]) * (c[1] - a[1]))
}

fn edge_sign(point: [f64; 2], from: [f64; 2], to: [f64; 2]) -> f64 {
    (from[0] - to[0]).mul_add(-(point[1] - to[1]), (point[0] - to[0]) * (from[1] - to[1]))
}

fn merge_masks(
    current: Option<RasterMask>,
    next: Option<RasterMask>,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    match (current, next) {
        (None, next) => Ok(next),
        (current, None) => Ok(current),
        (Some(current), Some(next)) => union_pair(&current, &next, limits).map(Some),
    }
}

fn union_pair(
    left: &RasterMask,
    right: &RasterMask,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    let min_x = left.origin_x.min(right.origin_x);
    let min_y = left.origin_y.min(right.origin_y);
    let max_x =
        (left.origin_x + f64::from(left.width)).max(right.origin_x + f64::from(right.width));
    let max_y =
        (left.origin_y + f64::from(left.height)).max(right.origin_y + f64::from(right.height));
    let Some((origin_x, origin_y, width, height)) =
        workspace_from_bounds(min_x, min_y, max_x, max_y, limits)?
    else {
        unreachable!("union of two non-empty raster masks has non-empty bounds")
    };
    let mut result = RasterMask {
        origin_x,
        origin_y,
        width,
        height,
        samples: allocate_u8_samples(width, height, limits)?,
    };
    let width_usize = usize_from_u32(width);
    for y in 0..usize_from_u32(height) {
        let surface_y = origin_y + usize_as_f64(y) + 0.5;
        for x in 0..width_usize {
            let surface_x = origin_x + usize_as_f64(x) + 0.5;
            if left.sample(surface_x, surface_y) != 0 || right.sample(surface_x, surface_y) != 0 {
                result.samples[y * width_usize + x] = u8::MAX;
            }
        }
    }
    Ok(result)
}

fn intersect_clip(mask: &mut RasterMask, clip: &SceneClip, scale: f64) -> Result<(), MaskError> {
    let width = usize_from_u32(mask.width);
    for y in 0..usize_from_u32(mask.height) {
        let surface_y = mask.origin_y + usize_as_f64(y) + 0.5;
        for x in 0..width {
            let index = y * width + x;
            if mask.samples[index] == 0 {
                continue;
            }
            let surface_x = mask.origin_x + usize_as_f64(x) + 0.5;
            let logical = runenui_core::LogicalPoint::new(
                f64_to_f32(surface_x / scale),
                f64_to_f32(surface_y / scale),
            )
            .map_err(|_| MaskError::NonFiniteWorkspace)?;
            if !clip.contains_surface_point(logical) {
                mask.samples[index] = 0;
            }
        }
    }
    Ok(())
}

fn signed_euclidean_spread(
    mask: RasterMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius == 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    if radius > 0.0 {
        dilate_euclidean(&mask, radius, limits)
    } else {
        erode_euclidean(&mask, -radius, limits)
    }
}

fn dilate_euclidean(
    mask: &RasterMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius <= 0.0 || mask.is_empty() {
        return Ok(mask.clone());
    }
    let pad = radius_pad(radius, 0)?;
    let padded = pad_mask(mask, pad, limits)?;
    let distances =
        squared_distance_transform(&padded.samples, padded.width, padded.height, true, limits)?;
    let threshold = radius * radius;
    let samples = distances
        .into_iter()
        .map(|distance| if distance <= threshold { u8::MAX } else { 0 })
        .collect();
    Ok(RasterMask { samples, ..padded })
}

fn erode_euclidean(
    mask: &RasterMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius <= 0.0 || mask.is_empty() {
        return Ok(mask.clone());
    }
    let pad = radius_pad(radius, 1)?;
    let padded = pad_mask(mask, pad, limits)?;
    let distances =
        squared_distance_transform(&padded.samples, padded.width, padded.height, false, limits)?;
    let threshold = radius * radius;
    let padded_width = usize_from_u32(padded.width);
    let source_width = usize_from_u32(mask.width);
    let source_height = usize_from_u32(mask.height);
    let pad_usize = usize_from_u32(pad);
    let mut samples = allocate_u8_samples(mask.width, mask.height, limits)?;
    for y in 0..source_height {
        for x in 0..source_width {
            let source_index = y * source_width + x;
            if mask.samples[source_index] == 0 {
                continue;
            }
            let padded_index = (y + pad_usize) * padded_width + x + pad_usize;
            samples[source_index] = if distances[padded_index] > threshold {
                u8::MAX
            } else {
                0
            };
        }
    }
    Ok(RasterMask {
        origin_x: mask.origin_x,
        origin_y: mask.origin_y,
        width: mask.width,
        height: mask.height,
        samples,
    })
}

fn radius_pad(radius: f64, extra: u32) -> Result<u32, MaskError> {
    if !radius.is_finite() || radius < 0.0 {
        return Err(MaskError::NonFiniteWorkspace);
    }
    let rounded = radius.ceil();
    if rounded > f64::from(u32::MAX) {
        return Err(MaskError::WorkspaceExtentOverflow);
    }
    f64_to_u32(rounded)
        .checked_add(extra)
        .ok_or(MaskError::WorkspaceExtentOverflow)
}

fn pad_mask(mask: &RasterMask, pad: u32, limits: MaskLimits) -> Result<RasterMask, MaskError> {
    if pad == 0 {
        return Ok(mask.clone());
    }
    let double_pad = pad
        .checked_mul(2)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let width = mask
        .width
        .checked_add(double_pad)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let height = mask
        .height
        .checked_add(double_pad)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let mut samples = allocate_u8_samples(width, height, limits)?;
    let destination_width = usize_from_u32(width);
    let source_width = usize_from_u32(mask.width);
    let pad_usize = usize_from_u32(pad);
    for y in 0..usize_from_u32(mask.height) {
        let source = y * source_width;
        let destination = (y + pad_usize) * destination_width + pad_usize;
        samples[destination..destination + source_width]
            .copy_from_slice(&mask.samples[source..source + source_width]);
    }
    Ok(RasterMask {
        origin_x: mask.origin_x - f64::from(pad),
        origin_y: mask.origin_y - f64::from(pad),
        width,
        height,
        samples,
    })
}

fn squared_distance_transform(
    samples: &[u8],
    width: u32,
    height: u32,
    feature: bool,
    limits: MaskLimits,
) -> Result<Vec<f64>, MaskError> {
    let width = usize_from_u32(width);
    let height = usize_from_u32(height);
    let len = sample_count(
        u32::try_from(width).map_err(|_| MaskError::WorkspaceExtentOverflow)?,
        u32::try_from(height).map_err(|_| MaskError::WorkspaceExtentOverflow)?,
        limits,
    )?;
    let mut intermediate = zeroed_vec(len, 0.0_f64)?;
    let mut result = zeroed_vec(len, 0.0_f64)?;
    let mut column = zeroed_vec(height, 0.0_f64)?;
    let mut transformed = zeroed_vec(height, 0.0_f64)?;
    let mut locations = zeroed_vec(height.max(width), 0_usize)?;
    let mut boundaries = zeroed_vec(height.max(width).saturating_add(1), 0.0_f64)?;
    for x in 0..width {
        for y in 0..height {
            let is_feature = (samples[y * width + x] != 0) == feature;
            column[y] = if is_feature { 0.0 } else { DISTANCE_INFINITY };
        }
        edt_1d(
            &column,
            &mut transformed,
            &mut locations[..height],
            &mut boundaries[..=height],
        );
        for y in 0..height {
            intermediate[y * width + x] = transformed[y];
        }
    }
    let mut row = zeroed_vec(width, 0.0_f64)?;
    let mut row_transformed = zeroed_vec(width, 0.0_f64)?;
    for y in 0..height {
        let start = y * width;
        row.copy_from_slice(&intermediate[start..start + width]);
        edt_1d(
            &row,
            &mut row_transformed,
            &mut locations[..width],
            &mut boundaries[..=width],
        );
        result[start..start + width].copy_from_slice(&row_transformed);
    }
    Ok(result)
}

fn edt_1d(input: &[f64], output: &mut [f64], locations: &mut [usize], boundaries: &mut [f64]) {
    if input.is_empty() {
        return;
    }
    let n = input.len();
    let mut envelope = 0_usize;
    locations[0] = 0;
    boundaries[0] = f64::NEG_INFINITY;
    boundaries[1] = f64::INFINITY;
    for q in 1..n {
        let mut intersection = parabola_intersection(input, q, locations[envelope]);
        while envelope > 0 && intersection <= boundaries[envelope] {
            envelope -= 1;
            intersection = parabola_intersection(input, q, locations[envelope]);
        }
        envelope += 1;
        locations[envelope] = q;
        boundaries[envelope] = intersection;
        boundaries[envelope + 1] = f64::INFINITY;
    }
    envelope = 0;
    for (q, output_value) in output.iter_mut().enumerate().take(n) {
        let q_float = usize_as_f64(q);
        while matches!(
            boundaries[envelope + 1].partial_cmp(&q_float),
            Some(std::cmp::Ordering::Less)
        ) {
            envelope += 1;
        }
        let location = locations[envelope];
        let delta = usize_as_f64(q.abs_diff(location));
        *output_value = delta.mul_add(delta, input[location]);
    }
}

fn parabola_intersection(input: &[f64], left: usize, right: usize) -> f64 {
    let left_f = usize_as_f64(left);
    let right_f = usize_as_f64(right);
    let numerator = left_f.mul_add(left_f, input[left]) - right_f.mul_add(right_f, input[right]);
    numerator / (2.0 * (left_f - right_f))
}

fn square_dilate(
    mask: RasterMask,
    radius: f64,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius <= 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    let pad = radius_pad(radius, 0)?;
    let kernel_radius = bounded_f64_to_usize(radius.floor());
    let padded = pad_mask(&mask, pad, limits)?;
    if kernel_radius == 0 {
        return Ok(padded);
    }
    let width = usize_from_u32(padded.width);
    let height = usize_from_u32(padded.height);
    let mut horizontal = zeroed_vec(padded.samples.len(), 0_u8)?;
    for y in 0..height {
        let mut prefix = zeroed_vec(width.saturating_add(1), 0_u32)?;
        for x in 0..width {
            prefix[x + 1] = prefix[x] + u32::from(padded.samples[y * width + x] != 0);
        }
        for x in 0..width {
            let start = x.saturating_sub(kernel_radius);
            let end = x.saturating_add(kernel_radius).saturating_add(1).min(width);
            horizontal[y * width + x] = u8::from(prefix[end] != prefix[start]);
        }
    }
    let mut samples = zeroed_vec(padded.samples.len(), 0_u8)?;
    for x in 0..width {
        let mut prefix = zeroed_vec(height.saturating_add(1), 0_u32)?;
        for y in 0..height {
            prefix[y + 1] = prefix[y] + u32::from(horizontal[y * width + x] != 0);
        }
        for y in 0..height {
            let start = y.saturating_sub(kernel_radius);
            let end = y
                .saturating_add(kernel_radius)
                .saturating_add(1)
                .min(height);
            samples[y * width + x] = if prefix[end] == prefix[start] {
                0
            } else {
                u8::MAX
            };
        }
    }
    Ok(RasterMask { samples, ..padded })
}

fn gaussian_blur(
    mask: RasterMask,
    sigma: f64,
    blur_radius: f64,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if blur_radius <= 0.0 || sigma <= 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    let pad = radius_pad(blur_radius, 0)?;
    let kernel_radius = bounded_f64_to_usize(blur_radius.floor());
    let padded = pad_mask(&mask, pad, limits)?;
    if kernel_radius == 0 {
        return Ok(padded);
    }
    let mut weights = (0..=kernel_radius)
        .map(|offset| (-0.5 * (usize_as_f64(offset) / sigma).powi(2)).exp())
        .collect::<Vec<_>>();
    let normalization = 2.0_f64.mul_add(weights.iter().skip(1).sum::<f64>(), weights[0]);
    for weight in &mut weights {
        *weight /= normalization;
    }
    let width = usize_from_u32(padded.width);
    let height = usize_from_u32(padded.height);
    let mut horizontal = zeroed_vec(padded.samples.len(), 0.0_f64)?;
    for y in 0..height {
        for x in 0..width {
            let mut value = weights[0] * u8_to_unit(padded.samples[y * width + x]);
            for (offset, &weight) in weights.iter().enumerate().skip(1) {
                if let Some(left) = x.checked_sub(offset) {
                    value = weight.mul_add(u8_to_unit(padded.samples[y * width + left]), value);
                }
                let right = x + offset;
                if right < width {
                    value = weight.mul_add(u8_to_unit(padded.samples[y * width + right]), value);
                }
            }
            horizontal[y * width + x] = value;
        }
    }
    let mut samples = zeroed_vec(padded.samples.len(), 0_u8)?;
    for y in 0..height {
        for x in 0..width {
            let mut value = weights[0] * horizontal[y * width + x];
            for (offset, &weight) in weights.iter().enumerate().skip(1) {
                if let Some(top) = y.checked_sub(offset) {
                    value = weight.mul_add(horizontal[top * width + x], value);
                }
                let bottom = y + offset;
                if bottom < height {
                    value = weight.mul_add(horizontal[bottom * width + x], value);
                }
            }
            samples[y * width + x] = unit_to_u8(value);
        }
    }
    Ok(RasterMask { samples, ..padded })
}

fn crop_to_final_canvas(
    mask: &RasterMask,
    canvas_extent: RasterCanvasExtent,
    target_extent: OffscreenExtent,
    limits: MaskLimits,
) -> Result<Option<AlphaMask>, MaskError> {
    if mask.is_empty() || canvas_extent.width() <= 0.0 || canvas_extent.height() <= 0.0 {
        return Ok(None);
    }
    let end_x = (mask.origin_x + f64::from(mask.width))
        .min(canvas_extent.width())
        .min(f64::from(target_extent.width()));
    let end_y = (mask.origin_y + f64::from(mask.height))
        .min(canvas_extent.height())
        .min(f64::from(target_extent.height()));
    let start_x = mask.origin_x.max(0.0).floor();
    let start_y = mask.origin_y.max(0.0).floor();
    if end_x <= start_x || end_y <= start_y {
        return Ok(None);
    }
    let origin_x = clamped_f64_to_u32(start_x, target_extent.width());
    let origin_y = clamped_f64_to_u32(start_y, target_extent.height());
    let end_x = clamped_f64_to_u32(end_x.ceil(), target_extent.width());
    let end_y = clamped_f64_to_u32(end_y.ceil(), target_extent.height());
    if end_x <= origin_x || end_y <= origin_y {
        return Ok(None);
    }
    let width = end_x - origin_x;
    let height = end_y - origin_y;
    let mut alpha = allocate_u8_samples(width, height, limits)?;
    let width_usize = usize_from_u32(width);
    for y in 0..usize_from_u32(height) {
        let target_y =
            origin_y + u32::try_from(y).map_err(|_| MaskError::WorkspaceExtentOverflow)?;
        let surface_y = f64::from(target_y) + 0.5;
        if surface_y >= canvas_extent.height() {
            continue;
        }
        for x in 0..width_usize {
            let target_x =
                origin_x + u32::try_from(x).map_err(|_| MaskError::WorkspaceExtentOverflow)?;
            let surface_x = f64::from(target_x) + 0.5;
            if surface_x >= canvas_extent.width() {
                continue;
            }
            alpha[y * width_usize + x] = mask.sample(surface_x, surface_y);
        }
    }
    trim_alpha_mask(origin_x, origin_y, width, height, &alpha)
}

fn trim_alpha_mask(
    origin_x: u32,
    origin_y: u32,
    width: u32,
    height: u32,
    alpha: &[u8],
) -> Result<Option<AlphaMask>, MaskError> {
    let width_usize = usize_from_u32(width);
    let height_usize = usize_from_u32(height);
    let mut min_x = width_usize;
    let mut min_y = height_usize;
    let mut max_x = 0_usize;
    let mut max_y = 0_usize;
    let mut found = false;
    for y in 0..height_usize {
        for x in 0..width_usize {
            if alpha[y * width_usize + x] == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + 1);
            max_y = max_y.max(y + 1);
        }
    }
    if !found {
        return Ok(None);
    }
    let trimmed_width = max_x - min_x;
    let trimmed_height = max_y - min_y;
    let mut trimmed = zeroed_vec(
        trimmed_width
            .checked_mul(trimmed_height)
            .ok_or(MaskError::WorkspaceExtentOverflow)?,
        0_u8,
    )?;
    for y in 0..trimmed_height {
        let source = (min_y + y) * width_usize + min_x;
        let destination = y * trimmed_width;
        trimmed[destination..destination + trimmed_width]
            .copy_from_slice(&alpha[source..source + trimmed_width]);
    }
    Ok(Some(AlphaMask {
        origin_x: origin_x
            .checked_add(u32::try_from(min_x).map_err(|_| MaskError::WorkspaceExtentOverflow)?)
            .ok_or(MaskError::WorkspaceExtentOverflow)?,
        origin_y: origin_y
            .checked_add(u32::try_from(min_y).map_err(|_| MaskError::WorkspaceExtentOverflow)?)
            .ok_or(MaskError::WorkspaceExtentOverflow)?,
        width: u32::try_from(trimmed_width).map_err(|_| MaskError::WorkspaceExtentOverflow)?,
        height: u32::try_from(trimmed_height).map_err(|_| MaskError::WorkspaceExtentOverflow)?,
        alpha: trimmed.into(),
    }))
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "callers prove the finite value is non-negative and bounded by the destination integer range"
)]
const fn bounded_f64_to_usize(value: f64) -> usize {
    value as usize
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "callers prove the finite value lies within the complete u32 range"
)]
const fn f64_to_u32(value: f64) -> u32 {
    value as u32
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "renderer coordinates originate from finite f32 publication geometry and remain bounded before semantic point reconstruction"
)]
const fn f64_to_f32(value: f64) -> f32 {
    value as f32
}

fn clamped_f64_to_u32(value: f64, upper: u32) -> u32 {
    if value <= 0.0 {
        0
    } else if value >= f64::from(upper) {
        upper
    } else {
        f64_to_u32(value)
    }
}

fn usize_from_u32(value: u32) -> usize {
    usize::try_from(value)
        .unwrap_or_else(|_| unreachable!("supported renderer targets address u32 dimensions"))
}

fn usize_as_f64(value: usize) -> f64 {
    f64::from(
        u32::try_from(value)
            .unwrap_or_else(|_| unreachable!("renderer raster dimensions are bounded by u32")),
    )
}

fn u8_to_unit(value: u8) -> f64 {
    f64::from(value) / f64::from(u8::MAX)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the clamped unit interval scaled by 255 is exactly representable by u8"
)]
fn unit_to_u8(value: f64) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use runenui_core::{LogicalRect, LogicalTransform, SceneShape};
    use runenui_runtime::RasterScale;

    use super::{MaskLimits, prepare_visual_shadow};
    use crate::backend::clipped::resource::group::support::{
        NeutralPrimitiveSupport, NeutralSupport,
    };
    use crate::backend::{OffscreenExtent, RasterCanvasExtent};

    fn rect_support() -> Arc<NeutralSupport> {
        Arc::new(NeutralSupport::Primitive(NeutralPrimitiveSupport::Fill {
            shape: SceneShape::rect(
                LogicalRect::try_new(0.0, 0.0, 8.0, 8.0)
                    .unwrap_or_else(|_| unreachable!("controlled rectangle is valid")),
            ),
            local_to_surface: LogicalTransform::IDENTITY,
        }))
    }

    fn limits() -> MaskLimits {
        MaskLimits::new(256 * 256 * 24)
    }

    fn canvas() -> RasterCanvasExtent {
        RasterCanvasExtent::new(64.0, 48.0)
    }

    fn target() -> OffscreenExtent {
        OffscreenExtent::new(64, 48)
            .unwrap_or_else(|_| unreachable!("controlled target is non-zero"))
    }

    #[test]
    fn zero_blur_support_reaches_alpha_mask_as_full_coverage() {
        let shadow = prepare_visual_shadow(
            &rect_support(),
            0.0,
            0.0,
            0.0,
            0.0,
            RasterScale::ONE,
            canvas(),
            target(),
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled mask resolves"))
        .unwrap_or_else(|| unreachable!("rectangle support remains visible"));
        assert_eq!(shadow.origin_x(), 0);
        assert_eq!(shadow.origin_y(), 0);
        assert_eq!(shadow.width(), 8);
        assert_eq!(shadow.height(), 8);
        assert!(shadow.alpha().iter().all(|sample| *sample == u8::MAX));
    }

    #[test]
    fn positive_and_negative_spread_use_sampled_euclidean_distance() {
        let expanded = prepare_visual_shadow(
            &rect_support(),
            2.0,
            0.0,
            0.0,
            0.0,
            RasterScale::ONE,
            canvas(),
            target(),
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled mask resolves"))
        .unwrap_or_else(|| unreachable!("positive spread remains visible"));
        assert!(expanded.width() >= 10 && expanded.height() >= 10);
        let eroded = prepare_visual_shadow(
            &rect_support(),
            -2.0,
            0.0,
            0.0,
            0.0,
            RasterScale::ONE,
            canvas(),
            target(),
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled erosion resolves"))
        .unwrap_or_else(|| unreachable!("8px square survives 2px erosion"));
        assert!(eroded.alpha().iter().filter(|sample| **sample != 0).count() < 64);
    }

    #[test]
    fn complete_erosion_produces_no_visual_shadow() {
        let result = prepare_visual_shadow(
            &rect_support(),
            -8.0,
            0.0,
            0.0,
            0.0,
            RasterScale::ONE,
            canvas(),
            target(),
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled erosion resolves"));
        assert!(result.is_none());
    }

    #[test]
    fn off_surface_source_is_cropped_only_after_offset_can_reach_canvas() {
        let source = Arc::new(NeutralSupport::Primitive(NeutralPrimitiveSupport::Fill {
            shape: SceneShape::rect(
                LogicalRect::try_new(-12.0, 4.0, 8.0, 8.0)
                    .unwrap_or_else(|_| unreachable!("controlled rectangle is valid")),
            ),
            local_to_surface: LogicalTransform::IDENTITY,
        }));
        let shadow = prepare_visual_shadow(
            &source,
            0.0,
            10.0,
            0.0,
            0.0,
            RasterScale::ONE,
            canvas(),
            target(),
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled offset resolves"))
        .unwrap_or_else(|| unreachable!("offset shadow reaches the canvas"));
        assert_eq!(shadow.origin_x(), 0);
        assert!(shadow.alpha().iter().any(|sample| *sample != 0));
    }

    #[test]
    fn gaussian_output_is_cropped_after_three_sigma_realization() {
        let shadow = prepare_visual_shadow(
            &rect_support(),
            0.0,
            4.0,
            4.0,
            3.0,
            RasterScale::ONE,
            canvas(),
            target(),
            limits(),
        )
        .unwrap_or_else(|_| unreachable!("controlled blur resolves"))
        .unwrap_or_else(|| unreachable!("blurred shadow remains visible"));
        assert!(shadow.origin_x() <= 4);
        assert!(shadow.origin_y() <= 4);
        assert!(shadow.width() <= 14);
        assert!(shadow.height() <= 14);
    }
}
