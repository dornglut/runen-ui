use crate::{CompositionDefinitionV1, CompositionDiagnosticRecord};

pub fn validate_transaction_candidate(
    candidate: &CompositionDefinitionV1,
) -> Vec<CompositionDiagnosticRecord> {
    super::definition::validate_definition(candidate)
}
