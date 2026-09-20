/// Queue-source classification of one accepted application action.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceActionCategory {
    DirectSubmission,
    RoutedCommand,
    ApplicationEffect,
    TransactionalEdit,
}

/// Redacted causal identity for one transactional edit-origin action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceEditIdentity {
    request: u64,
    session: u64,
    predecessor: Option<u64>,
}

impl TraceEditIdentity {
    pub(crate) const fn new(request: u64, session: u64, predecessor: Option<u64>) -> Self {
        Self {
            request,
            session,
            predecessor,
        }
    }

    #[must_use]
    pub const fn request(self) -> u64 {
        self.request
    }

    #[must_use]
    pub const fn session(self) -> u64 {
        self.session
    }

    #[must_use]
    pub const fn predecessor(self) -> Option<u64> {
        self.predecessor
    }
}

/// Redacted action identity that never retains or formats the action payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceActionIdentity {
    type_name: &'static str,
    category: TraceActionCategory,
    label: Option<&'static str>,
    edit: Option<TraceEditIdentity>,
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
            edit: None,
        }
    }

    pub(crate) fn editing<Action>(
        label: Option<&'static str>,
        origin: &crate::editing::EditActionOrigin,
    ) -> Self {
        Self {
            type_name: core::any::type_name::<Action>(),
            category: TraceActionCategory::TransactionalEdit,
            label,
            edit: Some(TraceEditIdentity::new(
                origin.request.get(),
                origin.session.get(),
                origin
                    .predecessor
                    .as_ref()
                    .map(runenui_core::EditRequestId::get),
            )),
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

    #[must_use]
    pub const fn edit(self) -> Option<TraceEditIdentity> {
        self.edit
    }
}
