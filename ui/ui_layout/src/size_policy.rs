//! File: domain/ui/ui_layout/src/size_policy.rs
//! Purpose: Child sizing policy contracts for layout.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizePolicy {
    /// Use the child's measured size.
    Auto,

    /// Force a fixed size on the relevant axis.
    Fixed(f32),

    /// Fill remaining available space proportionally.
    Flex(f32),
}

impl SizePolicy {
    pub fn flex(weight: f32) -> Self {
        Self::Flex(weight.max(0.0))
    }
}
