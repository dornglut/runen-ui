//! Host-neutral metric typography vocabulary.

use core::{error::Error, fmt};

use crate::LogicalLength;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Validated named font-family identity.
pub struct FontFamilyName(String);

impl FontFamilyName {
    /// Validates one named font family.
    ///
    /// Internal whitespace is preserved. Empty names, surrounding whitespace,
    /// and control characters are rejected so family identity is canonical.
    ///
    /// # Errors
    ///
    /// Returns [`FontFamilyNameError`] when `name` is not canonical.
    pub fn new(name: impl Into<String>) -> Result<Self, FontFamilyNameError> {
        let name = name.into();
        if name.is_empty() {
            return Err(FontFamilyNameError::Empty);
        }
        if name.trim() != name {
            return Err(FontFamilyNameError::SurroundingWhitespace);
        }
        if name.chars().any(char::is_control) {
            return Err(FontFamilyNameError::ControlCharacter);
        }
        Ok(Self(name))
    }

    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontFamilyNameError {
    Empty,
    SurroundingWhitespace,
    ControlCharacter,
}

impl fmt::Display for FontFamilyNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "font family name is empty",
            Self::SurroundingWhitespace => "font family name has surrounding whitespace",
            Self::ControlCharacter => "font family name contains a control character",
        })
    }
}

impl Error for FontFamilyNameError {}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Generic font-family intent independent of the shaping implementation.
pub enum GenericFontFamily {
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
    SystemUi,
    UiSerif,
    UiSansSerif,
    UiMonospace,
    UiRounded,
    Emoji,
    Math,
    FangSong,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// One named or generic family in an ordered fallback stack.
pub enum FontFamily {
    Named(FontFamilyName),
    Generic(GenericFontFamily),
}

impl FontFamily {
    /// Creates a validated named family.
    ///
    /// # Errors
    ///
    /// Returns [`FontFamilyNameError`] when `name` is invalid.
    pub fn named(name: impl Into<String>) -> Result<Self, FontFamilyNameError> {
        FontFamilyName::new(name).map(Self::Named)
    }

    #[must_use]
    pub const fn generic(family: GenericFontFamily) -> Self {
        Self::Generic(family)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
/// Requested font weight used for family matching.
pub struct FontWeight(f32);

impl FontWeight {
    pub const NORMAL: Self = Self(400.0);
    pub const BOLD: Self = Self(700.0);

    /// Creates a CSS-compatible numeric weight in the inclusive `1..=1000` range.
    ///
    /// # Errors
    ///
    /// Returns [`FontWeightError`] for non-finite or out-of-range values.
    pub fn new(value: f32) -> Result<Self, FontWeightError> {
        if !value.is_finite() {
            return Err(FontWeightError::NotFinite);
        }
        if !(1.0..=1000.0).contains(&value) {
            return Err(FontWeightError::OutOfRange);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontWeightError {
    NotFinite,
    OutOfRange,
}

impl fmt::Display for FontWeightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFinite => "font weight is not finite",
            Self::OutOfRange => "font weight is outside the inclusive 1..=1000 range",
        })
    }
}

impl Error for FontWeightError {}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
/// Requested font width as a ratio relative to normal width (`1.0`).
pub struct FontWidth(f32);

impl FontWidth {
    pub const NORMAL: Self = Self(1.0);

    /// Creates a CSS-compatible width ratio in the inclusive `0.5..=2.0` range.
    ///
    /// # Errors
    ///
    /// Returns [`FontWidthError`] for non-finite or out-of-range values.
    pub fn new(ratio: f32) -> Result<Self, FontWidthError> {
        if !ratio.is_finite() {
            return Err(FontWidthError::NotFinite);
        }
        if !(0.5..=2.0).contains(&ratio) {
            return Err(FontWidthError::OutOfRange);
        }
        Ok(Self(ratio))
    }

    #[must_use]
    pub const fn ratio(self) -> f32 {
        self.0
    }
}

impl Default for FontWidth {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontWidthError {
    NotFinite,
    OutOfRange,
}

impl fmt::Display for FontWidthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFinite => "font width is not finite",
            Self::OutOfRange => "font width is outside the inclusive 0.5..=2.0 ratio range",
        })
    }
}

impl Error for FontWidthError {}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
/// Finite oblique angle in degrees.
pub struct FontObliqueAngle(f32);

impl FontObliqueAngle {
    /// Creates one finite oblique angle in degrees.
    ///
    /// # Errors
    ///
    /// Returns [`FontObliqueAngleError::NotFinite`] for NaN or infinity.
    pub fn new(degrees: f32) -> Result<Self, FontObliqueAngleError> {
        if degrees.is_finite() {
            Ok(Self(degrees))
        } else {
            Err(FontObliqueAngleError::NotFinite)
        }
    }

