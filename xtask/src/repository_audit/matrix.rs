use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use super::Finding;

const ALLOWED_STATUSES: &[&str] = &[
    "blocked",
    "implementation-complete",
    "proof-complete",
    "owner-accepted",
];

const M4_DELIVERY_SLICES: &[&str] = &[
    "M4A", "M4B", "M4C0", "M4C1", "M4C2", "M4C3", "M4C4", "M4C5", "M4D1", "M4D2", "M4D3", "M5",
];
const M5_DELIVERY_SLICES: &[&str] = &["M5A0", "M5A", "M5B", "M5C", "M5D", "M5E"];
const M6_DELIVERY_SLICES: &[&str] = &["M6A", "M6B", "M6C", "M6D"];
const M7_DELIVERY_SLICES: &[&str] = &["M7A", "M7B", "M7C", "M7D"];
const M8_DELIVERY_SLICES: &[&str] = &["M8A", "M8B", "M8C", "M8D"];
const M9_DELIVERY_SLICES: &[&str] = &["M9A", "M9B", "M9C"];
const M10_DELIVERY_SLICES: &[&str] = &["M10B", "M10C", "M10D", "M10E", "M10F"];
const M11_DELIVERY_SLICES: &[&str] = &["M11A", "M11B"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GatePolicy {
    M4WithInheritedM5,
    Required,
}

impl GatePolicy {
    fn expected(self, delivery_slice: &str) -> &'static str {
        match self {
            Self::M4WithInheritedM5 if delivery_slice == "M5" => "M5 gate",
            Self::M4WithInheritedM5 | Self::Required => "Required",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MatrixSpec {
    path: &'static str,
    allowed_delivery_slices: &'static [&'static str],
    gate_policy: GatePolicy,
}

const M4_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m4-conformance-matrix.md",
    allowed_delivery_slices: M4_DELIVERY_SLICES,
    gate_policy: GatePolicy::M4WithInheritedM5,
};
const M5_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m5-conformance-matrix.md",
    allowed_delivery_slices: M5_DELIVERY_SLICES,
    gate_policy: GatePolicy::Required,
};
const M6_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m6-conformance-matrix.md",
    allowed_delivery_slices: M6_DELIVERY_SLICES,
    gate_policy: GatePolicy::Required,
};
const M7_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m7-conformance-matrix.md",
    allowed_delivery_slices: M7_DELIVERY_SLICES,
    gate_policy: GatePolicy::Required,
};
const M8_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m8-conformance-matrix.md",
    allowed_delivery_slices: M8_DELIVERY_SLICES,
    gate_policy: GatePolicy::Required,
};
const M9_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m9-conformance-matrix.md",
    allowed_delivery_slices: M9_DELIVERY_SLICES,
    gate_policy: GatePolicy::Required,
};
const M10_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m10-conformance-matrix.md",
    allowed_delivery_slices: M10_DELIVERY_SLICES,
    gate_policy: GatePolicy::Required,
};
const M11_SPEC: MatrixSpec = MatrixSpec {
    path: "docs/conformance/m11-conformance-matrix.md",
    allowed_delivery_slices: M11_DELIVERY_SLICES,
    gate_policy: GatePolicy::Required,
};
const MATRIX_SPECS: &[MatrixSpec] = &[
    M4_SPEC, M5_SPEC, M6_SPEC, M7_SPEC, M8_SPEC, M9_SPEC, M10_SPEC, M11_SPEC,
];

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct MatrixMetrics {
    pub(super) total_rows: usize,
    pub(super) owner_accepted: usize,
    pub(super) implementation_complete: usize,
    pub(super) proof_complete: usize,
    pub(super) blocked: usize,
}

impl MatrixMetrics {
    const fn absorb(&mut self, other: &Self) {
        self.total_rows += other.total_rows;
        self.owner_accepted += other.owner_accepted;
        self.implementation_complete += other.implementation_complete;
        self.proof_complete += other.proof_complete;
        self.blocked += other.blocked;
    }
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
    status_counts: BTreeMap<String, usize>,
}

