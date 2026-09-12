use runenui_core::{
    ImageFit, ImageMapping, ImagePaintDescriptor, ImagePrimitive, LogicalRect,
    PaintContributionItem, PaintPrimitive,
};

pub(super) fn publication_primitive(item: &PaintContributionItem) -> PaintPrimitive {
    match item.primitive() {
        PaintPrimitive::Image(image) => {
            let descriptor = image.authored_descriptor().unwrap_or_else(|| {
                unreachable!("paint contributions cannot author runtime-resolved image primitives")
            });
            PaintPrimitive::Image(resolve_image(descriptor))
        }
        primitive => primitive.clone(),
    }
}

fn resolve_image(descriptor: &ImagePaintDescriptor) -> ImagePrimitive {
    let image = descriptor.image();
    let patches = match descriptor.mapping() {
        ImageMapping::Fit {
            crop,
            alignment,
            fit,
        } => resolve_fit(
            image.intrinsic_size().width(),
            image.intrinsic_size().height(),
            crop.x().get(),
            crop.y().get(),
            crop.width().get(),
            crop.height().get(),
            alignment.x().get(),
            alignment.y().get(),
            fit,
            descriptor.destination(),
        ),
        ImageMapping::NineSlice {
            source,
            source_insets,
            destination_insets,
        } => resolve_nine_slice(
            image.intrinsic_size().width(),
            image.intrinsic_size().height(),
            source.x().get(),
            source.y().get(),
            source.width().get(),
            source.height().get(),
            [
                source_insets.top(),
                source_insets.right(),
                source_insets.bottom(),
                source_insets.left(),
            ],
            [
                destination_insets.top().get(),
                destination_insets.right().get(),
                destination_insets.bottom().get(),
                destination_insets.left().get(),
            ],
            descriptor.destination(),
        ),
    };
    ImagePrimitive::__runtime_resolved(
        image.resource_ref().clone(),
        image.intrinsic_size(),
        patches,
    )
    .unwrap_or_else(|| unreachable!("validated image policy resolves within intrinsic bounds"))
}

#[allow(
    clippy::similar_names,
    clippy::suboptimal_flops,
    clippy::too_many_arguments,
    reason = "axis-paired coordinate names express the accepted image mapping contract, and preserving explicit multiply-then-add order avoids changing framework-owned floating-point results"
)]
fn resolve_fit(
    intrinsic_width: u32,
    intrinsic_height: u32,
    crop_x: f32,
    crop_y: f32,
    crop_width: f32,
    crop_height: f32,
    align_x: f32,
    align_y: f32,
    fit: ImageFit,
    destination: LogicalRect,
) -> Vec<([f64; 4], LogicalRect)> {
    if destination.width() == 0.0 || destination.height() == 0.0 {
        return Vec::new();
    }

    let intrinsic_width = f64::from(intrinsic_width);
    let intrinsic_height = f64::from(intrinsic_height);
    let crop_x = f64::from(crop_x);
    let crop_y = f64::from(crop_y);
    let crop_width = f64::from(crop_width);
    let crop_height = f64::from(crop_height);
    let source_x = intrinsic_width * crop_x;
    let source_y = intrinsic_height * crop_y;
    let source_x1 = intrinsic_width * (crop_x + crop_width);
    let source_y1 = intrinsic_height * (crop_y + crop_height);
    let source_width = source_x1 - source_x;
    let source_height = source_y1 - source_y;

    let destination_x = f64::from(destination.x());
    let destination_y = f64::from(destination.y());
    let destination_width = f64::from(destination.width());
    let destination_height = f64::from(destination.height());
    let (rendered_width, rendered_height) = match fit {
        ImageFit::Fill => (destination_width, destination_height),
        ImageFit::Contain => {
            let scale = (destination_width / source_width).min(destination_height / source_height);
            (source_width * scale, source_height * scale)
        }
        ImageFit::Cover => {
            let scale = (destination_width / source_width).max(destination_height / source_height);
            (source_width * scale, source_height * scale)
        }
        ImageFit::None => (source_width, source_height),
        ImageFit::ScaleDown => {
            if source_width <= destination_width && source_height <= destination_height {
                (source_width, source_height)
            } else {
                let scale =
                    (destination_width / source_width).min(destination_height / source_height);
                (source_width * scale, source_height * scale)
            }
        }
    };

    let align_x = f64::from(align_x);
    let align_y = f64::from(align_y);
    let rendered_x = destination_x + (destination_width - rendered_width) * align_x;
    let rendered_y = destination_y + (destination_height - rendered_height) * align_y;
    let visible_x0 = destination_x.max(rendered_x);
    let visible_y0 = destination_y.max(rendered_y);
    let visible_x1 = (destination_x + destination_width).min(rendered_x + rendered_width);
    let visible_y1 = (destination_y + destination_height).min(rendered_y + rendered_height);
    if visible_x1 <= visible_x0 || visible_y1 <= visible_y0 {
        return Vec::new();
    }

    let u0 = ((visible_x0 - rendered_x) / rendered_width).clamp(0.0, 1.0);
    let v0 = ((visible_y0 - rendered_y) / rendered_height).clamp(0.0, 1.0);
    let u1 = ((visible_x1 - rendered_x) / rendered_width).clamp(0.0, 1.0);
    let v1 = ((visible_y1 - rendered_y) / rendered_height).clamp(0.0, 1.0);
    let source_visible_x0 = source_x + source_width * u0;
    let source_visible_y0 = source_y + source_height * v0;
    let source_visible_x1 = source_x + source_width * u1;
    let source_visible_y1 = source_y + source_height * v1;
    let source = [
        source_visible_x0,
        source_visible_y0,
        source_visible_x1 - source_visible_x0,
        source_visible_y1 - source_visible_y0,
    ];
    let Some(visible) = logical_rect_from_edges(visible_x0, visible_y0, visible_x1, visible_y1)
    else {
        return Vec::new();
    };
    if visible.width() == 0.0 || visible.height() == 0.0 {
        return Vec::new();
    }
    vec![(source, visible)]
}

