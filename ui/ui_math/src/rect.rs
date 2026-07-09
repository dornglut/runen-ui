//! File: domain/ui/ui_math/src/rect.rs
//! Purpose: Shared rectangle type and helpers.

use crate::{UiInsets, UiPoint, UiSize};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UiRect {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn from_point_size(origin: UiPoint, size: UiSize) -> Self {
        Self {
            x: origin.x,
            y: origin.y,
            width: size.width,
            height: size.height,
        }
    }

    pub fn origin(self) -> UiPoint {
        UiPoint::new(self.x, self.y)
    }

    pub fn size(self) -> UiSize {
        UiSize::new(self.width, self.height)
    }

    pub fn contains(self, point: UiPoint) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x + self.width
            && point.y <= self.y + self.height
    }

    pub fn inset(self, insets: UiInsets) -> Self {
        let width = (self.width - insets.horizontal()).max(0.0);
        let height = (self.height - insets.vertical()).max(0.0);

        Self {
            x: self.x + insets.left,
            y: self.y + insets.top,
            width,
            height,
        }
    }

    pub fn intersect(self, other: Self) -> Option<Self> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        if x2 <= x1 || y2 <= y1 {
            return None;
        }

        Some(Self::new(x1, y1, x2 - x1, y2 - y1))
    }
}
