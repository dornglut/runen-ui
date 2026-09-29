use core::num::NonZeroU64;
use std::collections::HashMap;

use runenui_core::SurfaceId;

use crate::semantic_compositor::{SemanticCandidate, SemanticCandidateNode};
use crate::{SemanticDiagnostic, SemanticDiagnosticReport};

use super::{
    SemanticFocusChange, SemanticNode, SemanticNodeState, SemanticPublication,
    SemanticRelationship, SemanticRevision, SemanticSnapshot, SemanticUpdate,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPublicationPlanError {
    RevisionExhausted,
}

#[derive(Clone)]
struct SemanticProducts {
    publication: SemanticPublication,
    diagnostics: SemanticDiagnosticReport,
}

pub struct SemanticPublicationPlan {
    products: Option<SemanticProducts>,
}

impl SemanticPublicationPlan {
    pub(crate) const fn publication(&self) -> Option<&SemanticPublication> {
        match self.products.as_ref() {
            Some(products) => Some(&products.publication),
            None => None,
        }
    }

    pub(crate) const fn diagnostics(&self) -> Option<&SemanticDiagnosticReport> {
        match self.products.as_ref() {
            Some(products) => Some(&products.diagnostics),
            None => None,
        }
    }
}

#[derive(Default)]
pub struct SemanticPublicationState {
    current: Option<SemanticProducts>,
}

impl SemanticPublicationState {
    pub(crate) const fn current_publication(&self) -> Option<&SemanticPublication> {
        match self.current.as_ref() {
            Some(products) => Some(&products.publication),
            None => None,
        }
    }

    pub(crate) fn plan(
        &self,
        surface: &SurfaceId,
        candidate: Option<(SemanticCandidate, Vec<SemanticDiagnostic>)>,
    ) -> Result<SemanticPublicationPlan, SemanticPublicationPlanError> {
        let Some((candidate, mut diagnostics)) = candidate else {
            return Ok(SemanticPublicationPlan {
                products: self.current.clone(),
            });
        };
        diagnostics.extend(
            candidate
                .diagnostics
                .iter()
                .cloned()
                .map(SemanticDiagnostic::from),
        );
        let diagnostics = SemanticDiagnosticReport::new(surface.clone(), diagnostics);
        let publication = match self.current.as_ref() {
            None => publication_from_candidate(surface, SemanticRevision::FIRST, candidate, None),
            Some(current)
                if candidate_matches_snapshot(&candidate, current.publication.snapshot()) =>
            {
                current.publication.clone()
            }
            Some(current) => {
                let revision = current
                    .publication
                    .snapshot()
                    .revision()
                    .get()
                    .checked_add(1)
                    .and_then(NonZeroU64::new)
                    .map(SemanticRevision)
                    .ok_or(SemanticPublicationPlanError::RevisionExhausted)?;
                publication_from_candidate(
                    surface,
                    revision,
                    candidate,
                    Some(current.publication.snapshot()),
                )
            }
        };
        Ok(SemanticPublicationPlan {
            products: Some(SemanticProducts {
                publication,
                diagnostics,
            }),
        })
    }

    pub(crate) fn commit(&mut self, plan: SemanticPublicationPlan) {
        if let Some(products) = plan.products {
            self.current = Some(products);
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
        && candidate.read_only == published.state.read_only
        && candidate.checked == published.state.checked
        && candidate.pressed == published.state.pressed
        && candidate.selected == published.state.selected
        && candidate.expanded == published.state.expanded
        && candidate.required == published.state.required
        && candidate.invalid == published.state.invalid
        && candidate.modal == published.state.modal
        && candidate.supported_actions == published.supported_actions
        && candidate.bounds == published.bounds
        && candidate.text == published.text
        && candidate.editable == published.editable
        && candidate.range == published.range
        && candidate.orientation == published.orientation
        && candidate.popup == published.popup
        && candidate.selection_mode == published.selection_mode
        && candidate.collection_position == published.collection_position
        && candidate.hierarchy_level == published.hierarchy_level
        && candidate.placeholder == published.placeholder
        && candidate.autocomplete == published.autocomplete
        && candidate.editable_mode == published.editable_mode
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
                read_only: node.read_only,
                checked: node.checked,
                pressed: node.pressed,
                selected: node.selected,
                expanded: node.expanded,
                required: node.required,
                invalid: node.invalid,
                modal: node.modal,
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
            editable: node.editable,
            range: node.range,
            orientation: node.orientation,
            popup: node.popup,
            selection_mode: node.selection_mode,
            collection_position: node.collection_position,
            hierarchy_level: node.hierarchy_level,
            placeholder: node.placeholder,
            autocomplete: node.autocomplete,
            editable_mode: node.editable_mode,
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
    SemanticPublication::new(snapshot, update)
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

    use runenui_core::{
        __runtime::RuntimeNamespace, LogicalPoint, LogicalRect, LogicalSize, SemanticAutocomplete,
        SemanticCheckedState, SemanticCollectionPosition, SemanticEditableMode,
        SemanticHierarchyLevel, SemanticInvalidState, SemanticNumber, SemanticOrientation,
        SemanticPopupKind, SemanticPressedState, SemanticRange, SemanticRole,
        SemanticSelectionMode,
    };

    use crate::semantic_compositor::{
        SemanticCandidate, SemanticCandidateNode, SemanticCompositionDiagnostic,
    };
    use crate::{SemanticDiagnostic, SemanticDiagnosticReport};

    use super::{
        SemanticProducts, SemanticPublicationPlanError, SemanticPublicationState, SemanticRevision,
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
                read_only: false,
                checked: None,
                pressed: None,
                selected: None,
                expanded: None,
                required: None,
                invalid: None,
                modal: None,
                supported_actions: Vec::new(),
                relationships: Vec::new(),
                bounds: rect(width),
                text: None,
                editable: None,
                range: None,
                orientation: None,
                popup: None,
                selection_mode: None,
                collection_position: None,
                hierarchy_level: None,
                placeholder: None,
                autocomplete: None,
                editable_mode: None,
            }],
            focused: Some(id),
            diagnostics,
        }
    }

    fn planned(candidate: SemanticCandidate) -> (SemanticCandidate, Vec<SemanticDiagnostic>) {
        (candidate, Vec::new())
    }

    #[test]
    fn first_commit_is_revision_one_without_synthetic_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let plan = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 10.0, Vec::new()))),
            )
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(plan);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("first semantic products committed"));
        assert_eq!(
            current.publication.snapshot().revision(),
            SemanticRevision::FIRST
        );
        assert!(current.publication.update().is_none());
        assert!(current.diagnostics.is_empty());
    }

    #[test]
    fn unchanged_and_clean_plans_reuse_the_exact_committed_products() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let initial = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 10.0, Vec::new()))),
            )
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);
        let committed = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("semantic products committed"))
            .clone();

        let unchanged = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 10.0, Vec::new()))),
            )
            .unwrap_or_else(|_| unreachable!("unchanged semantics need no revision"));
        let unchanged_publication = unchanged
            .publication()
            .unwrap_or_else(|| unreachable!("unchanged plan carries current publication"));
        assert!(
            committed
                .publication
                .shares_storage_with(unchanged_publication)
        );
        state.commit(unchanged);

        let clean = state
            .plan(&surface, None)
            .unwrap_or_else(|_| unreachable!("clean semantics need no revision"));
        let clean_publication = clean
            .publication()
            .unwrap_or_else(|| unreachable!("clean plan carries current publication"));
        let clean_diagnostics = clean
            .diagnostics()
            .unwrap_or_else(|| unreachable!("clean plan carries current diagnostics"));
        assert!(committed.publication.shares_storage_with(clean_publication));
        assert_eq!(&committed.diagnostics, clean_diagnostics);
    }

    #[test]
    fn diagnostics_only_candidate_reuses_publication_without_advancing_revision() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let initial = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 10.0, Vec::new()))),
            )
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);
        let committed = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("semantic products committed"))
            .publication
            .clone();

        let diagnostic = SemanticCompositionDiagnostic::FocusedOwnerMissingVisiblePrimary;
        let unchanged = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 10.0, vec![diagnostic]))),
            )
            .unwrap_or_else(|_| unreachable!("diagnostics do not need a semantic revision"));
        let planned_publication = unchanged
            .publication()
            .unwrap_or_else(|| unreachable!("diagnostic plan carries semantic publication"));
        assert!(committed.shares_storage_with(planned_publication));
        state.commit(unchanged);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("semantic products remain committed"));
        assert_eq!(
            current.publication.snapshot().revision(),
            SemanticRevision::FIRST
        );
        assert_eq!(
            current.diagnostics.diagnostics(),
            &[SemanticDiagnostic::FocusedOwnerMissingVisiblePrimary]
        );
    }

    #[test]
    fn checked_state_change_advances_revision_and_is_present_in_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let mut initial_candidate = candidate(&namespace, 10.0, Vec::new());
        initial_candidate.nodes[0].role = SemanticRole::Checkbox;
        initial_candidate.nodes[0].checked = Some(SemanticCheckedState::Unchecked);
        let initial = state
            .plan(&surface, Some(planned(initial_candidate)))
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);

        let mut changed_candidate = candidate(&namespace, 10.0, Vec::new());
        changed_candidate.nodes[0].role = SemanticRole::Checkbox;
        changed_candidate.nodes[0].checked = Some(SemanticCheckedState::Checked);
        let changed = state
            .plan(&surface, Some(planned(changed_candidate)))
            .unwrap_or_else(|_| unreachable!("second semantic revision is available"));
        state.commit(changed);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("changed semantic products committed"));
        assert_eq!(current.publication.snapshot().revision().get(), 2);
        assert_eq!(
            current.publication.snapshot().nodes()[0].state().checked(),
            Some(SemanticCheckedState::Checked)
        );
        let update = current
            .publication
            .update()
            .unwrap_or_else(|| unreachable!("checked-state change retains one delta"));
        assert_eq!(update.changed().len(), 1);
        assert_eq!(
            update.changed()[0].state().checked(),
            Some(SemanticCheckedState::Checked)
        );
    }

    #[test]
    fn selected_state_change_advances_revision_and_is_present_in_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();

        let mut initial_candidate = candidate(&namespace, 10.0, Vec::new());
        initial_candidate.nodes[0].role = SemanticRole::Option;
        initial_candidate.nodes[0].selected = Some(false);
        let initial = state
            .plan(&surface, Some(planned(initial_candidate)))
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);

        let mut changed_candidate = candidate(&namespace, 10.0, Vec::new());
        changed_candidate.nodes[0].role = SemanticRole::Option;
        changed_candidate.nodes[0].selected = Some(true);
        let changed = state
            .plan(&surface, Some(planned(changed_candidate)))
            .unwrap_or_else(|_| unreachable!("second semantic revision is available"));
        state.commit(changed);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("changed semantic products committed"));
        assert_eq!(current.publication.snapshot().revision().get(), 2);
        assert_eq!(
            current.publication.snapshot().nodes()[0].state().selected(),
            Some(true)
        );
        let update = current
            .publication
            .update()
            .unwrap_or_else(|| unreachable!("selected-state change retains one delta"));
        assert_eq!(update.changed().len(), 1);
        assert_eq!(update.changed()[0].state().selected(), Some(true));
    }

    #[test]
    fn range_and_orientation_changes_advance_revision_and_are_present_in_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let minimum = SemanticNumber::new(0.0)
            .unwrap_or_else(|_| unreachable!("controlled minimum is finite"));
        let maximum = SemanticNumber::new(100.0)
            .unwrap_or_else(|_| unreachable!("controlled maximum is finite"));

        let mut initial_candidate = candidate(&namespace, 10.0, Vec::new());
        initial_candidate.nodes[0].role = SemanticRole::Slider;
        initial_candidate.nodes[0].orientation = Some(SemanticOrientation::Horizontal);
        initial_candidate.nodes[0].range = Some(
            SemanticRange::new(
                Some(minimum),
                Some(maximum),
                Some(
                    SemanticNumber::new(25.0)
                        .unwrap_or_else(|_| unreachable!("controlled current is finite")),
                ),
            )
            .unwrap_or_else(|_| unreachable!("controlled range is valid")),
        );
        let initial = state
            .plan(&surface, Some(planned(initial_candidate)))
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);

        let mut changed_candidate = candidate(&namespace, 10.0, Vec::new());
        changed_candidate.nodes[0].role = SemanticRole::Slider;
        changed_candidate.nodes[0].orientation = Some(SemanticOrientation::Vertical);
        changed_candidate.nodes[0].range = Some(
            SemanticRange::new(
                Some(minimum),
                Some(maximum),
                Some(
                    SemanticNumber::new(50.0)
                        .unwrap_or_else(|_| unreachable!("controlled current is finite")),
                ),
            )
            .unwrap_or_else(|_| unreachable!("controlled range is valid")),
        );
        let changed = state
            .plan(&surface, Some(planned(changed_candidate)))
            .unwrap_or_else(|_| unreachable!("second semantic revision is available"));
        state.commit(changed);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("changed semantic products committed"));
        assert_eq!(current.publication.snapshot().revision().get(), 2);
        let node = &current.publication.snapshot().nodes()[0];
        assert_eq!(node.orientation(), Some(SemanticOrientation::Vertical));
        assert_eq!(
            node.range().and_then(SemanticRange::current),
            SemanticNumber::new(50.0).ok()
        );
        let update = current
            .publication
            .update()
            .unwrap_or_else(|| unreachable!("range/orientation change retains one delta"));
        assert_eq!(update.changed().len(), 1);
        assert_eq!(
            update.changed()[0].orientation(),
            Some(SemanticOrientation::Vertical)
        );
    }

    #[test]
    fn remaining_typed_property_families_advance_revision_and_are_present_in_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();

        let initial = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 10.0, Vec::new()))),
            )
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);

        let mut changed_candidate = candidate(&namespace, 10.0, Vec::new());
        let changed_node = &mut changed_candidate.nodes[0];
        changed_node.pressed = Some(SemanticPressedState::Pressed);
        changed_node.expanded = Some(true);
        changed_node.required = Some(true);
        changed_node.invalid = Some(SemanticInvalidState::Grammar);
        changed_node.modal = Some(true);
        changed_node.popup = Some(SemanticPopupKind::ListBox);
        changed_node.selection_mode = Some(SemanticSelectionMode::Multiple);
        changed_node.collection_position = Some(
            SemanticCollectionPosition::new(2, Some(4))
                .unwrap_or_else(|_| unreachable!("controlled collection position is valid")),
        );
        changed_node.hierarchy_level = Some(
            SemanticHierarchyLevel::new(3)
                .unwrap_or_else(|_| unreachable!("controlled hierarchy level is valid")),
        );
        changed_node.placeholder = Some("Filter".to_owned());
        changed_node.autocomplete = Some(SemanticAutocomplete::Both);
        changed_node.editable_mode = Some(SemanticEditableMode::SingleLine);

        let changed = state
            .plan(&surface, Some(planned(changed_candidate)))
            .unwrap_or_else(|_| unreachable!("second semantic revision is available"));
        state.commit(changed);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("changed semantic products committed"));
        assert_eq!(current.publication.snapshot().revision().get(), 2);
        let update = current
            .publication
            .update()
            .unwrap_or_else(|| unreachable!("typed property change retains one delta"));
        assert_eq!(update.changed().len(), 1);
        let node = &update.changed()[0];
        assert_eq!(node.state().pressed(), Some(SemanticPressedState::Pressed));
        assert_eq!(node.state().expanded(), Some(true));
        assert_eq!(node.state().required(), Some(true));
        assert_eq!(node.state().invalid(), Some(SemanticInvalidState::Grammar));
        assert_eq!(node.state().modal(), Some(true));
        assert_eq!(node.popup(), Some(SemanticPopupKind::ListBox));
        assert_eq!(node.selection_mode(), Some(SemanticSelectionMode::Multiple));
        assert_eq!(
            node.collection_position()
                .map(SemanticCollectionPosition::index),
            Some(2)
        );
        assert_eq!(
            node.hierarchy_level().map(SemanticHierarchyLevel::get),
            Some(3)
        );
        assert_eq!(node.placeholder(), Some("Filter"));
        assert_eq!(node.autocomplete(), Some(SemanticAutocomplete::Both));
        assert_eq!(node.editable_mode(), Some(SemanticEditableMode::SingleLine));
    }

    #[test]
    fn changed_candidate_advances_once_and_builds_exact_delta() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let mut state = SemanticPublicationState::default();
        let initial = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 10.0, Vec::new()))),
            )
            .unwrap_or_else(|_| unreachable!("first semantic revision is available"));
        state.commit(initial);

        let changed = state
            .plan(
                &surface,
                Some(planned(candidate(&namespace, 20.0, Vec::new()))),
            )
            .unwrap_or_else(|_| unreachable!("second semantic revision is available"));
        state.commit(changed);

        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("changed semantic products committed"));
        assert_eq!(current.publication.snapshot().revision().get(), 2);
        let update = current
            .publication
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
    fn exhausted_revision_refuses_without_mutating_committed_products() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let max_revision = SemanticRevision(NonZeroU64::MAX);
        let state = SemanticPublicationState {
            current: Some(SemanticProducts {
                publication: publication_from_candidate(
                    &surface,
                    max_revision,
                    candidate(&namespace, 10.0, Vec::new()),
                    None,
                ),
                diagnostics: SemanticDiagnosticReport::new(surface.clone(), Vec::new()),
            }),
        };

        let result = state.plan(
            &surface,
            Some(planned(candidate(&namespace, 20.0, Vec::new()))),
        );
        assert_eq!(
            result.err(),
            Some(SemanticPublicationPlanError::RevisionExhausted)
        );
        let current = state
            .current
            .as_ref()
            .unwrap_or_else(|| unreachable!("failed plan keeps current products"));
        assert_eq!(current.publication.snapshot().revision(), max_revision);
        assert_eq!(
            current.publication.snapshot().nodes()[0].bounds(),
            rect(10.0)
        );
        assert!(current.diagnostics.is_empty());
    }
}
