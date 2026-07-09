//! File: domain/ui/ui_surface/src/observation.rs
//! Purpose: Observation-frame trait used to prepare surface presentation models.

pub trait ObservationFrame<ItemId>
where
    ItemId: Copy,
{
    fn selected_primary_item(&self) -> Option<ItemId>;

    fn is_item_available(&self, item_id: ItemId) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldSpacePromptHostEntityId(u64);

impl WorldSpacePromptHostEntityId {
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSpacePromptAnchorPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl WorldSpacePromptAnchorPosition {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldSpacePromptAnchor {
    pub entity_id: WorldSpacePromptHostEntityId,
    pub position: WorldSpacePromptAnchorPosition,
}

impl WorldSpacePromptAnchor {
    pub const fn new(
        entity_id: WorldSpacePromptHostEntityId,
        position: WorldSpacePromptAnchorPosition,
    ) -> Self {
        Self {
            entity_id,
            position,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldSpacePromptLifetime {
    Alive,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSpacePromptVisibility {
    pub visible: bool,
    pub occluded: bool,
}

impl WorldSpacePromptVisibility {
    pub const fn visible() -> Self {
        Self {
            visible: true,
            occluded: false,
        }
    }

    pub const fn hidden() -> Self {
        Self {
            visible: false,
            occluded: false,
        }
    }

    pub const fn occluded() -> Self {
        Self {
            visible: false,
            occluded: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSpacePromptDataFeed {
    pub label: String,
    pub route_id: String,
}

impl WorldSpacePromptDataFeed {
    pub fn new(label: impl Into<String>, route_id: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            route_id: route_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldSpacePromptHostInput {
    pub anchor: WorldSpacePromptAnchor,
    pub lifetime: WorldSpacePromptLifetime,
    pub visibility: WorldSpacePromptVisibility,
    pub data_feed: WorldSpacePromptDataFeed,
}

impl WorldSpacePromptHostInput {
    pub fn new(
        anchor: WorldSpacePromptAnchor,
        lifetime: WorldSpacePromptLifetime,
        visibility: WorldSpacePromptVisibility,
        data_feed: WorldSpacePromptDataFeed,
    ) -> Self {
        Self {
            anchor,
            lifetime,
            visibility,
            data_feed,
        }
    }
}

pub trait WorldSpacePromptObservationFrame {
    fn prompt_input(&self) -> Option<WorldSpacePromptHostInput>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct DummyObservation;

    impl ObservationFrame<u64> for DummyObservation {
        fn selected_primary_item(&self) -> Option<u64> {
            Some(10)
        }

        fn is_item_available(&self, item_id: u64) -> bool {
            item_id == 10
        }
    }

    #[test]
    fn observation_frame_trait_provides_selected_and_availability_contract() {
        let frame = DummyObservation;
        assert_eq!(frame.selected_primary_item(), Some(10));
        assert!(frame.is_item_available(10));
        assert!(!frame.is_item_available(20));
    }

    #[test]
    fn world_space_prompt_observation_keeps_host_facts_without_ui_semantics() {
        struct PromptObservation;

        impl WorldSpacePromptObservationFrame for PromptObservation {
            fn prompt_input(&self) -> Option<WorldSpacePromptHostInput> {
                Some(WorldSpacePromptHostInput::new(
                    WorldSpacePromptAnchor::new(
                        WorldSpacePromptHostEntityId::new(42),
                        WorldSpacePromptAnchorPosition::new(1.0, 2.0, 3.0),
                    ),
                    WorldSpacePromptLifetime::Alive,
                    WorldSpacePromptVisibility::visible(),
                    WorldSpacePromptDataFeed::new("Open", "ui.world.prompt.open"),
                ))
            }
        }

        let input = PromptObservation
            .prompt_input()
            .expect("host should provide prompt facts");

        assert_eq!(input.anchor.entity_id.raw(), 42);
        assert_eq!(input.anchor.position.z, 3.0);
        assert_eq!(input.data_feed.label, "Open");
        assert_eq!(input.data_feed.route_id, "ui.world.prompt.open");
        assert_eq!(input.lifetime, WorldSpacePromptLifetime::Alive);
        assert_eq!(input.visibility, WorldSpacePromptVisibility::visible());
    }
}
