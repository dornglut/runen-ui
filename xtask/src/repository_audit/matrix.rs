use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use super::Finding;

const MATRIX_PATH: &str = "docs/architecture/m4-conformance-matrix.md";
const ALLOWED_STATUSES: &[&str] = &[
    "blocked",
    "implementation-complete",
    "proof-complete",
    "owner-accepted",
];
const ALLOWED_DELIVERY_SLICES: &[&str] = &[
    "M4A", "M4B", "M4C0", "M4C1", "M4C2", "M4C3", "M4C4", "M4C5", "M4D1", "M4D2", "M4D3", "M5",
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MatrixMetrics {
    pub(super) total_rows: usize,
    pub(super) owner_accepted: usize,
    pub(super) implementation_complete: usize,
    pub(super) proof_complete: usize,
    pub(super) blocked: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MatrixRow {
    line: usize,
    cells: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MatrixSummary {
    total_rows: Option<usize>,
    owner_accepted: Option<usize>,
    implementation_complete: Option<usize>,
    proof_complete: Option<usize>,
    blocked: Option<usize>,
    duplicate_ids: Option<usize>,
    invalid_statuses: Option<usize>,
    invalid_schemas: Option<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RowAnalysis {
    metrics: MatrixMetrics,
    duplicate_ids: usize,
    invalid_statuses: usize,
    invalid_schemas: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct RowState {
    analysis: RowAnalysis,
    seen_ids: BTreeSet<String>,
    status_counts: BTreeMap<String, usize>,
}

pub(super) fn audit(root: &Path, findings: &mut Vec<Finding>) -> Result<MatrixMetrics, String> {
    let path = root.join(MATRIX_PATH);
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {MATRIX_PATH}: {error}"))?;
    let summary = parse_summary(&contents);
    let rows = parse_rows(&contents, findings);
    let analysis = analyze_rows(&rows, findings);

    validate_status_total(&analysis, findings);
    compare_declared_summary(&summary, &analysis, findings);

    Ok(analysis.metrics)
}

fn analyze_rows(rows: &[MatrixRow], findings: &mut Vec<Finding>) -> RowAnalysis {
    let mut state = RowState::default();
    state.analysis.invalid_schemas = findings
        .iter()
        .filter(|finding| finding.code == "matrix.invalid_schema")
        .count();

    for row in rows {
        audit_row(row, &mut state, findings);
    }

    state.analysis.metrics = MatrixMetrics {
        total_rows: rows.len(),
        owner_accepted: status_count(&state.status_counts, "owner-accepted"),
        implementation_complete: status_count(&state.status_counts, "implementation-complete"),
        proof_complete: status_count(&state.status_counts, "proof-complete"),
        blocked: status_count(&state.status_counts, "blocked"),
    };
    state.analysis
}

fn audit_row(row: &MatrixRow, state: &mut RowState, findings: &mut Vec<Finding>) {
    let id = &row.cells[0];
    if !valid_id(id) {
        state.analysis.invalid_schemas += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_schema",
            Some(format!("{MATRIX_PATH}:{}", row.line)),
            format!("matrix ID `{id}` does not match the permanent FAMILY-NN contract"),
        ));
    }
    if !state.seen_ids.insert(id.clone()) {
        state.analysis.duplicate_ids += 1;
        findings.push(Finding::fatal(
            "matrix.duplicate_id",
            Some(format!("{MATRIX_PATH}:{}", row.line)),
            format!("matrix ID `{id}` is duplicated"),
        ));
    }

    if row.cells.iter().any(String::is_empty) {
        state.analysis.invalid_schemas += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_schema",
            Some(format!("{MATRIX_PATH}:{}", row.line)),
            format!("matrix row `{id}` contains an empty required column"),
        ));
    }

    let delivery_slice = &row.cells[5];
    if !ALLOWED_DELIVERY_SLICES.contains(&delivery_slice.as_str()) {
        state.analysis.invalid_schemas += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_schema",
            Some(format!("{MATRIX_PATH}:{}", row.line)),
            format!("matrix row `{id}` has invalid delivery slice `{delivery_slice}`"),
        ));
    }

    let status = &row.cells[6];
    if ALLOWED_STATUSES.contains(&status.as_str()) {
        *state.status_counts.entry(status.clone()).or_default() += 1;
    } else {
        state.analysis.invalid_statuses += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_status",
            Some(format!("{MATRIX_PATH}:{}", row.line)),
            format!("matrix row `{id}` has invalid status `{status}`"),
        ));
    }

    validate_gate(row, delivery_slice, findings, &mut state.analysis);
}

