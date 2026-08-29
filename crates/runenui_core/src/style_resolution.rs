//! Pure layered style-resolution helpers.

use crate::{
    Color, ColorToken, ColorValue, ComputedStyle, EdgeInsets, Radius, RadiusToken, RadiusValue,
    SpacingToken, SpacingValue, StyleEnvironment, StyleIntent, StyleInteractionFacts,
    StyleInteractionState, StylePreferenceKind, StyleProperties, StyleRecipeId, StyleTokens,
    StyleVariantId,
};

/// Exact precedence layer that last attempted to define one property.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleResolutionLayer {
    Inherited,
    FrameworkDefault,
    ThemeRecipe(StyleRecipeId),
    Variant(StyleVariantId),
    Interaction(StyleInteractionState),
    AuthoredOverride,
    Preference(StylePreferenceKind),
}

/// Resolution value-source provenance for one style property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum StyleFieldProvenance<Token> {
    #[default]
    Absent,
    Inherited,
    Literal,
    ResolvedToken(Token),
    MissingToken(Token),
}

/// Per-field provenance produced by style resolution.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StyleProvenance {
    foreground: StyleFieldProvenance<ColorToken>,
    foreground_layer: Option<StyleResolutionLayer>,
    background: StyleFieldProvenance<ColorToken>,
    background_layer: Option<StyleResolutionLayer>,
    padding: StyleFieldProvenance<SpacingToken>,
    padding_layer: Option<StyleResolutionLayer>,
    radius: StyleFieldProvenance<RadiusToken>,
    radius_layer: Option<StyleResolutionLayer>,
}

impl StyleProvenance {
    pub const EMPTY: Self = Self {
        foreground: StyleFieldProvenance::Absent,
        foreground_layer: None,
        background: StyleFieldProvenance::Absent,
        background_layer: None,
        padding: StyleFieldProvenance::Absent,
        padding_layer: None,
        radius: StyleFieldProvenance::Absent,
        radius_layer: None,
    };

    /// Creates value-source provenance without assigning production layers.
    ///
    /// This constructor remains useful for synthetic inspection fixtures. The
    /// production resolver also records the corresponding `*_layer` values.
    #[must_use]
    pub const fn new(
        foreground: StyleFieldProvenance<ColorToken>,
        background: StyleFieldProvenance<ColorToken>,
        padding: StyleFieldProvenance<SpacingToken>,
        radius: StyleFieldProvenance<RadiusToken>,
    ) -> Self {
        Self {
            foreground,
            foreground_layer: None,
            background,
            background_layer: None,
            padding,
            padding_layer: None,
            radius,
            radius_layer: None,
        }
    }

