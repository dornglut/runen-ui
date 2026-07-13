//! Runtime trace records.

use runenui_core::ElementId;

use crate::MountedNodeId;

/// Trace target for runtime events caused by a specific element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceTarget {
    mounted_node_id: MountedNodeId,
    authored_id: Option<ElementId>,
}

impl TraceTarget {
    /// Creates a trace target from generated runtime identity and optional authored identity.
    #[must_use]
    pub(crate) const fn new(
        mounted_node_id: MountedNodeId,
        authored_id: Option<ElementId>,
    ) -> Self {
        Self {
            mounted_node_id,
            authored_id,
        }
    }

    /// Returns the generated runtime node ID for this target.
    #[must_use]
    pub const fn mounted_node_id(&self) -> &MountedNodeId {
        &self.mounted_node_id
    }

    /// Returns the optional authored element ID for this target.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }
}

/// Coarse runtime trace events emitted by the headless loop.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeEvent {
    /// The runtime was mounted with initial state and root UI.
    Mounted,
    /// A typed action was accepted for dispatch.
    ActionDispatched,
    /// The application update function returned.
    StateUpdated,
    TreeReconciled,
    FocusRetained,
    FocusCleared,
    RuntimeShutdown,
}

/// One runtime trace record, with optional element target details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRecord {
    event: RuntimeEvent,
    target: Option<TraceTarget>,
}

impl TraceRecord {
    const fn new(event: RuntimeEvent, target: Option<TraceTarget>) -> Self {
        Self { event, target }
    }

    /// Returns the coarse runtime event.
    #[must_use]
    pub const fn event(&self) -> RuntimeEvent {
        self.event
    }

    /// Returns target details for events caused by a specific element.
    #[must_use]
    pub const fn target(&self) -> Option<&TraceTarget> {
        self.target.as_ref()
    }
}

/// Ordered runtime trace log.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    events: Vec<RuntimeEvent>,
    records: Vec<TraceRecord>,
}

impl Trace {
    /// Creates an empty trace log.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            events: Vec::new(),
            records: Vec::new(),
        }
    }

    /// Appends one untargeted runtime event.
    pub(crate) fn record(&mut self, event: RuntimeEvent) {
        self.record_with_target(event, None);
    }

    /// Appends one runtime event with optional target details.
    pub(crate) fn record_with_target(&mut self, event: RuntimeEvent, target: Option<TraceTarget>) {
        self.events.push(event);
        self.records.push(TraceRecord::new(event, target));
    }

    /// Returns the recorded coarse event sequence.
    #[must_use]
    pub const fn events(&self) -> &[RuntimeEvent] {
        self.events.as_slice()
    }

    /// Returns the recorded event sequence with target details.
    #[must_use]
    pub const fn records(&self) -> &[TraceRecord] {
        self.records.as_slice()
    }
}