fn validate_gate(
    row: &MatrixRow,
    delivery_slice: &str,
    findings: &mut Vec<Finding>,
    analysis: &mut RowAnalysis,
) {
    let gate = &row.cells[7];
    let expected_gate = if delivery_slice == "M5" {
        "M5 gate"
    } else {
        "Required"
    };
    if gate == expected_gate {
        return;
    }

    analysis.invalid_schemas += 1;
    findings.push(Finding::fatal(
        "matrix.invalid_schema",
        Some(format!("{MATRIX_PATH}:{}", row.line)),
        format!(
            "matrix row `{}` uses gate `{gate}`; delivery slice `{delivery_slice}` requires `{expected_gate}`",
            row.cells[0]
        ),
    ));
}

fn validate_status_total(analysis: &RowAnalysis, findings: &mut Vec<Finding>) {
    let metrics = &analysis.metrics;
    if metrics.owner_accepted
        + metrics.implementation_complete
        + metrics.proof_complete
        + metrics.blocked
        + analysis.invalid_statuses
        == metrics.total_rows
    {
        return;
    }

    findings.push(Finding::fatal(
        "matrix.inconsistent_status_total",
        Some(MATRIX_PATH.to_owned()),
        "matrix row total does not equal the sum of valid and invalid statuses",
    ));
}

fn compare_declared_summary(
    summary: &MatrixSummary,
    analysis: &RowAnalysis,
    findings: &mut Vec<Finding>,
) {
    let metrics = &analysis.metrics;
    compare_summary(
        findings,
        "total unique rows",
        summary.total_rows,
        metrics.total_rows,
    );
    compare_summary(
        findings,
        "owner-accepted",
        summary.owner_accepted,
        metrics.owner_accepted,
    );
    compare_optional_zero_summary(
        findings,
        "implementation-complete",
        summary.implementation_complete,
        metrics.implementation_complete,
    );
    compare_summary(
        findings,
        "proof-complete",
        summary.proof_complete,
        metrics.proof_complete,
    );
    compare_summary(findings, "blocked", summary.blocked, metrics.blocked);
    compare_summary(
        findings,
        "duplicate IDs",
        summary.duplicate_ids,
        analysis.duplicate_ids,
    );
    compare_summary(
        findings,
        "invalid statuses",
        summary.invalid_statuses,
        analysis.invalid_statuses,
    );
    compare_summary(
        findings,
        "invalid schemas",
        summary.invalid_schemas,
        analysis.invalid_schemas,
    );
}

fn parse_rows(contents: &str, findings: &mut Vec<Finding>) -> Vec<MatrixRow> {
    let mut rows = Vec::new();

    for (line_index, line) in contents.lines().enumerate() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || is_header_or_separator(trimmed) {
            continue;
        }

        let cells = trimmed
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let line_number = line_index + 1;

        if cells.len() != 8 {
            if cells.first().is_some_and(|cell| looks_like_id(cell)) {
                findings.push(Finding::fatal(
                    "matrix.invalid_schema",
                    Some(format!("{MATRIX_PATH}:{line_number}")),
                    format!(
                        "matrix data row has {} columns; the row contract requires 8",
                        cells.len()
                    ),
                ));
            }
            continue;
        }

        if cells.first().is_some_and(|cell| looks_like_id(cell)) {
            rows.push(MatrixRow {
                line: line_number,
                cells,
            });
        }
    }

    rows
}

