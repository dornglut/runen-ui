#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct InteractionState {
    pub(crate) hovered: bool,
    pub(crate) pressed: bool,
    pub(crate) capture_placeholder: bool,
    pub(crate) scroll_offset: (f32, f32),
}

#[derive(Clone, Copy, Debug)]
pub struct InteractionStateRef<'a>(pub(crate) &'a InteractionState);

impl InteractionStateRef<'_> {
    #[must_use]
    pub const fn hovered(self) -> bool {
        self.0.hovered
    }
    #[must_use]
    pub const fn pressed(self) -> bool {
        self.0.pressed
    }
    #[must_use]
    pub const fn capture_placeholder(self) -> bool {
        self.0.capture_placeholder
    }
    #[must_use]
    pub const fn scroll_offset(self) -> (f32, f32) {
        self.0.scroll_offset
    }
}
