//! Retained UI formation target.

mod collections;
mod containers;
mod context;
mod controls;
mod dispatch;
mod resolve;
mod scroll;
mod slots;
mod state;

pub use context::{UiDefinitionContext, WidgetIdScope};

use crate::{
    AuthoredUiNodePath, FormedInteractionModel, FormedUiEmbed, NormalizedUiTemplate, UiAvailability,
};
use dispatch::form_node;
use std::collections::BTreeMap;
use ui_tree::{UiNode, WidgetId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormedUiRoute {
    RouteSlot(crate::UiRouteSlotId),
    CollectionItemRoute {
        collection: crate::UiCollectionSlotId,
        item_key: String,
        route: crate::UiRouteSlotId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormedRetainedUiProduct {
    pub root: UiNode,
    pub routes_by_widget_id: BTreeMap<WidgetId, FormedUiRoute>,
    pub paths_by_widget_id: BTreeMap<WidgetId, AuthoredUiNodePath>,
    pub embeds_by_widget_id: BTreeMap<WidgetId, FormedUiEmbed>,
    pub diagnostics: Vec<crate::UiDefinitionDiagnostic>,
    pub availability_by_widget_id: BTreeMap<WidgetId, UiAvailability>,
    pub interaction_model: FormedInteractionModel,
}

pub fn form_retained_ui(
    template: &NormalizedUiTemplate,
    context: &mut UiDefinitionContext,
) -> FormedRetainedUiProduct {
    let mut state = state::FormationState::default();
    state.diagnostics.extend(template.diagnostics.clone());
    let root_path = AuthoredUiNodePath::root(template.root.id());
    let root = form_node(&template.root, &root_path, template, context, &mut state);
    FormedRetainedUiProduct {
        root,
        routes_by_widget_id: state.routes_by_widget_id,
        paths_by_widget_id: state.paths_by_widget_id,
        embeds_by_widget_id: state.embeds_by_widget_id,
        diagnostics: state.diagnostics,
        availability_by_widget_id: state.availability_by_widget_id,
        interaction_model: state.interaction_model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AuthoredUiNodePath, AuthoredUiTemplate, FormedScrollOwner, UiAvailabilityBinding,
        UiAxisDefinition, UiCollectionItem, UiCollectionSlotRef, UiNodeDefinition, UiNodeId,
        UiRouteSlotRef, UiScrollAxisDefinition, UiScrollBoundaryPolicyDefinition,
        UiScrollInputDefinition, UiScrollInputPolicyDefinition, UiScrollOwnershipDefinition,
        UiValue, UiValueBinding,
    };
    use std::collections::BTreeSet;
    use ui_layout::SizePolicy;
    use ui_math::Axis;
    use ui_theme::ThemeTokens;
    use ui_tree::{UiNodeKind, WidgetId};
    use ui_widgets::{ScrollInputPolicies, ScrollInputPolicy};

    #[test]
    fn disabled_button_forms_without_route() {
        let template = AuthoredUiTemplate {
            id: "test.toolbar".into(),
            root: UiNodeDefinition::Button {
                id: UiNodeId::from("root"),
                label: UiValueBinding::static_text("Add"),
                route: Some(UiRouteSlotRef {
                    id: "route.add".into(),
                }),
                availability: Some(UiAvailabilityBinding::Static(UiAvailability::Disabled {
                    reason: "not implemented".to_string(),
                })),
                selected: None,
            },
            templates: Vec::new(),
            menus: Vec::new(),
        };
        let normalized = crate::normalize_authored_template(template);
        let mut context = UiDefinitionContext::new(ThemeTokens::default());
        let product = form_retained_ui(&normalized, &mut context);
        assert!(product.routes_by_widget_id.is_empty());
        assert!(matches!(product.root.kind, UiNodeKind::Button(_)));
    }

    #[test]
    fn two_axis_scroll_forms_with_explicit_input_policy() {
        let template = AuthoredUiTemplate {
            id: "test.scroll".into(),
            root: UiNodeDefinition::Scroll {
                id: UiNodeId::from("root"),
                axis: UiScrollAxisDefinition::Both,
                input: UiScrollInputDefinition {
                    horizontal: UiScrollInputPolicyDefinition::MiddleDragOnly,
                    vertical: UiScrollInputPolicyDefinition::WheelOnly,
                },
                ownership: UiScrollOwnershipDefinition {
                    boundary: UiScrollBoundaryPolicyDefinition::ConsumeAtBoundary,
                },
                children: vec![UiNodeDefinition::Label {
                    id: "entry".into(),
                    label: UiValueBinding::static_text("line"),
                    availability: None,
                }],
            },
            templates: Vec::new(),
            menus: Vec::new(),
        };
        let normalized = crate::normalize_authored_template(template);
        let mut context = UiDefinitionContext::new(ThemeTokens::default());

        let product = form_retained_ui(&normalized, &mut context);

        let UiNodeKind::Scroll(scroll) = product.root.kind else {
            panic!("scroll definition should form a retained scroll node");
        };
        assert_eq!(scroll.axes, ui_widgets::ScrollAxes::Both);
        assert_eq!(
            scroll.input_policies,
            ScrollInputPolicies::new(
                ScrollInputPolicy::MiddleDragOnly,
                ScrollInputPolicy::WheelOnly,
            )
        );
        assert_eq!(
            product.interaction_model.scroll_owners,
            vec![FormedScrollOwner {
                widget_id: product.root.id,
                axes: vec![Axis::Horizontal, Axis::Vertical],
                boundary: UiScrollBoundaryPolicyDefinition::ConsumeAtBoundary,
            }]
        );
    }

    #[test]
    fn unavailable_children_are_omitted_while_disabled_children_remain_non_interactive() {
        let template = AuthoredUiTemplate {
            id: "test.availability".into(),
            root: UiNodeDefinition::Column {
                id: "root".into(),
                children: vec![
                    UiNodeDefinition::Label {
                        id: "available".into(),
                        label: UiValueBinding::static_text("Available"),
                        availability: None,
                    },
                    UiNodeDefinition::Button {
                        id: "disabled".into(),
                        label: UiValueBinding::static_text("Disabled"),
                        route: Some(UiRouteSlotRef {
                            id: "route.disabled".into(),
                        }),
                        availability: Some(UiAvailabilityBinding::Static(
                            UiAvailability::Disabled {
                                reason: "fixture disabled".to_string(),
                            },
                        )),
                        selected: None,
                    },
                    UiNodeDefinition::Button {
                        id: "unavailable".into(),
                        label: UiValueBinding::static_text("Unavailable"),
                        route: Some(UiRouteSlotRef {
                            id: "route.unavailable".into(),
                        }),
                        availability: Some(UiAvailabilityBinding::Ref(
                            "availability.unavailable".into(),
                        )),
                        selected: None,
                    },
                ],
            },
            templates: Vec::new(),
            menus: Vec::new(),
        };
        let normalized = crate::normalize_authored_template(template);
        let mut context = UiDefinitionContext::new(ThemeTokens::default());
        context.values.insert(
            "availability.unavailable".into(),
            UiValue::Availability(UiAvailability::Unavailable {
                reason: "wrong control kind".to_string(),
            }),
        );

        let product = form_retained_ui(&normalized, &mut context);
        let formed_paths = product
            .paths_by_widget_id
            .values()
            .map(AuthoredUiNodePath::as_str)
            .collect::<BTreeSet<_>>();

        assert!(formed_paths.contains("root/available"));
        assert!(formed_paths.contains("root/disabled"));
        assert!(!formed_paths.contains("root/unavailable"));
        assert!(
            product.routes_by_widget_id.is_empty(),
            "disabled and unavailable nodes must not expose route entries"
        );
    }

    #[test]
    fn vertical_separator_forms_fixed_length_divider() {
        let template = AuthoredUiTemplate {
            id: "test.separator".into(),
            root: UiNodeDefinition::Separator {
                id: UiNodeId::from("root"),
                axis: Some(UiAxisDefinition::Vertical),
                length: Some(18.0),
                thickness: Some(1.0),
            },
            templates: Vec::new(),
            menus: Vec::new(),
        };
        let normalized = crate::normalize_authored_template(template);
        let mut context = UiDefinitionContext::new(ThemeTokens::default());

        let product = form_retained_ui(&normalized, &mut context);

        let UiNodeKind::Divider(divider) = product.root.kind else {
            panic!("separator should form a divider node");
        };
        assert_eq!(divider.axis, Axis::Vertical);
        assert_eq!(divider.thickness, 1.0);
        assert_eq!(divider.length_policy, SizePolicy::Fixed(18.0));
    }

    #[test]
    fn repeat_children_use_source_map_paths_under_repeat_node() {
        let template = AuthoredUiTemplate {
            id: "test.repeat".into(),
            root: UiNodeDefinition::Column {
                id: "root".into(),
                children: vec![UiNodeDefinition::Repeat {
                    id: "rows".into(),
                    items: UiCollectionSlotRef {
                        id: "test.rows".into(),
                    },
                    template: "test.repeat.row".into(),
                    axis: Some(UiAxisDefinition::Vertical),
                }],
            },
            templates: vec![AuthoredUiTemplate {
                id: "test.repeat.row".into(),
                root: UiNodeDefinition::Label {
                    id: "entry".into(),
                    label: UiValueBinding::static_text("row"),
                    availability: None,
                },
                templates: Vec::new(),
                menus: Vec::new(),
            }],
            menus: Vec::new(),
        };
        let normalized = crate::normalize_authored_template(template);
        let mut context = UiDefinitionContext::new(ThemeTokens::default());
        context.collections.insert(
            "test.rows".into(),
            vec![
                UiCollectionItem::new("a", "A"),
                UiCollectionItem::new("b", "B"),
            ],
        );

        let product = form_retained_ui(&normalized, &mut context);
        let formed_paths = product
            .paths_by_widget_id
            .values()
            .map(AuthoredUiNodePath::as_str)
            .collect::<BTreeSet<_>>();

        assert!(formed_paths.contains("root/rows[a]/entry"));
        assert!(formed_paths.contains("root/rows[b]/entry"));
        assert!(!formed_paths.contains("root/rows/rows[a]/entry"));
    }

    #[test]
    fn generated_widget_ids_are_scoped_during_formation() {
        let template = AuthoredUiTemplate {
            id: "test.scoped".into(),
            root: UiNodeDefinition::Column {
                id: "root".into(),
                children: vec![UiNodeDefinition::Button {
                    id: "action".into(),
                    label: UiValueBinding::static_text("Action"),
                    route: Some(UiRouteSlotRef {
                        id: "route.action".into(),
                    }),
                    availability: None,
                    selected: None,
                }],
            },
            templates: Vec::new(),
            menus: Vec::new(),
        };
        let normalized = crate::normalize_authored_template(template);
        let mut first = UiDefinitionContext::new(ThemeTokens::default())
            .with_widget_id_scope(WidgetIdScope::new(10_000_000));
        let mut second = UiDefinitionContext::new(ThemeTokens::default())
            .with_widget_id_scope(WidgetIdScope::new(20_000_000));

        let first_product = form_retained_ui(&normalized, &mut first);
        let second_product = form_retained_ui(&normalized, &mut second);

        assert!(first_product.diagnostics.is_empty());
        assert!(second_product.diagnostics.is_empty());
        assert_eq!(first_product.root.id, WidgetId(11_000_000));
        assert_eq!(second_product.root.id, WidgetId(21_000_000));
        assert_ne!(first_product.root.id, second_product.root.id);
        assert_ne!(
            first_product.routes_by_widget_id.keys().next(),
            second_product.routes_by_widget_id.keys().next(),
            "identical authored surfaces must not collide after scoped formation",
        );
    }
}