fn is_header_or_separator(line: &str) -> bool {
    line.starts_with("| ID |")
        || line
            .chars()
            .all(|character| matches!(character, '|' | '-' | ':' | ' '))
}

fn looks_like_id(value: &str) -> bool {
    value.contains('-') && value.chars().any(|character| character.is_ascii_digit())
}

fn valid_id(id: &str) -> bool {
    let Some((family, number)) = id.rsplit_once('-') else {
        return false;
    };
    if number.len() != 2 || !number.chars().all(|character| character.is_ascii_digit()) {
        return false;
    }
    family.split('-').all(|segment| {
        !segment.is_empty()
            && segment
                .chars()
                .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit())
    })
}

fn parse_summary(contents: &str) -> MatrixSummary {
    MatrixSummary {
        total_rows: declared_metric(contents, "total unique rows"),
        owner_accepted: declared_metric(contents, "owner-accepted"),
        implementation_complete: declared_metric(contents, "implementation-complete"),
        proof_complete: declared_metric(contents, "proof-complete"),
        blocked: declared_metric(contents, "blocked"),
        duplicate_ids: declared_metric(contents, "duplicate IDs"),
        invalid_statuses: declared_metric(contents, "invalid statuses"),
        invalid_schemas: declared_metric(contents, "invalid schemas"),
    }
}

fn declared_metric(contents: &str, label: &str) -> Option<usize> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        let count = trimmed.strip_suffix(label)?.trim();
        count.parse::<usize>().ok()
    })
}

fn status_count(counts: &BTreeMap<String, usize>, status: &str) -> usize {
    counts.get(status).copied().unwrap_or_default()
}

fn compare_summary(
    findings: &mut Vec<Finding>,
    label: &str,
    declared: Option<usize>,
    actual: usize,
) {
    match declared {
        Some(declared) if declared == actual => {}
        Some(declared) => findings.push(Finding::fatal(
            "matrix.inconsistent_summary",
            Some(MATRIX_PATH.to_owned()),
            format!("declared `{label}` count is {declared}, but the matrix contains {actual}"),
        )),
        None => findings.push(Finding::fatal(
            "matrix.missing_summary_metric",
            Some(MATRIX_PATH.to_owned()),
            format!("matrix summary is missing the `{label}` count"),
        )),
    }
}

fn compare_optional_zero_summary(
    findings: &mut Vec<Finding>,
    label: &str,
    declared: Option<usize>,
    actual: usize,
) {
    if declared.is_none() && actual == 0 {
        return;
    }
    compare_summary(findings, label, declared, actual);
}

#[cfg(test)]
mod tests {
    use super::{declared_metric, parse_rows, valid_id};

    #[test]
    fn permanent_id_parser_accepts_multi_segment_families() {
        assert!(valid_id("PTR-01"));
        assert!(valid_id("TRACE-EVENT-10"));
        assert!(valid_id("M4-CLOSE-05"));
        assert!(!valid_id("PTR-1"));
        assert!(!valid_id("ptr-01"));
        assert!(!valid_id("PTR--01"));
    }

    #[test]
    fn matrix_row_parser_rejects_wrong_column_count() {
        let mut findings = Vec::new();
        let rows = parse_rows(
            "| ID | A | B | C | D | E | F | G |\n|---|---|---|---|---|---|---|---|\n| PTR-01 | A | B | C | D | E | F |\n",
            &mut findings,
        );
        assert!(rows.is_empty());
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn summary_parser_uses_exact_labels() {
        assert_eq!(
            declared_metric("237 total unique rows\n", "total unique rows"),
            Some(237)
        );
        assert_eq!(
            declared_metric("237 total rows\n", "total unique rows"),
            None
        );
    }
}
