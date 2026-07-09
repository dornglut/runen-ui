use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ui_adaptive_composition::{
    AdaptiveProjectionState, DragSession, RegionHitIndex, large_benchmark_fixture,
};
use ui_composition::{
    CanonicalCompositionDocuments, CompositionCapabilityPolicy, CompositionCommand,
    CompositionLifecyclePolicy, CompositionPolicies, CompositionPolicyDecision, CompositionState,
    CompositionTargetPolicy, CompositionTransaction, CompositionTransactionId, RegionKind,
    SplitFraction,
};
use ui_math::UiPoint;

struct Accept;

impl CompositionLifecyclePolicy for Accept {
    fn evaluate(
        &self,
        _: ui_composition::CompositionSnapshot<'_>,
        _: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}
impl CompositionCapabilityPolicy for Accept {
    fn evaluate(
        &self,
        _: ui_composition::CompositionSnapshot<'_>,
        _: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}
impl CompositionTargetPolicy for Accept {
    fn evaluate(
        &self,
        _: ui_composition::CompositionSnapshot<'_>,
        _: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}

fn main() {
    let fixture = large_benchmark_fixture();
    let projection = Arc::new(
        AdaptiveProjectionState::derive(fixture.state.snapshot(), &fixture.policy).unwrap(),
    );
    let hit_index = RegionHitIndex::new(projection.shared_regions());
    measure("region_hit_testing", 0.250, || {
        black_box(hit_index.hit_test(UiPoint::new(700.0, 500.0)))
    });
    measure("adaptive_proposal_generation", 0.750, || {
        let mut session = DragSession::begin(
            Arc::clone(&projection),
            ui_composition::MountedUnitId::new(1),
        );
        black_box(session.update_pointer(UiPoint::new(700.0, 500.0)).is_some())
    });
    measure("preview_projection", 1.000, || {
        let mut session = DragSession::begin(
            Arc::clone(&projection),
            ui_composition::MountedUnitId::new(1),
        );
        black_box(session.update_pointer(UiPoint::new(10.0, 10.0)).copied())
    });
    measure("drag_frame_update", 2.000, || {
        let mut session = DragSession::begin(
            Arc::clone(&projection),
            ui_composition::MountedUnitId::new(1),
        );
        session.update_pointer(UiPoint::new(10.0, 10.0));
        black_box(session.metrics())
    });

    let transaction = transaction_case(&fixture.state);
    let accept = Accept;
    let policies = || CompositionPolicies {
        lifecycle: &accept,
        capability: &accept,
        target: &accept,
    };
    measure("transaction_validation_64", 1.500, || {
        black_box(
            fixture
                .state
                .authorize(transaction.clone(), policies())
                .is_ok(),
        )
    });
    measure_committed_mutation(fixture.state.definition(), &accept);
    let source = CanonicalCompositionDocuments::definition(fixture.state.definition()).unwrap();
    measure("large_serialization", 20.000, || {
        black_box(CanonicalCompositionDocuments::definition(fixture.state.definition()).unwrap())
    });
    measure("large_validation_deserialization", 20.000, || {
        black_box(CanonicalCompositionDocuments::decode_definition(&source).unwrap())
    });
}

fn transaction_case(state: &CompositionState) -> CompositionTransaction {
    let split = state
        .definition()
        .regions()
        .iter()
        .find_map(|region| matches!(region.kind, RegionKind::Split { .. }).then_some(region.id))
        .unwrap();
    let commands = (0..64)
        .map(|index| {
            CompositionCommand::resize_split(split, SplitFraction::try_new(4_900 + index).unwrap())
        })
        .collect();
    CompositionTransaction::new(CompositionTransactionId::new(1), state.revision(), commands)
}

fn measure<T>(name: &str, budget_ms: f64, mut operation: impl FnMut() -> T) {
    for _ in 0..10 {
        black_box(operation());
    }
    let mut samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let start = Instant::now();
        black_box(operation());
        samples.push(start.elapsed());
    }
    samples.sort();
    let p95 = samples[28];
    report_and_enforce(name, p95, budget_ms);
}

fn measure_committed_mutation(
    definition: &ui_composition::CompositionDefinitionV1,
    accept: &Accept,
) {
    let mut prepared = (0..40)
        .map(|index| {
            let state = CompositionState::form(definition.clone()).unwrap();
            let transaction = CompositionTransaction::new(
                CompositionTransactionId::new(index + 1),
                state.revision(),
                transaction_case(&state).commands().to_vec(),
            );
            let authorized = state
                .authorize(
                    transaction,
                    CompositionPolicies {
                        lifecycle: accept,
                        capability: accept,
                        target: accept,
                    },
                )
                .unwrap();
            (state, authorized)
        })
        .collect::<Vec<_>>()
        .into_iter();
    for _ in 0..10 {
        let (mut state, authorized) = prepared.next().unwrap();
        black_box(state.apply_authorized(authorized).unwrap());
    }
    let mut samples = Vec::with_capacity(30);
    for _ in 0..30 {
        let (mut state, authorized) = prepared.next().unwrap();
        let start = Instant::now();
        black_box(state.apply_authorized(authorized).unwrap());
        samples.push(start.elapsed());
    }
    samples.sort();
    report_and_enforce("committed_mutation_64", samples[28], 1.500);
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn report_and_enforce(name: &str, p95: Duration, budget_ms: f64) {
    let observed_ms = millis(p95);
    println!("{name}: p95 {observed_ms:.3} ms (budget {budget_ms:.3} ms)");
    assert!(
        observed_ms <= budget_ms,
        "{name} p95 {observed_ms:.3} ms exceeded {budget_ms:.3} ms budget"
    );
}