#[allow(
    clippy::similar_names,
    clippy::too_many_arguments,
    reason = "axis-paired source and destination edge names keep the accepted nine-slice mapping explicit"
)]
fn resolve_nine_slice(
    intrinsic_width: u32,
    intrinsic_height: u32,
    crop_x: f32,
    crop_y: f32,
    crop_width: f32,
    crop_height: f32,
    source_insets: [f32; 4],
    destination_insets: [f32; 4],
    destination: LogicalRect,
) -> Vec<([f64; 4], LogicalRect)> {
    if destination.width() == 0.0 || destination.height() == 0.0 {
        return Vec::new();
    }

    let intrinsic_width = f64::from(intrinsic_width);
    let intrinsic_height = f64::from(intrinsic_height);
    let crop_x = f64::from(crop_x);
    let crop_y = f64::from(crop_y);
    let crop_width = f64::from(crop_width);
    let crop_height = f64::from(crop_height);
    let source_x = intrinsic_width * crop_x;
    let source_y = intrinsic_height * crop_y;
    let source_x1 = intrinsic_width * (crop_x + crop_width);
    let source_y1 = intrinsic_height * (crop_y + crop_height);
    let [source_top, source_right, source_bottom, source_left] = source_insets.map(f64::from);
    let [
        destination_top,
        destination_right,
        destination_bottom,
        destination_left,
    ] = destination_insets.map(f64::from);
    let destination_width = f64::from(destination.width());
    let destination_height = f64::from(destination.height());
    let (destination_left, destination_right) =
        normalize_pair(destination_left, destination_right, destination_width);
    let (destination_top, destination_bottom) =
        normalize_pair(destination_top, destination_bottom, destination_height);

    let source_xs = [
        source_x,
        source_x + source_left,
        source_x1 - source_right,
        source_x1,
    ];
    let source_ys = [
        source_y,
        source_y + source_top,
        source_y1 - source_bottom,
        source_y1,
    ];
    let destination_x = f64::from(destination.x());
    let destination_y = f64::from(destination.y());
    let destination_xs = [
        destination_x,
        destination_x + destination_left,
        destination_x + destination_width - destination_right,
        destination_x + destination_width,
    ];
    let destination_ys = [
        destination_y,
        destination_y + destination_top,
        destination_y + destination_height - destination_bottom,
        destination_y + destination_height,
    ];

    let mut patches = Vec::with_capacity(9);
    for row in 0..3 {
        for column in 0..3 {
            let source_patch_width = source_xs[column + 1] - source_xs[column];
            let source_patch_height = source_ys[row + 1] - source_ys[row];
            let destination_patch_width = destination_xs[column + 1] - destination_xs[column];
            let destination_patch_height = destination_ys[row + 1] - destination_ys[row];
            if source_patch_width <= 0.0
                || source_patch_height <= 0.0
                || destination_patch_width <= 0.0
                || destination_patch_height <= 0.0
            {
                continue;
            }
            let Some(destination_patch) = logical_rect_from_edges(
                destination_xs[column],
                destination_ys[row],
                destination_xs[column + 1],
                destination_ys[row + 1],
            ) else {
                continue;
            };
            if destination_patch.width() == 0.0 || destination_patch.height() == 0.0 {
                continue;
            }
            patches.push((
                [
                    source_xs[column],
                    source_ys[row],
                    source_patch_width,
                    source_patch_height,
                ],
                destination_patch,
            ));
        }
    }
    patches
}

