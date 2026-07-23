use super::{
    CollectedRoutedOutput, Effect, HostProtocol, MountedEffect, QueueCommitError,
    RegistryInsertError, TraceSequence, TraceWorkIdentity, TraceWorkOwner, WorkFamily, WorkOwner,
    WorkTraceIdentity,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CommitError {
    Queue,
    Registry,
}

impl From<QueueCommitError> for CommitError {
    fn from(_value: QueueCommitError) -> Self {
        Self::Queue
    }
}

impl From<RegistryInsertError> for CommitError {
    fn from(_value: RegistryInsertError) -> Self {
        Self::Registry
    }
}

pub(in crate::runtime) const fn trace_work_family(family: WorkFamily) -> crate::TraceWorkFamily {
    match family {
        WorkFamily::LocalTask => crate::TraceWorkFamily::LocalTask,
        WorkFamily::SendTask => crate::TraceWorkFamily::SendTask,
        WorkFamily::Timer => crate::TraceWorkFamily::Timer,
        WorkFamily::Subscription => crate::TraceWorkFamily::Subscription,
        WorkFamily::HostRequest => crate::TraceWorkFamily::HostRequest,
    }
}

pub(in crate::runtime) fn trace_work_owner(owner: &WorkOwner) -> TraceWorkOwner {
    match owner {
        WorkOwner::Application => TraceWorkOwner::Application,
        WorkOwner::Mounted(owner) => TraceWorkOwner::Mounted(owner.clone()),
    }
}

pub(in crate::runtime) fn public_trace_work_identity(
    identity: WorkTraceIdentity,
) -> TraceWorkIdentity {
    let owner = match identity.owner {
        WorkOwner::Application => TraceWorkOwner::Application,
        WorkOwner::Mounted(owner) => TraceWorkOwner::Mounted(owner),
    };
    TraceWorkIdentity::new(
        owner,
        trace_work_family(identity.family),
        identity.generation.get(),
        identity.key,
    )
}

pub(in crate::runtime) fn mounted_effect_into_effect<Action, Protocol: HostProtocol>(
    effect: MountedEffect<Action>,
) -> Effect<Action, Protocol> {
    match effect {
        MountedEffect::Action(action) => Effect::Action(action),
        MountedEffect::LocalTask(task) => Effect::LocalTask(task),
        MountedEffect::SendTask(task) => Effect::SendTask(task),
        MountedEffect::Timer(timer) => Effect::Timer(timer),
        MountedEffect::Cancel { family, key } => Effect::Cancel { family, key },
    }
}

pub(in crate::runtime) fn with_routed_parent<Action>(
    output: CollectedRoutedOutput<Action>,
    causal_parent: Option<TraceSequence>,
) -> CollectedRoutedOutput<Action> {
    match output {
        CollectedRoutedOutput::Action {
            action,
            current_target,
            ..
        } => CollectedRoutedOutput::Action {
            action,
            causal_parent,
            current_target,
        },
        CollectedRoutedOutput::Command {
            target,
            command,
            origin,
            ..
        } => CollectedRoutedOutput::Command {
            target,
            command,
            origin,
            causal_parent,
        },
    }
}
