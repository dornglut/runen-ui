//! File: domain/ui/ui_surface/src/intent.rs
//! Purpose: Surface intent contracts emitted by surface interactions.

use crate::{SurfaceCapability, SurfaceInstanceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceIntentKind {
    SelectPrimaryItem { item_id: u64 },
    SelectEntity { entity_id: u64 },
    ActivateField { field_index: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceIntent {
    pub surface_instance_id: SurfaceInstanceId,
    pub required_capability: SurfaceCapability,
    pub kind: SurfaceIntentKind,
}

impl SurfaceIntent {
    pub const fn select_primary_item(surface_instance_id: SurfaceInstanceId, item_id: u64) -> Self {
        Self {
            surface_instance_id,
            required_capability: SurfaceCapability::RequestMutation,
            kind: SurfaceIntentKind::SelectPrimaryItem { item_id },
        }
    }

    pub const fn select_entity(surface_instance_id: SurfaceInstanceId, entity_id: u64) -> Self {
        Self {
            surface_instance_id,
            required_capability: SurfaceCapability::RequestMutation,
            kind: SurfaceIntentKind::SelectEntity { entity_id },
        }
    }

    pub const fn activate_field(surface_instance_id: SurfaceInstanceId, field_index: u64) -> Self {
        Self {
            surface_instance_id,
            required_capability: SurfaceCapability::RequestMutation,
            kind: SurfaceIntentKind::ActivateField { field_index },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_primary_item_intent_requires_request_mutation_capability() {
        let intent = SurfaceIntent::select_primary_item(SurfaceInstanceId::new(5), 12);

        assert_eq!(intent.surface_instance_id, SurfaceInstanceId::new(5));
        assert_eq!(
            intent.required_capability,
            SurfaceCapability::RequestMutation
        );
        assert_eq!(
            intent.kind,
            SurfaceIntentKind::SelectPrimaryItem { item_id: 12 }
        );
    }

    #[test]
    fn select_entity_intent_requires_request_mutation_capability() {
        let intent = SurfaceIntent::select_entity(SurfaceInstanceId::new(7), 33);

        assert_eq!(intent.surface_instance_id, SurfaceInstanceId::new(7));
        assert_eq!(
            intent.required_capability,
            SurfaceCapability::RequestMutation
        );
        assert_eq!(
            intent.kind,
            SurfaceIntentKind::SelectEntity { entity_id: 33 }
        );
    }

    #[test]
    fn activate_field_intent_requires_request_mutation_capability() {
        let intent = SurfaceIntent::activate_field(SurfaceInstanceId::new(8), 4);

        assert_eq!(intent.surface_instance_id, SurfaceInstanceId::new(8));
        assert_eq!(
            intent.required_capability,
            SurfaceCapability::RequestMutation
        );
        assert_eq!(
            intent.kind,
            SurfaceIntentKind::ActivateField { field_index: 4 }
        );
    }
}