pub(super) fn audit(root: &Path, findings: &mut Vec<Finding>) -> Result<MatrixMetrics, String> {
    audit_inventory(root, findings)?;
    let mut aggregate = MatrixMetrics::default();
    let mut seen_ids = BTreeSet::new();

    for spec in MATRIX_SPECS {
        let analysis = audit_matrix(root, *spec, &mut seen_ids, findings)?;
        aggregate.absorb(&analysis.metrics);
    }

    Ok(aggregate)
}

fn audit_inventory(root: &Path, findings: &mut Vec<Finding>) -> Result<(), String> {
    const INDEX: &str = "docs/conformance/README.md";
    let index = fs::read_to_string(root.join(INDEX))
        .map_err(|error| format!("failed to read {INDEX}: {error}"))?;
    let directory = root.join("docs/conformance");
    let mut on_disk = BTreeSet::new();
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("failed to read docs/conformance: {error}"))?
    {
        let name = entry
            .map_err(|error| format!("failed to read conformance directory entry: {error}"))?
            .file_name()
            .into_string()
            .map_err(|_| "conformance directory contains a non-UTF-8 file name".to_owned())?;
        if name.ends_with("-conformance-matrix.md") {
            on_disk.insert(name);
        }
    }
    validate_inventory(&index, &on_disk, findings);
    Ok(())
}

fn validate_inventory(index: &str, on_disk: &BTreeSet<String>, findings: &mut Vec<Finding>) {
    const INDEX: &str = "docs/conformance/README.md";
    let registered = MATRIX_SPECS
        .iter()
        .filter_map(|spec| spec.path.strip_prefix("docs/conformance/"))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let indexed = index
        .lines()
        .filter_map(|line| {
            let (_, link) = line.trim().strip_prefix("- [")?.split_once("](")?;
            link.strip_suffix(')')
                .filter(|path| path.ends_with("-conformance-matrix.md"))
                .map(str::to_owned)
        })
        .collect::<BTreeSet<_>>();

    for path in indexed.difference(&registered) {
        findings.push(Finding::fatal(
            "matrix.unregistered",
            Some(INDEX.to_owned()),
            format!("indexed conformance matrix `{path}` is missing an audit specification"),
        ));
    }
    for path in registered.difference(&indexed) {
        findings.push(Finding::fatal(
            "matrix.unindexed",
            Some(INDEX.to_owned()),
            format!("configured conformance matrix `{path}` is missing from the index"),
        ));
    }
    for path in on_disk.difference(&registered) {
        findings.push(Finding::fatal(
            "matrix.unregistered",
            Some(format!("docs/conformance/{path}")),
            format!("conformance matrix `{path}` is missing an audit specification"),
        ));
    }
    for path in registered.difference(on_disk) {
        findings.push(Finding::fatal(
            "matrix.missing_file",
            Some(format!("docs/conformance/{path}")),
            format!("configured conformance matrix `{path}` is absent from the directory"),
        ));
    }
}

fn audit_matrix(
    root: &Path,
    spec: MatrixSpec,
    seen_ids: &mut BTreeSet<String>,
    findings: &mut Vec<Finding>,
) -> Result<RowAnalysis, String> {
    let path = root.join(spec.path);
    let contents = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", spec.path))?;
    let summary = parse_summary(&contents);
    let analysis = analyze_contents(spec, &contents, seen_ids, findings);

    validate_status_total(spec.path, &analysis, findings);
    compare_declared_summary(spec.path, &summary, &analysis, findings);

    Ok(analysis)
}

fn analyze_contents(
    spec: MatrixSpec,
    contents: &str,
    seen_ids: &mut BTreeSet<String>,
    findings: &mut Vec<Finding>,
) -> RowAnalysis {
    let (rows, parse_schema_errors) = parse_rows(contents, spec.path, findings);
    analyze_rows(spec, &rows, parse_schema_errors, seen_ids, findings)
}

