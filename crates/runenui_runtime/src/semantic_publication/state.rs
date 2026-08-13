use core::num::NonZeroU64;
use std::{collections::HashMap, sync::Arc};

use runenui_core::SurfaceId;

use crate::semantic_compositor::{
    SemanticCandidate, SemanticCandidateNode, SemanticCompositionDiagnostic,
};

use super::{
    SemanticFocusChange, SemanticNode, SemanticNodeState, SemanticPublication,
    SemanticRelationship, SemanticRevision, SemanticSnapshot, SemanticUpdate,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPublicationPlanError {
    RevisionExhausted,
}

pub struct SemanticPublicationPlan {
    publication: Option<Arc<SemanticPublication>>,
    diagnostics: Option<Vec<SemanticCompositionDiagnostic>>,
}

#[derive(Default)]
pub struct SemanticPublicationState {
    current: Option<Arc<SemanticPublication>>,
    diagnostics: Vec<SemanticCompositionDiagnostic>,
}

impl SemanticPublicationState {
    pub(crate) fn plan(
        &self,
        surface: &SurfaceId,
        candidate: Option<SemanticCandidate>,
    ) -> Result<SemanticPublicationPlan, SemanticPublicationPlanError> {
        let Some(candidate) = candidate else {
            return Ok(SemanticPublicationPlan {
                publication: None,
                diagnostics: None,
            });
        };
        let diagnostics = Some(candidate.diagnostics.clone());
        let publication = match self.current.as_ref() {
            None => Some(Arc::new(publication_from_candidate(
                surface,
                SemanticRevision::FIRST,
                candidate,
                None,
            ))),
            Some(current) if candidate_matches_snapshot(&candidate, current.snapshot()) => None,
            Some(current) => {
                let revision = current
                    .snapshot()
                    .revision()
                    .get()
                    .checked_add(1)
                    .and_then(NonZeroU64::new)
                    .map(SemanticRevision)
                    .ok_or(SemanticPublicationPlanError::RevisionExhausted)?;
                Some(Arc::new(publication_from_candidate(
                    surface,
                    revision,
                    candidate,
                    Some(current.snapshot()),
                )))
            }
        };
        Ok(SemanticPublicationPlan {
            publication,
            diagnostics,
        })
    }

    pub(crate) fn commit(&mut self, plan: SemanticPublicationPlan) {
        if let Some(publication) = plan.publication {
            self.current = Some(publication);
        }
        if let Some(diagnostics) = plan.diagnostics {
            self.diagnostics = diagnostics;
        }
    }
}

fn candidate_matches_snapshot(candidate: &SemanticCandidate, snapshot: &SemanticSnapshot) -> bool {
    candidate.roots == snapshot.roots
        && candidate.focused == snapshot.focused
        && candidate.nodes.len() == snapshot.nodes.len()
        && candidate
            .nodes
            .iter()
            .zip(&snapshot.nodes)
            .all(|(candidate, published)| candidate_node_matches(candidate, published))
}

fn candidate_node_matches(candidate: &SemanticCandidateNode, published: &SemanticNode) -> bool {
    candidate.id == published.id
        && candidate.parent == published.parent
        && candidate.children == published.children
        && candidate.role == published.role
        && candidate.name == published.name
        && candidate.description == published.description
        && candidate.value == published.value
        && candidate.disabled == published.state.disabled
        && candidate.inert == published.state.inert
        && candidate.supported_actions == published.supported_actions
        && candidate.bounds == published.bounds
        && candidate.text == published.text
        && candidate.relationships.len() == published.relationships.len()
        && candidate
            .relationships
            .iter()
            .zip(&published.relationships)
            .all(|(candidate, published)| {
                candidate.kind == published.kind && candidate.target == published.target
            })
}

fn publication_from_candidate(
    surface: &SurfaceId,
    revision: SemanticRevision,
    candidate: SemanticCandidate,
    previous: Option<&SemanticSnapshot>,
) -> SemanticPublication {
    let nodes = candidate
        .nodes
        .into_iter()
        .map(|node| SemanticNode {
            id: node.id,
            parent: node.parent,
            children: node.children,
            role: node.role,
            name: node.name,
            description: node.description,
            value: node.value,
            state: SemanticNodeState {
                disabled: node.disabled,
                inert: node.inert,
            },
            supported_actions: node.supported_actions,
            relationships: node
                .relationships
                .into_iter()
                .map(|relationship| SemanticRelationship {
                    kind: relationship.kind,
                    target: relationship.target,
                })
                .collect(),
            bounds: node.bounds,
            text: node.text,
        })
        .collect::<Vec<_>>();
    let index = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.id.clone(), position))
        .collect::<HashMap<_, _>>();
    let snapshot = SemanticSnapshot {
        surface: surface.clone(),
        revision,
        roots: candidate.roots,
        nodes,
        focused: candidate.focused,
        index,
    };
    let update = previous.map(|previous| semantic_update(previous, &snapshot));
    SemanticPublication { snapshot, update }
}

