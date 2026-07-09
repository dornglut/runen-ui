use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    CapabilityId, CompositionDefinitionId, CompositionRootId, DefinitionRevision,
    MountedContentRef, MountedUnitId, PresentationTargetId, RegionId, RegionKind, RegionProfileId,
    TargetProfileId, UnavailableContentPolicy,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationTargetDefinition {
    pub id: PresentationTargetId,
    pub profile: TargetProfileId,
}

impl PresentationTargetDefinition {
    pub fn new(id: PresentationTargetId, profile: TargetProfileId) -> Self {
        Self { id, profile }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionRootDefinition {
    pub id: CompositionRootId,
    pub target: PresentationTargetId,
    pub region: RegionId,
    pub primary: bool,
}

impl CompositionRootDefinition {
    pub const fn new(
        id: CompositionRootId,
        target: PresentationTargetId,
        region: RegionId,
        primary: bool,
    ) -> Self {
        Self {
            id,
            target,
            region,
            primary,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionDefinition {
    pub id: RegionId,
    pub profile: Option<RegionProfileId>,
    pub kind: RegionKind,
}

impl RegionDefinition {
    pub fn new(id: RegionId, profile: Option<RegionProfileId>, kind: RegionKind) -> Self {
        Self { id, profile, kind }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountedUnitDefinition {
    pub id: MountedUnitId,
    content: MountedContentRef,
    capabilities: BTreeSet<CapabilityId>,
    unavailable: UnavailableContentPolicy,
}

impl MountedUnitDefinition {
    pub fn new(
        id: MountedUnitId,
        content: MountedContentRef,
        capabilities: impl IntoIterator<Item = CapabilityId>,
        unavailable: UnavailableContentPolicy,
    ) -> Self {
        Self {
            id,
            content,
            capabilities: capabilities.into_iter().collect(),
            unavailable,
        }
    }

    pub fn content(&self) -> &MountedContentRef {
        &self.content
    }
    pub fn capabilities(&self) -> &BTreeSet<CapabilityId> {
        &self.capabilities
    }
    pub const fn unavailable_policy(&self) -> UnavailableContentPolicy {
        self.unavailable
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionDefinitionV1 {
    schema_version: u16,
    id: CompositionDefinitionId,
    revision: DefinitionRevision,
    targets: Vec<PresentationTargetDefinition>,
    roots: Vec<CompositionRootDefinition>,
    regions: Vec<RegionDefinition>,
    mounted_units: Vec<MountedUnitDefinition>,
}

impl CompositionDefinitionV1 {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn new(
        id: CompositionDefinitionId,
        revision: DefinitionRevision,
        targets: Vec<PresentationTargetDefinition>,
        roots: Vec<CompositionRootDefinition>,
        regions: Vec<RegionDefinition>,
        mounted_units: Vec<MountedUnitDefinition>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            id,
            revision,
            targets,
            roots,
            regions,
            mounted_units,
        }
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn id(&self) -> CompositionDefinitionId {
        self.id
    }
    pub const fn revision(&self) -> DefinitionRevision {
        self.revision
    }
    pub fn targets(&self) -> &[PresentationTargetDefinition] {
        &self.targets
    }
    pub fn roots(&self) -> &[CompositionRootDefinition] {
        &self.roots
    }
    pub fn regions(&self) -> &[RegionDefinition] {
        &self.regions
    }
    pub fn mounted_units(&self) -> &[MountedUnitDefinition] {
        &self.mounted_units
    }

    pub(crate) fn normalized(mut self) -> Self {
        self.targets.sort_by_key(|value| value.id);
        self.roots.sort_by_key(|value| value.id);
        self.regions.sort_by_key(|value| value.id);
        self.mounted_units.sort_by_key(|value| value.id);
        self
    }

    pub(crate) fn parts_mut(
        &mut self,
    ) -> (
        &mut Vec<PresentationTargetDefinition>,
        &mut Vec<CompositionRootDefinition>,
        &mut Vec<RegionDefinition>,
        &mut Vec<MountedUnitDefinition>,
    ) {
        (
            &mut self.targets,
            &mut self.roots,
            &mut self.regions,
            &mut self.mounted_units,
        )
    }

    pub(crate) fn set_revision(&mut self, revision: DefinitionRevision) {
        self.revision = revision;
    }
    pub(crate) fn set_id(&mut self, id: CompositionDefinitionId) {
        self.id = id;
    }
}
