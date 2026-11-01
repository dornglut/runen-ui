//! Owned producer refusal and trace-independent contract violation vocabulary.
use crate::{WidgetProducerPhase, WorkFamily};
use std::fmt;
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProducerPhase {
    InitialEffects,
    ApplicationUpdate,
    ApplicationRoot,
    ApplicationSubscriptions,
    Widget(WidgetProducerPhase),
}
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BudgetComponent {
    Outputs,
    ApplicationOutputs,
    QueueFrontier,
    MountedNodes,
    RetainedUpdates,
    DirtySubscriptionOwners,
    ApplicationSubscriptions,
    MountedSubscriptionsPerOwner,
    SubscriptionDeclarations,
    PublicationReveals,
    SubscriptionResidency,
    WorkStarts(WorkFamily),
    WorkResidency(WorkFamily),
}
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProducerViolationKind {
    DeclarationExceeded,
    DescendantProfileNotCovered,
}
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProducerContractViolation {
    pub kind: ProducerViolationKind,
    pub phase: ProducerPhase,
    pub component: BudgetComponent,
    pub declared: usize,
    /// None means the next count is not representable; never wrap a diagnostic.
    pub attempted: Option<usize>,
}
#[must_use]
pub struct EmitError<T> {
    value: T,
    violation: ProducerContractViolation,
}
impl<T> EmitError<T> {
    pub fn into_parts(self) -> (T, ProducerContractViolation) {
        (self.value, self.violation)
    }
    pub const fn violation(&self) -> ProducerContractViolation {
        self.violation
    }
}
impl<T> fmt::Debug for EmitError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmitError")
            .field("violation", &self.violation)
            .finish_non_exhaustive()
    }
}

impl ProducerContractViolation {
    pub(crate) const fn output(phase: ProducerPhase, declared: usize) -> Self {
        Self {
            kind: ProducerViolationKind::DeclarationExceeded,
            phase,
            component: BudgetComponent::Outputs,
            declared,
            attempted: declared.checked_add(1),
        }
    }

    pub(crate) const fn work_start(
        phase: ProducerPhase,
        family: WorkFamily,
        declared: usize,
        used: usize,
    ) -> Self {
        Self {
            kind: ProducerViolationKind::DeclarationExceeded,
            phase,
            component: BudgetComponent::WorkStarts(family),
            declared,
            attempted: used.checked_add(1),
        }
    }
}
impl<T> EmitError<T> {
    pub(crate) const fn new(value: T, violation: ProducerContractViolation) -> Self {
        Self { value, violation }
    }
}

pub const fn family_index(family: WorkFamily) -> usize {
    match family {
        WorkFamily::LocalTask => 0,
        WorkFamily::SendTask => 1,
        WorkFamily::Timer => 2,
        WorkFamily::HostRequest => 3,
        WorkFamily::FrameworkService => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::{BudgetComponent, ProducerContractViolation, ProducerPhase, ProducerViolationKind, family_index};
    use crate::WorkFamily;

    #[test]
    fn every_work_family_has_one_stable_credit_index_and_typed_overrun() {
        for (index, family) in [
            WorkFamily::LocalTask,
            WorkFamily::SendTask,
            WorkFamily::Timer,
            WorkFamily::HostRequest,
            WorkFamily::FrameworkService,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(family_index(family), index);
            assert_eq!(
                ProducerContractViolation::work_start(
                    ProducerPhase::ApplicationUpdate,
                    family,
                    index,
                    index,
                ),
                ProducerContractViolation {
                    kind: ProducerViolationKind::DeclarationExceeded,
                    phase: ProducerPhase::ApplicationUpdate,
                    component: BudgetComponent::WorkStarts(family),
                    declared: index,
                    attempted: Some(index + 1),
                },
            );
        }

        assert_eq!(
            ProducerContractViolation::work_start(
                ProducerPhase::ApplicationUpdate,
                WorkFamily::Timer,
                usize::MAX,
                usize::MAX,
            )
            .attempted,
            None,
            "overrun diagnostics must not wrap usize",
        );
    }
}
