//! Offline validation and causal reconstruction of exported trace JSONL.
//!
//! Replay consumes the serialized trace projection. It never owns or recreates
//! live runtime authority.

use core::fmt;

use serde_json::{Map, Value};

const TRACE_SCHEMA: &str = "runenui.trace";
const TRACE_RECORD_SCHEMA: &str = "runenui.trace.record";
const TRACE_VERSION: u64 = 1;

/// Sequence identity parsed from an exported trace.
///
/// This is replay-only diagnostic identity. It cannot be converted into the
/// runtime-issued [`crate::TraceSequence`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TraceReplaySequence(u64);

impl TraceReplaySequence {
    const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    /// Returns the serialized sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Work-sequence identity parsed from an exported trace.
///
/// This value is observational only and cannot be used as live queue authority.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TraceReplayWorkSequence(u64);

impl TraceReplayWorkSequence {
    const fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    /// Returns the serialized work-sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Stable record-kind name carried by the versioned trace protocol.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TraceReplayKind(Box<str>);

impl TraceReplayKind {
    /// Returns the serialized stable kind name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether the parsed projection contains the complete canonical trace prefix.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TraceReplayCompleteness {
    /// No canonical prefix was reported as dropped.
    Complete,
    /// Canonical records before this exclusive watermark were already evicted.
    DroppedPrefix { before: TraceReplaySequence },
}

/// One inert replay record reconstructed from the versioned JSONL projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceReplayRecord {
    sequence: TraceReplaySequence,
    kind: TraceReplayKind,
    work_sequence: Option<TraceReplayWorkSequence>,
    causal_parent: Option<TraceReplaySequence>,
    reconciliation_before: Option<u64>,
    reconciliation_after: Option<u64>,
    instant_nanos: Option<u64>,
}

impl TraceReplayRecord {
    /// Returns this record's replay-only trace sequence.
    #[must_use]
    pub const fn sequence(&self) -> TraceReplaySequence {
        self.sequence
    }

    /// Returns the stable protocol kind name.
    #[must_use]
    pub const fn kind(&self) -> &TraceReplayKind {
        &self.kind
    }

    /// Returns the replay-only work sequence when the exported record has one.
    #[must_use]
    pub const fn work_sequence(&self) -> Option<TraceReplayWorkSequence> {
        self.work_sequence
    }

    /// Returns the replay-only causal parent sequence when one was exported.
    #[must_use]
    pub const fn causal_parent(&self) -> Option<TraceReplaySequence> {
        self.causal_parent
    }

    /// Returns the exported reconciliation generation before the operation.
    #[must_use]
    pub const fn reconciliation_before(&self) -> Option<u64> {
        self.reconciliation_before
    }

    /// Returns the exported reconciliation generation after the operation.
    #[must_use]
    pub const fn reconciliation_after(&self) -> Option<u64> {
        self.reconciliation_after
    }

    /// Returns exported logical monotonic time in nanoseconds when present.
    #[must_use]
    pub const fn instant_nanos(&self) -> Option<u64> {
        self.instant_nanos
    }
}

/// Parsed offline trace projection used for deterministic causal reconstruction.
///
/// This type owns diagnostic facts only. It has no path back into a live
/// [`crate::AppRuntime`] and cannot submit, pump, route, mutate, or schedule work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceReplay {
    dropped_before_sequence: Option<TraceReplaySequence>,
    records: Vec<TraceReplayRecord>,
}

impl TraceReplay {
    /// Supported trace JSONL protocol version.
    pub const VERSION: u64 = TRACE_VERSION;

