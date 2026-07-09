//! Proof-local ECS host state and mutation evidence.

use serde::{Deserialize, Serialize};

use crate::bridge::UiAppResolvedAction;
use crate::ids::UiAppActionId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counter {
    pub count: u32,
}

impl Counter {
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

impl ecs::Resource for Counter {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CounterAction {
    Increment,
    Reset,
}

impl CounterAction {
    pub fn from_action_id(action_id: &UiAppActionId) -> Option<Self> {
        match action_id.as_str() {
            "counter.increment" => Some(Self::Increment),
            "counter.reset" => Some(Self::Reset),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppHostSnapshot {
    pub count: u32,
    pub active_screen: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAppHostMutationStatus {
    Applied,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppHostMutation {
    pub action_id: UiAppActionId,
    pub before: UiAppHostSnapshot,
    pub after: UiAppHostSnapshot,
    pub status: UiAppHostMutationStatus,
}

pub type UiAppHostMutationReport = UiAppHostMutation;

pub struct CounterHost {
    world: ecs::World,
}

impl CounterHost {
    pub fn new(count: u32) -> Self {
        let mut world = ecs::World::new();
        world.insert_resource(Counter::new(count));
        Self { world }
    }

    pub fn count(&self) -> u32 {
        self.world
            .resource::<Counter>()
            .expect("counter resource must exist in proof host")
            .count
    }

    pub fn active_screen(&self) -> &'static str {
        if self.count() >= 5 {
            "counter.win"
        } else {
            "counter.screen"
        }
    }

    pub fn snapshot(&self) -> UiAppHostSnapshot {
        UiAppHostSnapshot {
            count: self.count(),
            active_screen: self.active_screen().to_owned(),
        }
    }

    pub fn apply_resolved_action(
        &mut self,
        resolved_action: &UiAppResolvedAction,
    ) -> Option<UiAppHostMutation> {
        let action = CounterAction::from_action_id(&resolved_action.action_id)?;
        let before = self.snapshot();
        {
            let counter = self
                .world
                .resource_mut::<Counter>()
                .expect("counter resource must exist in proof host");
            match action {
                CounterAction::Increment => {
                    counter.count = counter.count.saturating_add(1);
                }
                CounterAction::Reset => {
                    counter.count = 0;
                }
            }
        }
        let after = self.snapshot();
        Some(UiAppHostMutation {
            action_id: resolved_action.action_id.clone(),
            before,
            after,
            status: UiAppHostMutationStatus::Applied,
        })
    }

    pub fn reject_without_mutation(&self, action_id: UiAppActionId) -> UiAppHostMutation {
        let snapshot = self.snapshot();
        UiAppHostMutation {
            action_id,
            before: snapshot.clone(),
            after: snapshot,
            status: UiAppHostMutationStatus::Rejected,
        }
    }
}
