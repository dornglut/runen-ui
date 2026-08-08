/// Queue-source classification of one accepted application action.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceActionCategory {
    DirectSubmission,
    RoutedCommand,
    ApplicationEffect,
}

/// Redacted action identity that never retains or formats the action payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceActionIdentity {
    type_name: &'static str,
    category: TraceActionCategory,
}

impl TraceActionIdentity {
    pub(crate) fn of<Action>(category: TraceActionCategory) -> Self {
        Self {
            type_name: core::any::type_name::<Action>(),
            category,
        }
    }

    /// Returns the Rust action type name without retaining a payload.
    #[must_use]
    pub const fn type_name(self) -> &'static str {
        self.type_name
    }

    /// Returns how the action entered the canonical queue.
    #[must_use]
    pub const fn category(self) -> TraceActionCategory {
        self.category
    }
}