    /// Parses and structurally validates one exported trace JSONL document.
    ///
    /// The first line must be the `runenui.trace` v1 header and every following
    /// line must be a `runenui.trace.record` v1 record. Retained trace sequences
    /// must form the exact contiguous canonical segment described by the header.
    /// Causal parents must be strictly earlier; parents below an explicit
    /// dropped-prefix watermark describe valid but incomplete reconstruction.
    ///
    /// # Errors
    ///
    /// Returns [`TraceReplayError`] for malformed JSON, schema/version mismatch,
    /// invalid sequence structure, retained-count mismatch, or invalid causal
    /// ancestry.
    pub fn parse_jsonl(input: &str) -> Result<Self, TraceReplayError> {
        let mut lines = input.lines().enumerate();
        let Some((header_index, header_line)) = lines.next() else {
            return Err(TraceReplayError::EmptyInput);
        };
        let header_line_number = header_index + 1;
        if header_line.is_empty() {
            return Err(TraceReplayError::EmptyInput);
        }

        let header_value = parse_json_line(header_line_number, header_line)?;
        let header = object(header_line_number, &header_value)?;
        validate_schema_and_version(header_line_number, header, TRACE_SCHEMA)?;
        let dropped_before_sequence =
            optional_replay_sequence(header_line_number, header, "dropped_before_sequence")?;
        let retained_records = required_usize(header_line_number, header, "retained_records")?;

        let mut records = Vec::with_capacity(retained_records.min(4096));
        let mut previous_sequence = None;

        for (index, line) in lines {
            let line_number = index + 1;
            if line.is_empty() {
                return Err(TraceReplayError::EmptyLine { line: line_number });
            }
            let record = parse_record(line_number, line)?;
            validate_contiguous_sequence(
                line_number,
                record.sequence,
                dropped_before_sequence,
                previous_sequence,
            )?;
            validate_causal_parent(line_number, &record, dropped_before_sequence)?;
            previous_sequence = Some(record.sequence);
            records.push(record);
        }

        if retained_records != records.len() {
            return Err(TraceReplayError::RetainedRecordCountMismatch {
                declared: retained_records,
                actual: records.len(),
            });
        }

        Ok(Self {
            dropped_before_sequence,
            records,
        })
    }

    /// Returns retained replay records in strictly increasing sequence order.
    #[must_use]
    pub fn records(&self) -> impl ExactSizeIterator<Item = &TraceReplayRecord> {
        self.records.iter()
    }

    /// Looks up one retained record by replay-only sequence identity.
    #[must_use]
    pub fn record(&self, sequence: TraceReplaySequence) -> Option<&TraceReplayRecord> {
        self.records
            .binary_search_by_key(&sequence, TraceReplayRecord::sequence)
            .ok()
            .map(|index| &self.records[index])
    }

    /// Returns the exclusive dropped-prefix watermark from the export header.
    #[must_use]
    pub const fn dropped_before_sequence(&self) -> Option<TraceReplaySequence> {
        self.dropped_before_sequence
    }

    /// Reports whether canonical ancestry before a watermark was already evicted.
    #[must_use]
    pub const fn completeness(&self) -> TraceReplayCompleteness {
        match self.dropped_before_sequence {
            Some(before) => TraceReplayCompleteness::DroppedPrefix { before },
            None => TraceReplayCompleteness::Complete,
        }
    }

    /// Returns true only when the export reports no dropped canonical prefix.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self.completeness(), TraceReplayCompleteness::Complete)
    }
}

/// Structural failure while decoding a versioned trace projection.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceReplayError {
    EmptyInput,
    EmptyLine {
        line: usize,
    },
    InvalidJson {
        line: usize,
        message: Box<str>,
    },
    ExpectedObject {
        line: usize,
    },
    MissingField {
        line: usize,
        field: &'static str,
    },
    InvalidFieldType {
        line: usize,
        field: &'static str,
    },
    IntegerOutOfRange {
        line: usize,
        field: &'static str,
    },
    SchemaMismatch {
        line: usize,
        expected: &'static str,
        actual: Box<str>,
    },
    UnsupportedVersion {
        line: usize,
        schema: &'static str,
        version: u64,
    },
    InvalidSequence {
        line: usize,
        field: &'static str,
        value: u64,
    },
    NonContiguousSequence {
        line: usize,
        expected: u64,
        actual: TraceReplaySequence,
    },
    CausalParentNotEarlier {
        line: usize,
        sequence: TraceReplaySequence,
        parent: TraceReplaySequence,
    },
    RetainedRecordCountMismatch {
        declared: usize,
        actual: usize,
    },
}

