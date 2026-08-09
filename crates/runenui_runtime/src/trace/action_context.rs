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
    label: Option<&'static str>,
}

impl TraceActionIdentity {
    pub(crate) fn of_labeled<Action>(
        category: TraceActionCategory,
        label: Option<&'static str>,
    ) -> Self {
        Self {
            type_name: core::any::type_name::<Action>(),
            category,
            label,
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

    /// Returns the optional application-supplied static diagnostic label.
    #[must_use]
    pub const fn label(self) -> Option<&'static str> {
        self.label
    }
}
