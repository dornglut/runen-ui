use core::{error::Error, fmt};

use runenui_core::{
    SemanticAction, SemanticActionRequest, SemanticNodeId, SemanticRole, SurfaceId,
};
use runenui_runtime::{SemanticNode, SemanticSnapshot};

/// Exact surface-scoped semantic target produced from a committed snapshot.
///
/// The fields are intentionally private. A target can be obtained only after
/// exact snapshot membership is proven, preventing helpers from guessing a
/// surface for a bare semantic identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticTarget {
    surface: SurfaceId,
    node: SemanticNodeId,
}

impl SemanticTarget {
    /// Validates one semantic identity against an exact committed snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTargetError::Missing`] when the exact node is not a
    /// member of the provided snapshot.
    pub fn from_snapshot(
        snapshot: &SemanticSnapshot,
        node: SemanticNodeId,
    ) -> Result<Self, SemanticTargetError> {
        if snapshot.node(&node).is_none() {
            return Err(SemanticTargetError::Missing);
        }
        Ok(Self {
            surface: snapshot.surface_id().clone(),
            node,
        })
    }

    /// Returns the exact surface scope proven by the source snapshot.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface
    }

    /// Returns the exact semantic lifetime proven by the source snapshot.
    #[must_use]
    pub const fn node_id(&self) -> &SemanticNodeId {
        &self.node
    }

    /// Creates the ordinary public M5C request for this exact scoped target.
    pub fn request(&self, action: SemanticAction) -> SemanticActionRequest {
        SemanticActionRequest::new(self.surface.clone(), self.node.clone(), action)
    }
}

/// Failure to bind a bare semantic identity to an exact snapshot scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticTargetError {
    Missing,
}

impl fmt::Display for SemanticTargetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("semantic node is not present in the snapshot"),
        }
    }
}

impl Error for SemanticTargetError {}

/// Deterministic exact-match query over one committed semantic snapshot.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SemanticQuery {
    role: Option<SemanticRole>,
    name: Option<String>,
    description: Option<String>,
    supported_action: Option<SemanticAction>,
    disabled: Option<bool>,
    inert: Option<bool>,
}

impl SemanticQuery {
    /// Creates a query with no filters; it matches every published semantic node.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            role: None,
            name: None,
            description: None,
            supported_action: None,
            disabled: None,
            inert: None,
        }
    }

    /// Requires an exact platform-neutral semantic role.
    #[must_use]
    pub const fn with_role(mut self, role: SemanticRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Requires an exact semantic name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Requires an exact semantic description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Requires one supported semantic action.
    #[must_use]
    pub const fn with_supported_action(mut self, action: SemanticAction) -> Self {
        self.supported_action = Some(action);
        self
    }

    /// Requires the exact composed disabled state.
    #[must_use]
    pub const fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Requires the exact semantic inert state.
    #[must_use]
    pub const fn with_inert(mut self, inert: bool) -> Self {
        self.inert = Some(inert);
        self
    }

    fn matches(&self, node: &SemanticNode) -> bool {
        self.role.is_none_or(|role| node.role() == role)
            && self
                .name
                .as_deref()
                .is_none_or(|name| node.name() == Some(name))
            && self
                .description
                .as_deref()
                .is_none_or(|description| node.description() == Some(description))
            && self.supported_action.as_ref().is_none_or(|action| {
                node.supported_actions()
                    .iter()
                    .any(|supported| supported == action)
            })
            && self
                .disabled
                .is_none_or(|disabled| node.state().disabled() == disabled)
            && self.inert.is_none_or(|inert| node.state().inert() == inert)
    }
}

/// Deterministic snapshot-scoped semantic query result in published preorder.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticQueryMatches {
    targets: Vec<SemanticTarget>,
}

impl SemanticQueryMatches {
    /// Returns all matching exact targets in deterministic published preorder.
    #[must_use]
    pub const fn targets(&self) -> &[SemanticTarget] {
        self.targets.as_slice()
    }

    /// Returns the number of matches.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.targets.len()
    }

    /// Returns whether there are no matches.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Requires exactly one match without first/last fallback.
    ///
    /// # Errors
    ///
    /// Distinguishes no match from ambiguity and preserves all ambiguous exact
    /// targets in deterministic order.
    pub fn unique(self) -> Result<SemanticTarget, UniqueSemanticQueryError> {
        match self.targets.len() {
            0 => Err(UniqueSemanticQueryError::Missing),
            1 => self
                .targets
                .into_iter()
                .next()
                .map_or(Err(UniqueSemanticQueryError::Missing), Ok),
            _ => Err(UniqueSemanticQueryError::Ambiguous {
                matches: self.targets,
            }),
        }
    }
}

/// Exact unique-query failure; ambiguity never selects an arbitrary target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UniqueSemanticQueryError {
    Missing,
    Ambiguous { matches: Vec<SemanticTarget> },
}

impl UniqueSemanticQueryError {
    /// Returns ambiguous exact targets, or an empty slice for a missing query.
    #[must_use]
    pub const fn matches(&self) -> &[SemanticTarget] {
        match self {
            Self::Missing => &[],
            Self::Ambiguous { matches } => matches.as_slice(),
        }
    }
}

impl fmt::Display for UniqueSemanticQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("semantic query matched no nodes"),
            Self::Ambiguous { matches } => write!(
                formatter,
                "semantic query matched {} nodes instead of exactly one",
                matches.len()
            ),
        }
    }
}

impl Error for UniqueSemanticQueryError {}

/// Evaluates an exact-match query against one committed semantic snapshot.
#[must_use]
pub fn query_semantics(snapshot: &SemanticSnapshot, query: &SemanticQuery) -> SemanticQueryMatches {
    let targets = snapshot
        .nodes()
        .iter()
        .filter(|node| query.matches(node))
        .map(|node| SemanticTarget {
            surface: snapshot.surface_id().clone(),
            node: node.id().clone(),
        })
        .collect();
    SemanticQueryMatches { targets }
}
