//! File: domain/ui/ui_layout/src/stack.rs
//! Purpose: Basic vertical/horizontal stack layout.

use ui_math::{Axis, UiRect, UiSize};

use crate::{CrossAxisAlignment, LayoutConstraints, MainAxisAlignment, SizePolicy};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackItem {
    pub size: UiSize,
    pub main_policy: SizePolicy,
}

impl StackItem {
    pub fn auto(size: UiSize) -> Self {
        Self {
            size,
            main_policy: SizePolicy::Auto,
        }
    }

    pub fn fixed(size: UiSize, value: f32) -> Self {
        Self {
            size,
            main_policy: SizePolicy::Fixed(value),
        }
    }

    pub fn flex(size: UiSize, weight: f32) -> Self {
        Self {
            size,
            main_policy: SizePolicy::flex(weight),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackLayout {
    pub axis: Axis,
    pub gap: f32,
    pub main_align: MainAxisAlignment,
    pub cross_align: CrossAxisAlignment,
}

impl StackLayout {
    pub fn vertical(gap: f32) -> Self {
        Self {
            axis: Axis::Vertical,
            gap,
            main_align: MainAxisAlignment::Start,
            cross_align: CrossAxisAlignment::Stretch,
        }
    }

    pub fn horizontal(gap: f32) -> Self {
        Self {
            axis: Axis::Horizontal,
            gap,
            main_align: MainAxisAlignment::Start,
            cross_align: CrossAxisAlignment::Stretch,
        }
    }

    pub fn with_main_align(mut self, value: MainAxisAlignment) -> Self {
        self.main_align = value;
        self
    }

    pub fn with_cross_align(mut self, value: CrossAxisAlignment) -> Self {
        self.cross_align = value;
        self
    }

    pub fn measure(&self, children: &[StackItem], constraints: LayoutConstraints) -> UiSize {
        let mut major = 0.0;
        let mut minor: f32 = 0.0;

        for (index, child) in children.iter().enumerate() {
            if index > 0 {
                major += self.gap.max(0.0);
            }

            let child_major = match self.axis {
                Axis::Vertical => match child.main_policy {
                    SizePolicy::Auto => child.size.height,
                    SizePolicy::Fixed(v) => v.max(0.0),
                    SizePolicy::Flex(_) => child.size.height,
                },
                Axis::Horizontal => match child.main_policy {
                    SizePolicy::Auto => child.size.width,
                    SizePolicy::Fixed(v) => v.max(0.0),
                    SizePolicy::Flex(_) => child.size.width,
                },
            };

            match self.axis {
                Axis::Vertical => {
                    major += child_major;
                    minor = minor.max(child.size.width);
                }
                Axis::Horizontal => {
                    major += child_major;
                    minor = minor.max(child.size.height);
                }
            }
        }

        let size = match self.axis {
            Axis::Vertical => UiSize::new(minor, major),
            Axis::Horizontal => UiSize::new(major, minor),
        };

        constraints.constrain(size)
    }

    pub fn arrange(&self, bounds: UiRect, children: &[StackItem]) -> Vec<UiRect> {
        let gap = self.gap.max(0.0);
        let child_count = children.len();
        if child_count == 0 {
            return Vec::new();
        }

        let available_major = match self.axis {
            Axis::Vertical => bounds.height,
            Axis::Horizontal => bounds.width,
        };

        let total_gap = gap * (child_count.saturating_sub(1) as f32);

        let mut resolved_major = vec![0.0; child_count];
        let mut fixed_major_sum = 0.0;
        let mut total_flex_weight = 0.0;

        for (index, child) in children.iter().enumerate() {
            let major = match self.axis {
                Axis::Vertical => match child.main_policy {
                    SizePolicy::Auto => child.size.height,
                    SizePolicy::Fixed(v) => v.max(0.0),
                    SizePolicy::Flex(weight) => {
                        total_flex_weight += weight.max(0.0);
                        0.0
                    }
                },
                Axis::Horizontal => match child.main_policy {
                    SizePolicy::Auto => child.size.width,
                    SizePolicy::Fixed(v) => v.max(0.0),
                    SizePolicy::Flex(weight) => {
                        total_flex_weight += weight.max(0.0);
                        0.0
                    }
                },
            };

            resolved_major[index] = major;
            fixed_major_sum += major;
        }

        let remaining_major = (available_major - total_gap - fixed_major_sum).max(0.0);

        if total_flex_weight > 0.0 {
            for (index, child) in children.iter().enumerate() {
                if let SizePolicy::Flex(weight) = child.main_policy {
                    resolved_major[index] = remaining_major * (weight.max(0.0) / total_flex_weight);
                }
            }
        }

        let used_major = resolved_major.iter().sum::<f32>() + total_gap;
        let extra_major = (available_major - used_major).max(0.0);

        let (leading_offset, distributed_gap) = match self.main_align {
            MainAxisAlignment::Start => (0.0, gap),
            MainAxisAlignment::Center => (extra_major * 0.5, gap),
            MainAxisAlignment::End => (extra_major, gap),
            MainAxisAlignment::SpaceBetween => {
                if child_count > 1 {
                    (0.0, gap + extra_major / ((child_count - 1) as f32))
                } else {
                    (0.0, gap)
                }
            }
            MainAxisAlignment::SpaceAround => {
                let slots = child_count as f32;
                if slots > 0.0 {
                    let extra = extra_major / slots;
                    (extra * 0.5, gap + extra)
                } else {
                    (0.0, gap)
                }
            }
            MainAxisAlignment::SpaceEvenly => {
                let slots = (child_count + 1) as f32;
                let extra = if slots > 0.0 {
                    extra_major / slots
                } else {
                    0.0
                };
                (extra, gap + extra)
            }
        };

        let mut out = Vec::with_capacity(child_count);
        let mut cursor_major = leading_offset;

        for (index, child) in children.iter().enumerate() {
            let major = resolved_major[index];

            let (cross_pos, cross_size) = match self.axis {
                Axis::Vertical => {
                    let size = match self.cross_align {
                        CrossAxisAlignment::Stretch => bounds.width,
                        _ => child.size.width.min(bounds.width),
                    };

                    let pos = match self.cross_align {
                        CrossAxisAlignment::Start | CrossAxisAlignment::Stretch => bounds.x,
                        CrossAxisAlignment::Center => bounds.x + (bounds.width - size) * 0.5,
                        CrossAxisAlignment::End => bounds.x + (bounds.width - size),
                    };

                    (pos, size)
                }
                Axis::Horizontal => {
                    let size = match self.cross_align {
                        CrossAxisAlignment::Stretch => bounds.height,
                        _ => child.size.height.min(bounds.height),
                    };

                    let pos = match self.cross_align {
                        CrossAxisAlignment::Start | CrossAxisAlignment::Stretch => bounds.y,
                        CrossAxisAlignment::Center => bounds.y + (bounds.height - size) * 0.5,
                        CrossAxisAlignment::End => bounds.y + (bounds.height - size),
                    };

                    (pos, size)
                }
            };

            let rect = match self.axis {
                Axis::Vertical => {
                    UiRect::new(cross_pos, bounds.y + cursor_major, cross_size, major)
                }
                Axis::Horizontal => {
                    UiRect::new(bounds.x + cursor_major, cross_pos, major, cross_size)
                }
            };

            out.push(rect);
            cursor_major += major + distributed_gap;
        }

        out
    }
}
