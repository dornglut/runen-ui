use crate::{CompositionDiagnosticRecord, CompositionSnapshot, CompositionTransaction};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionPolicyDecision {
    Accepted,
    Rejected(Vec<CompositionDiagnosticRecord>),
}

pub trait CompositionLifecyclePolicy {
    fn evaluate(
        &self,
        snapshot: CompositionSnapshot<'_>,
        transaction: &CompositionTransaction,
    ) -> CompositionPolicyDecision;
}

pub trait CompositionCapabilityPolicy {
    fn evaluate(
        &self,
        snapshot: CompositionSnapshot<'_>,
        transaction: &CompositionTransaction,
    ) -> CompositionPolicyDecision;
}

pub trait CompositionTargetPolicy {
    fn evaluate(
        &self,
        snapshot: CompositionSnapshot<'_>,
        transaction: &CompositionTransaction,
    ) -> CompositionPolicyDecision;
}

pub struct CompositionPolicies<'a> {
    pub lifecycle: &'a dyn CompositionLifecyclePolicy,
    pub capability: &'a dyn CompositionCapabilityPolicy,
    pub target: &'a dyn CompositionTargetPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizedTransaction {
    pub(crate) transaction: CompositionTransaction,
    pub(crate) authorized_revision: crate::StateRevision,
    pub(crate) allows_history_restore: bool,
}
