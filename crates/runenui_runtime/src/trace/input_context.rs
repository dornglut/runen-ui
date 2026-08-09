use core::fmt;

use runenui_core::{CompositionGeneration, CompositionRange, InputDeviceId};

use super::{TraceDeliveryOutcome, TraceEventContext, TraceEventFamily, TracePayloadCapture};

/// Semantic role of the typed input payload stored by one trace record.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceInputRecordRole {
    Keyboard,
    CommittedText,
    CompositionIdentity,
    CompositionUpdate,
    CompositionCleanup,
}

/// Exact composition lifetime retained without exposing text or preedit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceCompositionContext {
    generation: CompositionGeneration,
    device_id: Option<InputDeviceId>,
}

impl TraceCompositionContext {
    pub(crate) const fn new(
        generation: CompositionGeneration,
        device_id: Option<InputDeviceId>,
    ) -> Self {
        Self {
            generation,
            device_id,
        }
    }

    /// Returns the opaque exact composition generation.
    #[must_use]
    pub const fn generation(&self) -> &CompositionGeneration {
        &self.generation
    }

    /// Returns the optional host-neutral device identity captured at composition start.
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }
}

/// Redacted size facts for committed text or composition preedit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceTextMetrics {
    bytes: usize,
    scalars: usize,
}

impl TraceTextMetrics {
    pub(crate) fn redacted(text: &str) -> Self {
        Self {
            bytes: text.len(),
            scalars: text.chars().count(),
        }
    }

    /// Returns the UTF-8 byte length without retaining text.
    #[must_use]
    pub const fn bytes(self) -> usize {
        self.bytes
    }

    /// Returns the Unicode scalar count without retaining text.
    #[must_use]
    pub const fn scalars(self) -> usize {
        self.scalars
    }
}

/// Redacted checked byte and scalar range into one composition preedit value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceCompositionRange {
    byte_start: usize,
    byte_end: usize,
    scalar_start: usize,
    scalar_end: usize,
}

impl TraceCompositionRange {
    pub(crate) fn from_validated(preedit: &str, range: CompositionRange) -> Self {
        let byte_start = range.start();
        let byte_end = range.end();
        let prefix = preedit
            .get(..byte_start)
            .unwrap_or_else(|| unreachable!("validated range start is a scalar boundary"));
        let selected_prefix = preedit
            .get(..byte_end)
            .unwrap_or_else(|| unreachable!("validated range end is a scalar boundary"));
        Self {
            byte_start,
            byte_end,
            scalar_start: prefix.chars().count(),
            scalar_end: selected_prefix.chars().count(),
        }
    }

    /// Returns the inclusive UTF-8 byte start.
    #[must_use]
    pub const fn byte_start(self) -> usize {
        self.byte_start
    }

    /// Returns the exclusive UTF-8 byte end.
    #[must_use]
    pub const fn byte_end(self) -> usize {
        self.byte_end
    }

    /// Returns the inclusive Unicode scalar start.
    #[must_use]
    pub const fn scalar_start(self) -> usize {
        self.scalar_start
    }

    /// Returns the exclusive Unicode scalar end.
    #[must_use]
    pub const fn scalar_end(self) -> usize {
        self.scalar_end
    }
}

#[derive(Clone, Eq, PartialEq)]
struct CapturedTraceText(Box<str>);

impl CapturedTraceText {
    fn from_policy(text: &str, capture: TracePayloadCapture) -> Option<Self> {
        matches!(capture, TracePayloadCapture::FullText).then(|| Self(text.into()))
    }

    const fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CapturedTraceText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapturedTraceText(..)")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TraceInputContextData {
    Keyboard {
        device_id: Option<InputDeviceId>,
    },
    CommittedText {
        device_id: Option<InputDeviceId>,
        metrics: TraceTextMetrics,
        captured: Option<CapturedTraceText>,
    },
    CompositionIdentity {
        composition: TraceCompositionContext,
    },
    CompositionUpdate {
        composition: TraceCompositionContext,
        metrics: TraceTextMetrics,
        range: Option<TraceCompositionRange>,
        captured: Option<CapturedTraceText>,
    },
    CompositionCleanup {
        composition: TraceCompositionContext,
        delivery: TraceDeliveryOutcome,
    },
}

