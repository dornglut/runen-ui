//! Renderer-neutral explicit owner-local paint composition grouping.

use crate::{ContributionClip, DropShadow, PaintContributionItem, SceneOpacity};

/// One structurally authored owner-local paint contribution entry.
///
/// Explicit grouping is recursive authoring structure only. Runtime never receives a
/// widget-authored group identity; [`crate::PaintContribution::from_entries`] consumes
/// this tree and normalizes it immediately into flat contribution-local item order plus
/// private group membership facts.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PaintContributionEntry {
    /// One ordinary owner-local paint item.
    Item(PaintContributionItem),
    /// One explicit owner-local atomic composition group.
    Group(PaintContributionGroup),
}

impl PaintContributionEntry {
    /// Creates one ordinary paint-item entry.
    #[must_use]
    pub const fn item(item: PaintContributionItem) -> Self {
        Self::Item(item)
    }

    /// Creates one explicit owner-local composition-group entry.
    #[must_use]
    pub const fn group(group: PaintContributionGroup) -> Self {
        Self::Group(group)
    }

    /// Returns the contained item when this entry is an item.
    #[must_use]
    pub const fn as_item(&self) -> Option<&PaintContributionItem> {
        match self {
            Self::Item(item) => Some(item),
            Self::Group(_) => None,
        }
    }

    /// Returns the contained group when this entry is a group.
    #[must_use]
    pub const fn as_group(&self) -> Option<&PaintContributionGroup> {
        match self {
            Self::Item(_) => None,
            Self::Group(group) => Some(group),
        }
    }
}

impl From<PaintContributionItem> for PaintContributionEntry {
    fn from(item: PaintContributionItem) -> Self {
        Self::item(item)
    }
}

impl From<PaintContributionGroup> for PaintContributionEntry {
    fn from(group: PaintContributionGroup) -> Self {
        Self::group(group)
    }
}

/// One explicit owner-local atomic paint-composition group.
///
/// A group owns only the entries nested structurally inside this value. It has no
/// key, ID, layer, transform, mounted reference, renderer handle, or mechanism to
/// capture paint from another mounted owner. Group clips remain owner-local and are
/// independent from every child item's transform.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintContributionGroup {
    entries: Vec<PaintContributionEntry>,
    clips: Vec<ContributionClip>,
    opacity: SceneOpacity,
    shadows: Vec<DropShadow>,
}

impl PaintContributionGroup {
    /// Creates one explicit group with identity effects around ordered child entries.
    #[must_use]
    pub const fn new(entries: Vec<PaintContributionEntry>) -> Self {
        Self {
            entries,
            clips: Vec::new(),
            opacity: SceneOpacity::OPAQUE,
            shadows: Vec::new(),
        }
    }

    /// Appends one conjunctive owner-local group clip.
    #[must_use]
    pub fn with_clip(mut self, clip: ContributionClip) -> Self {
        self.clips.push(clip);
        self
    }

