use ui_math::UiPoint;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface2DTransform {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl Surface2DTransform {
    pub const fn new(pan_x: f32, pan_y: f32, zoom: f32) -> Self {
        Self { pan_x, pan_y, zoom }
    }

    pub fn is_valid(self) -> bool {
        self.zoom.is_finite() && self.zoom > 0.0 && self.pan_x.is_finite() && self.pan_y.is_finite()
    }

    pub fn world_to_screen(self, point: UiPoint) -> Option<UiPoint> {
        self.is_valid().then(|| {
            UiPoint::new(
                point.x * self.zoom + self.pan_x,
                point.y * self.zoom + self.pan_y,
            )
        })
    }

    pub fn screen_to_world(self, point: UiPoint) -> Option<UiPoint> {
        self.is_valid().then(|| {
            UiPoint::new(
                (point.x - self.pan_x) / self.zoom,
                (point.y - self.pan_y) / self.zoom,
            )
        })
    }
}
