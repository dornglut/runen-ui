//! Product surface widget constructor.

use crate::ProductSurfaceNode;
use crate::{UiNode, UiNodeKind, WidgetId};
use ui_math::{UiRect, UiSize};
use ui_render_data::{ProductSurfaceAlphaMode, ProductSurfaceTextureBindingSource, UiPaint};

pub fn product_surface(
    id: WidgetId,
    source: ProductSurfaceTextureBindingSource,
    min_size: UiSize,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::ProductSurface(ProductSurfaceNode::new(
            source,
            UiRect::new(0.0, 0.0, 1.0, 1.0),
            UiPaint::WHITE,
            ProductSurfaceAlphaMode::Straight,
            min_size,
        )),
    )
}
