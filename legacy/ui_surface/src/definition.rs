//! File: domain/ui/ui_surface/src/definition.rs
//! Purpose: Surface definition identities and registry contracts.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SurfaceDefinitionId(u64);

impl SurfaceDefinitionId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceDefinition {
    pub id: SurfaceDefinitionId,
    pub semantic_key: &'static str,
    pub display_name: &'static str,
}

impl SurfaceDefinition {
    pub const fn new(
        id: SurfaceDefinitionId,
        semantic_key: &'static str,
        display_name: &'static str,
    ) -> Self {
        Self {
            id,
            semantic_key,
            display_name,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SurfaceDefinitionRegistry {
    definitions_by_id: BTreeMap<SurfaceDefinitionId, SurfaceDefinition>,
}

impl SurfaceDefinitionRegistry {
    pub fn register(&mut self, definition: SurfaceDefinition) -> Option<SurfaceDefinition> {
        self.definitions_by_id.insert(definition.id, definition)
    }

    pub fn definition(&self, id: SurfaceDefinitionId) -> Option<SurfaceDefinition> {
        self.definitions_by_id.get(&id).copied()
    }

    pub fn definitions(&self) -> impl Iterator<Item = SurfaceDefinition> + '_ {
        self.definitions_by_id.values().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.definitions_by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_definition_registry_is_id_addressable() {
        let mut registry = SurfaceDefinitionRegistry::default();
        let definition = SurfaceDefinition::new(
            SurfaceDefinitionId::new(11),
            "editor.tool_surface.viewport",
            "Viewport",
        );

        registry.register(definition);

        assert_eq!(
            registry.definition(SurfaceDefinitionId::new(11)),
            Some(definition)
        );
        assert_eq!(registry.definition(SurfaceDefinitionId::new(22)), None);
    }
}
