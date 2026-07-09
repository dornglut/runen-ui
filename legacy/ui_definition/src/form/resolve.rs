use crate::{
    UiAvailability, UiAvailabilityBinding, UiAxisDefinition, UiCollectionItem, UiValue,
    UiValueBinding,
};
use ui_math::Axis;

use super::context::UiDefinitionContext;

pub(super) fn resolve_value(binding: &UiValueBinding, context: &UiDefinitionContext) -> UiValue {
    match binding {
        UiValueBinding::Static(value) => value.clone(),
        UiValueBinding::Slot(slot) => context
            .values
            .get(slot)
            .cloned()
            .unwrap_or_else(|| UiValue::Text(String::new())),
    }
}

pub(super) fn resolve_item_text(
    item: &UiCollectionItem,
    binding: &UiValueBinding,
    context: &UiDefinitionContext,
) -> String {
    match binding {
        UiValueBinding::Static(value) => value.as_text(),
        UiValueBinding::Slot(slot) => item
            .values
            .get(slot)
            .or_else(|| context.values.get(slot))
            .cloned()
            .unwrap_or_else(|| UiValue::Text(String::new()))
            .as_text(),
    }
}

pub(super) fn resolve_text(binding: &UiValueBinding, context: &UiDefinitionContext) -> String {
    resolve_value(binding, context).as_text()
}

pub(super) fn resolve_availability(
    binding: Option<&UiAvailabilityBinding>,
    context: &UiDefinitionContext,
) -> UiAvailability {
    match binding {
        Some(UiAvailabilityBinding::Static(value)) => value.clone(),
        Some(UiAvailabilityBinding::Ref(id)) => context
            .availability
            .get(id)
            .cloned()
            .or_else(|| context.values.get(id).and_then(UiValue::as_availability))
            .or_else(|| {
                context
                    .values
                    .get(id)
                    .and_then(UiValue::as_bool)
                    .map(|enabled| {
                        if enabled {
                            UiAvailability::Available
                        } else {
                            UiAvailability::Disabled {
                                reason: format!("availability '{}' is false", id),
                            }
                        }
                    })
            })
            .unwrap_or(UiAvailability::Unavailable {
                reason: format!("unresolved availability '{}'", id),
            }),
        None => UiAvailability::Available,
    }
}

pub(super) fn axis_to_runtime(axis: UiAxisDefinition) -> Axis {
    match axis {
        UiAxisDefinition::Horizontal => Axis::Horizontal,
        UiAxisDefinition::Vertical => Axis::Vertical,
    }
}
