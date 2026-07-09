use ui_app_integration::{UiAppIntegrationProof, UiAppProofId};

#[test]
fn counter_story_runs_full_ecs_backed_loop() {
    let run = UiAppIntegrationProof::builder(UiAppProofId::new("counter.proof"))
        .build()
        .run_counter_story();
    let report = run.report;

    assert!(report.passed(), "{:#?}", report.diagnostics);
    assert_eq!(report.initial.count, 0);
    assert_eq!(report.final_snapshot.count, 0);
    assert_eq!(report.final_snapshot.active_screen, "counter.screen");
    assert_eq!(report.steps.len(), 6);

    let first = &report.steps[0];
    assert_eq!(first.before.count, 0);
    assert_eq!(first.after.count, 1);
    assert_eq!(first.before.active_screen, "counter.screen");
    assert!(
        first
            .runtime
            .route_ids
            .iter()
            .any(|route| route == "counter.increment")
    );
    assert!(
        first
            .runtime
            .output_contains_text
            .iter()
            .any(|text| text == "Clicked 0 / 5")
    );
    assert!(
        first
            .source
            .source
            .node_ids()
            .iter()
            .any(|node| *node == "counter.increment_button")
    );
    assert!(first.formation.passed);
    assert!(first.formation.source_map_entries > 0);
    assert!(first.action.as_ref().is_some_and(|action| action.resolved));
    let first_action = first.action.as_ref().expect("increment action report");
    assert_eq!(first_action.action_id.as_str(), "counter.increment");
    assert_eq!(first_action.route, "counter.increment");
    assert_ne!(first_action.action_id.as_str(), "Click me");
    assert!(first.mutation.is_some());
    let first_mutation = first.mutation.as_ref().expect("increment mutation report");
    assert_eq!(first_mutation.before.count, 0);
    assert_eq!(first_mutation.after.count, 1);

    let win_step = &report.steps[5];
    assert_eq!(win_step.before.count, 5);
    assert_eq!(win_step.before.active_screen, "counter.win");
    assert_eq!(win_step.after.count, 0);
    assert_eq!(win_step.after.active_screen, "counter.screen");
    assert!(
        win_step
            .runtime
            .route_ids
            .iter()
            .any(|route| route == "counter.reset")
    );
    assert!(
        win_step
            .runtime
            .output_contains_text
            .iter()
            .any(|text| text == "You win!")
    );
    assert!(
        win_step
            .runtime
            .output_contains_text
            .iter()
            .any(|text| text == "Reset")
    );
    assert!(
        win_step
            .source
            .source
            .node_ids()
            .iter()
            .any(|node| *node == "counter.reset_button")
    );

    let routes = report.route_ids();
    assert!(routes.iter().any(|route| *route == "counter.increment"));
    assert!(routes.iter().any(|route| *route == "counter.reset"));
}
