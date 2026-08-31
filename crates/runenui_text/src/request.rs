use core::{error::Error, fmt, ops::Range};

use parley::Language;
use runenui_core::Typography;

use crate::TextConstraints;

/// Canonical language/script/region input for shaping and fallback.
///
/// RunenUI intentionally models the BCP 47 prefix that Parley consumes today.
/// Variants, extensions, and private-use subtags are rejected rather than silently
/// discarded from shaping identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextLanguage(String);

impl TextLanguage {
    /// Validates and canonicalizes a language tag used for text shaping.
    ///
    /// # Errors
    ///
    /// Returns [`TextLanguageError::Invalid`] for malformed language/script/region
    /// input and [`TextLanguageError::UnsupportedSubtags`] when otherwise-valid
    /// variants, extensions, or private-use subtags are present.
    pub fn new(value: impl Into<String>) -> Result<Self, TextLanguageError> {
        let value = value.into();
        let parsed = Language::parse(&value).map_err(|_| TextLanguageError::Invalid)?;
        let (_, remainder) =
            Language::parse_prefix(&value).map_err(|_| TextLanguageError::Invalid)?;
        if !remainder.is_empty() {
            return Err(TextLanguageError::UnsupportedSubtags);
        }
        Ok(Self(parsed.as_str().to_owned()))
    }

    /// Returns the canonical `language[-Script][-REGION]` form.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Failure while constructing a RunenUI language input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextLanguageError {
    Invalid,
    UnsupportedSubtags,
}

impl fmt::Display for TextLanguageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Invalid => "text language is not a valid language/script/region tag",
            Self::UnsupportedSubtags => {
                "text language contains variants, extensions, or private-use subtags"
            }
        })
    }
}

impl Error for TextLanguageError {}

/// Logical paragraph alignment independent of the shaping implementation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextAlignment {
    #[default]
    Start,
    End,
    Center,
    Justify,
}

/// Ordinary soft-wrap policy for one paragraph.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextWrapMode {
    #[default]
    Wrap,
    NoWrap,
}

/// Policy controlling ordinary word-internal break opportunities.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextWordBreak {
    #[default]
    Normal,
    BreakAll,
    KeepAll,
}

/// Policy controlling emergency wrapping for otherwise-unbreakable content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextOverflowWrap {
    #[default]
    Normal,
    Anywhere,
    BreakWord,
}

/// Text-only paragraph policy consumed by production shaping and line breaking.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TextParagraphStyle {
    language: Option<TextLanguage>,
    alignment: TextAlignment,
    wrap_mode: TextWrapMode,
    word_break: TextWordBreak,
    overflow_wrap: TextOverflowWrap,
}

impl TextParagraphStyle {
    /// Sets the paragraph language used by shaping and fallback.
    #[must_use]
    pub fn with_language(mut self, language: TextLanguage) -> Self {
        self.language = Some(language);
        self
    }

    /// Sets logical paragraph alignment.
    #[must_use]
    pub const fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Sets ordinary wrapping behavior.
    #[must_use]
    pub const fn with_wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.wrap_mode = wrap_mode;
        self
    }

    /// Sets ordinary word-break behavior.
    #[must_use]
    pub const fn with_word_break(mut self, word_break: TextWordBreak) -> Self {
        self.word_break = word_break;
        self
    }

    /// Sets emergency overflow wrapping behavior.
    #[must_use]
    pub const fn with_overflow_wrap(mut self, overflow_wrap: TextOverflowWrap) -> Self {
        self.overflow_wrap = overflow_wrap;
        self
    }

    /// Returns the paragraph language, when explicitly authored.
    #[must_use]
    pub const fn language(&self) -> Option<&TextLanguage> {
        self.language.as_ref()
    }

    /// Returns logical paragraph alignment.
    #[must_use]
    pub const fn alignment(&self) -> TextAlignment {
        self.alignment
    }

    /// Returns ordinary wrapping behavior.
    #[must_use]
    pub const fn wrap_mode(&self) -> TextWrapMode {
        self.wrap_mode
    }

    /// Returns ordinary word-break behavior.
    #[must_use]
    pub const fn word_break(&self) -> TextWordBreak {
        self.word_break
    }

    /// Returns emergency overflow wrapping behavior.
    #[must_use]
    pub const fn overflow_wrap(&self) -> TextOverflowWrap {
        self.overflow_wrap
    }
}

/// One non-overlapping UTF-8 byte range with complete metric typography.
///
/// Paint-only state such as foreground color is deliberately absent. Consumers may
/// correlate the preserved source range with paint style without changing shaped
/// identity.
#[derive(Clone, Debug, PartialEq)]
pub struct TextMetricSpan {
    range: Range<usize>,
    typography: Typography,
    language: Option<TextLanguage>,
}

