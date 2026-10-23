use std::{collections::HashMap, sync::Arc};

use runenui_core::{
    __runtime::transform_rect_aabb, Axis, ElementId, Focusability, LogicalRect, LogicalTransform,
    MountedNodeId, ScrollControlSnapshot, SemanticAction, SemanticAutocomplete, SemanticBounds,
    SemanticCheckedState, SemanticCollectionPosition, SemanticContribution, SemanticEditable,
    SemanticEditableMode, SemanticHierarchyLevel, SemanticInvalidState, SemanticItem, SemanticKey,
    SemanticNodeContribution, SemanticNumber, SemanticOrientation, SemanticPopupKind,
    SemanticPressedState, SemanticRange, SemanticReference, SemanticRelationshipKind, SemanticRole,
    SemanticSelectionMode, SemanticText, SemanticValue, WidgetActivation,
};

use crate::SemanticNodeId;

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticScrollControlFacts {
    pub owner: MountedNodeId,
    pub snapshot: ScrollControlSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticOwnerFacts {
    pub id: MountedNodeId,
    pub authored_id: Option<ElementId>,
    pub mounted_children: Vec<MountedNodeId>,
    pub contribution: SemanticContribution,
    pub bindings: Vec<(SemanticKey, SemanticNodeId)>,
    pub bounds: LogicalRect,
    pub activation: WidgetActivation,
    pub focusability: Focusability,
    pub scroll_control: Option<SemanticScrollControlFacts>,
    pub editable_source: Option<Arc<str>>,
    pub editable_selection: Option<runenui_core::TextSelection>,
    pub editable_caret_offsets: Option<Arc<[usize]>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSemanticRelationship {
    pub kind: SemanticRelationshipKind,
    pub target: SemanticNodeId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticCandidateNode {
    pub id: SemanticNodeId,
    pub parent: Option<SemanticNodeId>,
    pub children: Vec<SemanticNodeId>,
    pub role: SemanticRole,
    pub name: Option<String>,
    pub description: Option<String>,
    pub value: Option<SemanticValue>,
    pub disabled: bool,
    pub inert: bool,
    pub read_only: bool,
    pub checked: Option<SemanticCheckedState>,
    pub pressed: Option<SemanticPressedState>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub required: Option<bool>,
    pub invalid: Option<SemanticInvalidState>,
    pub modal: Option<bool>,
    pub supported_actions: Vec<SemanticAction>,
    pub relationships: Vec<ResolvedSemanticRelationship>,
    pub bounds: LogicalRect,
    pub text: Option<SemanticText>,
    pub editable: Option<SemanticEditable>,
    pub range: Option<SemanticRange>,
    pub orientation: Option<SemanticOrientation>,
    pub popup: Option<SemanticPopupKind>,
    pub selection_mode: Option<SemanticSelectionMode>,
    pub collection_position: Option<SemanticCollectionPosition>,
    pub hierarchy_level: Option<SemanticHierarchyLevel>,
    pub placeholder: Option<String>,
    pub autocomplete: Option<SemanticAutocomplete>,
    pub editable_mode: Option<SemanticEditableMode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticCompositionDiagnostic {
    MissingOwnerBinding {
        key: SemanticKey,
    },
    MissingMountedOwner,
    MissingLocalRelationshipTarget {
        source: SemanticNodeId,
        key: SemanticKey,
    },
    MissingAuthoredRelationshipOwner {
        source: SemanticNodeId,
        element_id: ElementId,
    },
    AmbiguousAuthoredRelationshipOwner {
        source: SemanticNodeId,
        element_id: ElementId,
    },
    MissingAuthoredRelationshipTarget {
        source: SemanticNodeId,
        element_id: ElementId,
        key: SemanticKey,
    },
    InvalidActiveDescendantTargetRole {
        source: SemanticNodeId,
        target: SemanticNodeId,
        role: SemanticRole,
    },
    ActiveDescendantOutsideControlledSubtree {
        source: SemanticNodeId,
        target: SemanticNodeId,
    },
    UnrepresentableBounds {
        source: SemanticNodeId,
    },
    FocusedOwnerMissingVisiblePrimary,
    MissingScrollControlBinding {
        source: SemanticNodeId,
    },
    MissingScrollControlTarget {
        source: SemanticNodeId,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticCandidate {
    pub roots: Vec<SemanticNodeId>,
    pub nodes: Vec<SemanticCandidateNode>,
    pub focused: Option<SemanticNodeId>,
    pub diagnostics: Vec<SemanticCompositionDiagnostic>,
}

pub fn compose_semantics(
    owners: &[SemanticOwnerFacts],
    owner_transforms: &[LogicalTransform],
    root: Option<&MountedNodeId>,
    focused_owner: Option<&MountedNodeId>,
) -> SemanticCandidate {
    let mut compositor = SemanticCompositor::new(owners, owner_transforms);
    let roots = match root.and_then(|id| compositor.owner_index(id)) {
        Some(root_index) => compositor.compose_owner(root_index, None),
        None if root.is_some() => {
            compositor
                .diagnostics
                .push(SemanticCompositionDiagnostic::MissingMountedOwner);
            Vec::new()
        }
        None => Vec::new(),
    };
    compositor.resolve_relationships();
    compositor.validate_active_descendants();
    let focused = focused_owner.and_then(|owner| {
        let focused = compositor.visible_id(owner, &SemanticKey::PRIMARY).cloned();
        if focused.is_none() {
            compositor
                .diagnostics
                .push(SemanticCompositionDiagnostic::FocusedOwnerMissingVisiblePrimary);
        }
        focused
    });
    SemanticCandidate {
        roots,
        nodes: compositor
            .drafts
            .into_iter()
            .map(|draft| draft.node)
            .collect(),
        focused,
        diagnostics: compositor.diagnostics,
    }
}

struct SemanticCompositor<'a> {
    owners: &'a [SemanticOwnerFacts],
    owner_transforms: &'a [LogicalTransform],
    owner_indices: HashMap<MountedNodeId, usize>,
    binding_ids: HashMap<MountedNodeId, HashMap<SemanticKey, SemanticNodeId>>,
    visible_ids: HashMap<MountedNodeId, HashMap<SemanticKey, SemanticNodeId>>,
    authored_owner_indices: HashMap<ElementId, AuthoredOwnerLookup>,
    drafts: Vec<SemanticNodeDraft>,
    diagnostics: Vec<SemanticCompositionDiagnostic>,
}

#[derive(Clone, Copy)]
enum AuthoredOwnerLookup {
    Unique(usize),
    Ambiguous,
}

struct SemanticNodeDraft {
    owner: MountedNodeId,
    authored_relationships: Vec<runenui_core::SemanticRelationship>,
    node: SemanticCandidateNode,
}

impl<'a> SemanticCompositor<'a> {
    fn new(owners: &'a [SemanticOwnerFacts], owner_transforms: &'a [LogicalTransform]) -> Self {
        assert_eq!(
            owners.len(),
            owner_transforms.len(),
            "semantic owner geometry remains publication-aligned"
        );
        let mut owner_indices = HashMap::with_capacity(owners.len());
        let mut binding_ids = HashMap::with_capacity(owners.len());
        let mut authored_owner_indices = HashMap::new();
        for (index, owner) in owners.iter().enumerate() {
            if owner_indices.insert(owner.id.clone(), index).is_some() {
                unreachable!("mounted publication owners are unique");
            }
            if !owner.bindings.is_empty() {
                let mut owner_bindings = HashMap::with_capacity(owner.bindings.len());
                for (key, id) in &owner.bindings {
                    if owner_bindings.insert(key.clone(), id.clone()).is_some() {
                        unreachable!("semantic owner bindings are unique");
                    }
                }
                binding_ids.insert(owner.id.clone(), owner_bindings);
            }
            if let Some(authored_id) = owner.authored_id.as_ref() {
                match authored_owner_indices.get_mut(authored_id) {
                    Some(lookup) => *lookup = AuthoredOwnerLookup::Ambiguous,
                    None => {
                        authored_owner_indices
                            .insert(authored_id.clone(), AuthoredOwnerLookup::Unique(index));
                    }
                }
            }
        }
        Self {
            owners,
            owner_transforms,
            owner_indices,
            binding_ids,
            visible_ids: HashMap::new(),
            authored_owner_indices,
            drafts: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn owner_index(&self, id: &MountedNodeId) -> Option<usize> {
        self.owner_indices.get(id).copied()
    }

    fn binding_id(&self, owner: &MountedNodeId, key: &SemanticKey) -> Option<&SemanticNodeId> {
        self.binding_ids
            .get(owner)
            .and_then(|bindings| bindings.get(key))
    }

    fn compose_owner(
        &mut self,
        owner_index: usize,
        parent: Option<&SemanticNodeId>,
    ) -> Vec<SemanticNodeId> {
        let owner = &self.owners[owner_index];
        if !contains_semantic_node(owner.contribution.roots()) {
            return self.compose_mounted_children(owner_index, parent);
        }
        self.compose_items(owner_index, owner.contribution.roots(), parent)
    }

    fn compose_items(
        &mut self,
        owner_index: usize,
        items: &[SemanticItem],
        parent: Option<&SemanticNodeId>,
    ) -> Vec<SemanticNodeId> {
        let mut roots = Vec::new();
        for item in items {
            match item {
                SemanticItem::Node(node) => {
                    roots.extend(self.compose_node(owner_index, node, parent));
                }
                SemanticItem::MountedChildren => {
                    roots.extend(self.compose_mounted_children(owner_index, parent));
                }
            }
        }
        roots
    }

    fn compose_node(
        &mut self,
        owner_index: usize,
        authored: &SemanticNodeContribution,
        parent: Option<&SemanticNodeId>,
    ) -> Vec<SemanticNodeId> {
        if authored.state().hidden() {
            return Vec::new();
        }
        let owner = &self.owners[owner_index];
        let owner_id = owner.id.clone();
        let Some(id) = self.binding_id(&owner_id, authored.key()).cloned() else {
            self.diagnostics
                .push(SemanticCompositionDiagnostic::MissingOwnerBinding {
                    key: authored.key().clone(),
                });
            return Vec::new();
        };
        let Some(bounds) = resolve_bounds(
            owner.bounds,
            self.owner_transforms[owner_index],
            authored.bounds(),
        ) else {
            self.diagnostics
                .push(SemanticCompositionDiagnostic::UnrepresentableBounds { source: id });
            return Vec::new();
        };
        if authored.role() == SemanticRole::ScrollBar && owner.scroll_control.is_none() {
            self.diagnostics
                .push(SemanticCompositionDiagnostic::MissingScrollControlBinding { source: id });
            return self.compose_items(owner_index, authored.children(), parent);
        }
        let editable = authored.editable().cloned().and_then(|editable| {
            let source = owner.editable_source.as_deref()?;
            let selection = owner.editable_selection?;
            let offsets = owner.editable_caret_offsets.clone()?;
            editable.__runtime_with_projection(source, selection, offsets)
        });
        let authored_editable = authored.editable().is_some();
        let (range, orientation) = semantic_range_and_orientation(authored, owner);
        let node = SemanticCandidateNode {
            id: id.clone(),
            parent: parent.cloned(),
            children: Vec::new(),
            role: authored.role(),
            name: authored.name().map(str::to_owned),
            description: authored.description().map(str::to_owned),
            value: if authored_editable {
                editable
                    .as_ref()
                    .and_then(SemanticEditable::value)
                    .map(|value| SemanticValue::Text(value.to_owned()))
            } else {
                authored.value().cloned()
            },
            disabled: authored.state().disabled() || !owner.activation.enabled(),
            inert: authored.state().inert(),
            read_only: authored.state().read_only()
                || authored.editable().is_some_and(SemanticEditable::read_only),
            checked: authored.state().checked(),
            pressed: authored.state().pressed(),
            selected: authored.state().selected(),
            expanded: authored.state().expanded(),
            required: authored.state().required(),
            invalid: authored.state().invalid(),
            modal: authored.state().modal(),
            supported_actions: supported_actions(authored, owner, editable.as_ref()),
            relationships: Vec::new(),
            bounds,
            text: (!authored_editable)
                .then(|| authored.text().cloned())
                .flatten(),
            editable,
            range,
            orientation,
            popup: authored.popup(),
            selection_mode: authored.selection_mode(),
            collection_position: authored.collection_position(),
            hierarchy_level: authored.hierarchy_level(),
            placeholder: authored.placeholder().map(str::to_owned),
            autocomplete: authored.autocomplete(),
            editable_mode: authored.editable_mode(),
        };
        if self
            .visible_ids
            .entry(owner_id.clone())
            .or_default()
            .insert(authored.key().clone(), id.clone())
            .is_some()
        {
            unreachable!("visible semantic owner keys are unique");
        }
        let draft_index = self.drafts.len();
        self.drafts.push(SemanticNodeDraft {
            owner: owner_id,
            authored_relationships: authored.relationships().to_vec(),
            node,
        });
        let children = self.compose_items(owner_index, authored.children(), Some(&id));
        self.drafts[draft_index].node.children = children;
        vec![id]
    }

    fn compose_mounted_children(
        &mut self,
        owner_index: usize,
        parent: Option<&SemanticNodeId>,
    ) -> Vec<SemanticNodeId> {
        let children = self.owners[owner_index].mounted_children.clone();
        let mut roots = Vec::new();
        for child in children {
            match self.owner_index(&child) {
                Some(child_index) => roots.extend(self.compose_owner(child_index, parent)),
                None => self
                    .diagnostics
                    .push(SemanticCompositionDiagnostic::MissingMountedOwner),
            }
        }
        roots
    }

    fn visible_id(&self, owner: &MountedNodeId, key: &SemanticKey) -> Option<&SemanticNodeId> {
        self.visible_ids
            .get(owner)
            .and_then(|visible| visible.get(key))
    }

    fn resolve_relationships(&mut self) {
        for index in 0..self.drafts.len() {
            let owner = self.drafts[index].owner.clone();
            let source = self.drafts[index].node.id.clone();
            let authored = self.drafts[index].authored_relationships.clone();
            let mut relationships = Vec::with_capacity(authored.len());
            for relationship in authored {
                let target = match relationship.target() {
                    SemanticReference::Local(key) => {
                        if let Some(target) = self.visible_id(&owner, key).cloned() {
                            Some(target)
                        } else {
                            self.diagnostics.push(
                                SemanticCompositionDiagnostic::MissingLocalRelationshipTarget {
                                    source: source.clone(),
                                    key: key.clone(),
                                },
                            );
                            None
                        }
                    }
                    SemanticReference::Authored {
                        element_id,
                        semantic_key,
                    } => self.resolve_authored_relationship_target(
                        &source,
                        element_id,
                        semantic_key.as_ref(),
                    ),
                };
                if let Some(target) = target {
                    relationships.push(ResolvedSemanticRelationship {
                        kind: relationship.kind(),
                        target,
                    });
                }
            }
            if self.drafts[index].node.role == SemanticRole::ScrollBar {
                match self
                    .owner_index(&owner)
                    .and_then(|owner_index| self.owners[owner_index].scroll_control.as_ref())
                {
                    Some(scroll_control) => {
                        if let Some(target) = self
                            .visible_id(&scroll_control.owner, &SemanticKey::PRIMARY)
                            .cloned()
                        {
                            relationships.push(ResolvedSemanticRelationship {
                                kind: SemanticRelationshipKind::Controls,
                                target,
                            });
                        } else {
                            self.diagnostics.push(
                                SemanticCompositionDiagnostic::MissingScrollControlTarget {
                                    source: source.clone(),
                                },
                            );
                        }
                    }
                    None => self.diagnostics.push(
                        SemanticCompositionDiagnostic::MissingScrollControlBinding {
                            source: source.clone(),
                        },
                    ),
                }
            }
            self.drafts[index].node.relationships = relationships;
        }
    }

    fn validate_active_descendants(&mut self) {
        let topology = self
            .drafts
            .iter()
            .map(|draft| {
                (
                    draft.node.id.clone(),
                    (draft.node.role, draft.node.parent.clone()),
                )
            })
            .collect::<HashMap<_, _>>();

        for index in 0..self.drafts.len() {
            let source = self.drafts[index].node.id.clone();
            let controlled_listboxes = self.drafts[index]
                .node
                .relationships
                .iter()
                .filter(|relationship| {
                    relationship.kind == SemanticRelationshipKind::Controls
                        && topology
                            .get(&relationship.target)
                            .is_some_and(|(role, _)| *role == SemanticRole::ListBox)
                })
                .map(|relationship| relationship.target.clone())
                .collect::<Vec<_>>();

            let relationships = core::mem::take(&mut self.drafts[index].node.relationships);
            let mut retained = Vec::with_capacity(relationships.len());
            for relationship in relationships {
                if relationship.kind != SemanticRelationshipKind::ActiveDescendant {
                    retained.push(relationship);
                    continue;
                }

                let Some((role, _)) = topology.get(&relationship.target) else {
                    unreachable!("resolved semantic relationship targets are current draft nodes");
                };
                if *role != SemanticRole::Option {
                    self.diagnostics.push(
                        SemanticCompositionDiagnostic::InvalidActiveDescendantTargetRole {
                            source: source.clone(),
                            target: relationship.target,
                            role: *role,
                        },
                    );
                    continue;
                }

                if !controlled_listboxes.iter().any(|controlled| {
                    semantic_is_strict_descendant(&relationship.target, controlled, &topology)
                }) {
                    self.diagnostics.push(
                        SemanticCompositionDiagnostic::ActiveDescendantOutsideControlledSubtree {
                            source: source.clone(),
                            target: relationship.target,
                        },
                    );
                    continue;
                }

                retained.push(relationship);
            }
            self.drafts[index].node.relationships = retained;
        }
    }

    fn resolve_authored_relationship_target(
        &mut self,
        source: &SemanticNodeId,
        element_id: &ElementId,
        semantic_key: Option<&SemanticKey>,
    ) -> Option<SemanticNodeId> {
        let owner_index = match self.authored_owner_indices.get(element_id) {
            None => {
                self.diagnostics.push(
                    SemanticCompositionDiagnostic::MissingAuthoredRelationshipOwner {
                        source: source.clone(),
                        element_id: element_id.clone(),
                    },
                );
                return None;
            }
            Some(AuthoredOwnerLookup::Unique(index)) => *index,
            Some(AuthoredOwnerLookup::Ambiguous) => {
                self.diagnostics.push(
                    SemanticCompositionDiagnostic::AmbiguousAuthoredRelationshipOwner {
                        source: source.clone(),
                        element_id: element_id.clone(),
                    },
                );
                return None;
            }
        };
        let target_owner = &self.owners[owner_index].id;
        let key = semantic_key.cloned().unwrap_or(SemanticKey::PRIMARY);
        if let Some(target) = self.visible_id(target_owner, &key).cloned() {
            Some(target)
        } else {
            self.diagnostics.push(
                SemanticCompositionDiagnostic::MissingAuthoredRelationshipTarget {
                    source: source.clone(),
                    element_id: element_id.clone(),
                    key,
                },
            );
            None
        }
    }
}

fn semantic_is_strict_descendant(
    target: &SemanticNodeId,
    ancestor: &SemanticNodeId,
    topology: &HashMap<SemanticNodeId, (SemanticRole, Option<SemanticNodeId>)>,
) -> bool {
    let mut current = target.clone();
    for _ in 0..topology.len() {
        let Some(parent) = topology
            .get(&current)
            .and_then(|(_, parent)| parent.as_ref())
        else {
            return false;
        };
        if parent == ancestor {
            return true;
        }
        current = parent.clone();
    }
    false
}

fn contains_semantic_node(items: &[SemanticItem]) -> bool {
    items
        .iter()
        .any(|item| matches!(item, SemanticItem::Node(_)))
}

fn semantic_range_and_orientation(
    authored: &SemanticNodeContribution,
    owner: &SemanticOwnerFacts,
) -> (Option<SemanticRange>, Option<SemanticOrientation>) {
    if authored.role() != SemanticRole::ScrollBar {
        return (authored.range().cloned(), authored.orientation());
    }
    let scroll_control = owner
        .scroll_control
        .as_ref()
        .unwrap_or_else(|| unreachable!("visible ScrollBar has a bound projection"));
    (
        Some(scrollbar_range(scroll_control.snapshot)),
        Some(scrollbar_orientation(scroll_control.snapshot)),
    )
}

fn supported_actions(
    authored: &SemanticNodeContribution,
    owner: &SemanticOwnerFacts,
    published_editable: Option<&SemanticEditable>,
) -> Vec<SemanticAction> {
    let editable = published_editable;
    let read_only =
        editable.is_some_and(SemanticEditable::read_only) || authored.state().read_only();
    authored
        .actions()
        .iter()
        .filter(|action| match action {
            SemanticAction::Activate => {
                !authored.key().is_primary() || owner.activation.is_actionable()
            }
            SemanticAction::RequestFocus => {
                authored.key().is_primary()
                    && match owner.focusability {
                        Focusability::Automatic => {
                            owner.activation.is_actionable()
                                && owner.scroll_control.as_ref().is_none_or(|projection| {
                                    projection.snapshot.maximum_offset().get() > 0.0
                                })
                        }
                        Focusability::Focusable | Focusability::FocusableWhenDisabled => true,
                        _ => false,
                    }
            }
            SemanticAction::OpenMenu | SemanticAction::OpenContextMenu => true,
            SemanticAction::MoveBackward
            | SemanticAction::MoveForward
            | SemanticAction::ExtendBackward
            | SemanticAction::ExtendForward
            | SemanticAction::SelectAll
            | SemanticAction::SetSelection => editable.is_some(),
            SemanticAction::DeleteBackward
            | SemanticAction::DeleteForward
            | SemanticAction::Undo
            | SemanticAction::Redo
            | SemanticAction::ReplaceSelection => editable.is_some() && !read_only,
            SemanticAction::Increment | SemanticAction::Decrement | SemanticAction::SetValue => {
                matches!(
                    authored.role(),
                    SemanticRole::Slider | SemanticRole::SpinButton | SemanticRole::Splitter
                ) && authored.range().is_some()
            }
            SemanticAction::Expand | SemanticAction::Collapse => {
                matches!(
                    authored.role(),
                    SemanticRole::Button
                        | SemanticRole::ComboBox
                        | SemanticRole::MenuItem
                        | SemanticRole::MenuItemCheckbox
                        | SemanticRole::MenuItemRadio
                        | SemanticRole::TreeItem
                ) && authored.state().expanded().is_some()
            }
            // Clipboard commands have routed editing defaults, but this semantic
            // projection does not advertise them as supported accessibility actions.
            _ => false,
        })
        .cloned()
        .chain(
            (authored.role() == SemanticRole::ScrollBar
                && owner
                    .scroll_control
                    .as_ref()
                    .is_some_and(|projection| projection.snapshot.maximum_offset().get() > 0.0))
            .then_some([
                SemanticAction::Increment,
                SemanticAction::Decrement,
                SemanticAction::SetValue,
            ])
            .into_iter()
            .flatten(),
        )
        .collect()
}

fn scrollbar_range(snapshot: ScrollControlSnapshot) -> SemanticRange {
    let minimum = SemanticNumber::new(0.0)
        .unwrap_or_else(|_| unreachable!("zero is a finite semantic number"));
    let maximum = SemanticNumber::new(100.0)
        .unwrap_or_else(|_| unreachable!("one hundred is a finite semantic number"));
    let current = SemanticNumber::new(f64::from(snapshot.normalized_position().get()) * 100.0)
        .unwrap_or_else(|_| unreachable!("normalized scroll percentage is finite"));
    SemanticRange::new(Some(minimum), Some(maximum), Some(current))
        .unwrap_or_else(|_| unreachable!("normalized scroll percentage is within 0..=100"))
}

const fn scrollbar_orientation(snapshot: ScrollControlSnapshot) -> SemanticOrientation {
    match snapshot.axis() {
        Axis::Horizontal => SemanticOrientation::Horizontal,
        Axis::Vertical => SemanticOrientation::Vertical,
    }
}

fn resolve_bounds(
    owner: LogicalRect,
    owner_to_surface: LogicalTransform,
    bounds: SemanticBounds,
) -> Option<LogicalRect> {
    match bounds {
        SemanticBounds::Owner => Some(owner),
        SemanticBounds::OwnerLocal(local) => transform_rect_aabb(owner_to_surface, local),
    }
}

#[cfg(test)]
mod tests;