    /// Replaces the opacity applied once to the composed group result.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: SceneOpacity) -> Self {
        self.opacity = opacity;
        self
    }

    /// Replaces ordinary group shadows in exact authored order.
    #[must_use]
    pub fn with_shadows(mut self, shadows: Vec<DropShadow>) -> Self {
        self.shadows = shadows;
        self
    }

    /// Returns direct child entries in exact authored order.
    #[must_use]
    pub const fn entries(&self) -> &[PaintContributionEntry] {
        self.entries.as_slice()
    }

    /// Returns conjunctive owner-local group clips in authored order.
    #[must_use]
    pub const fn clips(&self) -> &[ContributionClip] {
        self.clips.as_slice()
    }

    /// Returns the validated group opacity.
    #[must_use]
    pub const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }

    /// Returns ordinary group shadows in authored order.
    #[must_use]
    pub const fn shadows(&self) -> &[DropShadow] {
        self.shadows.as_slice()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedPaintGroup {
    pub(super) parent: Option<usize>,
    pub(super) clips: Vec<ContributionClip>,
    pub(super) opacity: SceneOpacity,
    pub(super) shadows: Vec<DropShadow>,
}

pub struct NormalizedPaintContribution {
    pub(super) items: Vec<PaintContributionItem>,
    pub(super) groups: Vec<NormalizedPaintGroup>,
    pub(super) item_groups: Vec<Option<usize>>,
}

pub fn normalize_entries(entries: Vec<PaintContributionEntry>) -> NormalizedPaintContribution {
    let mut normalized = NormalizedPaintContribution {
        items: Vec::new(),
        groups: Vec::new(),
        item_groups: Vec::new(),
    };
    normalize_entry_list(entries, None, &mut normalized);
    if normalized.groups.is_empty() {
        normalized.item_groups.clear();
    }
    normalized
}

fn normalize_entry_list(
    entries: Vec<PaintContributionEntry>,
    parent: Option<usize>,
    normalized: &mut NormalizedPaintContribution,
) {
    for entry in entries {
        match entry {
            PaintContributionEntry::Item(item) => {
                normalized.items.push(item);
                normalized.item_groups.push(parent);
            }
            PaintContributionEntry::Group(group) => normalize_group(group, parent, normalized),
        }
    }
}

fn normalize_group(
    group: PaintContributionGroup,
    parent: Option<usize>,
    normalized: &mut NormalizedPaintContribution,
) {
    if !group_contains_item(&group) {
        return;
    }
    let PaintContributionGroup {
        entries,
        clips,
        opacity,
        shadows,
    } = group;
    let group_index = normalized.groups.len();
    normalized.groups.push(NormalizedPaintGroup {
        parent,
        clips,
        opacity,
        shadows,
    });
    normalize_entry_list(entries, Some(group_index), normalized);
}

fn group_contains_item(group: &PaintContributionGroup) -> bool {
    group.entries.iter().any(|entry| match entry {
        PaintContributionEntry::Item(_) => true,
        PaintContributionEntry::Group(group) => group_contains_item(group),
    })
}

#[cfg(test)]
mod tests {
    use super::{PaintContributionEntry, PaintContributionGroup, normalize_entries};
    use crate::{Brush, Color, LogicalRect, PaintContributionItem, SceneOpacity, SceneShape};

    fn item(red: u8) -> PaintContributionItem {
        let rect = LogicalRect::try_new(0.0, 0.0, 1.0, 1.0)
            .unwrap_or_else(|_| unreachable!("controlled rectangle is valid"));
        PaintContributionItem::fill(
            SceneShape::rect(rect),
            Brush::solid(Color::rgba(red, 0, 0, 255)),
        )
    }

    #[test]
    fn item_only_structure_canonicalizes_to_flat_no_group_storage() {
        let normalized = normalize_entries(vec![item(1).into(), item(2).into()]);
        assert_eq!(normalized.items, vec![item(1), item(2)]);
        assert!(normalized.groups.is_empty());
        assert!(normalized.item_groups.is_empty());
    }

    #[test]
    fn nested_non_empty_groups_receive_private_preorder_ordinals() {
        let opacity = SceneOpacity::new(0.5).unwrap_or_else(|_| unreachable!("opacity is valid"));
        let nested = PaintContributionGroup::new(vec![item(2).into()]);
        let outer =
            PaintContributionGroup::new(vec![item(1).into(), nested.into()]).with_opacity(opacity);
        let normalized = normalize_entries(vec![outer.into(), item(3).into()]);

        assert_eq!(normalized.items, vec![item(1), item(2), item(3)]);
        assert_eq!(normalized.groups.len(), 2);
        assert_eq!(normalized.groups[0].parent, None);
        assert_eq!(normalized.groups[0].opacity, opacity);
        assert_eq!(normalized.groups[1].parent, Some(0));
        assert_eq!(normalized.item_groups, vec![Some(0), Some(1), None]);
    }

    #[test]
    fn structurally_empty_groups_are_omitted_but_identity_non_empty_groups_remain() {
        let empty =
            PaintContributionGroup::new(vec![PaintContributionGroup::new(Vec::new()).into()]);
        let identity = PaintContributionGroup::new(vec![item(7).into()]);
        let normalized = normalize_entries(vec![empty.into(), identity.into()]);

        assert_eq!(normalized.items, vec![item(7)]);
        assert_eq!(normalized.groups.len(), 1);
        assert_eq!(normalized.groups[0].parent, None);
        assert_eq!(normalized.groups[0].opacity, SceneOpacity::OPAQUE);
        assert!(normalized.groups[0].clips.is_empty());
        assert!(normalized.groups[0].shadows.is_empty());
        assert_eq!(normalized.item_groups, vec![Some(0)]);
    }

    #[test]
    fn public_entries_expose_structure_without_any_authored_group_identifier() {
        let group = PaintContributionGroup::new(vec![item(4).into()]);
        let entry = PaintContributionEntry::group(group.clone());
        assert_eq!(entry.as_group(), Some(&group));
        assert!(entry.as_item().is_none());
    }
}
