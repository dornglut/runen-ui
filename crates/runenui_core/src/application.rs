//! Host-neutral application and host-request protocol.

use crate::{IntoEffects, IntoUpdateOutput, SubscriptionSet, View, WorkFamily};

/// Upper bounds a producer promises for one app-owned turn.
///
/// The values describe callback output, reconciliation, declaration, and
/// scheduler work. Runtime capacity remains owned and enforced by
/// `runenui_runtime`; this is only the app's read-only producer contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkStartBudget {
    local_tasks: usize,
    send_tasks: usize,
    timers: usize,
    host_requests: usize,
    framework_services: usize,
}

impl WorkStartBudget {
    #[must_use]
    pub const fn new(
        local_tasks: usize,
        send_tasks: usize,
        timers: usize,
        host_requests: usize,
        framework_services: usize,
    ) -> Self {
        Self {
            local_tasks,
            send_tasks,
            timers,
            host_requests,
            framework_services,
        }
    }

    #[must_use]
    pub const fn for_family(self, family: WorkFamily) -> usize {
        match family {
            WorkFamily::LocalTask => self.local_tasks,
            WorkFamily::SendTask => self.send_tasks,
            WorkFamily::Timer => self.timers,
            WorkFamily::HostRequest => self.host_requests,
            WorkFamily::FrameworkService => self.framework_services,
        }
    }

    #[must_use]
    pub const fn total(self) -> Option<usize> {
        let Some(total) = self.local_tasks.checked_add(self.send_tasks) else {
            return None;
        };
        let Some(total) = total.checked_add(self.timers) else {
            return None;
        };
        let Some(total) = total.checked_add(self.host_requests) else {
            return None;
        };
        total.checked_add(self.framework_services)
    }
}

/// Maximum simultaneously live work-registry records of one causal producer root.
///
/// This includes descendants and records awaiting start or completion, until
/// logical invalidation. Remaining cleanup of invalidated generations is funded
/// separately by the root's canonical FIFO credits and work leases; invalidation
/// does not release the root's residency claim while that root can still produce.
/// It differs from `WorkStartBudget`, which bounds starts in one mutable turn.
/// Subscription records have their own capacity; their source implementation
/// does not consume a local-task or send-task registry slot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkResidencyBudget {
    families: [usize; 5],
    subscriptions: usize,
}

impl WorkResidencyBudget {
    #[must_use]
    pub const fn new(
        local_tasks: usize,
        send_tasks: usize,
        timers: usize,
        subscriptions: usize,
        host_requests: usize,
        framework_services: usize,
    ) -> Self {
        Self {
            families: [
                local_tasks,
                send_tasks,
                timers,
                host_requests,
                framework_services,
            ],
            subscriptions,
        }
    }

    #[must_use]
    pub const fn for_family(self, family: WorkFamily) -> usize {
        self.families[match family {
            WorkFamily::LocalTask => 0,
            WorkFamily::SendTask => 1,
            WorkFamily::Timer => 2,
            WorkFamily::HostRequest => 3,
            WorkFamily::FrameworkService => 4,
        }]
    }

    #[must_use]
    pub const fn subscriptions(self) -> usize {
        self.subscriptions
    }

    const fn covers(self, other: Self) -> bool {
        self.families[0] >= other.families[0]
            && self.families[1] >= other.families[1]
            && self.families[2] >= other.families[2]
            && self.families[3] >= other.families[3]
            && self.families[4] >= other.families[4]
            && self.subscriptions >= other.subscriptions
    }
}

/// Aggregate per-step budget for an application producer.
///
/// `queue_credits` is the finite live-frontier quota for this causal root. It
/// may be transferred to children as work progresses; it is not a bound on
/// total history, so a unary producer can renew itself indefinitely. The
/// queued root carries this bound across intervening FIFO state changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProducerBudget {
    queue_credits: usize,
    outputs: usize,
    application_outputs: usize,
    mounted_nodes: usize,
    retained_updates: usize,
    dirty_subscription_owners: usize,
    application_subscriptions: usize,
    mounted_subscriptions_per_owner: usize,
    work_starts: WorkStartBudget,
    work_residency: WorkResidencyBudget,
}

