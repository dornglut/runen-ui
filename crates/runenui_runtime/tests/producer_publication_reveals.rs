#![allow(refining_impl_trait)]

use runenui_core::{
    Element, NoHostProtocol, ProducerBudget, SemanticContribution, SemanticContributionContext,
    SemanticKey, SemanticNodeContribution, SemanticReference, SemanticRelationship,
    SemanticRelationshipKind, SemanticRole, SemanticState, StyleEnvironment, UiApp, Widget,
    WidgetProducerBudget, WidgetProducerPhase,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PublishSurfaceError, PumpBudget, RuntimeConfig, RuntimeStatus,
    RuntimeTerminalReason, SurfaceBuildContext, TraceConfig, TraceRecordKind,
};

#[derive(Debug)]
struct RevealWidget {
    declared: usize,
}

impl Widget<()> for RevealWidget {
    type State = ();

    fn create_state(&self) {}

    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        let budget = WidgetProducerBudget::new(0);
        if phase == WidgetProducerPhase::Publication {
            budget.with_publication_reveals(self.declared)
        } else {
            budget
        }
    }

    fn semantics(&self, (): &(), _: SemanticContributionContext) -> SemanticContribution {
        let key = SemanticKey::from_static("option")
            .unwrap_or_else(|_| unreachable!("static key is valid"));
        let list = SemanticKey::from_static("list")
            .unwrap_or_else(|_| unreachable!("static key is valid"));
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::ComboBox)
                .with_state(SemanticState::ENABLED.with_expanded(true))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Local(list.clone()),
                ))
                .with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::ActiveDescendant,
                    SemanticReference::Local(key.clone()),
                ))
                .with_child(
                    SemanticNodeContribution::new(list, SemanticRole::ListBox)
                        .with_child(SemanticNodeContribution::new(key, SemanticRole::Option)),
                ),
        )
    }
}

struct App;

impl UiApp for App {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(_: &usize) -> ProducerBudget {
        // One mounted owner, with no callback output, work, or subscriptions.
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }

    fn action_producer_budget(_: &usize, (): &()) -> ProducerBudget {
        ProducerBudget::new(1, 0)
    }

    fn interaction_producer_budget(_: &usize) -> ProducerBudget {
        // The reveal command routes through one output-free owner.
        ProducerBudget::new(1, 0)
    }

    fn root(declared: &usize) -> Element<()> {
        Element::new(RevealWidget {
            declared: *declared,
        })
    }

    fn update(_: &mut usize, (): ()) {}
}

#[test]
fn reveal_declaration_is_checked_before_retaining_publication() {
    for trace_capacity in [0, 128] {
        for declared in [0, 1] {
            let mut runtime = AppRuntime::<App>::mount_with_config(
                declared,
                RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
            )
            .unwrap_or_else(|error| unreachable!("single pure owner mounts: {error}"));
            let environment = StyleEnvironment::default();
            let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
            let result = runtime.publish_surface(&context);
            if declared == 0 {
                assert!(
                    matches!(
                        result,
                        Err(PublishSurfaceError::Terminal(
                            RuntimeTerminalReason::Poisoned
                        ))
                    ),
                    "undeclared candidate must fail before commit: {result:?}"
                );
                assert!(
                    !runtime
                        .trace()
                        .kinds()
                        .any(|kind| matches!(kind, TraceRecordKind::SurfacePublished))
                );
            } else {
                assert!(result.is_ok(), "declared reveal is funded: {result:?}");
                runtime
                    .pump(PumpBudget::new(16, 0, 0, 0))
                    .unwrap_or_else(|_| unreachable!("public pump observation fits"));
                assert_eq!(runtime.status(), RuntimeStatus::Running);
                assert!(runtime.publish_surface(&context).is_ok());
            }
        }
    }
}
