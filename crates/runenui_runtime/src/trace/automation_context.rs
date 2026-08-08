use runenui_core::{ElementId, SemanticCommand};

use crate::AutomationMatchDiagnostic;

/// Semantic role of one authored-automation resolution fact.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceAutomationRecordRole {
    Unique,
    Missing,
    Ambiguous,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TraceAutomationContextData {
    Unique {
        authored_id: ElementId,
        command: SemanticCommand,
    },
    Missing {
        authored_id: ElementId,
        command: SemanticCommand,
    },
    Ambiguous {
        authored_id: ElementId,
        command: SemanticCommand,
        candidates: Vec<AutomationMatchDiagnostic>,
    },
}

/// Exact authored automation intent retained without re-resolving a target later.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceAutomationContext {
    data: TraceAutomationContextData,
}

impl TraceAutomationContext {
    pub(crate) const fn unique(authored_id: ElementId, command: SemanticCommand) -> Self {
        Self {
            data: TraceAutomationContextData::Unique {
                authored_id,
                command,
            },
        }
    }

    pub(crate) const fn missing(authored_id: ElementId, command: SemanticCommand) -> Self {
        Self {
            data: TraceAutomationContextData::Missing {
                authored_id,
                command,
            },
        }
    }

    pub(crate) const fn ambiguous(
        authored_id: ElementId,
        command: SemanticCommand,
        candidates: Vec<AutomationMatchDiagnostic>,
    ) -> Self {
        Self {
            data: TraceAutomationContextData::Ambiguous {
                authored_id,
                command,
                candidates,
            },
        }
    }

    /// Returns the resolution role.
    #[must_use]
    pub const fn role(&self) -> TraceAutomationRecordRole {
        match &self.data {
            TraceAutomationContextData::Unique { .. } => TraceAutomationRecordRole::Unique,
            TraceAutomationContextData::Missing { .. } => TraceAutomationRecordRole::Missing,
            TraceAutomationContextData::Ambiguous { .. } => TraceAutomationRecordRole::Ambiguous,
        }
    }

    /// Returns the exact authored ID supplied by automation.
    #[must_use]
    pub const fn authored_id(&self) -> &ElementId {
        match &self.data {
            TraceAutomationContextData::Unique { authored_id, .. }
            | TraceAutomationContextData::Missing { authored_id, .. }
            | TraceAutomationContextData::Ambiguous { authored_id, .. } => authored_id,
        }
    }

    /// Returns the exact semantic command supplied by automation.
    #[must_use]
    pub const fn command(&self) -> SemanticCommand {
        match &self.data {
            TraceAutomationContextData::Unique { command, .. }
            | TraceAutomationContextData::Missing { command, .. }
            | TraceAutomationContextData::Ambiguous { command, .. } => *command,
        }
    }

    /// Returns deterministic redacted ambiguity diagnostics only for ambiguous resolution.
    #[must_use]
    pub const fn candidates(&self) -> Option<&[AutomationMatchDiagnostic]> {
        match &self.data {
            TraceAutomationContextData::Ambiguous { candidates, .. } => Some(candidates.as_slice()),
            TraceAutomationContextData::Unique { .. }
            | TraceAutomationContextData::Missing { .. } => None,
        }
    }
}
