//! File: domain/ui/ui_render_data/src/primitives/viewport_surface_embed.rs
//! Purpose: Viewport panel surface embed primitive.

use std::collections::BTreeMap;

use ui_math::UiRect;

use crate::{UiPaint, UiSortKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportSurfaceEmbedSlotId(pub u16);

impl ViewportSurfaceEmbedSlotId {
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSurfaceEmbedPrimitive {
    pub viewport_id: u64,
    pub slot: ViewportSurfaceEmbedSlotId,
    pub rect: UiRect,
    pub uv_rect: UiRect,
    pub tint: UiPaint,
    pub sort_key: UiSortKey,
}

impl ViewportSurfaceEmbedPrimitive {
    pub fn new(
        viewport_id: u64,
        slot: ViewportSurfaceEmbedSlotId,
        rect: UiRect,
        uv_rect: UiRect,
        tint: UiPaint,
        sort_key: UiSortKey,
    ) -> Self {
        Self {
            viewport_id,
            slot,
            rect,
            uv_rect,
            tint,
            sort_key,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewportSurfaceBindingSource {
    DynamicTexture {
        namespace: String,
        target_id: String,
    },
}

impl ViewportSurfaceBindingSource {
    pub fn dynamic_texture(namespace: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self::DynamicTexture {
            namespace: namespace.into(),
            target_id: target_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportSurfaceBinding {
    pub source: ViewportSurfaceBindingSource,
}

impl ViewportSurfaceBinding {
    pub fn dynamic_texture(namespace: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            source: ViewportSurfaceBindingSource::dynamic_texture(namespace, target_id),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ViewportSurfaceBindingRegistry {
    bindings: BTreeMap<(u64, ViewportSurfaceEmbedSlotId), ViewportSurfaceBinding>,
}

impl ViewportSurfaceBindingRegistry {
    pub fn clear(&mut self) {
        self.bindings.clear();
    }

    pub fn bind(
        &mut self,
        viewport_id: u64,
        slot: ViewportSurfaceEmbedSlotId,
        binding: ViewportSurfaceBinding,
    ) {
        self.bindings.insert((viewport_id, slot), binding);
    }

    pub fn get(
        &self,
        viewport_id: u64,
        slot: ViewportSurfaceEmbedSlotId,
    ) -> Option<&ViewportSurfaceBinding> {
        self.bindings.get(&(viewport_id, slot))
    }

    pub fn iter(
        &self,
    ) -> impl Iterator<Item = (&(u64, ViewportSurfaceEmbedSlotId), &ViewportSurfaceBinding)> {
        self.bindings.iter()
    }
}
