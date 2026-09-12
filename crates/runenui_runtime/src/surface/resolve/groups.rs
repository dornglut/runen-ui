use std::collections::HashMap;

use runenui_core::{DropShadow, SceneLayer, SceneOpacity};

use crate::scene::{
    PaintSceneComposition, PaintSceneEntry, PaintSceneGroup, PaintSceneGroupId, PaintSceneItem,
    SceneClip,
};

use super::{CachedStyleFacts, SurfaceTopologySnapshot};

/// Runtime-private staging reference to one resolved explicit owner-local group.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ExplicitGroupId(usize);

impl ExplicitGroupId {
    pub(super) const fn new(index: usize) -> Self {
        Self(index)
    }

    const fn index(self) -> usize {
        self.0
    }
}

/// Surface-space neutral facts for one valid explicit owner-local group.
pub(super) struct ResolvedExplicitGroup {
    owner: usize,
    parent: Option<ExplicitGroupId>,
    clips: Vec<SceneClip>,
    opacity: SceneOpacity,
    shadows: Vec<DropShadow>,
}

impl ResolvedExplicitGroup {
    pub(super) const fn new(
        owner: usize,
        parent: Option<ExplicitGroupId>,
        clips: Vec<SceneClip>,
        opacity: SceneOpacity,
        shadows: Vec<DropShadow>,
    ) -> Self {
        Self {
            owner,
            parent,
            clips,
            opacity,
            shadows,
        }
    }
}

/// One admitted paint item with the exact facts needed before group contraction.
pub(super) struct OrderedPaintItem {
    layer: SceneLayer,
    mounted_preorder: usize,
    contribution_local_order: usize,
    explicit_group: Option<ExplicitGroupId>,
    item: PaintSceneItem,
}

impl OrderedPaintItem {
    pub(super) const fn new(
        layer: SceneLayer,
        mounted_preorder: usize,
        contribution_local_order: usize,
        explicit_group: Option<ExplicitGroupId>,
        item: PaintSceneItem,
    ) -> Self {
        Self {
            layer,
            mounted_preorder,
            contribution_local_order,
            explicit_group,
            item,
        }
    }

    pub(super) const fn ordering_key(&self) -> (SceneLayer, usize, usize) {
        (
            self.layer,
            self.mounted_preorder,
            self.contribution_local_order,
        )
    }
}

#[derive(Clone, Copy)]
enum GroupSource {
    Node(usize),
    Explicit(ExplicitGroupId),
}

struct GroupPlan {
    sources: Vec<GroupSource>,
    parents: Vec<Option<usize>>,
    anchors: Vec<Option<usize>>,
    item_groups: Vec<Option<usize>>,
}

struct CompositionEntries {
    grouped: Vec<Vec<(usize, PaintSceneEntry)>>,
    root: Vec<(usize, PaintSceneEntry)>,
}

fn topology_parents(topology: &SurfaceTopologySnapshot) -> Vec<Option<usize>> {
    let index_by_id = topology
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect::<HashMap<_, _>>();
    topology
        .nodes
        .iter()
        .map(|node| {
            node.parent
                .as_ref()
                .and_then(|parent| index_by_id.get(parent).copied())
        })
        .collect()
}

const fn nearest_node_group(
    mut cursor: Option<usize>,
    parent: &[Option<usize>],
    node_groups: &[Option<usize>],
) -> Option<usize> {
    while let Some(node) = cursor {
        if let Some(group) = node_groups[node] {
            return Some(group);
        }
        cursor = parent[node];
    }
    None
}