impl fmt::Display for TraceReplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("trace replay input is empty"),
            Self::EmptyLine { line } => write!(formatter, "trace replay line {line} is empty"),
            Self::InvalidJson { line, message } => {
                write!(
                    formatter,
                    "trace replay line {line} is invalid JSON: {message}"
                )
            }
            Self::ExpectedObject { line } => {
                write!(formatter, "trace replay line {line} must be a JSON object")
            }
            Self::MissingField { line, field } => {
                write!(
                    formatter,
                    "trace replay line {line} is missing field `{field}`"
                )
            }
            Self::InvalidFieldType { line, field } => write!(
                formatter,
                "trace replay line {line} has the wrong type for field `{field}`"
            ),
            Self::IntegerOutOfRange { line, field } => write!(
                formatter,
                "trace replay line {line} has an out-of-range integer in field `{field}`"
            ),
            Self::SchemaMismatch {
                line,
                expected,
                actual,
            } => write!(
                formatter,
                "trace replay line {line} has schema `{actual}`; expected `{expected}`"
            ),
            Self::UnsupportedVersion {
                line,
                schema,
                version,
            } => write!(
                formatter,
                "trace replay line {line} uses unsupported `{schema}` version {version}"
            ),
            Self::InvalidSequence { line, field, value } => write!(
                formatter,
                "trace replay line {line} has invalid zero `{field}` sequence value {value}"
            ),
            Self::NonContiguousSequence {
                line,
                expected,
                actual,
            } => write!(
                formatter,
                "trace replay line {line} has sequence {}; expected contiguous sequence {expected}",
                actual.get()
            ),
            Self::CausalParentNotEarlier {
                line,
                sequence,
                parent,
            } => write!(
                formatter,
                "trace replay line {line} sequence {} has non-earlier causal parent {}",
                sequence.get(),
                parent.get()
            ),
            Self::RetainedRecordCountMismatch { declared, actual } => write!(
                formatter,
                "trace replay header declares {declared} retained records but contains {actual}"
            ),
        }
    }
}

impl std::error::Error for TraceReplayError {}

fn parse_record(line_number: usize, line: &str) -> Result<TraceReplayRecord, TraceReplayError> {
    let value = parse_json_line(line_number, line)?;
    let record = object(line_number, &value)?;
    validate_schema_and_version(line_number, record, TRACE_RECORD_SCHEMA)?;

    let sequence = required_replay_sequence(line_number, record, "sequence")?;
    let kind_object = required_object(line_number, record, "kind")?;
    let kind_name = required_string(line_number, kind_object, "name")?;
    let _ = required_object(line_number, kind_object, "data")?;
    let kind = TraceReplayKind(kind_name.into());

    let work_sequence = optional_replay_work_sequence(line_number, record, "work_sequence")?;
    let causal_parent = optional_replay_sequence(line_number, record, "causal_parent")?;
    let reconciliation_before = optional_u64(line_number, record, "reconciliation_before")?;
    let reconciliation_after = optional_u64(line_number, record, "reconciliation_after")?;
    let instant_nanos = optional_u64(line_number, record, "instant_nanos")?;

    for field in [
        "target",
        "work",
        "original_target",
        "current_target",
        "command_origin",
    ] {
        validate_nullable_object(line_number, record, field)?;
    }
    let _ = required_object(line_number, record, "context")?;
    validate_nullable_string(line_number, record, "sink_delivery")?;

    Ok(TraceReplayRecord {
        sequence,
        kind,
        work_sequence,
        causal_parent,
        reconciliation_before,
        reconciliation_after,
        instant_nanos,
    })
}

fn validate_contiguous_sequence(
    line: usize,
    current: TraceReplaySequence,
    dropped_before_sequence: Option<TraceReplaySequence>,
    previous: Option<TraceReplaySequence>,
) -> Result<(), TraceReplayError> {
    let expected = match previous {
        Some(previous) => previous.get().checked_add(1).unwrap_or(u64::MAX),
        None => dropped_before_sequence.map_or(1, TraceReplaySequence::get),
    };
    if current.get() != expected {
        return Err(TraceReplayError::NonContiguousSequence {
            line,
            expected,
            actual: current,
        });
    }
    Ok(())
}

fn validate_causal_parent(
    line: usize,
    record: &TraceReplayRecord,
    dropped_before_sequence: Option<TraceReplaySequence>,
) -> Result<(), TraceReplayError> {
    let Some(parent) = record.causal_parent else {
        return Ok(());
    };
    if parent >= record.sequence {
        return Err(TraceReplayError::CausalParentNotEarlier {
            line,
            sequence: record.sequence,
            parent,
        });
    }
    if let Some(dropped_before) = dropped_before_sequence
        && parent < dropped_before
    {
        return Ok(());
    }
    Ok(())
}

