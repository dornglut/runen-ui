//! File: domain/ui/ui_surface/src/ratification.rs
//! Purpose: Intent ratification adapter contracts at host/domain boundaries.
use diagnostics::DiagnosticSink;

use crate::{SurfaceCapability, SurfaceIntent, missing_capability_diagnostic};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RatificationOutcome {
    Applied,
    Ignored,
}

pub trait RatificationAdapter {
    type Error;

    fn has_capability(&self, capability: SurfaceCapability) -> bool;

    fn ratify_intent(&mut self, intent: SurfaceIntent) -> Result<RatificationOutcome, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatificationDispatchError<E> {
    MissingCapability(SurfaceCapability),
    Adapter(E),
}

pub fn ratify_surface_intent<A>(
    adapter: &mut A,
    intent: SurfaceIntent,
) -> Result<RatificationOutcome, RatificationDispatchError<A::Error>>
where
    A: RatificationAdapter,
{
    if !adapter.has_capability(intent.required_capability) {
        return Err(RatificationDispatchError::MissingCapability(
            intent.required_capability,
        ));
    }

    adapter
        .ratify_intent(intent)
        .map_err(RatificationDispatchError::Adapter)
}

pub fn ratify_surface_intent_with_diagnostics<A, S>(
    adapter: &mut A,
    intent: SurfaceIntent,
    sink: &mut S,
) -> Result<RatificationOutcome, RatificationDispatchError<A::Error>>
where
    A: RatificationAdapter,
    S: DiagnosticSink,
{
    if !adapter.has_capability(intent.required_capability) {
        sink.emit(missing_capability_diagnostic(
            intent.surface_instance_id,
            intent.required_capability,
        ));

        return Err(RatificationDispatchError::MissingCapability(
            intent.required_capability,
        ));
    }

    adapter
        .ratify_intent(intent)
        .map_err(RatificationDispatchError::Adapter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SurfaceInstanceId, SurfaceIntent, SurfaceIntentKind};

    #[derive(Default)]
    struct DummyAdapter {
        allow_request_mutation: bool,
        last_intent: Option<SurfaceIntent>,
    }

    impl RatificationAdapter for DummyAdapter {
        type Error = &'static str;

        fn has_capability(&self, capability: SurfaceCapability) -> bool {
            matches!(
                (capability, self.allow_request_mutation),
                (SurfaceCapability::RequestMutation, true)
            )
        }

        fn ratify_intent(
            &mut self,
            intent: SurfaceIntent,
        ) -> Result<RatificationOutcome, Self::Error> {
            self.last_intent = Some(intent);
            Ok(RatificationOutcome::Applied)
        }
    }

    #[test]
    fn ratification_fails_closed_when_required_capability_missing() {
        let mut adapter = DummyAdapter::default();
        let intent = SurfaceIntent::select_primary_item(SurfaceInstanceId::new(3), 9);

        let result = ratify_surface_intent(&mut adapter, intent);

        assert_eq!(
            result,
            Err(RatificationDispatchError::MissingCapability(
                SurfaceCapability::RequestMutation
            ))
        );
        assert!(adapter.last_intent.is_none());
    }

    #[test]
    fn ratification_dispatches_intent_when_capability_exists() {
        let mut adapter = DummyAdapter {
            allow_request_mutation: true,
            last_intent: None,
        };
        let intent = SurfaceIntent::select_primary_item(SurfaceInstanceId::new(5), 12);

        let result = ratify_surface_intent(&mut adapter, intent);

        assert_eq!(result, Ok(RatificationOutcome::Applied));
        assert_eq!(
            adapter.last_intent,
            Some(SurfaceIntent {
                surface_instance_id: SurfaceInstanceId::new(5),
                required_capability: SurfaceCapability::RequestMutation,
                kind: SurfaceIntentKind::SelectPrimaryItem { item_id: 12 },
            })
        );
    }

    #[test]
    fn ratification_with_diagnostics_emits_missing_capability_diagnostic() {
        let mut adapter = DummyAdapter::default();
        let mut report = diagnostics::DiagnosticReport::new();
        let intent = SurfaceIntent::select_primary_item(SurfaceInstanceId::new(3), 9);

        let result = ratify_surface_intent_with_diagnostics(&mut adapter, intent, &mut report);

        assert_eq!(
            result,
            Err(RatificationDispatchError::MissingCapability(
                SurfaceCapability::RequestMutation
            ))
        );

        assert_eq!(report.len(), 1);

        let diagnostic = &report.diagnostics()[0];

        assert_eq!(
            diagnostic.code().as_str(),
            "ui_surface.ratification.missing_capability"
        );
        assert_eq!(diagnostic.domain().as_str(), "ui_surface");
        assert_eq!(diagnostic.severity(), diagnostics::Severity::Error);
        assert_eq!(
            diagnostic.subject().unwrap().kind().as_str(),
            "surface_instance"
        );
        assert_eq!(diagnostic.subject().unwrap().id().unwrap().as_str(), "3");

        let metadata = diagnostic.metadata();
        assert_eq!(metadata.entries()[0].key().as_str(), "required_capability");
        assert_eq!(
            metadata.entries()[0].value(),
            &diagnostics::DiagnosticMetadataValue::String("request_mutation".to_string())
        );

        assert!(adapter.last_intent.is_none());
    }

    #[test]
    fn ratification_with_diagnostics_does_not_emit_when_dispatch_succeeds() {
        let mut adapter = DummyAdapter {
            allow_request_mutation: true,
            last_intent: None,
        };
        let mut report = diagnostics::DiagnosticReport::new();
        let intent = SurfaceIntent::select_primary_item(SurfaceInstanceId::new(5), 12);

        let result = ratify_surface_intent_with_diagnostics(&mut adapter, intent, &mut report);

        assert_eq!(result, Ok(RatificationOutcome::Applied));
        assert!(report.is_empty());
        assert_eq!(adapter.last_intent, Some(intent));
    }
}
