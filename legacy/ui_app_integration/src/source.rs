//! Minimal code-authored source helpers for the proof bridge.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ui_controls::BUTTON_CONTROL_KIND_ID;
use ui_definition::{
    AuthoredControlAccessibilityDefinition, AuthoredControlKindId, AuthoredControlValue,
    AuthoredId, AuthoredRouteId, UiNodeDefinition, UiRouteSlotRef, UiValueBinding,
};
use ui_program::RouteId;

use crate::ids::UiAppScreenId;
use crate::screen::{UiAppScreenDescriptor, UiAppScreenRoute};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppSourceNodeRef {
    pub node_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppSourceRouteRef {
    pub slot_id: String,
    pub route: RouteId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiAppSourceBuildReport {
    pub screen_id: UiAppScreenId,
    pub root_node_id: String,
    pub nodes: Vec<UiAppSourceNodeRef>,
    pub routes: Vec<UiAppSourceRouteRef>,
    pub root: UiNodeDefinition,
}

impl UiAppSourceBuildReport {
    pub fn node_ids(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .map(|node| node.node_id.as_str())
            .collect()
    }

    pub fn output_text_facts(&self) -> Vec<String> {
        let mut facts = Vec::new();
        collect_text_facts(&self.root, &mut facts);
        facts
    }
}

#[derive(Clone, Debug, Default)]
pub struct UiAppSourceBuilder;

impl UiAppSourceBuilder {
    pub fn counter_screen(count: u32) -> UiAppSourceBuildReport {
        let screen_id = UiAppScreenId::new("counter.screen");
        let route = RouteId::new("counter.increment");
        let root_id = AuthoredId::new("counter.root");
        let label_id = AuthoredId::new("counter.count_label");
        let button_id = AuthoredId::new("counter.increment_button");
        let route_slot_id = AuthoredId::new("counter.increment");

        let root = UiNodeDefinition::Column {
            id: root_id.clone(),
            children: vec![
                UiNodeDefinition::Label {
                    id: label_id.clone(),
                    label: UiValueBinding::static_text(format!("Clicked {count} / 5")),
                    availability: None,
                },
                button_control(button_id.clone(), "Click me", "counter.increment"),
            ],
        };

        UiAppSourceBuildReport {
            screen_id,
            root_node_id: root_id.as_str().to_owned(),
            nodes: vec![
                UiAppSourceNodeRef {
                    node_id: root_id.as_str().to_owned(),
                },
                UiAppSourceNodeRef {
                    node_id: label_id.as_str().to_owned(),
                },
                UiAppSourceNodeRef {
                    node_id: button_id.as_str().to_owned(),
                },
            ],
            routes: vec![UiAppSourceRouteRef {
                slot_id: route_slot_id.as_str().to_owned(),
                route,
            }],
            root,
        }
    }

    pub fn win_screen() -> UiAppSourceBuildReport {
        let screen_id = UiAppScreenId::new("counter.win");
        let route = RouteId::new("counter.reset");
        let root_id = AuthoredId::new("counter.win_root");
        let label_id = AuthoredId::new("counter.win_label");
        let button_id = AuthoredId::new("counter.reset_button");
        let route_slot_id = AuthoredId::new("counter.reset");

        let root = UiNodeDefinition::Column {
            id: root_id.clone(),
            children: vec![
                UiNodeDefinition::Label {
                    id: label_id.clone(),
                    label: UiValueBinding::static_text("You win!"),
                    availability: None,
                },
                button_control(button_id.clone(), "Reset", "counter.reset"),
            ],
        };

        UiAppSourceBuildReport {
            screen_id,
            root_node_id: root_id.as_str().to_owned(),
            nodes: vec![
                UiAppSourceNodeRef {
                    node_id: root_id.as_str().to_owned(),
                },
                UiAppSourceNodeRef {
                    node_id: label_id.as_str().to_owned(),
                },
                UiAppSourceNodeRef {
                    node_id: button_id.as_str().to_owned(),
                },
            ],
            routes: vec![UiAppSourceRouteRef {
                slot_id: route_slot_id.as_str().to_owned(),
                route,
            }],
            root,
        }
    }

    pub fn descriptor(report: &UiAppSourceBuildReport) -> UiAppScreenDescriptor {
        report.routes.iter().fold(
            UiAppScreenDescriptor::new(report.screen_id.clone(), report.root.clone()),
            |descriptor, route| {
                descriptor.with_route(UiAppScreenRoute {
                    slot: UiRouteSlotRef {
                        id: AuthoredId::new(route.slot_id.clone()),
                    },
                    route: route.route.clone(),
                })
            },
        )
    }
}

fn button_control(id: AuthoredId, label: &str, route: &str) -> UiNodeDefinition {
    let mut properties = BTreeMap::new();
    properties.insert(
        "label".to_owned(),
        AuthoredControlValue::String(label.to_owned()),
    );

    UiNodeDefinition::Control {
        id,
        kind: AuthoredControlKindId::new(BUTTON_CONTROL_KIND_ID),
        properties,
        bindings: BTreeMap::new(),
        route: Some(AuthoredRouteId::new(route)),
        accessibility: Some(AuthoredControlAccessibilityDefinition {
            role: "button".to_owned(),
            label: Some(label.to_owned()),
        }),
        children: Vec::new(),
    }
}

fn collect_text_facts(node: &UiNodeDefinition, facts: &mut Vec<String>) {
    match node {
        UiNodeDefinition::Label { label, .. } | UiNodeDefinition::Button { label, .. } => {
            if let Some(text) = static_text(label) {
                facts.push(text);
            }
        }
        UiNodeDefinition::Control {
            properties,
            children,
            ..
        } => {
            if let Some(AuthoredControlValue::String(label)) = properties.get("label") {
                facts.push(label.clone());
            }
            for child in children {
                collect_text_facts(child, facts);
            }
            return;
        }
        _ => {}
    }

    for child in node.children() {
        collect_text_facts(child, facts);
    }
}

fn static_text(binding: &UiValueBinding) -> Option<String> {
    match binding {
        UiValueBinding::Static(value) => Some(value.as_text()),
        UiValueBinding::Slot(_) => None,
    }
}
