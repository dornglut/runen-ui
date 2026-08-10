use runenui_core::{
    ChildLayout, SemanticContribution, SemanticContributionError, WidgetActivation,
    WidgetDiagnostic, WidgetMeasure, WidgetPaintProof, WidgetTextInput,
};

#[derive(Clone, Debug, Default)]
pub(crate) enum CachedCapability<T> {
    #[default]
    Unresolved,
    Ready(T),
    StatePayloadMismatch,
}

impl<T: Clone> CachedCapability<T> {
    pub(crate) fn ready(&self) -> Option<T> {
        match self {
            Self::Ready(value) => Some(value.clone()),
            Self::Unresolved | Self::StatePayloadMismatch => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) enum CachedSemanticContribution {
    #[default]
    Unresolved,
    Ready(SemanticContribution),
    Invalid(SemanticContributionError),
    StatePayloadMismatch,
}

impl CachedSemanticContribution {
    pub(crate) fn ready(&self) -> Option<SemanticContribution> {
        match self {
            Self::Ready(value) => Some(value.clone()),
            Self::Invalid(error) => {
                let _ = error;
                None
            }
            Self::Unresolved | Self::StatePayloadMismatch => None,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CapabilityCaches {
    pub(crate) activation: CachedCapability<WidgetActivation>,
    pub(crate) text_input: CachedCapability<WidgetTextInput>,
    pub(crate) measurement: CachedCapability<WidgetMeasure>,
    pub(crate) child_layout: CachedCapability<Option<ChildLayout>>,
    pub(crate) paint: CachedCapability<WidgetPaintProof>,
    pub(crate) semantics: CachedSemanticContribution,
    pub(crate) diagnostics: CachedCapability<Vec<WidgetDiagnostic>>,
}
