use runenui_core::{DropShadow, SceneOpacity};

use super::SceneClip;

/// Snapshot-local reference to one immutable paint composition group.
///
/// The value is issued only while runtime stages one [`super::PaintScene`].
/// It is a structural publication reference only: it has no mounted, semantic,
/// reconciliation, widget-state, lifecycle, or cross-publication meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaintSceneGroupId(usize);

impl PaintSceneGroupId {
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PaintSceneEntryKind {
    Item(usize),
    Group(PaintSceneGroupId),
}

/// One already-ordered direct child of the implicit scene root or a paint group.
///
/// Entries are runtime-issued. Downstream renderers may inspect whether an entry
/// references a pre-group paint item or a nested group, but cannot manufacture
/// scene structure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaintSceneEntry(PaintSceneEntryKind);

impl PaintSceneEntry {
    pub(crate) const fn item(item_index: usize) -> Self {
        Self(PaintSceneEntryKind::Item(item_index))
    }

    pub(crate) const fn group(group: PaintSceneGroupId) -> Self {
        Self(PaintSceneEntryKind::Group(group))
    }

    /// Returns the referenced pre-group paint-item index when this is an item.
    #[must_use]
    pub const fn item_index(self) -> Option<usize> {
        match self.0 {
            PaintSceneEntryKind::Item(index) => Some(index),
            PaintSceneEntryKind::Group(_) => None,
        }
    }

    /// Returns the referenced snapshot-local group when this is a group.
    #[must_use]
    pub const fn group_id(self) -> Option<PaintSceneGroupId> {
        match self.0 {
            PaintSceneEntryKind::Item(_) => None,
            PaintSceneEntryKind::Group(group) => Some(group),
        }
    }
}

/// One immutable renderer-neutral atomic paint-composition group.
///
/// Direct entries are already in runtime-decided first-member-contraction order.
/// Item clips have already been applied to item publication facts. Group shadows,
/// group clips, and group opacity are neutral composition semantics consumed in
/// that order by downstream realization.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintSceneGroup {
    parent: Option<PaintSceneGroupId>,
    entries: Vec<PaintSceneEntry>,
    clips: Vec<SceneClip>,
    opacity: SceneOpacity,
    shadows: Vec<DropShadow>,
}

impl PaintSceneGroup {
    pub(crate) const fn new(
        parent: Option<PaintSceneGroupId>,
        entries: Vec<PaintSceneEntry>,
        clips: Vec<SceneClip>,
        opacity: SceneOpacity,
        shadows: Vec<DropShadow>,
    ) -> Self {
        Self {
            parent,
            entries,
            clips,
            opacity,
            shadows,
        }
    }

    /// Returns this group's immediate parent, or `None` under the implicit root.
    #[must_use]
    pub const fn parent(&self) -> Option<PaintSceneGroupId> {
        self.parent
    }

    /// Returns direct child items/groups in exact runtime-decided composition order.
    #[must_use]
    pub const fn entries(&self) -> &[PaintSceneEntry] {
        self.entries.as_slice()
    }

    /// Returns conjunctive group clips in retained authored order.
    #[must_use]
    pub const fn clips(&self) -> &[SceneClip] {
        self.clips.as_slice()
    }

    /// Returns the validated opacity applied once to the composed group result.
    #[must_use]
    pub const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }

    /// Returns ordinary shadows in exact resolved order.
    #[must_use]
    pub const fn shadows(&self) -> &[DropShadow] {
        self.shadows.as_slice()
    }
}

/// Private scene-level aggregate for immutable group storage and contracted root entries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintSceneComposition {
    groups: Vec<PaintSceneGroup>,
    root_entries: Vec<PaintSceneEntry>,
}

impl PaintSceneComposition {
    pub(crate) fn ungrouped(item_count: usize) -> Self {
        Self {
            groups: Vec::new(),
            root_entries: (0..item_count).map(PaintSceneEntry::item).collect(),
        }
    }

    pub(crate) const fn new(
        groups: Vec<PaintSceneGroup>,
        root_entries: Vec<PaintSceneEntry>,
    ) -> Self {
        Self {
            groups,
            root_entries,
        }
    }

    pub(crate) const fn groups(&self) -> &[PaintSceneGroup] {
        self.groups.as_slice()
    }

    pub(crate) const fn root_entries(&self) -> &[PaintSceneEntry] {
        self.root_entries.as_slice()
    }

    pub(crate) fn group(&self, id: PaintSceneGroupId) -> Option<&PaintSceneGroup> {
        self.groups.get(id.index())
    }
}
