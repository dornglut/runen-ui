use crate::{
    CompositionDefinitionId, CompositionRootDefinition, CompositionState, MountedUnitDefinition,
    PresentationTargetDefinition, RegionDefinition, StateRevision,
};

#[derive(Clone, Copy, Debug)]
pub struct CompositionSnapshot<'a> {
    state: &'a CompositionState,
}

impl<'a> CompositionSnapshot<'a> {
    pub(crate) const fn new(state: &'a CompositionState) -> Self {
        Self { state }
    }
    pub const fn definition_id(self) -> CompositionDefinitionId {
        self.state.definition.id()
    }
    pub const fn revision(self) -> StateRevision {
        self.state.revision
    }
    pub fn targets(self) -> &'a [PresentationTargetDefinition] {
        self.state.definition.targets()
    }
    pub fn roots(self) -> &'a [CompositionRootDefinition] {
        self.state.definition.roots()
    }
    pub fn regions(self) -> &'a [RegionDefinition] {
        self.state.definition.regions()
    }
    pub fn mounted_units(self) -> &'a [MountedUnitDefinition] {
        self.state.definition.mounted_units()
    }
    pub fn target(
        self,
        id: crate::PresentationTargetId,
    ) -> Option<&'a PresentationTargetDefinition> {
        self.targets().iter().find(|value| value.id == id)
    }
    pub fn root(self, id: crate::CompositionRootId) -> Option<&'a CompositionRootDefinition> {
        self.roots().iter().find(|value| value.id == id)
    }
    pub fn region(self, id: crate::RegionId) -> Option<&'a RegionDefinition> {
        self.regions().iter().find(|value| value.id == id)
    }
    pub fn mounted_unit(self, id: crate::MountedUnitId) -> Option<&'a MountedUnitDefinition> {
        self.mounted_units().iter().find(|value| value.id == id)
    }
}
