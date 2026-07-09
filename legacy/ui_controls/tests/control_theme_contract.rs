use ui_controls::{
    ControlKindId, ControlStyleDiagnostic, ControlStyleDiagnosticKind, ControlStyleFallback,
    ControlStyleRequirement, ControlStyleRole, ControlThemeDescriptor, ControlThemeTokenKind,
    ControlThemeTokenRequirement, ControlThemeTokenRole, ControlVisualState,
    ControlVisualStateRequirement, LABEL_CONTROL_KIND_ID,
};

#[test]
fn control_theme_descriptor_records_token_kinds() {
    let summary = label_theme_descriptor().summary();

    assert!(summary.required_token_kinds.contains(&"color".to_owned()));
    assert!(summary.required_token_kinds.contains(&"spacing".to_owned()));
    assert!(
        summary
            .required_token_kinds
            .contains(&"typography".to_owned())
    );
    assert!(summary.required_token_kinds.contains(&"radius".to_owned()));
    assert!(summary.required_token_kinds.contains(&"border".to_owned()));
    assert!(summary.required_token_kinds.contains(&"opacity".to_owned()));
    assert!(
        summary
            .required_token_kinds
            .contains(&"elevation".to_owned())
    );
}

#[test]
fn control_theme_descriptor_records_visual_states_and_style_roles() {
    let summary = label_theme_descriptor().summary();

    assert!(summary.visual_states.contains(&"normal".to_owned()));
    assert!(summary.visual_states.contains(&"hover".to_owned()));
    assert!(summary.visual_states.contains(&"pressed".to_owned()));
    assert!(summary.visual_states.contains(&"focused".to_owned()));
    assert!(summary.visual_states.contains(&"selected".to_owned()));
    assert!(summary.visual_states.contains(&"disabled".to_owned()));
    assert!(summary.visual_states.contains(&"error".to_owned()));
    assert!(summary.visual_states.contains(&"warning".to_owned()));
    assert!(summary.visual_states.contains(&"info".to_owned()));
    assert!(summary.visual_states.contains(&"active".to_owned()));
    assert!(summary.visual_states.contains(&"loading".to_owned()));
    assert!(summary.visual_states.contains(&"read-only".to_owned()));
    assert!(summary.style_roles.contains(&"container".to_owned()));
    assert!(summary.style_roles.contains(&"label".to_owned()));
    assert!(summary.style_roles.contains(&"background".to_owned()));
    assert!(summary.style_roles.contains(&"focus-ring".to_owned()));
}

#[test]
fn control_theme_fallbacks_and_summaries_are_declarative() {
    let summary = label_theme_descriptor().summary();

    assert!(
        summary
            .fallback_tokens
            .contains(&"runenwerk.theme.fallback.color".to_owned())
    );
    assert!(summary.diagnostics.contains(&"fallback-token".to_owned()));
    assert!(
        summary
            .diagnostics
            .contains(&"missing-token-diagnostic".to_owned())
    );
    assert!(!summary.has_runtime_style_behavior);
}

fn label_theme_descriptor() -> ControlThemeDescriptor {
    ControlThemeDescriptor::new(ControlKindId::new(LABEL_CONTROL_KIND_ID))
        .with_token(
            ControlThemeTokenRequirement::new(
                "runenwerk.theme.label.color",
                ControlThemeTokenKind::Color,
                ControlThemeTokenRole::Text,
            )
            .with_fallback("runenwerk.theme.fallback.color"),
        )
        .with_token(ControlThemeTokenRequirement::new(
            "runenwerk.theme.label.spacing",
            ControlThemeTokenKind::Spacing,
            ControlThemeTokenRole::Base,
        ))
        .with_token(ControlThemeTokenRequirement::new(
            "runenwerk.theme.label.typography",
            ControlThemeTokenKind::Typography,
            ControlThemeTokenRole::Text,
        ))
        .with_token(ControlThemeTokenRequirement::new(
            "runenwerk.theme.label.radius",
            ControlThemeTokenKind::Radius,
            ControlThemeTokenRole::Base,
        ))
        .with_token(ControlThemeTokenRequirement::new(
            "runenwerk.theme.label.border",
            ControlThemeTokenKind::Border,
            ControlThemeTokenRole::Border,
        ))
        .with_token(ControlThemeTokenRequirement::new(
            "runenwerk.theme.label.opacity",
            ControlThemeTokenKind::Opacity,
            ControlThemeTokenRole::Feedback,
        ))
        .with_token(ControlThemeTokenRequirement::new(
            "runenwerk.theme.label.elevation",
            ControlThemeTokenKind::Elevation,
            ControlThemeTokenRole::Surface,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Normal,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Hover,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Pressed,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Focused,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Selected,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Disabled,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Error,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Warning,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(ControlVisualState::Info))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Active,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::Loading,
        ))
        .with_visual_state(ControlVisualStateRequirement::new(
            ControlVisualState::ReadOnly,
        ))
        .with_style(ControlStyleRequirement::new(
            ControlStyleRole::Container,
            "runenwerk.theme.label.spacing",
        ))
        .with_style(ControlStyleRequirement::new(
            ControlStyleRole::Label,
            "runenwerk.theme.label.typography",
        ))
        .with_style(ControlStyleRequirement::new(
            ControlStyleRole::Background,
            "runenwerk.theme.label.color",
        ))
        .with_style(ControlStyleRequirement::new(
            ControlStyleRole::FocusRing,
            "runenwerk.theme.label.border",
        ))
        .with_fallback(ControlStyleFallback::new(
            "runenwerk.theme.label.color",
            "runenwerk.theme.fallback.color",
        ))
        .with_diagnostic(ControlStyleDiagnostic::new(
            "label.theme.token.check",
            ControlStyleDiagnosticKind::MissingToken,
            "runenwerk.theme.label.color",
            "theme token check",
        ))
}
