use std::time::Duration;

use runenui_core::{
    ComputedStyle, MotionEasing, MotionTarget, SceneOpacity, StyleEnvironment, StyleIntent,
    StyleInteractionFacts, StyleProperties, StyleResolutionLayer, TransitionPolicy, TransitionSpec,
    Typography, resolve_style_in_environment,
};

fn transition(milliseconds: u64) -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(milliseconds),
        Duration::ZERO,
        MotionEasing::Linear,
        None,
    )
    .unwrap_or_else(|_| unreachable!("bounded test transition is valid"))
}

#[test]
fn transition_policy_uses_normal_target_keyed_style_precedence() {
    let lower_foreground = transition(100);
    let lower_opacity = transition(200);
    let environment = StyleEnvironment::default().with_framework_defaults(
        StyleProperties::EMPTY
            .with_transition(MotionTarget::Foreground, lower_foreground)
            .with_transition(MotionTarget::Opacity, lower_opacity.clone()),
    );
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY.with_transition_disabled(MotionTarget::Foreground),
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(
        resolution.transition_policy(MotionTarget::Foreground),
        Some(&TransitionPolicy::Disabled)
    );
    assert_eq!(
        resolution.transition_policy_layer(MotionTarget::Foreground),
        Some(&StyleResolutionLayer::AuthoredOverride)
    );
    assert_eq!(
        resolution.transition_policy(MotionTarget::Opacity),
        Some(&TransitionPolicy::Enabled(lower_opacity))
    );
    assert_eq!(
        resolution.transition_policy_layer(MotionTarget::Opacity),
        Some(&StyleResolutionLayer::FrameworkDefault)
    );
}

#[test]
fn transition_policy_absence_does_not_mask_lower_layers() {
    let lower = transition(75);
    let environment = StyleEnvironment::default().with_framework_defaults(
        StyleProperties::EMPTY.with_transition(MotionTarget::Padding, lower.clone()),
    );
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY.with_opacity(SceneOpacity::TRANSPARENT),
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(
        resolution.transition_policy(MotionTarget::Padding),
        Some(&TransitionPolicy::Enabled(lower))
    );
    assert_eq!(
        resolution.transition_policy_layer(MotionTarget::Padding),
        Some(&StyleResolutionLayer::FrameworkDefault)
    );
}

#[test]
fn transition_policy_is_non_inherited_and_does_not_change_computed_style() {
    let parent_environment = StyleEnvironment::default().with_framework_defaults(
        StyleProperties::EMPTY.with_transition(MotionTarget::Foreground, transition(120)),
    );
    let parent = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &parent_environment,
        StyleInteractionFacts::NONE,
        None,
    );
    assert!(parent.transition_policy(MotionTarget::Foreground).is_some());

    let child = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &StyleEnvironment::default(),
        StyleInteractionFacts::NONE,
        Some(parent.computed_style()),
    );
    assert_eq!(child.transition_policy(MotionTarget::Foreground), None);
    assert_eq!(
        child.computed_style(),
        &ComputedStyle::EMPTY.with_typography(Typography::default())
    );
}

#[test]
fn repeated_same_layer_target_replaces_without_duplicate_policy() {
    let replacement = transition(300);
    let properties = StyleProperties::EMPTY
        .with_transition(MotionTarget::Opacity, transition(100))
        .with_transition(MotionTarget::Foreground, transition(200))
        .with_transition(MotionTarget::Opacity, replacement.clone());
    let environment = StyleEnvironment::default().with_framework_defaults(properties);
    let resolution = resolve_style_in_environment(
        &StyleIntent::EMPTY,
        &environment,
        StyleInteractionFacts::NONE,
        None,
    );

    assert_eq!(
        resolution.transition_policy(MotionTarget::Opacity),
        Some(&TransitionPolicy::Enabled(replacement))
    );
    assert_eq!(resolution.transition_policies().count(), 2);
}
