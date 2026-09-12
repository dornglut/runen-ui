//! Pure layered style-resolution helpers.

use crate::{
    Brush, BrushToken, BrushValue, Color, ColorToken, ColorValue, ComputedStyle, DropShadow,
    EdgeInsets, OpacityToken, OpacityValue, Outline, OutlineToken, OutlineValue, PresentationToken,
    PresentationTransform, PresentationValue, Radius, RadiusToken, RadiusValue, SceneOpacity,
    ShadowToken, ShadowValue, SpacingToken, SpacingValue, StyleEnvironment, StyleIntent,
    StyleInteractionFacts, StyleInteractionState, StylePreferenceKind, StyleProperties,
    StyleRecipeId, StyleTokens, StyleVariantId, Typography, TypographyToken, TypographyValue,
};

/// Exact precedence layer that last attempted to define one property.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StyleResolutionLayer {
    Initial,
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
    background: StyleFieldProvenance<BrushToken>,
    background_layer: Option<StyleResolutionLayer>,
    padding: StyleFieldProvenance<SpacingToken>,
    padding_layer: Option<StyleResolutionLayer>,
    radius: StyleFieldProvenance<RadiusToken>,
    radius_layer: Option<StyleResolutionLayer>,
    typography: StyleFieldProvenance<TypographyToken>,
    typography_layer: Option<StyleResolutionLayer>,
    outline: StyleFieldProvenance<OutlineToken>,
    outline_layer: Option<StyleResolutionLayer>,
    shadows: StyleFieldProvenance<ShadowToken>,
    shadows_layer: Option<StyleResolutionLayer>,
    opacity: StyleFieldProvenance<OpacityToken>,
    opacity_layer: Option<StyleResolutionLayer>,
    presentation: StyleFieldProvenance<PresentationToken>,
    presentation_layer: Option<StyleResolutionLayer>,
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
        typography: StyleFieldProvenance::Absent,
        typography_layer: None,
        outline: StyleFieldProvenance::Absent,
        outline_layer: None,
        shadows: StyleFieldProvenance::Absent,
        shadows_layer: None,
        opacity: StyleFieldProvenance::Absent,
        opacity_layer: None,
        presentation: StyleFieldProvenance::Absent,
        presentation_layer: None,
    };

    /// Creates value-source provenance without assigning production layers.
    ///
    /// This constructor remains useful for synthetic inspection fixtures. The
    /// production resolver also records the corresponding `*_layer` values.
    #[must_use]
    pub const fn new(
        foreground: StyleFieldProvenance<ColorToken>,
        background: StyleFieldProvenance<BrushToken>,
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
            typography: StyleFieldProvenance::Absent,
            typography_layer: None,
            outline: StyleFieldProvenance::Absent,
            outline_layer: None,
            shadows: StyleFieldProvenance::Absent,
            shadows_layer: None,
            opacity: StyleFieldProvenance::Absent,
            opacity_layer: None,
            presentation: StyleFieldProvenance::Absent,
            presentation_layer: None,
        }
    }

    /// Adds synthetic typography provenance without assigning a production layer.
    #[must_use]
    pub fn with_typography(mut self, typography: StyleFieldProvenance<TypographyToken>) -> Self {
        self.typography = typography;
        self
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
    pub const fn background(&self) -> &StyleFieldProvenance<BrushToken> {
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
    #[must_use]
    pub const fn typography(&self) -> &StyleFieldProvenance<TypographyToken> {
        &self.typography
    }
    #[must_use]
    pub const fn typography_layer(&self) -> Option<&StyleResolutionLayer> {
        self.typography_layer.as_ref()
    }
    #[must_use]
    pub const fn outline(&self) -> &StyleFieldProvenance<OutlineToken> {
        &self.outline
    }
    #[must_use]
    pub const fn outline_layer(&self) -> Option<&StyleResolutionLayer> {
        self.outline_layer.as_ref()
    }
    #[must_use]
    pub const fn shadows(&self) -> &StyleFieldProvenance<ShadowToken> {
        &self.shadows
    }
    #[must_use]
    pub const fn shadows_layer(&self) -> Option<&StyleResolutionLayer> {
        self.shadows_layer.as_ref()
    }
    #[must_use]
    pub const fn opacity(&self) -> &StyleFieldProvenance<OpacityToken> {
        &self.opacity
    }
    #[must_use]
    pub const fn opacity_layer(&self) -> Option<&StyleResolutionLayer> {
        self.opacity_layer.as_ref()
    }
    #[must_use]
    pub const fn presentation(&self) -> &StyleFieldProvenance<PresentationToken> {
        &self.presentation
    }
    #[must_use]
    pub const fn presentation_layer(&self) -> Option<&StyleResolutionLayer> {
        self.presentation_layer.as_ref()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnresolvedStyleToken {
    Foreground(ColorToken),
    Background(BrushToken),
    Padding(SpacingToken),
    Radius(RadiusToken),
    Typography(TypographyToken),
    Outline(OutlineToken),
    Shadows(ShadowToken),
    Opacity(OpacityToken),
    Presentation(PresentationToken),
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
    pub const fn computed_style(&self) -> &ComputedStyle {
        &self.computed_style
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
    background: Option<Brush>,
    padding: Option<EdgeInsets>,
    radius: Option<Radius>,
    typography: Option<Typography>,
    outline: Option<Outline>,
    shadows: Option<Vec<DropShadow>>,
    opacity: Option<SceneOpacity>,
    presentation: Option<PresentationTransform>,
    provenance: StyleProvenance,
    unresolved_tokens: Vec<UnresolvedStyleToken>,
    diagnostics: Vec<StyleResolutionDiagnostic>,
}

impl ResolutionBuilder {
    fn with_initial_values() -> Self {
        Self {
            typography: Some(Typography::default()),
            shadows: Some(Vec::new()),
            opacity: Some(SceneOpacity::OPAQUE),
            provenance: StyleProvenance {
                typography: StyleFieldProvenance::Literal,
                typography_layer: Some(StyleResolutionLayer::Initial),
                shadows: StyleFieldProvenance::Literal,
                shadows_layer: Some(StyleResolutionLayer::Initial),
                opacity: StyleFieldProvenance::Literal,
                opacity_layer: Some(StyleResolutionLayer::Initial),
                ..StyleProvenance::default()
            },
            ..Self::default()
        }
    }

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
            self.apply_radius(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.typography() {
            self.apply_typography(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.outline() {
            self.apply_outline(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.shadows() {
            self.apply_shadows(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.opacity() {
            self.apply_opacity(value, layer.clone(), tokens);
        }
        if let Some(value) = properties.presentation() {
            self.apply_presentation(value, layer, tokens);
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
        value: &BrushValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.background_layer = Some(layer);
        match value {
            BrushValue::Literal(value) => {
                self.background = Some(value.clone());
                self.provenance.background = StyleFieldProvenance::Literal;
            }
            BrushValue::Token(token) => {
                if let Some(value) = tokens.brush(token) {
                    self.background = Some(value.clone());
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

    fn apply_typography(
        &mut self,
        value: &TypographyValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.typography_layer = Some(layer);
        match value {
            TypographyValue::Literal(value) => {
                self.typography = Some(value.clone());
                self.provenance.typography = StyleFieldProvenance::Literal;
            }
            TypographyValue::Token(token) => {
                if let Some(value) = tokens.typography(token) {
                    self.typography = Some(value.clone());
                    self.provenance.typography = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.typography = None;
                    self.provenance.typography = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Typography(token.clone()));
                }
            }
        }
    }

    fn apply_outline(
        &mut self,
        value: &OutlineValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.outline_layer = Some(layer);
        match value {
            OutlineValue::Literal(value) => {
                self.outline = Some(value.clone());
                self.provenance.outline = StyleFieldProvenance::Literal;
            }
            OutlineValue::Token(token) => {
                if let Some(value) = tokens.outline(token) {
                    self.outline = Some(value.clone());
                    self.provenance.outline = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.outline = None;
                    self.provenance.outline = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Outline(token.clone()));
                }
            }
        }
    }

    fn apply_shadows(
        &mut self,
        value: &ShadowValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.shadows_layer = Some(layer);
        match value {
            ShadowValue::Literal(value) => {
                self.shadows = Some(value.clone());
                self.provenance.shadows = StyleFieldProvenance::Literal;
            }
            ShadowValue::Token(token) => {
                if let Some(value) = tokens.shadows(token) {
                    self.shadows = Some(value.to_vec());
                    self.provenance.shadows = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.shadows = None;
                    self.provenance.shadows = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Shadows(token.clone()));
                }
            }
        }
    }

    fn apply_opacity(
        &mut self,
        value: &OpacityValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.opacity_layer = Some(layer);
        match value {
            OpacityValue::Literal(value) => {
                self.opacity = Some(*value);
                self.provenance.opacity = StyleFieldProvenance::Literal;
            }
            OpacityValue::Token(token) => {
                if let Some(value) = tokens.opacity(token) {
                    self.opacity = Some(value);
                    self.provenance.opacity = StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.opacity = None;
                    self.provenance.opacity = StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Opacity(token.clone()));
                }
            }
        }
    }

    fn apply_presentation(
        &mut self,
        value: &PresentationValue,
        layer: StyleResolutionLayer,
        tokens: &StyleTokens,
    ) {
        self.provenance.presentation_layer = Some(layer);
        match value {
            PresentationValue::Literal(value) => {
                self.presentation = Some(*value);
                self.provenance.presentation = StyleFieldProvenance::Literal;
            }
            PresentationValue::Token(token) => {
                if let Some(value) = tokens.presentation(token) {
                    self.presentation = Some(value);
                    self.provenance.presentation =
                        StyleFieldProvenance::ResolvedToken(token.clone());
                } else {
                    self.presentation = None;
                    self.provenance.presentation =
                        StyleFieldProvenance::MissingToken(token.clone());
                    self.record_missing(UnresolvedStyleToken::Presentation(token.clone()));
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
        let Self {
            foreground,
            background,
            padding,
            radius,
            typography,
            outline,
            shadows,
            opacity,
            presentation,
            provenance,
            unresolved_tokens,
            diagnostics,
        } = self;

        let mut computed_style = ComputedStyle::EMPTY
            .with_shadows(shadows.unwrap_or_default())
            .with_opacity(opacity.unwrap_or(SceneOpacity::OPAQUE));
        if let Some(value) = foreground {
            computed_style = computed_style.with_foreground(value);
        }
        if let Some(value) = background {
            computed_style = computed_style.with_background(value);
        }
        if let Some(value) = padding {
            computed_style = computed_style.with_padding(value);
        }
        if let Some(value) = radius {
            computed_style = computed_style.with_radius(value);
        }
        if let Some(value) = typography {
            computed_style = computed_style.with_typography(value);
        }
        if let Some(value) = outline {
            computed_style = computed_style.with_outline(value);
        }
        if let Some(value) = presentation {
            computed_style = computed_style.with_presentation(value);
        }

        StyleResolution::new(computed_style, provenance, unresolved_tokens, diagnostics)
    }
}

/// Resolves one authored style against the complete production environment.
#[must_use]
pub fn resolve_style_in_environment(
    intent: &StyleIntent,
    environment: &StyleEnvironment,
    interaction: StyleInteractionFacts,
    parent: Option<&ComputedStyle>,
) -> StyleResolution {
    let tokens = environment.theme().tokens();
    let mut builder = ResolutionBuilder::with_initial_values();

    if let Some(parent) = parent {
        if let Some(foreground) = parent.foreground() {
            builder.foreground = Some(foreground);
            builder.provenance.foreground = StyleFieldProvenance::Inherited;
            builder.provenance.foreground_layer = Some(StyleResolutionLayer::Inherited);
        }
        if let Some(typography) = parent.typography() {
            builder.typography = Some(typography.clone());
            builder.provenance.typography = StyleFieldProvenance::Inherited;
            builder.provenance.typography_layer = Some(StyleResolutionLayer::Inherited);
        }
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