fn parse_json_line(line: usize, input: &str) -> Result<Value, TraceReplayError> {
    serde_json::from_str(input).map_err(|error| TraceReplayError::InvalidJson {
        line,
        message: error.to_string().into_boxed_str(),
    })
}

fn object(line: usize, value: &Value) -> Result<&Map<String, Value>, TraceReplayError> {
    value
        .as_object()
        .ok_or(TraceReplayError::ExpectedObject { line })
}

fn validate_schema_and_version(
    line: usize,
    object: &Map<String, Value>,
    expected_schema: &'static str,
) -> Result<(), TraceReplayError> {
    let schema = required_string(line, object, "schema")?;
    if schema != expected_schema {
        return Err(TraceReplayError::SchemaMismatch {
            line,
            expected: expected_schema,
            actual: schema.into(),
        });
    }
    let version = required_u64(line, object, "version")?;
    if version != TRACE_VERSION {
        return Err(TraceReplayError::UnsupportedVersion {
            line,
            schema: expected_schema,
            version,
        });
    }
    Ok(())
}

fn required_value<'a>(
    line: usize,
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Value, TraceReplayError> {
    object
        .get(field)
        .ok_or(TraceReplayError::MissingField { line, field })
}

fn required_object<'a>(
    line: usize,
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a Map<String, Value>, TraceReplayError> {
    required_value(line, object, field)?
        .as_object()
        .ok_or(TraceReplayError::InvalidFieldType { line, field })
}

fn required_string<'a>(
    line: usize,
    object: &'a Map<String, Value>,
    field: &'static str,
) -> Result<&'a str, TraceReplayError> {
    required_value(line, object, field)?
        .as_str()
        .ok_or(TraceReplayError::InvalidFieldType { line, field })
}