    #[must_use]
    pub const fn degrees(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontObliqueAngleError {
    NotFinite,
}

impl fmt::Display for FontObliqueAngleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("font oblique angle is not finite")
    }
}

impl Error for FontObliqueAngleError {}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Requested visual font style.
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    /// Oblique style with an optional explicit angle in degrees.
    ///
    /// `None` preserves the shaping engine's ordinary default-oblique intent;
    /// `Some` preserves an exact finite authored angle.
    Oblique(Option<FontObliqueAngle>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Validated four-byte OpenType tag.
pub struct OpenTypeTag([u8; 4]);

impl OpenTypeTag {
    /// Creates a tag from four printable ASCII bytes.
    ///
    /// # Errors
    ///
    /// Returns [`OpenTypeTagError`] when any byte is outside `0x20..=0x7e`.
    pub fn new(bytes: [u8; 4]) -> Result<Self, OpenTypeTagError> {
        if bytes.iter().all(|byte| (0x20..=0x7e).contains(byte)) {
            Ok(Self(bytes))
        } else {
            Err(OpenTypeTagError::NonPrintableAscii)
        }
    }

    #[must_use]
    pub const fn bytes(self) -> [u8; 4] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenTypeTagError {
    NonPrintableAscii,
}

impl fmt::Display for OpenTypeTagError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OpenType tag contains a non-printable ASCII byte")
    }
}

impl Error for OpenTypeTagError {}

#[derive(Clone, Copy, Debug, PartialEq)]
/// One normalized OpenType variation-axis setting.
pub struct FontVariation {
    tag: OpenTypeTag,
    value: f32,
}

impl FontVariation {
    /// Creates one finite variation setting.
    ///
    /// # Errors
    ///
    /// Returns [`FontVariationError::NotFinite`] when `value` is not finite.
    pub fn new(tag: OpenTypeTag, value: f32) -> Result<Self, FontVariationError> {
        if value.is_finite() {
            Ok(Self { tag, value })
        } else {
            Err(FontVariationError::NotFinite)
        }
    }

    #[must_use]
    pub const fn tag(self) -> OpenTypeTag {
        self.tag
    }

    #[must_use]
    pub const fn value(self) -> f32 {
        self.value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontVariationError {
    NotFinite,
}

impl fmt::Display for FontVariationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("font variation value is not finite")
    }
}

impl Error for FontVariationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// One normalized OpenType feature setting.
pub struct FontFeature {
    tag: OpenTypeTag,
    value: u16,
}

impl FontFeature {
    #[must_use]
    pub const fn new(tag: OpenTypeTag, value: u16) -> Self {
        Self { tag, value }
    }

    #[must_use]
    pub const fn tag(self) -> OpenTypeTag {
        self.tag
    }

    #[must_use]
    pub const fn value(self) -> u16 {
        self.value
    }
}

#[derive(Clone, Debug, PartialEq)]
/// Complete metric typography value participating atomically in style resolution.
///
/// Family fallback order is semantic. Variation and feature settings are kept in
/// canonical tag order with at most one value per tag so equivalent authored
/// settings compare identically for cache compatibility.
pub struct Typography {
    families: Vec<FontFamily>,
    size: LogicalLength,
    weight: FontWeight,
    width: FontWidth,
    style: FontStyle,
    variations: Vec<FontVariation>,
    features: Vec<FontFeature>,
}