fn derive_group_plan(
    topology: &SurfaceTopologySnapshot,
    styles: &CachedStyleFacts,
    explicit_groups: &[ResolvedExplicitGroup],
    item_owners: &[usize],
    item_explicit_groups: &[Option<ExplicitGroupId>],
) -> GroupPlan {
    let topology_parent = topology_parents(topology);
    let requires_node_group = styles
        .resolutions
        .iter()
        .map(|resolution| {
            let computed = resolution.computed_style();
            computed.opacity() != SceneOpacity::OPAQUE || !computed.shadows().is_empty()
        })
        .collect::<Vec<_>>();

    let mut sources = Vec::new();
    let mut node_groups = vec![None; topology.nodes.len()];
    for (node, requires_group) in requires_node_group.into_iter().enumerate() {
        if requires_group {
            let candidate = sources.len();
            node_groups[node] = Some(candidate);
            sources.push(GroupSource::Node(node));
        }
    }

    let mut explicit_candidates = Vec::with_capacity(explicit_groups.len());
    for explicit_index in 0..explicit_groups.len() {
        let candidate = sources.len();
        explicit_candidates.push(candidate);
        sources.push(GroupSource::Explicit(ExplicitGroupId::new(explicit_index)));
    }

    let mut parents = vec![None; sources.len()];
    for (node, candidate) in node_groups.iter().copied().enumerate() {
        let Some(candidate) = candidate else {
            continue;
        };
        parents[candidate] =
            nearest_node_group(topology_parent[node], &topology_parent, &node_groups);
    }
    for (explicit_index, explicit) in explicit_groups.iter().enumerate() {
        let candidate = explicit_candidates[explicit_index];
        parents[candidate] = explicit.parent.map_or_else(
            || nearest_node_group(Some(explicit.owner), &topology_parent, &node_groups),
            |parent| Some(explicit_candidates[parent.index()]),
        );
    }

    let item_groups = item_owners
        .iter()
        .copied()
        .zip(item_explicit_groups.iter().copied())
        .map(|(owner, explicit)| {
            explicit.map_or_else(
                || nearest_node_group(Some(owner), &topology_parent, &node_groups),
                |explicit| Some(explicit_candidates[explicit.index()]),
            )
        })
        .collect::<Vec<_>>();

    let mut anchors = vec![None; sources.len()];
    for (item_index, immediate_group) in item_groups.iter().copied().enumerate() {
        let mut cursor = immediate_group;
        while let Some(group) = cursor {
            anchors[group] =
                Some(anchors[group].map_or(item_index, |anchor: usize| anchor.min(item_index)));
            cursor = parents[group];
        }
    }

    GroupPlan {
        sources,
        parents,
        anchors,
        item_groups,
    }
}

fn scene_group_ids(anchors: &[Option<usize>]) -> Vec<Option<PaintSceneGroupId>> {
    let mut next_group = 0;
    anchors
        .iter()
        .map(|anchor| {
            anchor.map(|_| {
                let group = PaintSceneGroupId::new(next_group);
                next_group += 1;
                group
            })
        })
        .collect()
}

fn build_composition_entries(
    item_groups: &[Option<usize>],
    parents: &[Option<usize>],
    anchors: &[Option<usize>],
    candidate_to_scene: &[Option<PaintSceneGroupId>],
) -> CompositionEntries {
    let group_count = candidate_to_scene.iter().flatten().count();
    let mut grouped = vec![Vec::<(usize, PaintSceneEntry)>::new(); group_count];
    let mut root = Vec::<(usize, PaintSceneEntry)>::new();
    for (item_index, candidate) in item_groups.iter().copied().enumerate() {
        let entry = (item_index, PaintSceneEntry::item(item_index));
        if let Some(group) = candidate.and_then(|candidate| candidate_to_scene[candidate]) {
            grouped[group.index()].push(entry);
        } else {
            root.push(entry);
        }
    }

    for candidate in 0..candidate_to_scene.len() {
        let Some(group) = candidate_to_scene[candidate] else {
            continue;
        };
        let anchor = anchors[candidate]
            .unwrap_or_else(|| unreachable!("published group has a descendant anchor"));
        let entry = (anchor, PaintSceneEntry::group(group));
        let parent = parents[candidate].and_then(|parent| candidate_to_scene[parent]);
        if let Some(parent) = parent {
            grouped[parent.index()].push(entry);
        } else {
            root.push(entry);
        }
    }
    for entries in &mut grouped {
        entries.sort_by_key(|(anchor, _)| *anchor);
    }
    root.sort_by_key(|(anchor, _)| *anchor);
    CompositionEntries { grouped, root }
}