fn analyze_rows(
    spec: MatrixSpec,
    rows: &[MatrixRow],
    parse_schema_errors: usize,
    seen_ids: &mut BTreeSet<String>,
    findings: &mut Vec<Finding>,
) -> RowAnalysis {
    let mut state = RowState::default();
    state.analysis.invalid_schemas = parse_schema_errors;

    for row in rows {
        audit_row(spec, row, &mut state, seen_ids, findings);
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

fn audit_row(
    spec: MatrixSpec,
    row: &MatrixRow,
    state: &mut RowState,
    seen_ids: &mut BTreeSet<String>,
    findings: &mut Vec<Finding>,
) {
    let id = &row.cells[0];
    if !valid_id(id) {
        state.analysis.invalid_schemas += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_schema",
            Some(format!("{}:{}", spec.path, row.line)),
            format!("matrix ID `{id}` does not match the permanent FAMILY-NN contract"),
        ));
    }
    if !seen_ids.insert(id.clone()) {
        state.analysis.duplicate_ids += 1;
        findings.push(Finding::fatal(
            "matrix.duplicate_id",
            Some(format!("{}:{}", spec.path, row.line)),
            format!("matrix ID `{id}` is duplicated across configured conformance matrices"),
        ));
    }

    if row.cells.iter().any(String::is_empty) {
        state.analysis.invalid_schemas += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_schema",
            Some(format!("{}:{}", spec.path, row.line)),
            format!("matrix row `{id}` contains an empty required column"),
        ));
    }

    let delivery_slice = &row.cells[5];
    if !spec
        .allowed_delivery_slices
        .contains(&delivery_slice.as_str())
    {
        state.analysis.invalid_schemas += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_schema",
            Some(format!("{}:{}", spec.path, row.line)),
            format!(
                "matrix row `{id}` has invalid delivery slice `{delivery_slice}` for {}",
                spec.path
            ),
        ));
    }

    let status = &row.cells[6];
    if ALLOWED_STATUSES.contains(&status.as_str()) {
        *state.status_counts.entry(status.clone()).or_default() += 1;
    } else {
        state.analysis.invalid_statuses += 1;
        findings.push(Finding::fatal(
            "matrix.invalid_status",
            Some(format!("{}:{}", spec.path, row.line)),
            format!("matrix row `{id}` has invalid status `{status}`"),
        ));
    }

    validate_gate(spec, row, delivery_slice, findings, &mut state.analysis);
}

fn validate_gate(
    spec: MatrixSpec,
    row: &MatrixRow,
    delivery_slice: &str,
    findings: &mut Vec<Finding>,
    analysis: &mut RowAnalysis,
) {
    let gate = &row.cells[7];
    let expected_gate = spec.gate_policy.expected(delivery_slice);
    if gate == expected_gate {
        return;
    }

    analysis.invalid_schemas += 1;
    findings.push(Finding::fatal(
        "matrix.invalid_schema",
        Some(format!("{}:{}", spec.path, row.line)),
        format!(
            "matrix row `{}` uses gate `{gate}`; delivery slice `{delivery_slice}` in {} requires `{expected_gate}`",
            row.cells[0], spec.path
        ),
    ));
}

fn validate_status_total(path: &str, analysis: &RowAnalysis, findings: &mut Vec<Finding>) {
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
        Some(path.to_owned()),
        "matrix row total does not equal the sum of valid and invalid statuses",
    ));
}

