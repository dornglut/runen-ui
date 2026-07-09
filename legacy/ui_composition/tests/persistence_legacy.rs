use std::fs;

use ui_composition::*;

#[test]
fn legacy_versions_one_through_five_are_rejected_read_only() {
    for version in 1..=5 {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(format!("legacy-v{version}.ron"));
        let original = format!("(version:{version},payload:\"preserve me\")\n");
        fs::write(&path, original.as_bytes()).unwrap();
        let metadata_before = fs::metadata(&path).unwrap();
        let source = fs::read_to_string(&path).unwrap();

        let rejection = probe_composition_source(&source).unwrap_err();
        assert!(rejection.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == CompositionPersistenceDiagnosticCode::UnsupportedLegacySchema
        }));

        let metadata_after = fs::metadata(&path).unwrap();
        assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
        assert_eq!(metadata_after.len(), metadata_before.len());
        assert_eq!(
            metadata_after.permissions().readonly(),
            metadata_before.permissions().readonly()
        );
        assert_eq!(
            metadata_after.modified().unwrap(),
            metadata_before.modified().unwrap()
        );
    }
}

#[test]
fn unknown_and_malformed_sources_have_distinct_stable_diagnostics() {
    let unknown = probe_composition_source("(version:6)\n").unwrap_err();
    assert!(unknown.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::UnsupportedSchema
    }));

    let malformed = probe_composition_source("not ron\n").unwrap_err();
    assert!(malformed.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::CanonicalDecodeFailed
    }));
}

#[test]
fn current_core_envelope_is_recognized_without_decoding_legacy_shapes() {
    assert_eq!(
        probe_composition_source("(envelope_schema_version:1)\n").unwrap(),
        CompositionSourceSchema::CoreEnvelopeV1
    );
}
