use serde::Deserialize;

use super::{
    CompositionPersistenceDiagnosticCode as Code, CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionSourceSchema {
    CoreEnvelopeV1,
}

#[derive(Deserialize)]
struct EnvelopeProbe {
    envelope_schema_version: u16,
}

#[derive(Deserialize)]
struct LegacyProbe {
    version: u32,
}

pub fn probe_composition_source(
    source: &str,
) -> Result<CompositionSourceSchema, CompositionPersistenceRejection> {
    if let Ok(probe) = ron::from_str::<EnvelopeProbe>(source) {
        if probe.envelope_schema_version == 1 {
            return Ok(CompositionSourceSchema::CoreEnvelopeV1);
        }
        return Err(super::diagnostic::rejection(
            Code::UnsupportedSchema,
            Stage::Legacy,
            Subject::General(probe.envelope_schema_version.to_string()),
            "Select a composition envelope using schema version 1.",
        ));
    }
    if let Ok(probe) = ron::from_str::<LegacyProbe>(source) {
        if (1..=5).contains(&probe.version) {
            return Err(super::diagnostic::rejection(
                Code::UnsupportedLegacySchema,
                Stage::Legacy,
                Subject::General(probe.version.to_string()),
                "Legacy layout versions 1 through 5 are unsupported; reset or select a valid composition layout without changing the legacy file.",
            ));
        }
        return Err(super::diagnostic::rejection(
            Code::UnsupportedSchema,
            Stage::Legacy,
            Subject::General(probe.version.to_string()),
            "Select a supported composition envelope instead of this unknown source version.",
        ));
    }
    Err(super::diagnostic::rejection(
        Code::CanonicalDecodeFailed,
        Stage::Legacy,
        Subject::General("composition_source".to_owned()),
        "Provide a valid canonical composition envelope.",
    ))
}
