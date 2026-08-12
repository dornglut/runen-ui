use runenui_core::{
    ElementId, Focusability, LogicalPoint, LogicalRect, MountedNodeId, SemanticAction,
    SemanticBounds, SemanticContribution, SemanticItem, SemanticKey, SemanticNodeContribution,
    SemanticReference, SemanticRelationshipKind, SemanticRole, SemanticText, SemanticValue,
    WidgetActivation,
};

use crate::SemanticNodeId;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SemanticOwnerFacts {
    pub(crate) id: MountedNodeId,
    pub(crate) authored_id: Option<ElementId>,
    pub(crate) mounted_children: Vec<MountedNodeId>,
    pub(crate) contribution: SemanticContribution,
    pub(crate) bindings: Vec<(SemanticKey, SemanticNodeId)>,
    pub(crate) bounds: LogicalRect,
    pub(crate) activation: WidgetActivation,
    pub(crate) focusability: Focusability,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResolvedSemanticRelationship {
    pub(crate) kind: SemanticRelationshipKind,
    pub(crate) target: SemanticNodeId,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SemanticCandidateNode {
    pub(crate) id: SemanticNodeId,
    pub(crate) parent: Option<SemanticNodeId>,
    pub(crate) children: Vec<SemanticNodeId>,
    pub(crate) role: SemanticRole,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) value: Option<SemanticValue>,
    pub(crate) disabled: bool,
    pub(crate) inert: bool,
    pub(crate) supported_actions: Vec<SemanticAction>,
    pub(crate) relationships: Vec<ResolvedSemanticRelationship>,
    pub(crate) bounds: LogicalRect,
    pub(crate) text: Option<SemanticText>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SemanticCompositionDiagnostic {
    MissingOwnerBinding { key: SemanticKey },
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
    FocusedOwnerMissingVisiblePrimary,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SemanticCandidate {
    pub(crate) roots: Vec<SemanticNodeId>,
    pub(crate) nodes: Vec<SemanticCandidateNode>,
    pub(crate) focused: Option<SemanticNodeId>,
    pub(crate) diagnostics: Vec<SemanticCompositionDiagnostic>,
}

impl SemanticCandidate {
    pub(crate) fn adapter_visible_eq(&self, other: &Self) -> bool {
        self.roots == other.roots && self.nodes == other.nodes && self.focused == other.focused
    }
}

pub(crate) fn compose_semantics(
    owners: &[SemanticOwnerFacts],
    root: Option<&MountedNodeId>,
    focused_owner: Option<&MountedNodeId>,
) -> SemanticCandidate {
    let mut compositor = SemanticCompositor {
        owners,
        drafts: Vec::new(),
        diagnostics: Vec::new(),
    };
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
    drafts: Vec<SemanticNodeDraft>,
    diagnostics: Vec<SemanticCompositionDiagnostic>,
}

struct SemanticNodeDraft {
    owner: MountedNodeId,
    key: SemanticKey,
    authored_relationships: Vec<runenui_core::SemanticRelationship>,
    node: SemanticCandidateNode,
}

impl SemanticCompositor<'_> {
    fn owner_index(&self, id: &MountedNodeId) -> Option<usize> {
        self.owners.iter().position(|owner| &owner.id == id)
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
        let Some(id) = owner
            .bindings
            .iter()
            .find(|(key, _)| key == authored.key())
            .map(|(_, id)| id.clone())
        else {
            self.diagnostics
                .push(SemanticCompositionDiagnostic::MissingOwnerBinding {
                    key: authored.key().clone(),
                });
            return Vec::new();
        };

        let draft_index = self.drafts.len();
        self.drafts.push(SemanticNodeDraft {
            owner: owner.id.clone(),
            key: authored.key().clone(),
            authored_relationships: authored.relationships().to_vec(),
            node: SemanticCandidateNode {
                id: id.clone(),
                parent: parent.cloned(),
                children: Vec::new(),
                role: authored.role(),
                name: authored.name().map(str::to_owned),
                description: authored.description().map(str::to_owned),
                value: authored.value().cloned(),
                disabled: authored.state().disabled() || !owner.activation.enabled(),
                inert: authored.state().inert(),
                supported_actions: supported_actions(authored, owner),
                relationships: Vec::new(),
                bounds: resolve_bounds(owner.bounds, authored.bounds()),
                text: authored.text().cloned(),
            },
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
        self.drafts
            .iter()
            .find(|draft| &draft.owner == owner && &draft.key == key)
            .map(|draft| &draft.node.id)
    }

    fn resolve_relationships(&mut self) {
        for index in 0..self.drafts.len() {
            let owner = self.drafts[index].owner.clone();
            let source = self.drafts[index].node.id.clone();
            let authored = self.drafts[index].authored_relationships.clone();
            let mut relationships = Vec::with_capacity(authored.len());
            for relationship in authored {
                let target = match relationship.target() {
                    SemanticReference::Local(key) => match self.visible_id(&owner, key).cloned() {
                        Some(target) => Some(target),
                        None => {
                            self.diagnostics.push(
                                SemanticCompositionDiagnostic::MissingLocalRelationshipTarget {
                                    source: source.clone(),
                                    key: key.clone(),
                                },
                            );
                            None
                        }
                    },
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
            self.drafts[index].node.relationships = relationships;
        }
    }

    fn resolve_authored_relationship_target(
        &mut self,
        source: &SemanticNodeId,
        element_id: &ElementId,
        semantic_key: Option<&SemanticKey>,
    ) -> Option<SemanticNodeId> {
        let matches = self
            .owners
            .iter()
            .filter(|owner| owner.authored_id.as_ref() == Some(element_id))
            .collect::<Vec<_>>();
        let target_owner = match matches.as_slice() {
            [] => {
                self.diagnostics.push(
                    SemanticCompositionDiagnostic::MissingAuthoredRelationshipOwner {
                        source: source.clone(),
                        element_id: element_id.clone(),
                    },
                );
                return None;
            }
            [owner] => owner.id.clone(),
            _ => {
                self.diagnostics.push(
                    SemanticCompositionDiagnostic::AmbiguousAuthoredRelationshipOwner {
                        source: source.clone(),
                        element_id: element_id.clone(),
                    },
                );
                return None;
            }
        };
        let key = semantic_key.cloned().unwrap_or(SemanticKey::PRIMARY);
        match self.visible_id(&target_owner, &key).cloned() {
            Some(target) => Some(target),
            None => {
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
}

fn contains_semantic_node(items: &[SemanticItem]) -> bool {
    items
        .iter()
        .any(|item| matches!(item, SemanticItem::Node(_)))
}

fn supported_actions(
    authored: &SemanticNodeContribution,
    owner: &SemanticOwnerFacts,
) -> Vec<SemanticAction> {
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
                        Focusability::Automatic => owner.activation.is_actionable(),
                        Focusability::Focusable => true,
                        Focusability::NotFocusable | Focusability::Hidden => false,
                        _ => false,
                    }
            }
            SemanticAction::OpenMenu | SemanticAction::OpenContextMenu => true,
            _ => false,
        })
        .cloned()
        .collect()
}

fn resolve_bounds(owner: LogicalRect, bounds: SemanticBounds) -> LogicalRect {
    match bounds {
        SemanticBounds::Owner => owner,
        SemanticBounds::OwnerLocal(local) => LogicalRect::new(
            LogicalPoint::new(
                finite_saturating_add(owner.x(), local.x()),
                finite_saturating_add(owner.y(), local.y()),
            )
            .unwrap_or_else(|_| unreachable!("saturating semantic translation remains finite")),
            local.size(),
        ),
    }
}

fn finite_saturating_add(left: f32, right: f32) -> f32 {
    let sum = left + right;
    if sum.is_finite() {
        sum
    } else if left.is_sign_negative() && right.is_sign_negative() {
        -f32::MAX
    } else {
        f32::MAX
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, ElementId, Focusability, LogicalPoint, LogicalRect,
        LogicalSize, SemanticAction, SemanticBounds, SemanticContribution, SemanticItem,
        SemanticKey, SemanticNodeContribution, SemanticReference, SemanticRelationship,
        SemanticRelationshipKind, SemanticRole, SemanticState, WidgetActivation,
    };

    use super::{SemanticCompositionDiagnostic, SemanticOwnerFacts, compose_semantics};

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::new(
            LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite")),
            LogicalSize::try_new(width, height)
                .unwrap_or_else(|_| unreachable!("test size is valid")),
        )
    }

    fn key(value: &'static str) -> SemanticKey {
        SemanticKey::from_static(value).unwrap_or_else(|_| unreachable!("test key is valid"))
    }

    fn element_id(value: &'static str) -> ElementId {
        ElementId::from_static(value).unwrap_or_else(|_| unreachable!("test id is valid"))
    }

    #[test]
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
            },
            SemanticOwnerFacts {
                id: control.clone(),
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
            },
        ];

        let candidate = compose_semantics(&owners, Some(&root), None);
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
            },
        ];

        let candidate = compose_semantics(&owners, Some(&owner), Some(&owner));
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
        }];

        let candidate = compose_semantics(&owners, Some(&owner), None);
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
            },
        ];

        let candidate = compose_semantics(&owners, Some(&root), Some(&source_owner));
        assert!(candidate.diagnostics.is_empty());
        assert_eq!(candidate.focused, Some(source_primary));
        assert_eq!(candidate.nodes[0].relationships.len(), 2);
        assert_eq!(candidate.nodes[0].relationships[0].target, source_named);
        assert_eq!(candidate.nodes[0].relationships[1].target, target_primary);
    }
}