/// Role-typed host-neutral input facts attached to one canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceInputContext {
    data: TraceInputContextData,
}

impl TraceInputContext {
    pub(crate) const fn keyboard(device_id: Option<InputDeviceId>) -> Self {
        Self {
            data: TraceInputContextData::Keyboard { device_id },
        }
    }

    pub(crate) fn committed_text(text: &str, device_id: Option<InputDeviceId>) -> Self {
        Self::committed_text_with_capture(text, device_id, TracePayloadCapture::Redacted)
    }

    pub(crate) fn committed_text_with_capture(
        text: &str,
        device_id: Option<InputDeviceId>,
        capture: TracePayloadCapture,
    ) -> Self {
        Self {
            data: TraceInputContextData::CommittedText {
                device_id,
                metrics: TraceTextMetrics::redacted(text),
                captured: CapturedTraceText::from_policy(text, capture),
            },
        }
    }

    pub(crate) const fn composition_identity(composition: TraceCompositionContext) -> Self {
        Self {
            data: TraceInputContextData::CompositionIdentity { composition },
        }
    }

    pub(crate) fn composition_update(
        composition: TraceCompositionContext,
        preedit: &str,
        range: Option<CompositionRange>,
    ) -> Self {
        Self::composition_update_with_capture(
            composition,
            preedit,
            range,
            TracePayloadCapture::Redacted,
        )
    }

    pub(crate) fn composition_update_with_capture(
        composition: TraceCompositionContext,
        preedit: &str,
        range: Option<CompositionRange>,
        capture: TracePayloadCapture,
    ) -> Self {
        Self {
            data: TraceInputContextData::CompositionUpdate {
                composition,
                metrics: TraceTextMetrics::redacted(preedit),
                range: range.map(|range| TraceCompositionRange::from_validated(preedit, range)),
                captured: CapturedTraceText::from_policy(preedit, capture),
            },
        }
    }

    pub(crate) const fn composition_cleanup(
        composition: TraceCompositionContext,
        delivery: TraceDeliveryOutcome,
    ) -> Self {
        Self {
            data: TraceInputContextData::CompositionCleanup {
                composition,
                delivery,
            },
        }
    }

    /// Returns the semantic role of these input facts.
    #[must_use]
    pub const fn role(&self) -> TraceInputRecordRole {
        match &self.data {
            TraceInputContextData::Keyboard { .. } => TraceInputRecordRole::Keyboard,
            TraceInputContextData::CommittedText { .. } => TraceInputRecordRole::CommittedText,
            TraceInputContextData::CompositionIdentity { .. } => {
                TraceInputRecordRole::CompositionIdentity
            }
            TraceInputContextData::CompositionUpdate { .. } => {
                TraceInputRecordRole::CompositionUpdate
            }
            TraceInputContextData::CompositionCleanup { .. } => {
                TraceInputRecordRole::CompositionCleanup
            }
        }
    }

    /// Returns the normalized routed-event family and cancelability implied by this input role.
    #[must_use]
    pub const fn event(&self) -> TraceEventContext {
        match &self.data {
            TraceInputContextData::Keyboard { .. } => {
                TraceEventContext::new(TraceEventFamily::Keyboard, true)
            }
            TraceInputContextData::CommittedText { .. } => {
                TraceEventContext::new(TraceEventFamily::CommittedText, true)
            }
            TraceInputContextData::CompositionIdentity { .. }
            | TraceInputContextData::CompositionUpdate { .. }
            | TraceInputContextData::CompositionCleanup { .. } => {
                TraceEventContext::new(TraceEventFamily::Composition, false)
            }
        }
    }