impl ProducerBudget {
    #[must_use]
    pub const fn new(queue_credits: usize, outputs: usize) -> Self {
        Self {
            queue_credits,
            outputs,
            application_outputs: outputs,
            mounted_nodes: 0,
            retained_updates: 0,
            dirty_subscription_owners: 0,
            application_subscriptions: 0,
            mounted_subscriptions_per_owner: 0,
            work_starts: WorkStartBudget::new(0, 0, 0, 0, 0),
            work_residency: WorkResidencyBudget::new(0, 0, 0, 0, 0, 0),
        }
    }

    /// Bounds new mounts, retained update callbacks, and dirty declaration owners.
    /// New mounts plus retained updates are the conservative live-identity peak
    /// used for admission; opaque callbacks do not certify slot retirement.
    #[must_use]
    pub const fn with_reconciliation(
        mut self,
        mounted_nodes: usize,
        retained_updates: usize,
        dirty_subscription_owners: usize,
    ) -> Self {
        self.mounted_nodes = mounted_nodes;
        self.retained_updates = retained_updates;
        self.dirty_subscription_owners = dirty_subscription_owners;
        self
    }

    #[must_use]
    pub const fn with_subscriptions(
        mut self,
        application_subscriptions: usize,
        mounted_subscriptions_per_owner: usize,
    ) -> Self {
        self.application_subscriptions = application_subscriptions;
        self.mounted_subscriptions_per_owner = mounted_subscriptions_per_owner;
        self
    }

    #[must_use]
    pub const fn with_work_starts(mut self, work_starts: WorkStartBudget) -> Self {
        self.work_starts = work_starts;
        self
    }

    /// Declares a closed bound for all resident descendant work of this root.
    /// The default is no resident work, independently of per-turn start limits.
    #[must_use]
    pub const fn with_work_residency(mut self, residency: WorkResidencyBudget) -> Self {
        self.work_residency = residency;
        self
    }

    #[must_use]
    pub const fn work_residency(self) -> WorkResidencyBudget {
        self.work_residency
    }

    #[must_use]
    pub const fn queue_credits(self) -> usize {
        self.queue_credits
    }

    #[must_use]
    pub const fn outputs(self) -> usize {
        self.outputs
    }

    /// Bounds outputs of one application/reconciliation transaction, including
    /// its mounted lifecycle outputs. Separately routed cleanup passes keep
    /// their own allowances within the aggregate `outputs` bound.
    /// This maximum also covers every descendant application step sharing
    /// the accepted causal root.
    ///
    /// The constructor conservatively uses `outputs` for this component too.
    #[must_use]
    pub const fn with_application_outputs(mut self, outputs: usize) -> Self {
        self.application_outputs = outputs;
        self
    }

    #[must_use]
    pub const fn application_outputs(self) -> usize {
        self.application_outputs
    }

    #[must_use]
    pub const fn mounted_nodes(self) -> usize {
        self.mounted_nodes
    }

    #[must_use]
    pub const fn retained_updates(self) -> usize {
        self.retained_updates
    }

    #[must_use]
    pub const fn dirty_subscription_owners(self) -> usize {
        self.dirty_subscription_owners
    }

    #[must_use]
    pub const fn application_subscriptions(self) -> usize {
        self.application_subscriptions
    }

    #[must_use]
    pub const fn mounted_subscriptions_per_owner(self) -> usize {
        self.mounted_subscriptions_per_owner
    }

    #[must_use]
    pub const fn work_starts(self) -> WorkStartBudget {
        self.work_starts
    }

    /// Returns whether this profile covers the limits declared by `other`.
    #[must_use]
    pub const fn covers(self, other: Self) -> bool {
        self.queue_credits >= other.queue_credits
            && self.outputs >= other.outputs
            && self.application_outputs >= other.application_outputs
            && self.mounted_nodes >= other.mounted_nodes
            && self.retained_updates >= other.retained_updates
            && self.dirty_subscription_owners >= other.dirty_subscription_owners
            && self.application_subscriptions >= other.application_subscriptions
            && self.mounted_subscriptions_per_owner >= other.mounted_subscriptions_per_owner
            && self.work_starts.local_tasks >= other.work_starts.local_tasks
            && self.work_starts.send_tasks >= other.work_starts.send_tasks
            && self.work_starts.timers >= other.work_starts.timers
            && self.work_starts.host_requests >= other.work_starts.host_requests
            && self.work_starts.framework_services >= other.work_starts.framework_services
            && self.work_residency.covers(other.work_residency)
    }
}