fn normalize_pair(first: f64, second: f64, available: f64) -> (f64, f64) {
    let sum = first + second;
    if sum <= available || sum == 0.0 {
        (first, second)
    } else {
        let scale = available / sum;
        (first * scale, second * scale)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the origin and extent were derived from already-validated f32 logical geometry; f64 is used only for overflow-safe intermediate mapping arithmetic"
)]
fn logical_rect_from_edges(x0: f64, y0: f64, x1: f64, y1: f64) -> Option<LogicalRect> {
    let width = x1 - x0;
    let height = y1 - y0;
    if ![x0, y0, width, height].into_iter().all(f64::is_finite) || width < 0.0 || height < 0.0 {
        return None;
    }
    let x = x0 as f32;
    let y = y0 as f32;
    let width = width.min(f64::from(f32::MAX)) as f32;
    let height = height.min(f64::from(f32::MAX)) as f32;
    LogicalRect::try_new(x, y, width, height).ok()
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        ImageAlignment, ImageCrop, ImageDescriptor, ImageDestinationInsets, ImageFit,
        ImageIntrinsicSize, ImageMapping, ImagePaintDescriptor, ImageSourceInsets, LogicalLength,
        LogicalRect, PaintContributionItem, PaintPrimitive, ResourceKind, ResourceRef,
        UnitInterval,
    };

    use super::{logical_rect_from_edges, normalize_pair, publication_primitive, resolve_fit};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
    }

    fn image(mapping: ImageMapping, destination: LogicalRect) -> PaintContributionItem {
        let descriptor = ImageDescriptor::new(
            ResourceRef::new(ResourceKind::Image),
            ImageIntrinsicSize::new(100, 50)
                .unwrap_or_else(|| unreachable!("fixture extent is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("fixture ref is image-kind"));
        PaintContributionItem::image(
            ImagePaintDescriptor::new(descriptor, destination, mapping)
                .unwrap_or_else(|_| unreachable!("fixture mapping is valid")),
        )
    }

    fn resolved(item: &PaintContributionItem) -> runenui_core::ImagePrimitive {
        let PaintPrimitive::Image(image) = publication_primitive(item) else {
            unreachable!("fixture resolves to image")
        };
        image
    }

    fn assert_source_eq(actual: [f64; 4], expected: [f64; 4]) {
        assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
    }

    #[test]
    fn contain_and_cover_resolve_before_publication() {
        let contain = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Contain,
            },
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        assert_eq!(contain.resolved_patch_count(), Some(1));
        let (source, destination) = contain
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("contain has one patch"));
        assert_source_eq(source, [0.0, 0.0, 100.0, 50.0]);
        assert_eq!(destination, rect(0.0, 25.0, 100.0, 50.0));

        let cover = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Cover,
            },
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        let (source, destination) = cover
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("cover has one patch"));
        assert_source_eq(source, [25.0, 0.0, 50.0, 50.0]);
        assert_eq!(destination, rect(0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn none_and_scale_down_use_exact_alignment_and_fit_rules() {
        let left = UnitInterval::ZERO;
        let bottom = UnitInterval::ONE;
        let none = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::new(left, bottom),
                fit: ImageFit::None,
            },
            rect(10.0, 20.0, 60.0, 30.0),
        ));
        let (source, destination) = none
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("none has visible patch"));
        assert_source_eq(source, [0.0, 20.0, 60.0, 30.0]);
        assert_eq!(destination, rect(10.0, 20.0, 60.0, 30.0));

        let scale_down = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::ScaleDown,
            },
            rect(0.0, 0.0, 50.0, 50.0),
        ));
        let (_, destination) = scale_down
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("scale-down has one patch"));
        assert_eq!(destination, rect(0.0, 12.5, 50.0, 25.0));
    }

    #[test]
    fn crop_is_resolved_to_intrinsic_pixel_source_geometry() {
        let half = UnitInterval::HALF;
        let crop = ImageCrop::new(UnitInterval::ZERO, UnitInterval::ZERO, half, half)
            .unwrap_or_else(|_| unreachable!("fixture crop is valid"));
        let image = resolved(&image(
            ImageMapping::Fit {
                crop,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Fill,
            },
            rect(1.0, 2.0, 40.0, 20.0),
        ));
        assert_eq!(
            image.resolved_patch(0),
            Some(([0.0, 0.0, 50.0, 25.0], rect(1.0, 2.0, 40.0, 20.0)))
        );
    }

    #[test]
    fn nine_slice_normalizes_destination_edges_and_keeps_row_major_patches() {
        let source_insets = ImageSourceInsets::new(10.0, 20.0, 10.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture source insets are valid"));
        let length = |value| {
            LogicalLength::new(value)
                .unwrap_or_else(|_| unreachable!("fixture destination inset is valid"))
        };
        let image = resolved(&image(
            ImageMapping::NineSlice {
                source: ImageCrop::FULL,
                source_insets,
                destination_insets: ImageDestinationInsets::new(
                    length(20.0),
                    length(40.0),
                    length(20.0),
                    length(40.0),
                ),
            },
            rect(0.0, 0.0, 40.0, 20.0),
        ));
        assert_eq!(image.resolved_patch_count(), Some(4));
        assert_eq!(
            image.resolved_patch(0),
            Some(([0.0, 0.0, 20.0, 10.0], rect(0.0, 0.0, 20.0, 10.0)))
        );
        assert_eq!(
            image.resolved_patch(1),
            Some(([80.0, 0.0, 20.0, 10.0], rect(20.0, 0.0, 20.0, 10.0)))
        );
        assert_eq!(
            image.resolved_patch(2),
            Some(([0.0, 40.0, 20.0, 10.0], rect(0.0, 10.0, 20.0, 10.0)))
        );
        assert_eq!(
            image.resolved_patch(3),
            Some(([80.0, 40.0, 20.0, 10.0], rect(20.0, 10.0, 20.0, 10.0)))
        );
    }

    #[test]
    fn large_intrinsic_extent_is_not_narrowed_before_publication() {
        let patches = resolve_fit(
            16_777_217,
            1,
            0.0,
            0.0,
            1.0,
            1.0,
            0.5,
            0.5,
            ImageFit::Fill,
            rect(0.0, 0.0, 1.0, 1.0),
        );
        assert_source_eq(patches[0].0, [0.0, 0.0, 16_777_217.0, 1.0]);
    }

    #[test]
    fn destination_inset_normalization_uses_overflow_safe_wider_arithmetic() {
        let huge = f64::from(f32::MAX);
        let (first, second) = normalize_pair(huge, huge, 10.0);
        assert_eq!((first, second), (5.0, 5.0));
    }

    #[test]
    fn destination_reconstruction_keeps_finite_extent_when_far_edge_exceeds_f32() {
        let x = f64::from(f32::MAX) / 2.0;
        let width = f64::from(f32::MAX);
        let resolved = logical_rect_from_edges(x, 0.0, x + width, 1.0).unwrap_or_else(|| {
            unreachable!("finite logical origin and extent remain representable")
        });
        assert!(resolved.x().is_finite());
        assert_eq!(resolved.width().to_bits(), f32::MAX.to_bits());
        assert_eq!(resolved.height().to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    fn zero_destination_publishes_no_image_patches() {
        let image = resolved(&image(ImageMapping::default(), rect(0.0, 0.0, 0.0, 10.0)));
        assert_eq!(image.resolved_patch_count(), Some(0));
    }
}