fn required_u64(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<u64, TraceReplayError> {
    required_value(line, object, field)?
        .as_u64()
        .ok_or(TraceReplayError::InvalidFieldType { line, field })
}

fn required_usize(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<usize, TraceReplayError> {
    let value = required_u64(line, object, field)?;
    usize::try_from(value).map_err(|_| TraceReplayError::IntegerOutOfRange { line, field })
}

fn optional_u64(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<u64>, TraceReplayError> {
    let value = required_value(line, object, field)?;
    if value.is_null() {
        Ok(None)
    } else {
        value
            .as_u64()
            .map(Some)
            .ok_or(TraceReplayError::InvalidFieldType { line, field })
    }
}

fn validate_nullable_object(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<(), TraceReplayError> {
    let value = required_value(line, object, field)?;
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(TraceReplayError::InvalidFieldType { line, field })
    }
}

fn validate_nullable_string(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<(), TraceReplayError> {
    let value = required_value(line, object, field)?;
    if value.is_null() || value.is_string() {
        Ok(())
    } else {
        Err(TraceReplayError::InvalidFieldType { line, field })
    }
}

fn required_replay_sequence(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<TraceReplaySequence, TraceReplayError> {
    let value = required_u64(line, object, field)?;
    TraceReplaySequence::new(value).ok_or(TraceReplayError::InvalidSequence { line, field, value })
}

fn optional_replay_sequence(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<TraceReplaySequence>, TraceReplayError> {
    optional_u64(line, object, field)?
        .map(|value| {
            TraceReplaySequence::new(value).ok_or(TraceReplayError::InvalidSequence {
                line,
                field,
                value,
            })
        })
        .transpose()
}

fn optional_replay_work_sequence(
    line: usize,
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<Option<TraceReplayWorkSequence>, TraceReplayError> {
    optional_u64(line, object, field)?
        .map(|value| {
            TraceReplayWorkSequence::new(value).ok_or(TraceReplayError::InvalidSequence {
                line,
                field,
                value,
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::{TraceReplay, TraceReplayCompleteness, TraceReplayError};

    fn header(dropped_before: &str, retained: usize) -> String {
        format!(
            "{{\"schema\":\"runenui.trace\",\"version\":1,\"dropped_before_sequence\":{dropped_before},\"retained_records\":{retained}}}"
        )
    }

    fn record(sequence: u64, parent: &str, kind: &str) -> String {
        format!(
            "{{\"schema\":\"runenui.trace.record\",\"version\":1,\"sequence\":{sequence},\"kind\":{{\"name\":\"{kind}\",\"data\":{{}}}},\"work_sequence\":null,\"causal_parent\":{parent},\"reconciliation_before\":null,\"reconciliation_after\":null,\"target\":null,\"work\":null,\"instant_nanos\":null,\"original_target\":null,\"current_target\":null,\"command_origin\":null,\"context\":{{}},\"sink_delivery\":null}}"
        )
    }

    #[test]
    fn complete_projection_parses_and_indexes_causal_records() {
        let input = format!(
            "{}\n{}\n{}\n",
            header("null", 2),
            record(1, "null", "runtime_mounted"),
            record(2, "1", "redraw_requested")
        );
        let replay = TraceReplay::parse_jsonl(&input)
            .unwrap_or_else(|error| unreachable!("fixture must parse: {error}"));
        assert!(replay.is_complete());
        assert_eq!(replay.records().len(), 2);
        let second = replay
            .records()
            .nth(1)
            .unwrap_or_else(|| unreachable!("second replay record exists"));
        let parent = second
            .causal_parent()
            .unwrap_or_else(|| unreachable!("second replay record has a parent"));
        assert_eq!(
            replay
                .record(parent)
                .unwrap_or_else(|| unreachable!("parent is retained"))
                .kind()
                .as_str(),
            "runtime_mounted"
        );
    }

    #[test]
    fn dropped_prefix_explains_missing_older_parent_without_claiming_completeness() {
        let input = format!(
            "{}\n{}\n",
            header("5", 1),
            record(5, "4", "redraw_requested")
        );
        let replay = TraceReplay::parse_jsonl(&input)
            .unwrap_or_else(|error| unreachable!("dropped parent is explained: {error}"));
        assert_eq!(
            replay.completeness(),
            TraceReplayCompleteness::DroppedPrefix {
                before: replay
                    .dropped_before_sequence()
                    .unwrap_or_else(|| unreachable!("watermark exists"))
            }
        );
        assert!(!replay.is_complete());
    }

    #[test]
    fn non_earlier_causal_parent_is_rejected() {
        let input = format!(
            "{}\n{}\n",
            header("null", 1),
            record(1, "2", "redraw_requested")
        );
        assert!(matches!(
            TraceReplay::parse_jsonl(&input),
            Err(TraceReplayError::CausalParentNotEarlier { .. })
        ));
    }

    #[test]
    fn unexplained_sequence_gap_is_rejected() {
        let input = format!(
            "{}\n{}\n{}\n",
            header("null", 2),
            record(1, "null", "runtime_mounted"),
            record(3, "2", "redraw_requested")
        );
        assert!(matches!(
            TraceReplay::parse_jsonl(&input),
            Err(TraceReplayError::NonContiguousSequence {
                expected: 2,
                ..
            })
        ));
    }

    #[test]
    fn dropped_prefix_must_begin_at_the_declared_watermark() {
        let input = format!(
            "{}\n{}\n",
            header("5", 1),
            record(6, "null", "redraw_requested")
        );
        assert!(matches!(
            TraceReplay::parse_jsonl(&input),
            Err(TraceReplayError::NonContiguousSequence {
                expected: 5,
                ..
            })
        ));
    }

    #[test]
    fn wrong_typed_required_top_level_field_is_rejected() {
        let malformed = record(1, "null", "runtime_mounted").replace("\"context\":{}", "\"context\":[]");
        let input = format!("{}\n{malformed}\n", header("null", 1));
        assert!(matches!(
            TraceReplay::parse_jsonl(&input),
            Err(TraceReplayError::InvalidFieldType {
                field: "context",
                ..
            })
        ));
    }

    #[test]
    fn unsupported_version_is_rejected_before_reconstruction() {
        let input = "{\"schema\":\"runenui.trace\",\"version\":2,\"dropped_before_sequence\":null,\"retained_records\":0}\n";
        assert!(matches!(
            TraceReplay::parse_jsonl(input),
            Err(TraceReplayError::UnsupportedVersion { version: 2, .. })
        ));
    }

    #[test]
    fn declared_record_count_must_match_projection() {
        let input = format!(
            "{}\n{}\n",
            header("null", 2),
            record(1, "null", "runtime_mounted")
        );
        assert_eq!(
            TraceReplay::parse_jsonl(&input),
            Err(TraceReplayError::RetainedRecordCountMismatch {
                declared: 2,
                actual: 1
            })
        );
    }
}
