use runenui_core::{
    ChildLayout, HitContribution, HitContributionContext, PaintContribution,
    PaintContributionContext, SemanticContribution, SemanticContributionError, WidgetActivation,
    WidgetDiagnostic, WidgetMeasure, WidgetTextInput,
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
    IdentityExhausted,
    IndexIntegrityFailure,
    StatePayloadMismatch,
}

#[derive(Debug, Default)]
pub(crate) struct CapabilityCaches {
    pub(crate) activation: CachedCapability<WidgetActivation>,
    pub(crate) text_input: CachedCapability<WidgetTextInput>,
    pub(crate) measurement: CachedCapability<WidgetMeasure>,
    pub(crate) child_layout: CachedCapability<Option<ChildLayout>>,
    pub(crate) paint: CachedCapability<PaintContribution>,
    pub(crate) paint_context: Option<PaintContributionContext>,
    pub(crate) hit_test: CachedCapability<HitContribution>,
    pub(crate) hit_test_context: Option<HitContributionContext>,
    pub(crate) semantics: CachedSemanticContribution,
    pub(crate) diagnostics: CachedCapability<Vec<WidgetDiagnostic>>,
}