impl Typography {
    #[must_use]
    pub fn new(primary_family: FontFamily, size: LogicalLength) -> Self {
        Self {
            families: vec![primary_family],
            size,
            weight: FontWeight::NORMAL,
            width: FontWidth::NORMAL,
            style: FontStyle::Normal,
            variations: Vec::new(),
            features: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_fallback(mut self, family: FontFamily) -> Self {
        if !self.families.contains(&family) {
            self.families.push(family);
        }
        self
    }

    #[must_use]
    pub const fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    #[must_use]
    pub const fn with_width(mut self, width: FontWidth) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub const fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    #[must_use]
    pub fn with_variation(mut self, variation: FontVariation) -> Self {
        upsert_by_tag(&mut self.variations, variation, FontVariation::tag);
        self
    }

    #[must_use]
    pub fn with_feature(mut self, feature: FontFeature) -> Self {
        upsert_by_tag(&mut self.features, feature, FontFeature::tag);
        self
    }

    #[must_use]
    pub const fn families(&self) -> &[FontFamily] {
        self.families.as_slice()
    }

    #[must_use]
    pub const fn size(&self) -> LogicalLength {
        self.size
    }

    #[must_use]
    pub const fn weight(&self) -> FontWeight {
        self.weight
    }

    #[must_use]
    pub const fn width(&self) -> FontWidth {
        self.width
    }

    #[must_use]
    pub const fn style(&self) -> FontStyle {
        self.style
    }

    #[must_use]
    pub const fn variations(&self) -> &[FontVariation] {
        self.variations.as_slice()
    }

    #[must_use]
    pub const fn features(&self) -> &[FontFeature] {
        self.features.as_slice()
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self::new(
            FontFamily::generic(GenericFontFamily::SansSerif),
            LogicalLength::from(16_u8),
        )
    }
}

fn upsert_by_tag<Value: Copy>(
    values: &mut Vec<Value>,
    value: Value,
    tag: impl Fn(Value) -> OpenTypeTag,
) {
    let value_tag = tag(value);
    if let Some(existing) = values.iter_mut().find(|existing| tag(**existing) == value_tag) {
        *existing = value;
    } else {
        values.push(value);
    }
    values.sort_by_key(|value| tag(*value));
}

#[cfg(test)]
mod tests {
    use super::{
        FontFamily, FontFamilyName, FontFamilyNameError, FontFeature, FontObliqueAngle,
        FontObliqueAngleError, FontStyle, FontVariation, FontWeight, FontWeightError, FontWidth,
        FontWidthError, GenericFontFamily, OpenTypeTag, Typography,
    };
    use crate::LogicalLength;

    #[test]
    fn family_names_are_canonical_but_preserve_internal_whitespace() {
        assert_eq!(
            FontFamilyName::new("Noto Sans").as_ref().map(FontFamilyName::as_str),
            Ok("Noto Sans")
        );
        assert_eq!(FontFamilyName::new(""), Err(FontFamilyNameError::Empty));
        assert_eq!(
            FontFamilyName::new(" Noto Sans"),
            Err(FontFamilyNameError::SurroundingWhitespace)
        );
        assert_eq!(
            FontFamilyName::new("Noto\nSans"),
            Err(FontFamilyNameError::ControlCharacter)
        );
    }

    #[test]
    fn weight_width_and_oblique_angle_reject_invalid_numeric_values() {
        assert_eq!(FontWeight::new(f32::NAN), Err(FontWeightError::NotFinite));
        assert_eq!(FontWeight::new(0.0), Err(FontWeightError::OutOfRange));
        assert_eq!(FontWeight::new(1000.0).map(FontWeight::get), Ok(1000.0));
        assert_eq!(FontWidth::new(f32::INFINITY), Err(FontWidthError::NotFinite));
        assert_eq!(FontWidth::new(0.49), Err(FontWidthError::OutOfRange));
        assert_eq!(FontWidth::new(2.0).map(FontWidth::ratio), Ok(2.0));
        assert_eq!(
            FontObliqueAngle::new(f32::NEG_INFINITY),
            Err(FontObliqueAngleError::NotFinite)
        );
        let angle = FontObliqueAngle::new(21.5).expect("finite fixture angle");
        assert_eq!(FontStyle::Oblique(Some(angle)), FontStyle::Oblique(Some(angle)));
        assert_eq!(angle.degrees(), 21.5);
    }

    #[test]
    fn typography_normalizes_fallback_and_setting_identity()
    -> Result<(), Box<dyn std::error::Error>> {
        let liga = OpenTypeTag::new(*b"liga")?;
        let kern = OpenTypeTag::new(*b"kern")?;
        let wght = OpenTypeTag::new(*b"wght")?;
        let wdth = OpenTypeTag::new(*b"wdth")?;
        let primary = FontFamily::named("Cantarell")?;
        let fallback = FontFamily::generic(GenericFontFamily::SansSerif);
        let typography = Typography::new(primary.clone(), LogicalLength::new(18.0)?)
            .with_fallback(fallback.clone())
            .with_fallback(primary)
            .with_feature(FontFeature::new(liga, 1))
            .with_feature(FontFeature::new(kern, 1))
            .with_feature(FontFeature::new(liga, 0))
            .with_variation(FontVariation::new(wght, 520.0)?)
            .with_variation(FontVariation::new(wdth, 95.0)?)
            .with_variation(FontVariation::new(wght, 540.0)?);

        assert_eq!(typography.families(), &[FontFamily::named("Cantarell")?, fallback]);
        assert_eq!(typography.features().len(), 2);
        assert_eq!(typography.features()[0].tag().bytes(), *b"kern");
        assert_eq!(typography.features()[1].tag().bytes(), *b"liga");
        assert_eq!(typography.features()[1].value(), 0);
        assert_eq!(typography.variations().len(), 2);
        assert_eq!(typography.variations()[0].tag().bytes(), *b"wdth");
        assert_eq!(typography.variations()[1].tag().bytes(), *b"wght");
        assert_eq!(typography.variations()[1].value(), 540.0);
        Ok(())
    }
}