fn compare_declared_summary(
    path: &str,
    summary: &MatrixSummary,
    analysis: &RowAnalysis,
    findings: &mut Vec<Finding>,
) {
    let metrics = &analysis.metrics;
    compare_summary(
        path,
        findings,
        "total unique rows",
        summary.total_rows,
        metrics.total_rows,
    );
    compare_summary(
        path,
        findings,
        "owner-accepted",
        summary.owner_accepted,
        metrics.owner_accepted,
    );
    compare_optional_zero_summary(
        path,
        findings,
        "implementation-complete",
        summary.implementation_complete,
        metrics.implementation_complete,
    );
    compare_summary(
        path,
        findings,
        "proof-complete",
        summary.proof_complete,
        metrics.proof_complete,
    );
    compare_summary(path, findings, "blocked", summary.blocked, metrics.blocked);
    compare_summary(
        path,
        findings,
        "duplicate IDs",
        summary.duplicate_ids,
        analysis.duplicate_ids,
    );
    compare_summary(
        path,
        findings,
        "invalid statuses",
        summary.invalid_statuses,
        analysis.invalid_statuses,
    );
    compare_summary(
        path,
        findings,
        "invalid schemas",
        summary.invalid_schemas,
        analysis.invalid_schemas,
    );
}

fn parse_rows(contents: &str, path: &str, findings: &mut Vec<Finding>) -> (Vec<MatrixRow>, usize) {
    let mut rows = Vec::new();
    let mut invalid_schemas = 0;

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
                invalid_schemas += 1;
                findings.push(Finding::fatal(
                    "matrix.invalid_schema",
                    Some(format!("{path}:{line_number}")),
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

    (rows, invalid_schemas)
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
    path: &str,
    findings: &mut Vec<Finding>,
    label: &str,
    declared: Option<usize>,
    actual: usize,
) {
    match declared {
        Some(declared) if declared == actual => {}
        Some(declared) => findings.push(Finding::fatal(
            "matrix.inconsistent_summary",
            Some(path.to_owned()),
            format!("declared `{label}` count is {declared}, but the matrix contains {actual}"),
        )),
        None => findings.push(Finding::fatal(
            "matrix.missing_summary_metric",
            Some(path.to_owned()),
            format!("matrix summary is missing the `{label}` count"),
        )),
    }
}

fn compare_optional_zero_summary(
    path: &str,
    findings: &mut Vec<Finding>,
    label: &str,
    declared: Option<usize>,
    actual: usize,
) {
    if declared.is_none() && actual == 0 {
        return;
    }
    compare_summary(path, findings, label, declared, actual);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        M4_SPEC, M5_SPEC, M6_SPEC, M7_SPEC, M8_SPEC, M9_SPEC, M10_SPEC, M11_SPEC, MATRIX_SPECS,
        analyze_contents, audit_inventory, compare_declared_summary, declared_metric, parse_rows,
        parse_summary, valid_id, validate_inventory,
    };

    #[test]
    fn permanent_id_parser_accepts_multi_segment_families() {
        assert!(valid_id("PTR-01"));
        assert!(valid_id("TRACE-EVENT-10"));
        assert!(valid_id("M4-CLOSE-05"));
        assert!(valid_id("SEM-ID-01"));
        assert!(valid_id("M5-CLOSE-03"));
        assert!(valid_id("SCENE-PUB-01"));
        assert!(!valid_id("PTR-1"));
        assert!(!valid_id("ptr-01"));
        assert!(!valid_id("PTR--01"));
    }

    #[test]
    fn matrix_row_parser_rejects_wrong_column_count() {
        let mut findings = Vec::new();
        let (rows, invalid_schemas) = parse_rows(
            "| ID | A | B | C | D | E | F | G |\n|---|---|---|---|---|---|---|---|\n| PTR-01 | A | B | C | D | E | F |\n",
            M4_SPEC.path,
            &mut findings,
        );
        assert!(rows.is_empty());
        assert_eq!(invalid_schemas, 1);
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

    #[test]
    fn m4_gate_policy_preserves_inherited_m5_rows() {
        let contents = "| ID | A | B | C | D | E | F | G |\n\
|---|---|---|---|---|---|---|---|\n\
| PTR-01 | A | B | C | D | M4C3 | blocked | Required |\n\
| ACCESS-01 | A | B | C | D | M5 | blocked | M5 gate |\n";
        let mut findings = Vec::new();
        let mut seen = BTreeSet::new();
        let analysis = analyze_contents(M4_SPEC, contents, &mut seen, &mut findings);
        assert_eq!(analysis.invalid_schemas, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn m5_gate_policy_requires_required_for_m5_slices() {
        let valid = "| ID | A | B | C | D | E | F | G |\n\
|---|---|---|---|---|---|---|---|\n\
| SEM-ID-01 | A | B | C | D | M5A | blocked | Required |\n";
        let mut findings = Vec::new();
        let mut seen = BTreeSet::new();
        let analysis = analyze_contents(M5_SPEC, valid, &mut seen, &mut findings);
        assert_eq!(analysis.invalid_schemas, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn m6_gate_policy_requires_required_for_m6_slices() {
        let valid = "| ID | A | B | C | D | E | F | G |\n\
|---|---|---|---|---|---|---|---|\n\
| SCENE-PUB-01 | A | B | C | D | M6A | blocked | Required |\n";
        let mut findings = Vec::new();
        let mut seen = BTreeSet::new();
        let analysis = analyze_contents(M6_SPEC, valid, &mut seen, &mut findings);
        assert_eq!(analysis.invalid_schemas, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn m7_gate_policy_requires_required_for_m7_slices() {
        let valid = "| ID | A | B | C | D | E | F | G |\n\
|---|---|---|---|---|---|---|---|\n\
| RENDER-01 | A | B | C | D | M7A | blocked | Required |\n";
        let mut findings = Vec::new();
        let mut seen = BTreeSet::new();
        let analysis = analyze_contents(M7_SPEC, valid, &mut seen, &mut findings);
        assert_eq!(analysis.invalid_schemas, 0);
        assert!(findings.is_empty());
    }

    #[test]
    fn duplicate_ids_are_rejected_across_configured_matrices() {
        let m4 = "| ID | A | B | C | D | E | F | G |\n\
|---|---|---|---|---|---|---|---|\n\
| SHARED-01 | A | B | C | D | M4C3 | blocked | Required |\n";
        let m5 = "| ID | A | B | C | D | E | F | G |\n\
|---|---|---|---|---|---|---|---|\n\
| SHARED-01 | A | B | C | D | M5A | blocked | Required |\n";
        let mut findings = Vec::new();
        let mut seen = BTreeSet::new();
        let first = analyze_contents(M4_SPEC, m4, &mut seen, &mut findings);
        let second = analyze_contents(M5_SPEC, m5, &mut seen, &mut findings);
        assert_eq!(first.duplicate_ids, 0);
        assert_eq!(second.duplicate_ids, 1);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn m8_m9_and_m10_contracts_reject_invalid_slices_gates_statuses_and_schemas() {
        for (spec, slice) in [(M8_SPEC, "M8A"), (M9_SPEC, "M9A"), (M10_SPEC, "M10B")] {
            let valid = format!(
                "| ID | A | B | C | D | E | F | G |\n|---|---|---|---|---|---|---|---|\n| TEST-01 | A | B | C | D | {slice} | owner-accepted | Required |\n"
            );
            let mut findings = Vec::new();
            let mut seen = BTreeSet::new();
            let analysis = analyze_contents(spec, &valid, &mut seen, &mut findings);
            assert_eq!(analysis.metrics.total_rows, 1);
            assert!(findings.is_empty());

            for (mutant, expected_code) in [
                (
                    valid.replace(&format!("| {slice} |"), "| M7D |"),
                    "matrix.invalid_schema",
                ),
                (
                    valid.replace("| Required |", "| M5 gate |"),
                    "matrix.invalid_schema",
                ),
                (
                    valid.replace("| owner-accepted |", "| unknown |"),
                    "matrix.invalid_status",
                ),
                (
                    valid.replace("| owner-accepted | Required |", "| owner-accepted |"),
                    "matrix.invalid_schema",
                ),
            ] {
                let mut findings = Vec::new();
                let mut seen = BTreeSet::new();
                analyze_contents(spec, &mutant, &mut seen, &mut findings);
                assert!(findings.iter().any(|finding| finding.code == expected_code));
            }
            let mut findings = Vec::new();
            compare_declared_summary(
                spec.path,
                &parse_summary(
                    "2 total unique rows\n0 owner-accepted\n0 proof-complete\n0 blocked\n0 duplicate IDs\n0 invalid statuses\n0 invalid schemas\n",
                ),
                &analysis,
                &mut findings,
            );
            assert!(
                findings
                    .iter()
                    .any(|finding| finding.code == "matrix.inconsistent_summary")
            );
        }
    }

    #[test]
    fn m8_m9_m10_and_m11_ids_cannot_duplicate_earlier_or_each_other() {
        let mut seen = BTreeSet::new();
        let mut findings = Vec::new();
        for (spec, slice) in [
            (M7_SPEC, "M7A"),
            (M8_SPEC, "M8A"),
            (M9_SPEC, "M9A"),
            (M10_SPEC, "M10B"),
            (M11_SPEC, "M11A"),
        ] {
            let row = format!("| SAME-01 | A | B | C | D | {slice} | blocked | Required |\n");
            analyze_contents(spec, &row, &mut seen, &mut findings);
        }
        assert_eq!(
            findings
                .iter()
                .filter(|finding| finding.code == "matrix.duplicate_id")
                .count(),
            4
        );
    }

    #[test]
    fn m10_inventory_accepts_integrated_m10f_closure() -> Result<(), String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| "xtask has no repository root".to_owned())?;
        let contents = std::fs::read_to_string(root.join(M10_SPEC.path))
            .map_err(|error| format!("failed to read {}: {error}", M10_SPEC.path))?;
        let mut findings = Vec::new();
        let (rows, parse_schema_errors) = parse_rows(&contents, M10_SPEC.path, &mut findings);
        assert_eq!(parse_schema_errors, 0);
        assert_eq!(rows.len(), 28);
        let (accepted, integrated): (Vec<_>, Vec<_>) = rows
            .iter()
            .partition(|row| matches!(row.cells[5].as_str(), "M10B" | "M10C" | "M10D" | "M10E"));
        assert_eq!(accepted.len(), 23);
        let text_mapping = accepted
            .iter()
            .filter(|row| row.cells[5] == "M10B")
            .collect::<Vec<_>>();
        let transactional_editing = accepted
            .iter()
            .filter(|row| row.cells[5] == "M10C")
            .collect::<Vec<_>>();
        let framework_services = accepted
            .iter()
            .filter(|row| row.cells[5] == "M10D")
            .collect::<Vec<_>>();
        let interaction = accepted
            .iter()
            .filter(|row| row.cells[5] == "M10E")
            .collect::<Vec<_>>();
        assert_eq!(text_mapping.len(), 4);
        assert!(
            text_mapping
                .iter()
                .all(|row| row.cells[0].starts_with("M10TEXT-")
                    && row.cells[6] == "owner-accepted")
        );
        assert_eq!(transactional_editing.len(), 9);
        assert!(transactional_editing.iter().all(|row| {
            row.cells[0].starts_with("M10EDIT-") && row.cells[6] == "owner-accepted"
        }));
        assert_eq!(framework_services.len(), 5);
        assert!(framework_services.iter().all(|row| {
            row.cells[0].starts_with("M10SVC-") && row.cells[6] == "owner-accepted"
        }));
        assert_eq!(interaction.len(), 5);
        assert!(interaction.iter().all(|row| {
            matches!(
                row.cells[0].as_str(),
                "M10SCROLL-01" | "M10SCROLL-02" | "M10POINT-01" | "M10TOUCH-01" | "M10CTRL-01"
            ) && row.cells[6] == "owner-accepted"
        }));
        assert_eq!(integrated.len(), 5);
        assert!(
            integrated
                .iter()
                .all(|row| { row.cells[5] == "M10F" && row.cells[6] == "owner-accepted" })
        );
        assert!(rows.iter().all(|row| row.cells[7] == "Required"));
        assert_eq!(
            rows.iter()
                .map(|row| row.cells[5].as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["M10B", "M10C", "M10D", "M10E", "M10F"])
        );
        assert!(findings.is_empty());
        Ok(())
    }

    #[test]
    fn m11_inventory_accepts_m11a_and_m11b_control_contracts() -> Result<(), String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| "xtask has no repository root".to_owned())?;
        let contents = std::fs::read_to_string(root.join(M11_SPEC.path))
            .map_err(|error| format!("failed to read {}: {error}", M11_SPEC.path))?;
        let mut findings = Vec::new();
        let (rows, parse_schema_errors) = parse_rows(&contents, M11_SPEC.path, &mut findings);
        assert_eq!(parse_schema_errors, 0);
        assert_eq!(rows.len(), 8);
        let (m11a, m11b): (Vec<_>, Vec<_>) =
            rows.iter().partition(|row| row.cells[5] == "M11A");
        assert_eq!(m11a.len(), 5);
        assert!(m11a.iter().all(|row| {
            row.cells[0].starts_with("M11CTRL-")
                && row.cells[6] == "owner-accepted"
                && row.cells[7] == "Required"
        }));
        assert_eq!(m11b.len(), 3);
        assert!(m11b.iter().all(|row| {
            row.cells[0].starts_with("M11CTRL-")
                && row.cells[5] == "M11B"
                && row.cells[6] == "implementation-complete"
                && row.cells[7] == "Required"
        }));
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }

    #[test]
    fn index_and_directory_cannot_silently_omit_a_matrix() {
        let indexed = MATRIX_SPECS
            .iter()
            .map(|spec| {
                let name = spec
                    .path
                    .strip_prefix("docs/conformance/")
                    .unwrap_or_else(|| {
                        unreachable!("all matrix specifications are under docs/conformance")
                    });
                format!("- [M conformance matrix]({name})")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let files = MATRIX_SPECS
            .iter()
            .map(|spec| {
                spec.path
                    .strip_prefix("docs/conformance/")
                    .unwrap_or_else(|| {
                        unreachable!("all matrix specifications are under docs/conformance")
                    })
                    .to_owned()
            })
            .collect::<BTreeSet<_>>();
        let mut findings = Vec::new();
        validate_inventory(&indexed, &files, &mut findings);
        assert!(findings.is_empty());

        validate_inventory(
            &format!("{indexed}\n- [M12 conformance matrix](m12-conformance-matrix.md)"),
            &files,
            &mut findings,
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "matrix.unregistered")
        );
        findings.clear();
        let mut new_file = files.clone();
        new_file.insert("m12-conformance-matrix.md".to_owned());
        validate_inventory(&indexed, &new_file, &mut findings);
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "matrix.unregistered")
        );
        findings.clear();
        validate_inventory("", &files, &mut findings);
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "matrix.unindexed")
        );
        findings.clear();
        validate_inventory(
            &format!("{indexed}\n- [M12 matrix](m12-conformance-matrix.md)"),
            &files,
            &mut findings,
        );
        assert!(
            findings
                .iter()
                .any(|finding| finding.code == "matrix.unregistered")
        );
    }

    #[test]
    fn accepted_repository_matrices_are_registered_and_parse_cleanly() -> Result<(), String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| "xtask has no repository root".to_owned())?;
        let mut findings = Vec::new();
        audit_inventory(root, &mut findings)?;
        let mut seen = BTreeSet::new();
        let mut total = 0;
        for spec in MATRIX_SPECS {
            let contents = std::fs::read_to_string(root.join(spec.path))
                .map_err(|error| format!("failed to read {}: {error}", spec.path))?;
            let summary = parse_summary(&contents);
            let analysis = analyze_contents(*spec, &contents, &mut seen, &mut findings);
            compare_declared_summary(spec.path, &summary, &analysis, &mut findings);
            total += analysis.metrics.total_rows;
        }
        assert_eq!(total, 438);
        assert!(findings.is_empty(), "{findings:?}");
        Ok(())
    }
}
