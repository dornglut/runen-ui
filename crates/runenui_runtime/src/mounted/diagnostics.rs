use core::fmt;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DuplicateIdentityKind {
    InvalidElementId,
    InvalidElementKey,
    ElementId,
    SiblingKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityDiagnostic {
    pub(crate) kind: DuplicateIdentityKind,
    pub(crate) value: String,
    pub(crate) first_path: String,
    pub(crate) duplicate_path: String,
    pub(crate) preorder_index: usize,
}

impl IdentityDiagnostic {
    #[must_use]
    pub const fn kind(&self) -> DuplicateIdentityKind {
        self.kind
    }
    #[must_use]
    pub const fn value(&self) -> &str {
        self.value.as_str()
    }
    #[must_use]
    pub const fn first_path(&self) -> &str {
        self.first_path.as_str()
    }
    #[must_use]
    pub const fn duplicate_path(&self) -> &str {
        self.duplicate_path.as_str()
    }
    #[must_use]
    pub const fn preorder_index(&self) -> usize {
        self.preorder_index
    }
}

impl fmt::Display for IdentityDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?}: {} -> {}",
            self.kind, self.value, self.first_path, self.duplicate_path
        )
    }
}
