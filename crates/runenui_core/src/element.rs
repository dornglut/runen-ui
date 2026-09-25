//! Transient element and view authoring.

use core::fmt;
use std::rc::Rc;

use crate::widget_erasure::{ElementParts, ErasedWidget, MountedWidget, WidgetAdapter};
use crate::widget_mapping::MappedWidget;
use crate::widget_protocol::Widget;
use crate::{
    BrushValue, ColorValue, ElementId, ElementKey, ExplicitTimeline, FocusScope, Focusability,
    IdentifierError, IntoElementId, IntoElementKey, LayoutStyle, MotionTarget, OpacityValue,
    OutlineValue, PresentationValue, RadiusValue, ShadowValue, SpacingValue, StyleIntent,
    StyleRecipeId, StyleVariantId, TransitionSpec, TypographyValue,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CommonNodeAuthoring {
    pub(crate) id: Option<ElementId>,
    pub(crate) key: Option<ElementKey>,
    pub(crate) layout: LayoutStyle,
    pub(crate) style: StyleIntent,
    pub(crate) timelines: Vec<ExplicitTimeline>,
    pub(crate) diagnostics: Vec<AuthoringDiagnostic>,
}

impl Default for CommonNodeAuthoring {
    fn default() -> Self {
        Self {
            id: None,
            key: None,
            layout: LayoutStyle::default(),
            style: StyleIntent::EMPTY,
            timelines: Vec::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl CommonNodeAuthoring {
    pub(crate) fn from_authored_fields(
        fields: AuthoredElementFields,
        diagnostics: Vec<AuthoringDiagnostic>,
    ) -> (Self, Focusability, Option<FocusScope>) {
        (
            Self {
                id: fields.id,
                key: fields.key,
                layout: fields.layout,
                style: fields.style,
                timelines: fields.timelines,
                diagnostics,
            },
            fields.focusability,
            fields.focus_scope,
        )
    }

    pub(crate) fn into_authored_fields(
        self,
        focusability: Focusability,
        focus_scope: Option<FocusScope>,
    ) -> (AuthoredElementFields, Vec<AuthoringDiagnostic>) {
        (
            AuthoredElementFields::new(
                self.id,
                self.key,
                self.layout,
                self.style,
                self.timelines,
                focusability,
                focus_scope,
            ),
            self.diagnostics,
        )
    }

    pub(crate)
