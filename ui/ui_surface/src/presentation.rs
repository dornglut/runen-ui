//! File: domain/ui/ui_surface/src/presentation.rs
//! Purpose: Prepared presentation model contracts for surface rendering and interaction.

use std::collections::BTreeSet;

use crate::{
    ObservationFrame, WorldSpacePromptHostEntityId, WorldSpacePromptHostInput,
    WorldSpacePromptLifetime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfacePresentationModel<ItemId>
where
    ItemId: Copy + Ord,
{
    pub selected_primary_item: Option<ItemId>,
    selectable_primary_items: BTreeSet<ItemId>,
}

impl<ItemId> SurfacePresentationModel<ItemId>
where
    ItemId: Copy + Ord,
{
    pub fn new(
        selected_primary_item: Option<ItemId>,
        selectable_primary_items: impl IntoIterator<Item = ItemId>,
    ) -> Self {
        Self {
            selected_primary_item,
            selectable_primary_items: selectable_primary_items.into_iter().collect(),
        }
    }

    pub fn from_observation_frame(
        frame: &impl ObservationFrame<ItemId>,
        candidate_items: impl IntoIterator<Item = ItemId>,
    ) -> Self {
        let selectable = candidate_items
            .into_iter()
            .filter(|item| frame.is_item_available(*item));
        Self::new(frame.selected_primary_item(), selectable)
    }

    pub fn is_primary_selectable(&self, item_id: ItemId) -> bool {
        self.selectable_primary_items.contains(&item_id)
    }

    pub fn selectable_primary_items(&self) -> impl Iterator<Item = ItemId> + '_ {
        self.selectable_primary_items.iter().copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSpacePromptProjection {
    pub screen_x: f32,
    pub screen_y: f32,
    pub depth: f32,
    pub on_screen: bool,
}

impl WorldSpacePromptProjection {
    pub const fn new(screen_x: f32, screen_y: f32, depth: f32, on_screen: bool) -> Self {
        Self {
            screen_x,
            screen_y,
            depth,
            on_screen,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldSpacePromptPresentation {
    pub entity_id: WorldSpacePromptHostEntityId,
    pub label: String,
    pub route_id: String,
    pub screen_x: f32,
    pub screen_y: f32,
    pub depth: f32,
}

impl WorldSpacePromptPresentation {
    pub fn from_host_input(
        input: &WorldSpacePromptHostInput,
        projection: WorldSpacePromptProjection,
    ) -> Option<Self> {
        if input.lifetime != WorldSpacePromptLifetime::Alive
            || !input.visibility.visible
            || projection.depth < 0.0
            || !projection.on_screen
        {
            return None;
        }
        Some(Self {
            entity_id: input.anchor.entity_id,
            label: input.data_feed.label.clone(),
            route_id: input.data_feed.route_id.clone(),
            screen_x: projection.screen_x,
            screen_y: projection.screen_y,
            depth: projection.depth,
        })
    }

    pub fn event_packet(&self) -> WorldSpacePromptEventPacket {
        WorldSpacePromptEventPacket {
            route_id: self.route_id.clone(),
            payload: WorldSpacePromptEventPayload {
                entity_id: self.entity_id,
                label: self.label.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSpacePromptEventPayload {
    pub entity_id: WorldSpacePromptHostEntityId,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSpacePromptEventPacket {
    pub route_id: String,
    pub payload: WorldSpacePromptEventPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldSpacePromptHostCommand {
    FocusEntity(WorldSpacePromptHostEntityId),
}

pub fn host_command_for_world_space_prompt(
    packet: &WorldSpacePromptEventPacket,
) -> WorldSpacePromptHostCommand {
    WorldSpacePromptHostCommand::FocusEntity(packet.payload.entity_id)
}

#[cfg(test)]
mod tests {

    use super::*;

    struct DummyObservation;

    impl ObservationFrame<u64> for DummyObservation {
        fn selected_primary_item(&self) -> Option<u64> {
            Some(2)
        }

        fn is_item_available(&self, item_id: u64) -> bool {
            item_id.is_multiple_of(2)
        }
    }

    #[test]
    fn presentation_model_filters_selectable_items_from_observation() {
        let frame = DummyObservation;
        let model = SurfacePresentationModel::from_observation_frame(&frame, [1, 2, 3, 4]);

        assert_eq!(model.selected_primary_item, Some(2));
        assert_eq!(
            model.selectable_primary_items().collect::<Vec<_>>(),
            vec![2, 4]
        );
        assert!(model.is_primary_selectable(2));
        assert!(!model.is_primary_selectable(3));
    }

    #[test]
    fn world_space_prompt_presentation_filters_projection_lifetime_and_visibility() {
        let input = crate::WorldSpacePromptHostInput::new(
            crate::WorldSpacePromptAnchor::new(
                crate::WorldSpacePromptHostEntityId::new(7),
                crate::WorldSpacePromptAnchorPosition::new(1.0, 0.0, 3.0),
            ),
            crate::WorldSpacePromptLifetime::Alive,
            crate::WorldSpacePromptVisibility::visible(),
            crate::WorldSpacePromptDataFeed::new("Inspect", "ui.world.prompt.inspect"),
        );

        let presentation = WorldSpacePromptPresentation::from_host_input(
            &input,
            WorldSpacePromptProjection::new(128.0, 64.0, 0.5, true),
        )
        .expect("visible alive prompt should project");

        assert_eq!(presentation.entity_id.raw(), 7);
        assert_eq!(presentation.screen_x, 128.0);
        assert_eq!(presentation.label, "Inspect");
        assert_eq!(presentation.route_id, "ui.world.prompt.inspect");
        assert!(
            WorldSpacePromptPresentation::from_host_input(
                &input,
                WorldSpacePromptProjection::new(128.0, 64.0, 0.5, false),
            )
            .is_none()
        );
    }

    #[test]
    fn world_space_prompt_event_packet_maps_to_host_command_without_domain_mutation() {
        let presentation = WorldSpacePromptPresentation {
            entity_id: crate::WorldSpacePromptHostEntityId::new(12),
            label: "Use".to_string(),
            route_id: "ui.world.prompt.use".to_string(),
            screen_x: 10.0,
            screen_y: 20.0,
            depth: 0.25,
        };
        let packet = presentation.event_packet();
        let command = host_command_for_world_space_prompt(&packet);

        assert_eq!(packet.route_id, "ui.world.prompt.use");
        assert_eq!(packet.payload.entity_id.raw(), 12);
        assert_eq!(
            command,
            WorldSpacePromptHostCommand::FocusEntity(crate::WorldSpacePromptHostEntityId::new(12))
        );
    }
}