fn publish_groups(
    styles: &CachedStyleFacts,
    explicit_groups: &[ResolvedExplicitGroup],
    sources: Vec<GroupSource>,
    parents: &[Option<usize>],
    candidate_to_scene: &[Option<PaintSceneGroupId>],
    mut grouped_entries: Vec<Vec<(usize, PaintSceneEntry)>>,
) -> Vec<PaintSceneGroup> {
    let mut groups = Vec::with_capacity(grouped_entries.len());
    for (candidate, source) in sources.into_iter().enumerate() {
        let Some(group) = candidate_to_scene[candidate] else {
            continue;
        };
        let parent = parents[candidate].and_then(|parent| candidate_to_scene[parent]);
        let entries = std::mem::take(&mut grouped_entries[group.index()])
            .into_iter()
            .map(|(_, entry)| entry)
            .collect();
        let published = match source {
            GroupSource::Node(node) => {
                let computed = styles.resolutions[node].computed_style();
                PaintSceneGroup::new(
                    parent,
                    entries,
                    Vec::new(),
                    computed.opacity(),
                    computed.shadows().to_vec(),
                )
            }
            GroupSource::Explicit(explicit) => {
                let explicit = &explicit_groups[explicit.index()];
                PaintSceneGroup::new(
                    parent,
                    entries,
                    explicit.clips.clone(),
                    explicit.opacity,
                    explicit.shadows.clone(),
                )
            }
        };
        groups.push(published);
    }
    groups
}

/// Applies accepted ADR 0012 node and explicit owner-local grouping to one exact
/// inherited M6 pre-group item sequence.
///
/// `items()` preserve that sequence exactly. Only the separate composition forest
/// contracts non-empty groups at their first descendant; explicit local groups and
/// runtime node-effect groups share one runtime-issued snapshot-local group table.
pub(super) fn derive_composition_groups(
    topology: &SurfaceTopologySnapshot,
    styles: &CachedStyleFacts,
    explicit_groups: &[ResolvedExplicitGroup],
    ordered: Vec<OrderedPaintItem>,
) -> (Vec<PaintSceneItem>, PaintSceneComposition) {
    debug_assert_eq!(topology.nodes.len(), styles.resolutions.len());

    let item_owners = ordered
        .iter()
        .map(|item| item.mounted_preorder)
        .collect::<Vec<_>>();
    let item_explicit_groups = ordered
        .iter()
        .map(|item| item.explicit_group)
        .collect::<Vec<_>>();
    let mut items = ordered
        .into_iter()
        .map(|item| item.item)
        .collect::<Vec<_>>();
    if items.is_empty() || topology.nodes.is_empty() {
        let item_count = items.len();
        return (items, PaintSceneComposition::ungrouped(item_count));
    }

    let GroupPlan {
        sources,
        parents,
        anchors,
        item_groups,
    } = derive_group_plan(
        topology,
        styles,
        explicit_groups,
        &item_owners,
        &item_explicit_groups,
    );
    let candidate_to_scene = scene_group_ids(&anchors);
    if candidate_to_scene.iter().all(Option::is_none) {
        let item_count = items.len();
        return (items, PaintSceneComposition::ungrouped(item_count));
    }

    for (item, candidate) in items.iter_mut().zip(&item_groups) {
        item.set_group(candidate.and_then(|candidate| candidate_to_scene[candidate]));
    }
    let CompositionEntries { grouped, root } =
        build_composition_entries(&item_groups, &parents, &anchors, &candidate_to_scene);
    let groups = publish_groups(
        styles,
        explicit_groups,
        sources,
        &parents,
        &candidate_to_scene,
        grouped,
    );
    let root_entries = root.into_iter().map(|(_, entry)| entry).collect();
    (items, PaintSceneComposition::new(groups, root_entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_member_contraction_ordering_is_stable_for_entries() {
        let group = PaintSceneGroupId::new(0);
        let mut entries = [
            (2, PaintSceneEntry::item(2)),
            (0, PaintSceneEntry::group(group)),
            (1, PaintSceneEntry::item(1)),
        ];
        entries.sort_by_key(|(anchor, _)| *anchor);
        assert_eq!(entries[0].1.group_id(), Some(group));
        assert_eq!(entries[1].1.item_index(), Some(1));
        assert_eq!(entries[2].1.item_index(), Some(2));
    }
}