    #[must_use]
    pub const fn foreground(&self) -> &StyleFieldProvenance<ColorToken> {
        &self.foreground
    }
    #[must_use]
    pub const fn foreground_layer(&self) -> Option<&StyleResolutionLayer> {
        self.foreground_layer.as_ref()
    }
    #[must_use]
    pub const fn background(&self) -> &StyleFieldProvenance<ColorToken> {
        &self.background
    }
    #[must_use]
    pub const fn background_layer(&self) -> Option<&StyleResolutionLayer> {
        self.background_layer.as_ref()
    }
    #[must_use]
    pub const fn padding(&self) -> &StyleFieldProvenance<SpacingToken> {
        &self.padding
    }
    #[must_use]
    pub const fn padding_layer(&self) -> Option<&StyleResolutionLayer> {
        self.padding_layer.as_ref()
    }
    #[must_use]
    pub const fn radius(&self) -> &StyleFieldProvenance<RadiusToken> {
        &self.radius
    }
    #[must_use]
    pub const fn radius_layer(&self) -> Option<&StyleResolutionLayer> {
        self.radius_layer.as_ref()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnresolvedStyleToken {
    Foreground(ColorToken),
    Background(ColorToken),
    Padding(SpacingToken),
    Radius(RadiusToken),
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleResolutionDiagnostic {
    MissingRecipe(StyleRecipeId),
    MissingVariant(StyleVariantId),
    MissingToken(UnresolvedStyleToken),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleResolution {
    computed_style: ComputedStyle,
    provenance: StyleProvenance,
    unresolved_tokens: Vec<UnresolvedStyleToken>,
    diagnostics: Vec<StyleResolutionDiagnostic>,
}

impl StyleResolution {
    const fn new(
        computed_style: ComputedStyle,
        provenance: StyleProvenance,
        unresolved_tokens: Vec<UnresolvedStyleToken>,
        diagnostics: Vec<StyleResolutionDiagnostic>,
    ) -> Self {
        Self {
            computed_style,
            provenance,
            unresolved_tokens,
            diagnostics,
        }
    }

    #[must_use]
    pub const fn computed_style(&self) -> ComputedStyle {
        self.computed_style
    }
    #[must_use]
    pub const fn provenance(&self) -> &StyleProvenance {
        &self.provenance
    }
    #[must_use]
    pub const fn unresolved_tokens(&self) -> &[UnresolvedStyleToken] {
        self.unresolved_tokens.as_slice()
    }
    #[must_use]
    pub const fn diagnostics(&self) -> &[StyleResolutionDiagnostic] {
        self.diagnostics.as_slice()
    }
    #[must_use]
    pub const fn is_fully_resolved(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Default)]
struct ResolutionBuilder {
    foreground: Option<Color>,
    background: Option<Color>,
    padding: Option<EdgeInsets>,
    radius: Option<Radius>,
    provenance: StyleProvenance,
    unresolved_tokens: Vec<UnresolvedStyleToken>,
    diagnostics: Vec<StyleResolutionDiagnostic>,
}

impl ResolutionBuilder {
    fn apply(
        &mut self,
        properties: &StyleProperties,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        if let Some(value) = properties.foreground() {
            self.apply_foreground(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.background() {
            self.apply_background(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.padding() {
            self.apply_padding(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.radius() {
            self.apply_radius(value, layer, tokens);
        }
    }

    fn apply_foreground(
        &mut self,
        value: &ColorValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.foreground_layer = Some(layer);
        match value {
            ColorValue::Literal(value) => {
                self.foreground = Some(*value);
                self.provenance.foreground = StyleFieldProvenance::Literal;
            }
            ColorValue::Token(token) => {
                if let Some(value) = tokens.color(token) {
                    self.foreground = Some(value);
                    self.provenance.foreground = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.foreground = None;
                    self.provenance.foreground = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Foreground(token.clone()));
                }
            }
        }
    }

    fn apply_background(
        &mut self,
        value: &ColorValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.background_layer = Some(layer);
        match value {
            ColorValue::Literal(value) => {
                self.background = Some(*value);
                self.provenance.background = StyleFieldProvenance::Literal;
            }
            ColorValue::Token(token) => {
                if let Some(value) = tokens.color(token) {
                    self.background = Some(value);
                    self.provenance.background = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.background = None;
                    self.provenance.background = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Background(token.clone()));
                }
            }
        }
    }

    fn apply_padding(
        &mut self,
        value: &SpacingValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.padding_layer = Some(layer);
        match value {
            SpacingValue::Literal(value) => {
                self.padding = Some(*value);
                self.provenance.padding = StyleFieldProvenance::Literal;
            }
            SpacingValue::Token(token) => {
                if let Some(value) = tokens.spacing(token) {
                    self.padding = Some(value);
                    self.provenance.padding = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.padding = None;
                    self.provenance.padding = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Padding(token.clone()));
                }
            }
        }
    }

    fn apply_radius(
        &mut self,
        value: &RadiusValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.radius_layer = Some(layer);
        match value {
            RadiusValue::Literal(value) => {
                self.radius = Some(*value);
                self.provenance.radius = StyleFieldProvenance::Literal;
            }
            RadiusValue::Token(token) => {
                if let Some(value) = tokens.radius(token) {
                    self.radius = Some(value);
                    self.provenance.radius = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.radius = None;
                    self.provenance.radius = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Radius(token.clone()));
                }
            }
        }
    }

    fn record_missing(&mut self, token: UnresolvedStyleToken) {
        self.unresolved_tokens.push(token.clone());
        self.diagnostics
            .push(StyleResolutionDiagnostic::MissingToken(token));
    }

    fn finish(self) -> StyleResolution {
        StyleResolution::new(
            ComputedStyle::from_parts(self.foreground, self.background, self.padding, self.radius),
            self.provenance,
            self.unresolved_tokens,
            self.diagnostics,
        )
    }
}

/// Resolves one authored style against the complete production environment.
#[must_use]
pub fn resolve_style_in_environment(
    intent: &StyleIntent,
    environment: &StyleEnvironment,
    interaction: StyleInteractionFacts,
    parent: Option<ComputedStyle>,
) -> StyleResolution {
    let tokens = environment.theme().tokens();
    let mut builder = ResolutionBuilder::default();

    if let Some(foreground) = parent.and_then(|style| style.foreground()) {
        builder.foreground = Some(foreground);
        builder.provenance.foreground = StyleFieldProvenance::Inherited;
        builder.provenance.foreground_layer = Some(StyleResolutionLayer::Inherited);
    }

    builder.apply(
        environment.framework_defaults(),
        StyleResolutionLayer::FrameworkDefault,
        tokens,
    );

    if let Some(recipe_id) = intent.recipe() {
        if let Some(recipe) = environment.theme().recipe(recipe_id) {
            builder.apply(
                recipe.base(),
                StyleResolutionLayer::ThemeRecipe(recipe_id.clone()),
                tokens,
            );
            for variant_id in intent.variants() {
                if let Some(properties) = recipe.variant(variant_id) {
                    builder.apply(
                        properties,
                        StyleResolutionLayer::Variant(variant_id.clone()),
                        tokens,
                    );
                } else {
                    builder
                        .diagnostics
                        .push(StyleResolutionDiagnostic::MissingVariant(
                            variant_id.clone(),
                        ));
                }
            }
            for state in StyleInteractionState::ORDERED {
                if state.is_active(interaction)
                    && let Some(properties) = recipe.interaction(state)
                {
                    builder.apply(properties, StyleResolutionLayer::Interaction(state), tokens);
                }
            }
        } else {
            builder
                .diagnostics
                .push(StyleResolutionDiagnostic::MissingRecipe(recipe_id.clone()));
            for variant_id in intent.variants() {
                builder
                    .diagnostics
                    .push(StyleResolutionDiagnostic::MissingVariant(
                        variant_id.clone(),
                    ));
            }
        }
    } else {
        for variant_id in intent.variants() {
            builder
                .diagnostics
                .push(StyleResolutionDiagnostic::MissingVariant(
                    variant_id.clone(),
                ));
        }
    }

    builder.apply(
        intent.overrides(),
        StyleResolutionLayer::AuthoredOverride,
        tokens,
    );

    if environment.preferences().high_contrast() {
        builder.apply(
            environment.preference_policy().high_contrast(),
            StyleResolutionLayer::Preference(StylePreferenceKind::HighContrast),
            tokens,
        );
    }

    builder.finish()
}

/// Resolves direct authored properties without theme/state/preference layers.
///
/// This remains only as an in-flight bridge while M8A migrates the runtime to
/// [`resolve_style_in_environment`]. It is removed before M8A acceptance.
#[must_use]
pub fn resolve_style(intent: &StyleIntent, tokens: &StyleTokens) -> StyleResolution {
    let environment = StyleEnvironment::from_tokens(tokens.clone());
    resolve_style_in_environment(intent, &environment, StyleInteractionFacts::default(), None)
}

#[must_use]
pub fn resolve_literal_style(intent: &StyleIntent) -> StyleResolution {
    resolve_style(intent, &StyleTokens::new())
}