    /// Returns the optional host-neutral input device identity.
    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        match &self.data {
            TraceInputContextData::Keyboard { device_id }
            | TraceInputContextData::CommittedText { device_id, .. } => *device_id,
            TraceInputContextData::CompositionIdentity { composition }
            | TraceInputContextData::CompositionUpdate { composition, .. }
            | TraceInputContextData::CompositionCleanup { composition, .. } => {
                composition.device_id()
            }
        }
    }

    /// Returns exact composition lifetime identity for composition roles.
    #[must_use]
    pub const fn composition(&self) -> Option<&TraceCompositionContext> {
        match &self.data {
            TraceInputContextData::CompositionIdentity { composition }
            | TraceInputContextData::CompositionUpdate { composition, .. }
            | TraceInputContextData::CompositionCleanup { composition, .. } => Some(composition),
            TraceInputContextData::Keyboard { .. }
            | TraceInputContextData::CommittedText { .. } => None,
        }
    }

    /// Returns redacted committed-text or preedit metrics when the role owns them.
    #[must_use]
    pub const fn text_metrics(&self) -> Option<TraceTextMetrics> {
        match &self.data {
            TraceInputContextData::CommittedText { metrics, .. }
            | TraceInputContextData::CompositionUpdate { metrics, .. } => Some(*metrics),
            TraceInputContextData::Keyboard { .. }
            | TraceInputContextData::CompositionIdentity { .. }
            | TraceInputContextData::CompositionCleanup { .. } => None,
        }
    }

    /// Returns explicitly captured committed text or preedit without copying it.
    ///
    /// Default-redacted traces return `None` and never allocate a trace-owned
    /// payload copy.
    #[must_use]
    pub fn captured_text(&self) -> Option<&str> {
        match &self.data {
            TraceInputContextData::CommittedText { captured, .. }
            | TraceInputContextData::CompositionUpdate { captured, .. } => {
                captured.as_ref().map(CapturedTraceText::as_str)
            }
            TraceInputContextData::Keyboard { .. }
            | TraceInputContextData::CompositionIdentity { .. }
            | TraceInputContextData::CompositionCleanup { .. } => None,
        }
    }

    /// Returns the checked redacted composition range for an update that supplied one.
    #[must_use]
    pub const fn composition_range(&self) -> Option<TraceCompositionRange> {
        match &self.data {
            TraceInputContextData::CompositionUpdate { range, .. } => *range,
            _ => None,
        }
    }

    /// Returns explicit cleanup delivery/suppression for cleanup records.
    #[must_use]
    pub const fn delivery(&self) -> Option<TraceDeliveryOutcome> {
        match &self.data {
            TraceInputContextData::CompositionCleanup { delivery, .. } => Some(*delivery),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::__runtime::RuntimeNamespace;

    use super::{
        TraceCompositionContext, TraceInputContext, TraceInputRecordRole, TraceTextMetrics,
    };
    use crate::{TraceDeliveryOutcome, TracePayloadCapture};

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn committed_text_redaction_never_retains_a_payload_copy() {
        let context = TraceInputContext::committed_text("hé", None);

        assert_eq!(context.role(), TraceInputRecordRole::CommittedText);
        assert_eq!(
            context.text_metrics(),
            Some(TraceTextMetrics::redacted("hé"))
        );
        assert_eq!(context.text_metrics().map(TraceTextMetrics::bytes), Some(3));
        assert_eq!(
            context.text_metrics().map(TraceTextMetrics::scalars),
            Some(2)
        );
        assert_eq!(context.captured_text(), None);
        assert_eq!(context.composition(), None);
        assert_eq!(context.delivery(), None);
        assert!(!format!("{context:?}").contains("hé"));
    }

    #[test]
    fn explicit_full_text_policy_retains_payload_without_debug_formatting_it() {
        let context = TraceInputContext::committed_text_with_capture(
            "hé",
            None,
            TracePayloadCapture::FullText,
        );
        assert_eq!(context.captured_text(), Some("hé"));
        assert!(!format!("{context:?}").contains("hé"));
    }

    #[test]
    fn trace_input_context_remains_send_and_sync() {
        assert_send_sync::<TraceInputContext>();
    }

    #[test]
    fn composition_cleanup_has_one_explicit_role_and_outcome() {
        let namespace = RuntimeNamespace::__runtime_new();
        let generation = namespace.__runtime_composition_generation(7);
        let context = TraceInputContext::composition_cleanup(
            TraceCompositionContext::new(generation, None),
            TraceDeliveryOutcome::Suppressed,
        );

        assert_eq!(context.role(), TraceInputRecordRole::CompositionCleanup);
        assert_eq!(context.delivery(), Some(TraceDeliveryOutcome::Suppressed));
        assert_eq!(
            context
                .composition()
                .map(|composition| composition.generation().get()),
            Some(7)
        );
        assert_eq!(context.text_metrics(), None);
        assert_eq!(context.captured_text(), None);
        assert_eq!(context.composition_range(), None);
    }
}
