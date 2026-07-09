use super::{
    CompositionPersistenceDiagnosticCode as Code, CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection,
    LoadedCompositionBundle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompositionLayoutScope {
    BuiltIn,
    Project,
    User,
}

#[derive(Clone, Debug)]
pub enum CompositionScopeLoad {
    Missing,
    Valid(LoadedCompositionBundle),
    Invalid(CompositionPersistenceRejection),
}

#[derive(Clone, Debug)]
pub struct CompositionScopedSelection {
    pub scope: CompositionLayoutScope,
    pub bundle: LoadedCompositionBundle,
}

#[derive(Clone, Debug, Default)]
pub struct CompositionLayoutCatalog {
    built_in: Option<CompositionScopeLoad>,
    project: Option<CompositionScopeLoad>,
    user: Option<CompositionScopeLoad>,
}

impl CompositionLayoutCatalog {
    pub fn with_scope(mut self, scope: CompositionLayoutScope, load: CompositionScopeLoad) -> Self {
        match scope {
            CompositionLayoutScope::BuiltIn => self.built_in = Some(load),
            CompositionLayoutScope::Project => self.project = Some(load),
            CompositionLayoutScope::User => self.user = Some(load),
        }
        self
    }

    pub fn select(
        self,
    ) -> Result<Option<CompositionScopedSelection>, CompositionPersistenceRejection> {
        for (scope, load) in [
            (CompositionLayoutScope::User, self.user),
            (CompositionLayoutScope::Project, self.project),
            (CompositionLayoutScope::BuiltIn, self.built_in),
        ] {
            match load.unwrap_or(CompositionScopeLoad::Missing) {
                CompositionScopeLoad::Missing => continue,
                CompositionScopeLoad::Valid(bundle) => {
                    return Ok(Some(CompositionScopedSelection { scope, bundle }));
                }
                CompositionScopeLoad::Invalid(rejection) => {
                    let mut diagnostics = rejection.diagnostics().to_vec();
                    diagnostics.push(super::CompositionPersistenceDiagnosticRecord::error(
                        Code::ScopeSelectionFailed,
                        Stage::Scope,
                        Subject::General(format!("{scope:?}")),
                        "Repair or explicitly remove the invalid higher-precedence layout before selecting a lower scope.",
                    ));
                    return Err(CompositionPersistenceRejection::new(diagnostics));
                }
            }
        }
        Ok(None)
    }
}