fn semantic_update(previous: &SemanticSnapshot, current: &SemanticSnapshot) -> SemanticUpdate {
    let removed = previous
        .nodes
        .iter()
        .filter(|node| !current.index.contains_key(&node.id))
        .map(|node| node.id.clone())
        .collect();
    let mut added = Vec::new();
    let mut changed = Vec::new();
    for node in &current.nodes {
        match previous.node(&node.id) {
            None => added.push(node.clone()),
            Some(previous) if previous != node => changed.push(node.clone()),
            Some(_) => {}
        }
    }
    let roots = (previous.roots != current.roots).then(|| current.roots.clone());
    let focus = (previous.focused != current.focused).then(|| SemanticFocusChange {
        previous: previous.focused.clone(),
        current: current.focused.clone(),
    });
    SemanticUpdate {
        surface: current.surface.clone(),
        previous_revision: previous.revision,
        revision: current.revision,
        removed,
        added,
        changed,
        roots,
        focus,
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;
    use std::sync::Arc;

    use runenui_core::{
        __runtime::RuntimeNamespace, LogicalPoint, LogicalRect, LogicalSize, SemanticRole,
    };

    use crate::semantic_compositor::{
        SemanticCandidate, SemanticCandidateNode, SemanticCompositionDiagnostic,
    };

    use super::{
        SemanticPublicationPlanError, SemanticPublicationState, SemanticRevision,
        publication_from_candidate,
    };

    fn rect(width: f32) -> LogicalRect {
        LogicalRect::new(
            LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!("finite test point")),
            LogicalSize::try_new(width, 10.0).unwrap_or_else(|_| unreachable!("valid test size")),
        )
    }

    fn candidate(
        namespace: &RuntimeNamespace,
        width: f32,
        diagnostics: Vec<SemanticCompositionDiagnostic>,
    ) -> SemanticCandidate {
        let id = namespace.__runtime_semantic_id(0, 1);
        SemanticCandidate {
            roots: vec![id.clone()],
            nodes: vec![SemanticCandidateNode {
                id: id.clone(),
                parent: None,
                children: Vec::new(),
                role: SemanticRole::Button,
                name: Some("Save".to_owned()),
                description: None,
                value: None,
                disabled: false,
                inert: false,
                supported_actions: Vec::new(),
                relationships: Vec::new(),
                bounds: rect(width),
                text: None,
            }],
            focused: Some(id),
            diagnostics,
        }
    }

    #[test]
    fn first_commit_is_revision_one_without_synthetic_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let plan = state
            .plan(&surface, Some(candidate(&namespace, 10.0, Vec::new())))
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(plan);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("first semantic publication committed"));
        assert_eq!(current.snapshot().revision(), SemanticRevision::FIRST);
        assert!(current.update().is_none());
    }

    #[test]
    fn unchanged_and_diagnostics_only_candidates_do_not_advance_revision() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let initial = state
            .plan(&surface, Some(candidate(&namespace, 10.0, Vec::new())))
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);

        let diagnostic = SemanticCompositionDiagnostic::FocusedOwnerMissingVisiblePrimary;
        let unchanged = state
            .plan(
                &surface,
                Some(candidate(&namespace, 10.0, vec![diagnostic.clone()])),
            )
            .unwrap_or_else(|_| unreachable!("unchanged semantics need no revision"));
        state.commit(unchanged);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("semantic publication remains committed"));
        assert_eq!(current.snapshot().revision(), SemanticRevision::FIRST);
        assert_eq!(state.diagnostics, vec![diagnostic]);
    }

    #[test]
    fn changed_candidate_advances_once_and_builds_exact_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let initial = state
            .plan(&surface, Some(candidate(&namespace, 10.0, Vec::new())))
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);

        let changed = state
            .plan(&surface, Some(candidate(&namespace, 20.0, Vec::new())))
            .unwrap_or_else(|_| unreachable!("second semantic revision is available"));
        state.commit(changed);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("changed semantic publication committed"));
        assert_eq!(current.snapshot().revision().get(), 2);
        let update = current
            .update()
            .unwrap_or_else(|| unreachable!("changed publication retains one delta"));
        assert_eq!(update.previous_revision(), SemanticRevision::FIRST);
        assert!(update.removed().is_empty());
        assert!(update.added().is_empty());
        assert_eq!(update.changed().len(), 1);
        assert!(update.roots().is_none());
        assert!(update.focus().is_none());
    }

    #[test]
    fn exhausted_revision_refuses_without_mutating_committed_publication() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let max_revision = SemanticRevision(NonZeroU64::MAX);
        let state = SemanticPublicationState {
            current: Some(Arc::new(publication_from_candidate(
                &surface,
                max_revision,
                candidate(&namespace, 10.0, Vec::new()),
                None,
            ))),
            diagnostics: Vec::new(),
        };

        let result = state.plan(&surface, Some(candidate(&namespace, 20.0, Vec::new())));
        assert_eq!(
            result.err(),
            Some(SemanticPublicationPlanError::RevisionExhausted)
        );
        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("failed plan keeps current publication"));
        assert_eq!(current.snapshot().revision(), max_revision);
        assert_eq!(current.snapshot().nodes()[0].bounds(), rect(10.0));
    }
}
