//! Host-neutral focus authoring and routed notification protocol.

use crate::MountedNodeId;

/// Retained source modality of the last accepted interaction.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputModality {
    Pointer,
    Keyboard,
    Controller,
    Accessibility,
    Automation,
    Programmatic,
}

/// Reason committed with one focus transition.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FocusReason {
    Pointer,
    LinearNavigation,
    DirectionalNavigation,
    ProgrammaticRequest,
    Removal,
    Disablement,
    RememberedRestoration,
    Shutdown,
}

/// Direction used by directional focus navigation and logical focus scrolling.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FocusDirection {
    Left,
    Right,
    Up,
    Down,
}

/// Behavior at one focus-scope traversal boundary.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FocusBoundaryPolicy {
    Delegate,
    Trap,
    Stop,
    Wrap,
    LogicalScroll,
}

/// Separate linear and directional boundary policy for one nested focus scope.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FocusScopePolicy {
    linear: FocusBoundaryPolicy,
    directional: FocusBoundaryPolicy,
}

impl FocusScopePolicy {
    /// Creates an explicit scope policy.
    #[must_use]
    pub const fn new(linear: FocusBoundaryPolicy, directional: FocusBoundaryPolicy) -> Self {
        Self {
            linear,
            directional,
        }
    }

    /// Default nested-scope behavior: delegate both boundaries to the parent.
    #[must_use]
    pub const fn nested_default() -> Self {
        Self::new(FocusBoundaryPolicy::Delegate, FocusBoundaryPolicy::Delegate)
    }

    #[must_use]
    pub const fn linear(self) -> FocusBoundaryPolicy {
        self.linear
    }

    #[must_use]
    pub const fn directional(self) -> FocusBoundaryPolicy {
        self.directional
    }

    #[must_use]
    pub const fn with_linear(mut self, policy: FocusBoundaryPolicy) -> Self {
        self.linear = policy;
        self
    }

    #[must_use]
    pub const fn with_directional(mut self, policy: FocusBoundaryPolicy) -> Self {
        self.directional = policy;
        self
    }
}

impl Default for FocusScopePolicy {
    fn default() -> Self {
        Self::nested_default()
    }
}

/// Authored configuration of one nested focus scope.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FocusScope {
    policy: FocusScopePolicy,
    remember_last: bool,
}

impl FocusScope {
    /// Creates a remembering nested scope with delegating boundaries.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            policy: FocusScopePolicy::nested_default(),
            remember_last: true,
        }
    }

    #[must_use]
    pub const fn with_policy(mut self, policy: FocusScopePolicy) -> Self {
        self.policy = policy;
        self
    }

    #[must_use]
    pub const fn remember_last(mut self, remember: bool) -> Self {
        self.remember_last = remember;
        self
    }

    #[must_use]
    pub const fn policy(self) -> FocusScopePolicy {
        self.policy
    }

    #[must_use]
    pub const fn remembers_last(self) -> bool {
        self.remember_last
    }
}

impl Default for FocusScope {
    fn default() -> Self {
        Self::new()
    }
}

/// Authored participation of one mounted node in focus selection.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Focusability {
    /// Use the widget's current enabled/actionable capability.
    #[default]
    Automatic,
    /// Participate while the widget remains enabled, even when not actionable.
    Focusable,
    /// Do not participate in focus selection.
    NotFocusable,
    /// Exclude this node as hidden from focus selection.
    Hidden,
}

/// Kind of one routed, non-cancelable focus notification.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FocusEventKind {
    Out,
    In,
}

/// Immutable routed focus-transition payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FocusEvent {
    kind: FocusEventKind,
    reason: FocusReason,
    target: MountedNodeId,
}

impl FocusEvent {
    /// Runtime-only construction for a committed exact mounted lifetime.
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        kind: FocusEventKind,
        reason: FocusReason,
        target: MountedNodeId,
    ) -> Self {
        Self {
            kind,
            reason,
            target,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> FocusEventKind {
        self.kind
    }

    #[must_use]
    pub const fn reason(&self) -> FocusReason {
        self.reason
    }

    #[must_use]
    pub const fn target(&self) -> &MountedNodeId {
        &self.target
    }
}
