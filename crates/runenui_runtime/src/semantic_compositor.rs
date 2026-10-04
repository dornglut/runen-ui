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
            // Clipboard commands remain unadvertised until M10D supplies service authority.
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
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, Axis, ElementId, Focusability, LogicalPoint, LogicalRect,
        LogicalSize, LogicalTransform, ScrollControlSnapshot, SemanticAction, SemanticBounds,
        SemanticContribution, SemanticItem, SemanticKey, SemanticNodeContribution, SemanticNumber,
        SemanticOrientation, SemanticPopupKind, SemanticRange, SemanticReference,
        SemanticRelationship, SemanticRelationshipKind, SemanticRole, SemanticState,
        WidgetActivation,
    };

    use super::{
        ResolvedSemanticRelationship, SemanticCandidate, SemanticCompositionDiagnostic,
        SemanticOwnerFacts, SemanticScrollControlFacts, compose_semantics,
    };

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::new(
            LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite")),
            LogicalSize::try_new(width, height)
                .unwrap_or_else(|_| unreachable!("test size is valid")),
        )
    }

    fn scroll_projection(
        owner: runenui_core::MountedNodeId,
        axis: Axis,
        offset: f32,
        viewport: f32,
        content: f32,
    ) -> SemanticScrollControlFacts {
        SemanticScrollControlFacts {
            owner,
            snapshot: ScrollControlSnapshot::__runtime_from_metrics(
                axis, offset, viewport, content,
            )
            .unwrap_or_else(|| unreachable!("controlled scroll projection is valid")),
        }
    }

    fn key(value: &'static str) -> SemanticKey {
        SemanticKey::from_static(value).unwrap_or_else(|_| unreachable!("test key is valid"))
    }

    fn element_id(value: &'static str) -> ElementId {
        ElementId::from_static(value).unwrap_or_else(|_| unreachable!("test id is valid"))
    }

    fn compose(
        owners: &[SemanticOwnerFacts],
        root: Option<&runenui_core::MountedNodeId>,
        focused_owner: Option<&runenui_core::MountedNodeId>,
    ) -> SemanticCandidate {
        let transforms = owners
            .iter()
            .map(|owner| {
                LogicalTransform::translation(owner.bounds.x(), owner.bounds.y())
                    .unwrap_or_else(|_| unreachable!("test owner origin is finite"))
            })
            .collect::<Vec<_>>();
        compose_semantics(owners, &transforms, root, focused_owner)
    }

    fn semantic_owner(
        id: runenui_core::MountedNodeId,
        authored_id: Option<ElementId>,
        mounted_children: Vec<runenui_core::MountedNodeId>,
        contribution: SemanticContribution,
        bindings: Vec<(SemanticKey, runenui_core::SemanticNodeId)>,
        bounds: LogicalRect,
    ) -> SemanticOwnerFacts {
        SemanticOwnerFacts {
            id,
            authored_id,
            mounted_children,
            contribution,
            bindings,
            bounds,
            activation: WidgetActivation::NONE,
            focusability: Focusability::NotFocusable,
            scroll_control: None,
            editable_source: None,
            editable_selection: None,
            editable_caret_offsets: None,
        }
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn transparent_owner_and_marker_preserve_exact_semantic_order() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let control = runtime.__runtime_mounted_id(1, 1);
        let leaf = runtime.__runtime_mounted_id(2, 1);
        let control_primary = runtime.__runtime_semantic_id(0, 1);
        let before_id = runtime.__runtime_semantic_id(1, 1);
        let after_id = runtime.__runtime_semantic_id(2, 1);
        let leaf_primary = runtime.__runtime_semantic_id(3, 1);
        let before = key("before");
        let after = key("after");

        let control_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group).with_children(vec![
                SemanticItem::node(SemanticNodeContribution::new(
                    before.clone(),
                    SemanticRole::Text,
                )),
                SemanticItem::mounted_children(),
                SemanticItem::node(SemanticNodeContribution::new(
                    after.clone(),
                    SemanticRole::Text,
                )),
            ]),
        );
        let owners = vec![
            SemanticOwnerFacts {
                id: root.clone(),
                authored_id: None,
                mounted_children: vec![control.clone()],
                contribution: SemanticContribution::empty(),
                bindings: Vec::new(),
                bounds: rect(0.0, 0.0, 100.0, 100.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: control,
                authored_id: None,
                mounted_children: vec![leaf.clone()],
                contribution: control_contribution,
                bindings: vec![
                    (SemanticKey::PRIMARY, control_primary.clone()),
                    (before, before_id.clone()),
                    (after, after_id.clone()),
                ],
                bounds: rect(10.0, 20.0, 50.0, 40.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: leaf,
                authored_id: None,
                mounted_children: Vec::new(),
                contribution: SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::Text,
                )),
                bindings: vec![(SemanticKey::PRIMARY, leaf_primary.clone())],
                bounds: rect(12.0, 22.0, 10.0, 5.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
        ];

        let candidate = compose(&owners, Some(&root), None);
        assert!(candidate.diagnostics.is_empty());
        assert_eq!(candidate.roots, vec![control_primary.clone()]);
        assert_eq!(
            candidate.nodes[0].children,
            vec![before_id.clone(), leaf_primary.clone(), after_id.clone()]
        );
        assert_eq!(candidate.nodes[1].parent, Some(control_primary.clone()));
        assert_eq!(candidate.nodes[2].parent, Some(control_primary.clone()));
        assert_eq!(candidate.nodes[3].parent, Some(control_primary.clone()));
        assert_eq!(
            candidate
                .nodes
                .iter()
                .map(|node| node.id.clone())
                .collect::<Vec<_>>(),
            vec![control_primary, before_id, leaf_primary, after_id]
        );
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn hidden_semantic_subtree_also_hides_spliced_mounted_children() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(0, 1);
        let child = runtime.__runtime_mounted_id(1, 1);
        let hidden_id = runtime.__runtime_semantic_id(0, 1);
        let child_id = runtime.__runtime_semantic_id(1, 1);
        let contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group)
                .with_state(SemanticState::ENABLED.with_hidden(true))
                .with_mounted_children(),
        );
        let owners = vec![
            SemanticOwnerFacts {
                id: owner.clone(),
                authored_id: None,
                mounted_children: vec![child.clone()],
                contribution,
                bindings: vec![(SemanticKey::PRIMARY, hidden_id)],
                bounds: rect(0.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: child,
                authored_id: None,
                mounted_children: Vec::new(),
                contribution: SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::Text,
                )),
                bindings: vec![(SemanticKey::PRIMARY, child_id)],
                bounds: rect(1.0, 1.0, 5.0, 5.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
        ];

        let candidate = compose(&owners, Some(&owner), Some(&owner));
        assert!(candidate.roots.is_empty());
        assert!(candidate.nodes.is_empty());
        assert!(candidate.focused.is_none());
        assert_eq!(
            candidate.diagnostics,
            vec![SemanticCompositionDiagnostic::FocusedOwnerMissingVisiblePrimary]
        );
    }

    #[test]
    fn support_state_and_owner_local_bounds_are_composed_without_availability_guessing() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(0, 1);
        let primary_id = runtime.__runtime_semantic_id(0, 1);
        let virtual_id = runtime.__runtime_semantic_id(1, 1);
        let virtual_key = key("virtual");
        let contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Button)
                .with_action(SemanticAction::Activate)
                .with_action(SemanticAction::RequestFocus)
                .with_action(SemanticAction::OpenMenu)
                .with_bounds(SemanticBounds::OwnerLocal(rect(2.0, 3.0, 4.0, 5.0)))
                .with_child(
                    SemanticNodeContribution::new(virtual_key.clone(), SemanticRole::Button)
                        .with_state(SemanticState::ENABLED.with_disabled(true))
                        .with_action(SemanticAction::Activate),
                ),
        );
        let owners = vec![SemanticOwnerFacts {
            id: owner.clone(),
            authored_id: None,
            mounted_children: Vec::new(),
            contribution,
            bindings: vec![
                (SemanticKey::PRIMARY, primary_id.clone()),
                (virtual_key, virtual_id.clone()),
            ],
            bounds: rect(10.0, 20.0, 30.0, 40.0),
            activation: WidgetActivation::disabled(),
            focusability: Focusability::Focusable,
            scroll_control: None,
            editable_source: None,
            editable_selection: None,
            editable_caret_offsets: None,
        }];

        let candidate = compose(&owners, Some(&owner), None);
        let primary = &candidate.nodes[0];
        let virtual_node = &candidate.nodes[1];
        assert!(primary.disabled);
        assert_eq!(
            primary.supported_actions,
            vec![SemanticAction::RequestFocus, SemanticAction::OpenMenu]
        );
        assert_eq!(primary.bounds, rect(12.0, 23.0, 4.0, 5.0));
        assert!(virtual_node.disabled);
        assert_eq!(
            virtual_node.supported_actions,
            vec![SemanticAction::Activate]
        );
        assert_eq!(virtual_node.id, virtual_id);
        assert_eq!(primary.id, primary_id);
    }

    #[test]
    fn owner_local_bounds_follow_full_presentation_affine_before_aabb_projection() {
        let runtime = RuntimeNamespace::__runtime_new();
        let owner = runtime.__runtime_mounted_id(0, 1);
        let primary_id = runtime.__runtime_semantic_id(0, 1);
        let owners = vec![SemanticOwnerFacts {
            id: owner.clone(),
            authored_id: None,
            mounted_children: Vec::new(),
            contribution: SemanticContribution::single(
                SemanticNodeContribution::primary(SemanticRole::Button)
                    .with_bounds(SemanticBounds::OwnerLocal(rect(1.0, 2.0, 3.0, 4.0))),
            ),
            bindings: vec![(SemanticKey::PRIMARY, primary_id)],
            bounds: rect(4.0, 10.0, 8.0, 12.0),
            activation: WidgetActivation::NONE,
            focusability: Focusability::NotFocusable,
            scroll_control: None,
            editable_source: None,
            editable_selection: None,
            editable_caret_offsets: None,
        }];
        let transform = LogicalTransform::try_new(0.0, 2.0, -1.0, 0.0, 10.0, 10.0)
            .unwrap_or_else(|_| unreachable!("test affine is finite"));

        let candidate = compose_semantics(&owners, &[transform], Some(&owner), None);
        assert_eq!(candidate.nodes[0].bounds, rect(4.0, 12.0, 4.0, 6.0));
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn relationships_and_focus_use_exact_visible_targets_without_fallback() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let source_owner = runtime.__runtime_mounted_id(1, 1);
        let target_owner = runtime.__runtime_mounted_id(2, 1);
        let source_primary = runtime.__runtime_semantic_id(0, 1);
        let source_named = runtime.__runtime_semantic_id(1, 1);
        let target_primary = runtime.__runtime_semantic_id(2, 1);
        let named_key = key("named");
        let target_element = element_id("target");
        let source_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::LabelledBy,
                    SemanticReference::Local(named_key.clone()),
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: target_element.clone(),
                        semantic_key: None,
                    },
                ))
                .with_child(SemanticNodeContribution::new(
                    named_key.clone(),
                    SemanticRole::Text,
                )),
        );
        let owners = vec![
            SemanticOwnerFacts {
                id: root.clone(),
                authored_id: None,
                mounted_children: vec![source_owner.clone(), target_owner.clone()],
                contribution: SemanticContribution::empty(),
                bindings: Vec::new(),
                bounds: rect(0.0, 0.0, 100.0, 100.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: source_owner.clone(),
                authored_id: Some(element_id("source")),
                mounted_children: Vec::new(),
                contribution: source_contribution,
                bindings: vec![
                    (SemanticKey::PRIMARY, source_primary.clone()),
                    (named_key, source_named.clone()),
                ],
                bounds: rect(0.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::Focusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: target_owner,
                authored_id: Some(target_element),
                mounted_children: Vec::new(),
                contribution: SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::Button,
                )),
                bindings: vec![(SemanticKey::PRIMARY, target_primary.clone())],
                bounds: rect(30.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::Focusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
        ];

        let candidate = compose(&owners, Some(&root), Some(&source_owner));
        assert!(candidate.diagnostics.is_empty());
        assert_eq!(candidate.focused, Some(source_primary));
        assert_eq!(candidate.nodes[0].relationships.len(), 2);
        assert_eq!(candidate.nodes[0].relationships[0].target, source_named);
        assert_eq!(candidate.nodes[0].relationships[1].target, target_primary);
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn owner_local_visible_index_keeps_identical_keys_isolated_across_owners() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let source_owner = runtime.__runtime_mounted_id(1, 1);
        let target_owner = runtime.__runtime_mounted_id(2, 1);
        let source_primary = runtime.__runtime_semantic_id(0, 1);
        let source_shared = runtime.__runtime_semantic_id(1, 1);
        let target_primary = runtime.__runtime_semantic_id(2, 1);
        let target_shared = runtime.__runtime_semantic_id(3, 1);
        let shared = key("shared");
        let target_element = element_id("target");
        let source_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::LabelledBy,
                    SemanticReference::Local(shared.clone()),
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: target_element.clone(),
                        semantic_key: Some(shared.clone()),
                    },
                ))
                .with_child(SemanticNodeContribution::new(
                    shared.clone(),
                    SemanticRole::Text,
                )),
        );
        let target_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group).with_child(
                SemanticNodeContribution::new(shared.clone(), SemanticRole::Text),
            ),
        );
        let owners = vec![
            SemanticOwnerFacts {
                id: root.clone(),
                authored_id: None,
                mounted_children: vec![source_owner.clone(), target_owner.clone()],
                contribution: SemanticContribution::empty(),
                bindings: Vec::new(),
                bounds: rect(0.0, 0.0, 100.0, 100.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: source_owner,
                authored_id: Some(element_id("source")),
                mounted_children: Vec::new(),
                contribution: source_contribution,
                bindings: vec![
                    (SemanticKey::PRIMARY, source_primary),
                    (shared.clone(), source_shared.clone()),
                ],
                bounds: rect(0.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: target_owner,
                authored_id: Some(target_element),
                mounted_children: Vec::new(),
                contribution: target_contribution,
                bindings: vec![
                    (SemanticKey::PRIMARY, target_primary),
                    (shared, target_shared.clone()),
                ],
                bounds: rect(30.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
        ];

        let candidate = compose(&owners, Some(&root), None);
        assert!(candidate.diagnostics.is_empty());
        assert_eq!(candidate.nodes[0].relationships.len(), 2);
        assert_eq!(candidate.nodes[0].relationships[0].target, source_shared);
        assert_eq!(candidate.nodes[0].relationships[1].target, target_shared);
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn active_descendant_requires_option_inside_controlled_listbox() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let combo = runtime.__runtime_mounted_id(1, 1);
        let popup = runtime.__runtime_mounted_id(2, 1);
        let combo_id = runtime.__runtime_semantic_id(0, 1);
        let listbox_id = runtime.__runtime_semantic_id(1, 1);
        let option_id = runtime.__runtime_semantic_id(2, 1);
        let option_key = key("active-option");
        let popup_element = element_id("popup");

        let combo_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ComboBox)
                .with_popup(SemanticPopupKind::ListBox)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: popup_element.clone(),
                        semantic_key: None,
                    },
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::ActiveDescendant,
                    SemanticReference::Authored {
                        element_id: popup_element.clone(),
                        semantic_key: Some(option_key.clone()),
                    },
                )),
        );
        let popup_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ListBox).with_child(
                SemanticNodeContribution::new(option_key.clone(), SemanticRole::Option),
            ),
        );
        let owners = vec![
            SemanticOwnerFacts {
                id: root.clone(),
                authored_id: None,
                mounted_children: vec![combo.clone(), popup.clone()],
                contribution: SemanticContribution::empty(),
                bindings: Vec::new(),
                bounds: rect(0.0, 0.0, 100.0, 100.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: combo,
                authored_id: Some(element_id("combo")),
                mounted_children: Vec::new(),
                contribution: combo_contribution,
                bindings: vec![(SemanticKey::PRIMARY, combo_id.clone())],
                bounds: rect(0.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::Focusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: popup,
                authored_id: Some(popup_element),
                mounted_children: Vec::new(),
                contribution: popup_contribution,
                bindings: vec![
                    (SemanticKey::PRIMARY, listbox_id.clone()),
                    (option_key, option_id.clone()),
                ],
                bounds: rect(0.0, 30.0, 60.0, 60.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
        ];

        let candidate = compose(&owners, Some(&root), None);
        assert!(candidate.diagnostics.is_empty());
        assert_eq!(candidate.nodes[0].id, combo_id);
        assert_eq!(candidate.nodes[0].relationships.len(), 2);
        assert_eq!(
            candidate.nodes[0].relationships[0],
            ResolvedSemanticRelationship {
                kind: SemanticRelationshipKind::Controls,
                target: listbox_id,
            }
        );
        assert_eq!(
            candidate.nodes[0].relationships[1],
            ResolvedSemanticRelationship {
                kind: SemanticRelationshipKind::ActiveDescendant,
                target: option_id,
            }
        );
    }

    #[test]
    fn scrollbar_semantics_are_derived_from_exact_bound_snapshot_and_owner() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let viewport = runtime.__runtime_mounted_id(1, 1);
        let scrollbar_owner_id = runtime.__runtime_mounted_id(2, 1);
        let viewport_id = runtime.__runtime_semantic_id(0, 1);
        let scrollbar_id = runtime.__runtime_semantic_id(1, 1);

        let root_owner = semantic_owner(
            root.clone(),
            None,
            vec![viewport.clone(), scrollbar_owner_id.clone()],
            SemanticContribution::empty(),
            Vec::new(),
            rect(0.0, 0.0, 100.0, 100.0),
        );
        let viewport_owner = semantic_owner(
            viewport.clone(),
            None,
            Vec::new(),
            SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Group)),
            vec![(SemanticKey::PRIMARY, viewport_id.clone())],
            rect(0.0, 0.0, 80.0, 100.0),
        );
        let mut scrollbar_owner = semantic_owner(
            scrollbar_owner_id,
            None,
            Vec::new(),
            SemanticContribution::single(SemanticNodeContribution::primary(
                SemanticRole::ScrollBar,
            )),
            vec![(SemanticKey::PRIMARY, scrollbar_id.clone())],
            rect(80.0, 0.0, 20.0, 100.0),
        );
        scrollbar_owner.scroll_control = Some(scroll_projection(
            viewport,
            Axis::Vertical,
            15.0,
            40.0,
            100.0,
        ));

        let candidate = compose(
            &[root_owner, viewport_owner, scrollbar_owner],
            Some(&root),
            None,
        );
        assert_eq!(candidate.diagnostics, Vec::new());
        let scrollbar = candidate
            .nodes
            .iter()
            .find(|node| node.id == scrollbar_id)
            .unwrap_or_else(|| unreachable!("scrollbar semantic node is published"));
        assert_eq!(scrollbar.orientation, Some(SemanticOrientation::Vertical));
        let range = scrollbar
            .range
            .as_ref()
            .unwrap_or_else(|| unreachable!("runtime publishes scrollbar range"));
        assert_eq!(range.minimum().map(SemanticNumber::get), Some(0.0));
        assert_eq!(range.maximum().map(SemanticNumber::get), Some(100.0));
        assert_eq!(range.current().map(SemanticNumber::get), Some(25.0));
        assert_eq!(
            scrollbar.supported_actions,
            vec![
                SemanticAction::Increment,
                SemanticAction::Decrement,
                SemanticAction::SetValue,
            ]
        );
        assert_eq!(
            scrollbar.relationships,
            vec![ResolvedSemanticRelationship {
                kind: SemanticRelationshipKind::Controls,
                target: viewport_id,
            }]
        );
    }

    #[test]
    fn authored_scrollbar_mutable_range_actions_never_override_runtime_scrollability() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let viewport = runtime.__runtime_mounted_id(1, 1);
        let scrollbar_owner_id = runtime.__runtime_mounted_id(2, 1);
        let viewport_id = runtime.__runtime_semantic_id(0, 1);
        let scrollbar_id = runtime.__runtime_semantic_id(1, 1);

        let root_owner = semantic_owner(
            root.clone(),
            None,
            vec![viewport.clone(), scrollbar_owner_id.clone()],
            SemanticContribution::empty(),
            Vec::new(),
            rect(0.0, 0.0, 100.0, 100.0),
        );
        let viewport_owner = semantic_owner(
            viewport.clone(),
            None,
            Vec::new(),
            SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Group)),
            vec![(SemanticKey::PRIMARY, viewport_id)],
            rect(0.0, 0.0, 80.0, 100.0),
        );
        let mut scrollbar_owner = semantic_owner(
            scrollbar_owner_id,
            None,
            Vec::new(),
            SemanticContribution::single(
                SemanticNodeContribution::primary(SemanticRole::ScrollBar)
                    .with_action(SemanticAction::Increment)
                    .with_action(SemanticAction::Decrement)
                    .with_action(SemanticAction::SetValue),
            ),
            vec![(SemanticKey::PRIMARY, scrollbar_id.clone())],
            rect(80.0, 0.0, 20.0, 100.0),
        );
        scrollbar_owner.scroll_control = Some(scroll_projection(
            viewport,
            Axis::Vertical,
            0.0,
            100.0,
            100.0,
        ));

        let candidate = compose(
            &[root_owner, viewport_owner, scrollbar_owner],
            Some(&root),
            None,
        );
        let scrollbar = candidate
            .nodes
            .iter()
            .find(|node| node.id == scrollbar_id)
            .unwrap_or_else(|| unreachable!("zero-range scrollbar remains published"));

        assert_eq!(
            scrollbar.supported_actions,
            Vec::new(),
            "authored mutable-range actions cannot recreate action authority when the runtime-derived scroll range is zero"
        );
    }

    #[test]
    fn non_scrollable_scrollbar_withholds_runtime_mutable_actions() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let viewport = runtime.__runtime_mounted_id(1, 1);
        let scrollbar_owner_id = runtime.__runtime_mounted_id(2, 1);
        let viewport_id = runtime.__runtime_semantic_id(0, 1);
        let scrollbar_id = runtime.__runtime_semantic_id(1, 1);

        let root_owner = semantic_owner(
            root.clone(),
            None,
            vec![viewport.clone(), scrollbar_owner_id.clone()],
            SemanticContribution::empty(),
            Vec::new(),
            rect(0.0, 0.0, 100.0, 100.0),
        );
        let viewport_owner = semantic_owner(
            viewport.clone(),
            None,
            Vec::new(),
            SemanticContribution::single(SemanticNodeContribution::primary(SemanticRole::Group)),
            vec![(SemanticKey::PRIMARY, viewport_id)],
            rect(0.0, 0.0, 80.0, 100.0),
        );
        let mut scrollbar_owner = semantic_owner(
            scrollbar_owner_id,
            None,
            Vec::new(),
            SemanticContribution::single(SemanticNodeContribution::primary(
                SemanticRole::ScrollBar,
            )),
            vec![(SemanticKey::PRIMARY, scrollbar_id.clone())],
            rect(80.0, 0.0, 20.0, 100.0),
        );
        scrollbar_owner.scroll_control = Some(scroll_projection(
            viewport,
            Axis::Vertical,
            0.0,
            100.0,
            100.0,
        ));

        let candidate = compose(
            &[root_owner, viewport_owner, scrollbar_owner],
            Some(&root),
            None,
        );
        let scrollbar = candidate
            .nodes
            .iter()
            .find(|node| node.id == scrollbar_id)
            .unwrap_or_else(|| unreachable!("Always scrollbar semantics remain representable"));
        assert_eq!(scrollbar.supported_actions, Vec::new());
        assert_eq!(
            scrollbar
                .range
                .as_ref()
                .and_then(SemanticRange::current)
                .map(SemanticNumber::get),
            Some(0.0)
        );
    }

    #[test]
    fn scrollbar_without_bound_projection_is_diagnosed_and_withheld() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let scrollbar_owner_id = runtime.__runtime_mounted_id(1, 1);
        let scrollbar_id = runtime.__runtime_semantic_id(0, 1);

        let root_owner = semantic_owner(
            root.clone(),
            None,
            vec![scrollbar_owner_id.clone()],
            SemanticContribution::empty(),
            Vec::new(),
            rect(0.0, 0.0, 100.0, 100.0),
        );
        let scrollbar_owner = semantic_owner(
            scrollbar_owner_id,
            None,
            Vec::new(),
            SemanticContribution::single(SemanticNodeContribution::primary(
                SemanticRole::ScrollBar,
            )),
            vec![(SemanticKey::PRIMARY, scrollbar_id.clone())],
            rect(80.0, 0.0, 20.0, 100.0),
        );

        let candidate = compose(&[root_owner, scrollbar_owner], Some(&root), None);
        assert_eq!(candidate.nodes, Vec::new());
        assert_eq!(
            candidate.diagnostics,
            vec![SemanticCompositionDiagnostic::MissingScrollControlBinding {
                source: scrollbar_id,
            }]
        );
    }

    #[test]
    fn scrollbar_missing_bound_primary_is_diagnosed_and_controls_are_withheld() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let viewport = runtime.__runtime_mounted_id(1, 1);
        let scrollbar_owner_id = runtime.__runtime_mounted_id(2, 1);
        let scrollbar_id = runtime.__runtime_semantic_id(0, 1);

        let root_owner = semantic_owner(
            root.clone(),
            None,
            vec![viewport.clone(), scrollbar_owner_id.clone()],
            SemanticContribution::empty(),
            Vec::new(),
            rect(0.0, 0.0, 100.0, 100.0),
        );
        let viewport_owner = semantic_owner(
            viewport.clone(),
            None,
            Vec::new(),
            SemanticContribution::empty(),
            Vec::new(),
            rect(0.0, 0.0, 80.0, 100.0),
        );
        let mut scrollbar_owner = semantic_owner(
            scrollbar_owner_id,
            None,
            Vec::new(),
            SemanticContribution::single(SemanticNodeContribution::primary(
                SemanticRole::ScrollBar,
            )),
            vec![(SemanticKey::PRIMARY, scrollbar_id.clone())],
            rect(80.0, 0.0, 20.0, 100.0),
        );
        scrollbar_owner.scroll_control = Some(scroll_projection(
            viewport,
            Axis::Vertical,
            15.0,
            40.0,
            100.0,
        ));

        let candidate = compose(
            &[root_owner, viewport_owner, scrollbar_owner],
            Some(&root),
            None,
        );
        let scrollbar = candidate
            .nodes
            .iter()
            .find(|node| node.id == scrollbar_id)
            .unwrap_or_else(|| unreachable!("bound scrollbar semantics remain published"));
        assert_eq!(scrollbar.relationships, Vec::new());
        assert!(scrollbar.range.is_some());
        assert_eq!(
            candidate.diagnostics,
            vec![SemanticCompositionDiagnostic::MissingScrollControlTarget {
                source: scrollbar_id,
            }]
        );
    }

    #[test]
    fn active_descendant_missing_current_target_is_diagnosed_and_withheld() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let combo = runtime.__runtime_mounted_id(1, 1);
        let popup = runtime.__runtime_mounted_id(2, 1);
        let combo_id = runtime.__runtime_semantic_id(0, 1);
        let listbox_id = runtime.__runtime_semantic_id(1, 1);
        let popup_element = element_id("popup");
        let missing_option = element_id("removed-option");

        let combo_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ComboBox)
                .with_popup(SemanticPopupKind::ListBox)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: popup_element.clone(),
                        semantic_key: None,
                    },
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::ActiveDescendant,
                    SemanticReference::Authored {
                        element_id: missing_option.clone(),
                        semantic_key: None,
                    },
                )),
        );
        let mut combo_owner = semantic_owner(
            combo,
            Some(element_id("combo")),
            Vec::new(),
            combo_contribution,
            vec![(SemanticKey::PRIMARY, combo_id.clone())],
            rect(0.0, 0.0, 20.0, 20.0),
        );
        combo_owner.focusability = Focusability::Focusable;
        let owners = vec![
            semantic_owner(
                root.clone(),
                None,
                vec![combo_owner.id.clone(), popup.clone()],
                SemanticContribution::empty(),
                Vec::new(),
                rect(0.0, 0.0, 100.0, 100.0),
            ),
            combo_owner,
            semantic_owner(
                popup,
                Some(popup_element),
                Vec::new(),
                SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::ListBox,
                )),
                vec![(SemanticKey::PRIMARY, listbox_id.clone())],
                rect(0.0, 30.0, 60.0, 60.0),
            ),
        ];

        let candidate = compose(&owners, Some(&root), None);
        assert_eq!(
            candidate.nodes[0].relationships,
            vec![ResolvedSemanticRelationship {
                kind: SemanticRelationshipKind::Controls,
                target: listbox_id,
            }]
        );
        assert_eq!(
            candidate.diagnostics,
            vec![
                SemanticCompositionDiagnostic::MissingAuthoredRelationshipOwner {
                    source: combo_id,
                    element_id: missing_option,
                }
            ]
        );
    }

    #[test]
    fn active_descendant_wrong_role_is_diagnosed_and_withheld() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let combo = runtime.__runtime_mounted_id(1, 1);
        let popup = runtime.__runtime_mounted_id(2, 1);
        let combo_id = runtime.__runtime_semantic_id(0, 1);
        let listbox_id = runtime.__runtime_semantic_id(1, 1);
        let target_id = runtime.__runtime_semantic_id(2, 1);
        let target_key = key("not-an-option");
        let popup_element = element_id("popup");

        let combo_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ComboBox)
                .with_popup(SemanticPopupKind::ListBox)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: popup_element.clone(),
                        semantic_key: None,
                    },
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::ActiveDescendant,
                    SemanticReference::Authored {
                        element_id: popup_element.clone(),
                        semantic_key: Some(target_key.clone()),
                    },
                )),
        );
        let popup_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ListBox).with_child(
                SemanticNodeContribution::new(target_key.clone(), SemanticRole::Text),
            ),
        );
        let owners = vec![
            SemanticOwnerFacts {
                id: root.clone(),
                authored_id: None,
                mounted_children: vec![combo.clone(), popup.clone()],
                contribution: SemanticContribution::empty(),
                bindings: Vec::new(),
                bounds: rect(0.0, 0.0, 100.0, 100.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: combo,
                authored_id: Some(element_id("combo")),
                mounted_children: Vec::new(),
                contribution: combo_contribution,
                bindings: vec![(SemanticKey::PRIMARY, combo_id.clone())],
                bounds: rect(0.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::Focusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: popup,
                authored_id: Some(popup_element),
                mounted_children: Vec::new(),
                contribution: popup_contribution,
                bindings: vec![
                    (SemanticKey::PRIMARY, listbox_id.clone()),
                    (target_key, target_id.clone()),
                ],
                bounds: rect(0.0, 30.0, 60.0, 60.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
        ];

        let candidate = compose(&owners, Some(&root), None);
        assert_eq!(candidate.nodes[0].relationships.len(), 1);
        assert_eq!(
            candidate.nodes[0].relationships[0],
            ResolvedSemanticRelationship {
                kind: SemanticRelationshipKind::Controls,
                target: listbox_id,
            }
        );
        assert_eq!(
            candidate.diagnostics,
            vec![
                SemanticCompositionDiagnostic::InvalidActiveDescendantTargetRole {
                    source: combo_id,
                    target: target_id,
                    role: SemanticRole::Text,
                }
            ]
        );
    }

    #[test]
    fn active_descendant_outside_controlled_listbox_is_diagnosed_and_withheld() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let combo = runtime.__runtime_mounted_id(1, 1);
        let popup = runtime.__runtime_mounted_id(2, 1);
        let outside = runtime.__runtime_mounted_id(3, 1);
        let combo_id = runtime.__runtime_semantic_id(0, 1);
        let listbox_id = runtime.__runtime_semantic_id(1, 1);
        let outside_option_id = runtime.__runtime_semantic_id(2, 1);
        let popup_element = element_id("popup");
        let outside_element = element_id("outside-option");

        let combo_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ComboBox)
                .with_popup(SemanticPopupKind::ListBox)
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: popup_element.clone(),
                        semantic_key: None,
                    },
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::ActiveDescendant,
                    SemanticReference::Authored {
                        element_id: outside_element.clone(),
                        semantic_key: None,
                    },
                )),
        );
        let mut combo_owner = semantic_owner(
            combo,
            Some(element_id("combo")),
            Vec::new(),
            combo_contribution,
            vec![(SemanticKey::PRIMARY, combo_id.clone())],
            rect(0.0, 0.0, 20.0, 20.0),
        );
        combo_owner.focusability = Focusability::Focusable;
        let owners = vec![
            semantic_owner(
                root.clone(),
                None,
                vec![combo_owner.id.clone(), popup.clone(), outside.clone()],
                SemanticContribution::empty(),
                Vec::new(),
                rect(0.0, 0.0, 100.0, 100.0),
            ),
            combo_owner,
            semantic_owner(
                popup,
                Some(popup_element),
                Vec::new(),
                SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::ListBox,
                )),
                vec![(SemanticKey::PRIMARY, listbox_id.clone())],
                rect(0.0, 30.0, 60.0, 60.0),
            ),
            semantic_owner(
                outside,
                Some(outside_element),
                Vec::new(),
                SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::Option,
                )),
                vec![(SemanticKey::PRIMARY, outside_option_id.clone())],
                rect(70.0, 30.0, 20.0, 20.0),
            ),
        ];

        let candidate = compose(&owners, Some(&root), None);
        assert_eq!(candidate.nodes[0].relationships.len(), 1);
        assert_eq!(
            candidate.nodes[0].relationships[0],
            ResolvedSemanticRelationship {
                kind: SemanticRelationshipKind::Controls,
                target: listbox_id,
            }
        );
        assert_eq!(
            candidate.diagnostics,
            vec![
                SemanticCompositionDiagnostic::ActiveDescendantOutsideControlledSubtree {
                    source: combo_id,
                    target: outside_option_id,
                }
            ]
        );
    }

    #[test]
    #[allow(clippy::assert_is_empty)]
    fn authored_owner_index_rejects_ambiguity_without_first_or_last_fallback() {
        let runtime = RuntimeNamespace::__runtime_new();
        let root = runtime.__runtime_mounted_id(0, 1);
        let source_owner = runtime.__runtime_mounted_id(1, 1);
        let first_target = runtime.__runtime_mounted_id(2, 1);
        let second_target = runtime.__runtime_mounted_id(3, 1);
        let source_primary = runtime.__runtime_semantic_id(0, 1);
        let first_primary = runtime.__runtime_semantic_id(1, 1);
        let second_primary = runtime.__runtime_semantic_id(2, 1);
        let ambiguous_element = element_id("duplicate-target");
        let source_contribution = SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::Group).with_relationship(
                SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: ambiguous_element.clone(),
                        semantic_key: None,
                    },
                ),
            ),
        );
        let owners = vec![
            SemanticOwnerFacts {
                id: root.clone(),
                authored_id: None,
                mounted_children: vec![
                    source_owner.clone(),
                    first_target.clone(),
                    second_target.clone(),
                ],
                contribution: SemanticContribution::empty(),
                bindings: Vec::new(),
                bounds: rect(0.0, 0.0, 100.0, 100.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: source_owner,
                authored_id: Some(element_id("source")),
                mounted_children: Vec::new(),
                contribution: source_contribution,
                bindings: vec![(SemanticKey::PRIMARY, source_primary.clone())],
                bounds: rect(0.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: first_target,
                authored_id: Some(ambiguous_element.clone()),
                mounted_children: Vec::new(),
                contribution: SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::Button,
                )),
                bindings: vec![(SemanticKey::PRIMARY, first_primary)],
                bounds: rect(30.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
            SemanticOwnerFacts {
                id: second_target,
                authored_id: Some(ambiguous_element.clone()),
                mounted_children: Vec::new(),
                contribution: SemanticContribution::single(SemanticNodeContribution::primary(
                    SemanticRole::Button,
                )),
                bindings: vec![(SemanticKey::PRIMARY, second_primary)],
                bounds: rect(60.0, 0.0, 20.0, 20.0),
                activation: WidgetActivation::NONE,
                focusability: Focusability::NotFocusable,
                scroll_control: None,
                editable_source: None,
                editable_selection: None,
                editable_caret_offsets: None,
            },
        ];

        let candidate = compose(&owners, Some(&root), None);
        assert!(candidate.nodes[0].relationships.is_empty());
        assert_eq!(
            candidate.diagnostics,
            vec![
                SemanticCompositionDiagnostic::AmbiguousAuthoredRelationshipOwner {
                    source: source_primary,
                    element_id: ambiguous_element,
                }
            ]
        );
    }
}