/// Widget producer phase whose output profile is queried before entry.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WidgetProducerPhase {
    Publication,
    Event,
    Mount,
    Update,
    Subscriptions,
    Activation,
}

/// One widget's bounded output for one callback phase.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WidgetProducerBudget {
    outputs: usize,
    subscription_declarations: usize,
    work_starts: WorkStartBudget,
    publication_reveals: usize,
}

impl WidgetProducerBudget {
    #[must_use]
    pub const fn new(outputs: usize) -> Self {
        Self {
            outputs,
            subscription_declarations: 0,
            work_starts: WorkStartBudget::new(0, 0, 0, 0, 0),
            publication_reveals: 0,
        }
    }

    #[must_use]
    pub const fn with_subscriptions(mut self, declarations: usize) -> Self {
        self.subscription_declarations = declarations;
        self
    }

    #[must_use]
    pub const fn with_work_starts(mut self, work_starts: WorkStartBudget) -> Self {
        self.work_starts = work_starts;
        self
    }

    #[must_use]
    pub const fn outputs(self) -> usize {
        self.outputs
    }

    #[must_use]
    pub const fn subscription_declarations(self) -> usize {
        self.subscription_declarations
    }

    /// Bounds publication candidates for the runtime's active-descendant reveal
    /// command. Declared in the Publication phase before capability callbacks;
    /// ordinary widgets that cannot produce such candidates keep zero.
    #[must_use]
    pub const fn with_publication_reveals(mut self, maximum: usize) -> Self {
        self.publication_reveals = maximum;
        self
    }

    #[must_use]
    pub const fn publication_reveals(self) -> usize {
        self.publication_reveals
    }

    #[must_use]
    pub const fn work_starts(self) -> WorkStartBudget {
        self.work_starts
    }
}

/// The sole application contract consumed by the `RunenUI` runtime.
pub trait UiApp {
    type State;
    type Action;
    type HostProtocol: HostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action>;

    fn initial_effects(_state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        {}
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol>;

    fn subscriptions(_state: &Self::State, _subscriptions: &mut SubscriptionSet<Self::Action>) {}

    /// Maximum cost of constructing and committing initial effects/subscriptions.
    fn initial_producer_budget(state: &Self::State) -> ProducerBudget;

    /// Maximum cost of one exact application action and its resulting
    /// reconciliation, including any still-live old-focus notification route.
    /// The returned promise must remain valid for every application state in
    /// which this action can reach the FIFO head. Earlier queued actions may
    /// change state after this profile is captured.
    fn action_producer_budget(state: &Self::State, action: &Self::Action) -> ProducerBudget;

    /// Maximum cost of a routed external interaction before any callback runs.
    /// The promise must remain valid for every application state and mounted
    /// topology in which the accepted interaction can reach the FIFO head;
    /// earlier accepted actions may change state after this profile is captured.
    fn interaction_producer_budget(state: &Self::State) -> ProducerBudget;

    /// Returns an optional low-cardinality diagnostic label for one action.
    ///
    /// Labels are static by design: applications may provide stable semantic
    /// names such as `"increment"` or `"loaded"`, but this hook is not a
    /// generic payload serializer and never requires `Action: Debug`.
    #[must_use]
    fn trace_action_label(_action: &Self::Action) -> Option<&'static str> {
        None
    }
}

/// Closed application-defined command and response protocol used by host effects.
pub trait HostProtocol {
    type Command;
    type Response: 'static;
    type ResponseKind: Copy + Eq + 'static;

    fn expected_response(command: &Self::Command) -> Self::ResponseKind;
    fn response_kind(response: &Self::Response) -> Self::ResponseKind;
}

/// Explicit protocol for applications which issue no host requests.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct NoHostProtocol;

/// Uninhabited command type for [`NoHostProtocol`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NoHostCommand {}

/// Uninhabited response type for [`NoHostProtocol`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NoHostResponse {}

/// Uninhabited response-kind type for [`NoHostProtocol`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NoHostResponseKind {}

impl HostProtocol for NoHostProtocol {
    type Command = NoHostCommand;
    type Response = NoHostResponse;
    type ResponseKind = NoHostResponseKind;

    fn expected_response(command: &Self::Command) -> Self::ResponseKind {
        let _ = command;
        unreachable!("NoHostCommand is uninhabited")
    }

    fn response_kind(response: &Self::Response) -> Self::ResponseKind {
        let _ = response;
        unreachable!("NoHostResponse is uninhabited")
    }
}
