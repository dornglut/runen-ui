use crate::{
    AuthoredUiNodePath, UiAvailability, UiAvailabilityId, UiCollectionItem, UiCollectionSlotId,
    UiEmbedSlotId, UiMenuSlotId, UiSelectionSlotId, UiValue, UiValueSlotId,
};
use std::collections::BTreeMap;
use ui_theme::ThemeTokens;
use ui_tree::WidgetId;

#[derive(Debug, Clone)]
pub struct UiDefinitionContext {
    pub theme: ThemeTokens,
    pub values: BTreeMap<UiValueSlotId, UiValue>,
    pub collections: BTreeMap<UiCollectionSlotId, Vec<UiCollectionItem>>,
    pub selections: BTreeMap<UiSelectionSlotId, String>,
    pub availability: BTreeMap<UiAvailabilityId, UiAvailability>,
    pub widget_ids_by_path: BTreeMap<AuthoredUiNodePath, WidgetId>,
    pub embed_slots: BTreeMap<UiEmbedSlotId, u16>,
    pub menus: BTreeMap<UiMenuSlotId, Vec<UiCollectionItem>>,
    pub next_widget_id: u64,
    pub widget_id_scope: Option<WidgetIdScope>,
}

impl UiDefinitionContext {
    pub fn new(theme: ThemeTokens) -> Self {
        Self {
            theme,
            values: BTreeMap::new(),
            collections: BTreeMap::new(),
            selections: BTreeMap::new(),
            availability: BTreeMap::new(),
            widget_ids_by_path: BTreeMap::new(),
            embed_slots: BTreeMap::new(),
            menus: BTreeMap::new(),
            next_widget_id: 1_000_000,
            widget_id_scope: None,
        }
    }

    pub fn with_widget_id_scope(mut self, scope: WidgetIdScope) -> Self {
        self.widget_id_scope = Some(scope);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetIdScope {
    base: u64,
}

impl WidgetIdScope {
    pub const fn new(base: u64) -> Self {
        Self { base }
    }

    pub fn scoped_widget_id(self, local_id: u64) -> WidgetId {
        WidgetId(self.base.saturating_add(local_id))
    }
}
