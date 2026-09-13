//! Disposable raster realization of ADR 0015 neutral support.
//!
//! The symbolic [`NeutralSupport`] tree remains the geometry authority. This module
//! projects it onto renderer-private physical-pixel masks solely to realize ordinary
//! shadows. Source color, image payload alpha, text/MSDF alpha, item/group opacity,
//! target contents, and device cache state never participate in support generation.
//! Off-surface source support is retained through spread/offset/blur and is cropped
//! only after the complete shadow has been realized.

use std::{fmt, mem::size_of, sync::Arc};

use runenui_core::{LogicalTransform, SceneShape};
use runenui_runtime::{RasterScale, SceneClip};

use crate::tessellation::{TessellatedGeometry, tessellate_fill, tessellate_stroke};

use super::super::super::super::{OffscreenExtent, RasterCanvasExtent};
use super::support::{NeutralPrimitiveSupport, NeutralSupport};

const DISTANCE_INFINITY: f64 = 1.0e30;

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

    fn ensure(self, required_bytes: u64) -> Result<(), MaskError> {
        if required_bytes > self.max_workspace_bytes {
            return Err(MaskError::AllocationExceedsLimit {
                required_bytes,
                max_bytes: self.max_workspace_bytes,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MaskResidency {
    bytes: u64,
}

impl MaskResidency {
    const ZERO: Self = Self { bytes: 0 };

    fn with_bytes(self, additional_bytes: u64, limits: MaskLimits) -> Result<Self, MaskError> {
        let bytes = self
            .bytes
            .checked_add(additional_bytes)
            .ok_or(MaskError::WorkspaceExtentOverflow)?;
        limits.ensure(bytes)?;
        Ok(Self { bytes })
    }

    fn with_payload<T>(self, len: usize, limits: MaskLimits) -> Result<Self, MaskError> {
        self.with_bytes(payload_bytes::<T>(len)?, limits)
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct AlphaMask {
    origin_x: u32,
    origin_y: u32,
    width: u32,
    height: u32,
    alpha: Vec<u8>,
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

    pub(super) const fn alpha(&self) -> &[u8] {
        self.alpha.as_slice()
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
    let residency = MaskResidency::ZERO;
    let Some(mut source) = rasterize_support(source, scale, residency, limits)? else {
        return Ok(None);
    };
    source = signed_euclidean_spread(source, spread * scale, residency, limits)?;
    if source.is_empty() {
        return Ok(None);
    }
    source.origin_x = offset_x.mul_add(scale, source.origin_x);
    source.origin_y = offset_y.mul_add(scale, source.origin_y);
    let sigma = blur_square_half_extent / 3.0 * scale;
    let blur_radius = blur_square_half_extent * scale;
    let blurred = gaussian_blur(source, sigma, blur_radius, residency, limits)?;
    crop_to_final_canvas(blurred, canvas_extent, target_extent, residency, limits)
}

fn rasterize_support(
    support: &Arc<NeutralSupport>,
    scale: f64,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    match support.as_ref() {
        NeutralSupport::Empty => Ok(None),
        NeutralSupport::Primitive(primitive) => {
            rasterize_primitive(primitive, scale, residency, limits)
        }
        NeutralSupport::Union(members) => {
            rasterize_support_union(members, scale, residency, limits)
        }
        NeutralSupport::Shadow {
            source,
            spread,
            offset_x,
            offset_y,
            blur_square_half_extent,
        } => {
            let Some(source) = rasterize_support(source, scale, residency, limits)? else {
                return Ok(None);
            };
            let mut shadow =
                signed_euclidean_spread(source, *spread * scale, residency, limits)?;
            if shadow.is_empty() {
                return Ok(None);
            }
            shadow.origin_x = (*offset_x).mul_add(scale, shadow.origin_x);
            shadow.origin_y = (*offset_y).mul_add(scale, shadow.origin_y);
            square_dilate(
                shadow,
                *blur_square_half_extent * scale,
                residency,
                limits,
            )
            .map(Some)
        }
        NeutralSupport::Clip { source, clips } => {
            let Some(mut source) = rasterize_support(source, scale, residency, limits)? else {
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
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    let mut union = None;
    for member in members {
        let member_residency = residency_with_mask(residency, union.as_ref(), limits)?;
        let next = rasterize_support(member, scale, member_residency, limits)?;
        union = merge_masks(union, next, residency, limits)?;
    }
    Ok(union)
}

fn rasterize_primitive(
    primitive: &NeutralPrimitiveSupport,
    scale: f64,
    residency: MaskResidency,
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
            residency,
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
            residency,
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
                let member_residency = residency_with_mask(residency, union.as_ref(), limits)?;
                let next = geometry_mask(
                    &geometry,
                    *local_to_surface,
                    scale,
                    member_residency,
                    limits,
                )?;
                union = merge_masks(union, next, residency, limits)?;
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
                let member_residency = residency_with_mask(residency, union.as_ref(), limits)?;
                let next = geometry_mask(
                    &geometry,
                    *local_to_surface,
                    scale,
                    member_residency,
                    limits,
                )?;
                union = merge_masks(union, next, residency, limits)?;
            }
            Ok(union)
        }
    }
}

#[derive(Debug, PartialEq)]
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

fn residency_with_mask(
    residency: MaskResidency,
    mask: Option<&RasterMask>,
    limits: MaskLimits,
) -> Result<MaskResidency, MaskError> {
    match mask {
        Some(mask) => residency.with_payload::<u8>(mask.samples.len(), limits),
        None => Ok(residency),
    }
}

fn geometry_mask(
    geometry: &TessellatedGeometry,
    transform: LogicalTransform,
    scale: f64,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    if geometry.positions().is_empty() || geometry.indices().is_empty() {
        return Ok(None);
    }
    let Some((origin_x, origin_y, width, height)) =
        transformed_workspace(geometry, transform, scale)?
    else {
        return Ok(None);
    };
    let mut mask = RasterMask {
        origin_x,
        origin_y,
        width,
        height,
        samples: allocate_u8_samples(width, height, residency, limits)?,
    };
    for triangle in geometry.indices().chunks_exact(3) {
        let a = transform_physical(
            geometry.positions()[usize_from_u32(triangle[0])],
            transform,
            scale,
        );
        let b = transform_physical(
            geometry.positions()[usize_from_u32(triangle[1])],
            transform,
            scale,
        );
        let c = transform_physical(
            geometry.positions()[usize_from_u32(triangle[2])],
            transform,
            scale,
        );
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

fn transformed_workspace(
    geometry: &TessellatedGeometry,
    transform: LogicalTransform,
    scale: f64,
) -> Result<Option<(f64, f64, u32, u32)>, MaskError> {
    let mut points = geometry
        .positions()
        .iter()
        .copied()
        .map(|point| transform_physical(point, transform, scale));
    let Some(first) = points.next() else {
        return Ok(None);
    };
    let mut min_x = first[0];
    let mut min_y = first[1];
    let mut max_x = first[0];
    let mut max_y = first[1];
    for [x, y] in points {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    workspace_from_bounds(min_x, min_y, max_x, max_y)
}

fn workspace_from_bounds(
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
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
    Ok(Some((origin_x, origin_y, width, height)))
}

fn finite_dimension(value: f64) -> Result<u32, MaskError> {
    if !value.is_finite() || value < 0.0 || value > f64::from(u32::MAX) {
        return Err(MaskError::WorkspaceExtentOverflow);
    }
    Ok(f64_to_u32(value))
}

fn sample_count(width: u32, height: u32) -> Result<usize, MaskError> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    usize::try_from(pixels).map_err(|_| MaskError::WorkspaceExtentOverflow)
}

fn payload_bytes<T>(len: usize) -> Result<u64, MaskError> {
    let len = u64::try_from(len).map_err(|_| MaskError::WorkspaceExtentOverflow)?;
    let item_size =
        u64::try_from(size_of::<T>()).map_err(|_| MaskError::WorkspaceExtentOverflow)?;
    len.checked_mul(item_size)
        .ok_or(MaskError::WorkspaceExtentOverflow)
}

fn allocate_u8_samples(
    width: u32,
    height: u32,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<Vec<u8>, MaskError> {
    allocate_filled_vec(sample_count(width, height)?, 0_u8, residency, limits)
}

fn allocate_filled_vec<T: Clone>(
    len: usize,
    value: T,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<Vec<T>, MaskError> {
    residency.with_payload::<T>(len, limits)?;
    fallible_filled_vec(len, value)
}

fn fallible_filled_vec<T: Clone>(len: usize, value: T) -> Result<Vec<T>, MaskError> {
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
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<Option<RasterMask>, MaskError> {
    match (current, next) {
        (None, next) => Ok(next),
        (current, None) => Ok(current),
        (Some(current), Some(next)) => union_pair(current, next, residency, limits).map(Some),
    }
}

fn union_pair(
    left: RasterMask,
    right: RasterMask,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    let min_x = left.origin_x.min(right.origin_x);
    let min_y = left.origin_y.min(right.origin_y);
    let max_x =
        (left.origin_x + f64::from(left.width)).max(right.origin_x + f64::from(right.width));
    let max_y =
        (left.origin_y + f64::from(left.height)).max(right.origin_y + f64::from(right.height));
    let Some((origin_x, origin_y, width, height)) =
        workspace_from_bounds(min_x, min_y, max_x, max_y)?
    else {
        unreachable!("union of two non-empty raster masks has non-empty bounds")
    };
    let live = residency
        .with_payload::<u8>(left.samples.len(), limits)?
        .with_payload::<u8>(right.samples.len(), limits)?;
    let mut result = RasterMask {
        origin_x,
        origin_y,
        width,
        height,
        samples: allocate_u8_samples(width, height, live, limits)?,
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
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius == 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    if radius > 0.0 {
        dilate_euclidean(mask, radius, residency, limits)
    } else {
        erode_euclidean(mask, -radius, residency, limits)
    }
}

fn dilate_euclidean(
    mask: RasterMask,
    radius: f64,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius <= 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    let pad = radius_pad(radius, 0)?;
    let mut padded = pad_mask(mask, pad, residency, limits)?;
    let padded_residency = residency.with_payload::<u8>(padded.samples.len(), limits)?;
    let distances = squared_distance_transform(
        &padded.samples,
        padded.width,
        padded.height,
        true,
        padded_residency,
        limits,
    )?;
    let threshold = radius * radius;
    for (sample, distance) in padded.samples.iter_mut().zip(distances.iter().copied()) {
        *sample = if distance <= threshold { u8::MAX } else { 0 };
    }
    Ok(padded)
}

fn erode_euclidean(
    mask: RasterMask,
    radius: f64,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius <= 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    let origin_x = mask.origin_x;
    let origin_y = mask.origin_y;
    let width = mask.width;
    let height = mask.height;
    let pad = radius_pad(radius, 1)?;
    let padded = pad_mask(mask, pad, residency, limits)?;
    let padded_width = usize_from_u32(padded.width);
    let padded_residency = residency.with_payload::<u8>(padded.samples.len(), limits)?;
    let distances = squared_distance_transform(
        &padded.samples,
        padded.width,
        padded.height,
        false,
        padded_residency,
        limits,
    )?;
    drop(padded);

    let distance_residency = residency.with_payload::<f64>(distances.len(), limits)?;
    let mut samples = allocate_u8_samples(width, height, distance_residency, limits)?;
    let threshold = radius * radius;
    let source_width = usize_from_u32(width);
    let source_height = usize_from_u32(height);
    let pad_usize = usize_from_u32(pad);
    for y in 0..source_height {
        for x in 0..source_width {
            let padded_index = (y + pad_usize) * padded_width + x + pad_usize;
            samples[y * source_width + x] = if distances[padded_index] > threshold {
                u8::MAX
            } else {
                0
            };
        }
    }
    Ok(RasterMask {
        origin_x,
        origin_y,
        width,
        height,
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

fn pad_mask(
    mask: RasterMask,
    pad: u32,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if pad == 0 {
        return Ok(mask);
    }
    let RasterMask {
        origin_x,
        origin_y,
        width: source_width,
        height: source_height,
        samples: source_samples,
    } = mask;
    let double_pad = pad
        .checked_mul(2)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let width = source_width
        .checked_add(double_pad)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let height = source_height
        .checked_add(double_pad)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let source_residency = residency.with_payload::<u8>(source_samples.len(), limits)?;
    let mut samples = allocate_u8_samples(width, height, source_residency, limits)?;
    let destination_width = usize_from_u32(width);
    let source_width_usize = usize_from_u32(source_width);
    let pad_usize = usize_from_u32(pad);
    for y in 0..usize_from_u32(source_height) {
        let source = y * source_width_usize;
        let destination = (y + pad_usize) * destination_width + pad_usize;
        samples[destination..destination + source_width_usize]
            .copy_from_slice(&source_samples[source..source + source_width_usize]);
    }
    drop(source_samples);
    Ok(RasterMask {
        origin_x: origin_x - f64::from(pad),
        origin_y: origin_y - f64::from(pad),
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
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<Vec<f64>, MaskError> {
    let width = usize_from_u32(width);
    let height = usize_from_u32(height);
    let len = sample_count(
        u32::try_from(width).map_err(|_| MaskError::WorkspaceExtentOverflow)?,
        u32::try_from(height).map_err(|_| MaskError::WorkspaceExtentOverflow)?,
    )?;
    let max_dimension = width.max(height);
    let boundary_len = max_dimension
        .checked_add(1)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    residency
        .with_payload::<f64>(len, limits)?
        .with_payload::<f64>(max_dimension, limits)?
        .with_payload::<f64>(max_dimension, limits)?
        .with_payload::<usize>(max_dimension, limits)?
        .with_payload::<f64>(boundary_len, limits)?;

    let mut grid = fallible_filled_vec(len, 0.0_f64)?;
    let mut line = fallible_filled_vec(max_dimension, 0.0_f64)?;
    let mut transformed = fallible_filled_vec(max_dimension, 0.0_f64)?;
    let mut locations = fallible_filled_vec(max_dimension, 0_usize)?;
    let mut boundaries = fallible_filled_vec(boundary_len, 0.0_f64)?;

    for x in 0..width {
        for y in 0..height {
            let is_feature = (samples[y * width + x] != 0) == feature;
            line[y] = if is_feature { 0.0 } else { DISTANCE_INFINITY };
        }
        edt_1d(
            &line[..height],
            &mut transformed[..height],
            &mut locations[..height],
            &mut boundaries[..=height],
        );
        for y in 0..height {
            grid[y * width + x] = transformed[y];
        }
    }

    for y in 0..height {
        let start = y * width;
        line[..width].copy_from_slice(&grid[start..start + width]);
        edt_1d(
            &line[..width],
            &mut transformed[..width],
            &mut locations[..width],
            &mut boundaries[..=width],
        );
        grid[start..start + width].copy_from_slice(&transformed[..width]);
    }
    Ok(grid)
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
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if radius <= 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    let pad = radius_pad(radius, 0)?;
    let kernel_radius = bounded_f64_to_usize(radius.floor());
    let mut padded = pad_mask(mask, pad, residency, limits)?;
    if kernel_radius == 0 {
        return Ok(padded);
    }
    let width = usize_from_u32(padded.width);
    let height = usize_from_u32(padded.height);
    let prefix_len = width
        .max(height)
        .checked_add(1)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let padded_residency = residency.with_payload::<u8>(padded.samples.len(), limits)?;
    let mut prefix = allocate_filled_vec(prefix_len, 0_u32, padded_residency, limits)?;

    for y in 0..height {
        prefix[0] = 0;
        for x in 0..width {
            prefix[x + 1] = prefix[x] + u32::from(padded.samples[y * width + x] != 0);
        }
        for x in 0..width {
            let start = x.saturating_sub(kernel_radius);
            let end = x.saturating_add(kernel_radius).saturating_add(1).min(width);
            padded.samples[y * width + x] = if prefix[end] == prefix[start] {
                0
            } else {
                u8::MAX
            };
        }
    }

    for x in 0..width {
        prefix[0] = 0;
        for y in 0..height {
            prefix[y + 1] = prefix[y] + u32::from(padded.samples[y * width + x] != 0);
        }
        for y in 0..height {
            let start = y.saturating_sub(kernel_radius);
            let end = y
                .saturating_add(kernel_radius)
                .saturating_add(1)
                .min(height);
            padded.samples[y * width + x] = if prefix[end] == prefix[start] {
                0
            } else {
                u8::MAX
            };
        }
    }
    Ok(padded)
}

fn gaussian_blur(
    mask: RasterMask,
    sigma: f64,
    blur_radius: f64,
    residency: MaskResidency,
    limits: MaskLimits,
) -> Result<RasterMask, MaskError> {
    if blur_radius <= 0.0 || sigma <= 0.0 || mask.is_empty() {
        return Ok(mask);
    }
    let pad = radius_pad(blur_radius, 0)?;
    let kernel_radius = bounded_f64_to_usize(blur_radius.floor());
    let mut padded = pad_mask(mask, pad, residency, limits)?;
    if kernel_radius == 0 {
        return Ok(padded);
    }
    let weights_len = kernel_radius
        .checked_add(1)
        .ok_or(MaskError::WorkspaceExtentOverflow)?;
    let padded_residency = residency.with_payload::<u8>(padded.samples.len(), limits)?;
    padded_residency
        .with_payload::<f64>(padded.samples.len(), limits)?
        .with_payload::<f64>(weights_len, limits)?;

    let mut weights = fallible_filled_vec(weights_len, 0.0_f64)?;
    for (offset, weight) in weights.iter_mut().enumerate() {
        *weight = (-0.5 * (usize_as_f64(offset) / sigma).powi(2)).exp();
    }
    let normalization = 2.0_f64.mul_add(weights.iter().skip(1).sum::<f64>(), weights[0]);
    for weight in &mut weights {
        *weight /= normalization;
    }

    let width = usize_from_u32(padded.width);
    let height = usize_from_u32(padded.height);
    let mut horizontal = fallible_filled_vec(padded.samples.len(), 0.0_f64)?;
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
            padded.samples[y * width + x] = unit_to_u8(value);
        }
    }
    Ok(padded)
}

fn crop_to_final_canvas(
    mask: RasterMask,
    canvas_extent: RasterCanvasExtent,
    target_extent: OffscreenExtent,
    residency: MaskResidency,
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

    let mut min_x = end_x;
    let mut min_y = end_y;
    let mut max_x = origin_x;
    let mut max_y = origin_y;
    let mut found = false;
    for target_y in origin_y..end_y {
        let surface_y = f64::from(target_y) + 0.5;
        if surface_y >= canvas_extent.height() {
            continue;
        }
        for target_x in origin_x..end_x {
            let surface_x = f64::from(target_x) + 0.5;
            if surface_x >= canvas_extent.width() || mask.sample(surface_x, surface_y) == 0 {
                continue;
            }
            found = true;
            min_x = min_x.min(target_x);
            min_y = min_y.min(target_y);
            max_x = max_x.max(target_x.saturating_add(1));
            max_y = max_y.max(target_y.saturating_add(1));
        }
    }
    if !found {
        return Ok(None);
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let source_residency = residency.with_payload::<u8>(mask.samples.len(), limits)?;
    let mut alpha = allocate_u8_samples(width, height, source_residency, limits)?;
    let width_usize = usize_from_u32(width);
    for y in 0..usize_from_u32(height) {
        let target_y = min_y
            .checked_add(u32::try_from(y).map_err(|_| MaskError::WorkspaceExtentOverflow)?)
            .ok_or(MaskError::WorkspaceExtentOverflow)?;
        let surface_y = f64::from(target_y) + 0.5;
        for x in 0..width_usize {
            let target_x = min_x
                .checked_add(u32::try_from(x).map_err(|_| MaskError::WorkspaceExtentOverflow)?)
                .ok_or(MaskError::WorkspaceExtentOverflow)?;
            let surface_x = f64::from(target_x) + 0.5;
            alpha[y * width_usize + x] = mask.sample(surface_x, surface_y);
        }
    }
    drop(mask);
    Ok(Some(AlphaMask {
        origin_x: min_x,
        origin_y: min_y,
        width,
        height,
        alpha,
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

    use super::{
        MaskError, MaskLimits, MaskResidency, prepare_visual_shadow, rasterize_support,
        squared_distance_transform,
    };
    use crate::backend::clipped::resource::group::support::{
        NeutralPrimitiveSupport, NeutralSupport,
    };
    use crate::backend::{OffscreenExtent, RasterCanvasExtent};

    fn rect_support() -> Arc<NeutralSupport> {
        rect_support_at(0.0, 0.0, 8.0, 8.0)
    }

    fn rect_support_at(x: f32, y: f32, width: f32, height: f32) -> Arc<NeutralSupport> {
        Arc::new(NeutralSupport::Primitive(NeutralPrimitiveSupport::Fill {
            shape: SceneShape::rect(
                LogicalRect::try_new(x, y, width, height)
                    .unwrap_or_else(|_| unreachable!("controlled rectangle is valid")),
            ),
            local_to_surface: LogicalTransform::IDENTITY,
        }))
    }

    fn limits() -> MaskLimits {
        MaskLimits::new(64 * 1024 * 1024)
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

    #[test]
    fn anisotropic_edt_accounts_true_live_payload_beyond_old_scalar_oracle() {
        let width = 128_u32;
        let height = 1_u32;
        let samples = vec![0_u8; usize::try_from(width).unwrap_or(0)];
        let obsolete_limit = u64::from(width) * u64::from(height) * 24;
        let unlimited = MaskLimits::new(u64::MAX);
        let residency = MaskResidency::ZERO
            .with_payload::<u8>(samples.len(), unlimited)
            .unwrap_or_else(|_| unreachable!("controlled sample payload is addressable"));
        let error = squared_distance_transform(
            &samples,
            width,
            height,
            true,
            residency,
            MaskLimits::new(obsolete_limit),
        )
        .expect_err("anisotropic EDT must reject the obsolete scalar budget");
        match error {
            MaskError::AllocationExceedsLimit {
                required_bytes,
                max_bytes,
            } => {
                assert_eq!(max_bytes, obsolete_limit);
                assert!(required_bytes > obsolete_limit);
            }
            other => panic!("unexpected anisotropic accounting failure: {other}"),
        }
    }

    #[test]
    fn accumulated_union_residency_is_carried_into_next_member_realization() {
        let first = rect_support_at(0.0, 0.0, 8.0, 8.0);
        let second = rect_support_at(8.0, 0.0, 8.0, 8.0);
        let constrained = MaskLimits::new(96);
        assert!(
            rasterize_support(&first, 1.0, MaskResidency::ZERO, constrained).is_ok(),
            "an individual 8x8 member must fit the controlled budget"
        );
        assert!(
            rasterize_support(&second, 1.0, MaskResidency::ZERO, constrained).is_ok(),
            "an individual 8x8 member must fit the controlled budget"
        );
        let union = NeutralSupport::union([first, second]);
        let error = rasterize_support(&union, 1.0, MaskResidency::ZERO, constrained)
            .expect_err("retained union plus next member must exceed the controlled budget");
        assert!(matches!(error, MaskError::AllocationExceedsLimit { .. }));
    }

    #[test]
    fn ordinary_square_edt_remains_admitted_under_explicit_generous_budget() {
        let width = 16_u32;
        let height = 16_u32;
        let samples = vec![0_u8; 16 * 16];
        let generous = MaskLimits::new(64 * 1024);
        let residency = MaskResidency::ZERO
            .with_payload::<u8>(samples.len(), generous)
            .unwrap_or_else(|_| unreachable!("controlled sample payload fits"));
        let distances = squared_distance_transform(
            &samples,
            width,
            height,
            true,
            residency,
            generous,
        )
        .unwrap_or_else(|_| unreachable!("ordinary square EDT remains admitted"));
        assert_eq!(distances.len(), samples.len());
    }
}