impl TextMetricSpan {
    #[must_use]
    pub const fn new(range: Range<usize>, typography: Typography) -> Self {
        Self {
            range,
            typography,
            language: None,
        }
    }

    /// Overrides shaping language for this metric span.
    #[must_use]
    pub fn with_language(mut self, language: TextLanguage) -> Self {
        self.language = Some(language);
        self
    }

    /// Returns the source UTF-8 byte range covered by this span.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// Returns the complete metric typography for this span.
    #[must_use]
    pub const fn typography(&self) -> &Typography {
        &self.typography
    }

    /// Returns the span-specific shaping language, when any.
    #[must_use]
    pub const fn language(&self) -> Option<&TextLanguage> {
        self.language.as_ref()
    }
}

/// Immutable RunenUI-owned input to one shaping/line-breaking operation.
#[derive(Clone, Debug, PartialEq)]
pub struct TextRequest {
    text: String,
    typography: Typography,
    constraints: TextConstraints,
    paragraph: TextParagraphStyle,
    metric_spans: Vec<TextMetricSpan>,
}

impl TextRequest {
    /// Creates a request with one complete base typography and default paragraph policy.
    #[must_use]
    pub fn new(
        text: impl Into<String>,
        typography: Typography,
        constraints: TextConstraints,
    ) -> Self {
        Self {
            text: text.into(),
            typography,
            constraints,
            paragraph: TextParagraphStyle::default(),
            metric_spans: Vec::new(),
        }
    }

    /// Replaces the text-only paragraph policy.
    #[must_use]
    pub fn with_paragraph_style(mut self, paragraph: TextParagraphStyle) -> Self {
        self.paragraph = paragraph;
        self
    }

    /// Adds complete metric spans after validating source ranges and deterministic precedence.
    ///
    /// Input order is not semantic: validated spans are normalized into source order. Overlap is
    /// rejected so dependency insertion order cannot become a hidden RunenUI precedence rule.
    ///
    /// # Errors
    ///
    /// Returns [`TextRequestError`] when a span is empty/reversed, out of bounds, does not end on
    /// UTF-8 character boundaries, or overlaps another metric span.
    pub fn try_with_metric_spans(
        mut self,
        mut spans: Vec<TextMetricSpan>,
    ) -> Result<Self, TextRequestError> {
        for (index, span) in spans.iter().enumerate() {
            if span.range.start >= span.range.end {
                return Err(TextRequestError::InvalidSpanRange { index });
            }
            if span.range.end > self.text.len() {
                return Err(TextRequestError::SpanOutOfBounds { index });
            }
            if !self.text.is_char_boundary(span.range.start)
                || !self.text.is_char_boundary(span.range.end)
            {
                return Err(TextRequestError::SpanNotCharBoundary { index });
            }
        }

        spans.sort_by_key(|span| (span.range.start, span.range.end));
        if spans
            .windows(2)
            .any(|pair| pair[1].range.start < pair[0].range.end)
        {
            return Err(TextRequestError::OverlappingSpans);
        }
        self.metric_spans = spans;
        Ok(self)
    }

    /// Returns the complete source text.
    #[must_use]
    pub const fn text(&self) -> &str {
        self.text.as_str()
    }

    /// Returns the complete base metric typography.
    #[must_use]
    pub const fn typography(&self) -> &Typography {
        &self.typography
    }

    /// Returns renderer/runtime-neutral logical text constraints.
    #[must_use]
    pub const fn constraints(&self) -> TextConstraints {
        self.constraints
    }

    /// Returns paragraph language/wrap/alignment policy.
    #[must_use]
    pub const fn paragraph_style(&self) -> &TextParagraphStyle {
        &self.paragraph
    }

    /// Returns normalized non-overlapping metric spans in source order.
    #[must_use]
    pub fn metric_spans(&self) -> &[TextMetricSpan] {
        &self.metric_spans
    }
}

/// Failure while validating metric spans for one [`TextRequest`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRequestError {
    InvalidSpanRange { index: usize },
    SpanOutOfBounds { index: usize },
    SpanNotCharBoundary { index: usize },
    OverlappingSpans,
}

impl fmt::Display for TextRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpanRange { index } => {
                write!(
                    formatter,
                    "text metric span {index} has an empty or reversed range"
                )
            }
            Self::SpanOutOfBounds { index } => {
                write!(
                    formatter,
                    "text metric span {index} exceeds the source text"
                )
            }
            Self::SpanNotCharBoundary { index } => write!(
                formatter,
                "text metric span {index} does not end on UTF-8 character boundaries"
            ),
            Self::OverlappingSpans => formatter.write_str("text metric spans overlap"),
        }
    }
}

impl Error for TextRequestError {}
