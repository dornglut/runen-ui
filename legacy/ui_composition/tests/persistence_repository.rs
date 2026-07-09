use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ui_composition::*;

fn definition(revision: u64) -> CompositionDefinitionV1 {
    CompositionDefinitionV1::new(
        CompositionDefinitionId::new(1),
        DefinitionRevision::new(revision),
        vec![PresentationTargetDefinition::new(
            PresentationTargetId::new(1),
            TargetProfileId::new("fixture.desktop").unwrap(),
        )],
        vec![CompositionRootDefinition::new(
            CompositionRootId::new(1),
            PresentationTargetId::new(1),
            RegionId::new(1),
            true,
        )],
        vec![RegionDefinition::new(
            RegionId::new(1),
            None,
            RegionKind::MountPoint {
                mounted_unit: MountedUnitId::new(1),
            },
        )],
        vec![MountedUnitDefinition::new(
            MountedUnitId::new(1),
            MountedContentRef::new(
                ContentOwnerId::new("fixture.owner").unwrap(),
                ContentProfileId::new("fixture.content").unwrap(),
                ContentInstanceRef::new("fixture.main").unwrap(),
            ),
            [],
            UnavailableContentPolicy::ShowFallback,
        )],
    )
}

fn candidate(revision: u64) -> CompositionBundleCandidate {
    CompositionBundleCandidate::form(
        definition(revision),
        CompositionCompatibility::new(
            AppProfileId::new("fixture.editor").unwrap(),
            AppSchemaVersion::new(1).unwrap(),
            AppSchemaVersion::new(2).unwrap(),
        )
        .unwrap(),
        vec![
            CanonicalExtensionPayload::new(
                ExtensionProfileId::new("fixture.editor.session").unwrap(),
                ExtensionSchemaVersion::new(1).unwrap(),
                format!("(revision:{revision})\n"),
            )
            .unwrap(),
            CanonicalExtensionPayload::new(
                ExtensionProfileId::new("fixture.editor.panels").unwrap(),
                ExtensionSchemaVersion::new(1).unwrap(),
                format!("(revision:{revision},visible:true)\n"),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn pointer_path(root: &Path) -> PathBuf {
    root.join("active-generation.ron")
}

fn core_path(root: &Path, generation: &CompositionGenerationId) -> PathBuf {
    root.join("generations")
        .join(generation.directory_name())
        .join("core.ron")
}

#[test]
fn activation_is_compare_and_swap_and_tracks_one_explicit_previous_generation() {
    let directory = tempfile::tempdir().unwrap();
    let repository = CompositionBundleRepository::new(directory.path());
    let first = candidate(1);
    let second = candidate(2);

    let first_activation = repository.activate(&first, None).unwrap();
    assert_eq!(first_activation.active_generation, *first.generation_id());
    assert_eq!(first_activation.previous_generation, None);

    let pointer_before_stale = fs::read(pointer_path(directory.path())).unwrap();
    let stale = repository.activate(&second, None).unwrap_err();
    assert!(stale.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::StaleGeneration
    }));
    assert_eq!(
        fs::read(pointer_path(directory.path())).unwrap(),
        pointer_before_stale
    );

    let second_activation = repository
        .activate(&second, Some(first.generation_id()))
        .unwrap();
    assert_eq!(second_activation.active_generation, *second.generation_id());
    assert_eq!(
        second_activation.previous_generation.as_ref(),
        Some(first.generation_id())
    );
    let loaded = repository.load_active(None).unwrap();
    assert_eq!(loaded.status, CompositionGenerationLoadStatus::Active);
    assert_eq!(loaded.bundle.generation_id(), second.generation_id());
}

#[test]
fn corrupt_active_generation_recovers_previous_without_rewriting_pointer() {
    let directory = tempfile::tempdir().unwrap();
    let repository = CompositionBundleRepository::new(directory.path());
    let first = candidate(1);
    let second = candidate(2);
    repository.activate(&first, None).unwrap();
    repository
        .activate(&second, Some(first.generation_id()))
        .unwrap();
    let pointer_before = fs::read(pointer_path(directory.path())).unwrap();

    fs::write(
        core_path(directory.path(), second.generation_id()),
        "not canonical RON\n",
    )
    .unwrap();
    let recovered = repository.load_active(None).unwrap();
    assert_eq!(
        recovered.status,
        CompositionGenerationLoadStatus::RecoveredLastGood
    );
    assert_eq!(recovered.bundle.generation_id(), first.generation_id());
    assert!(recovered.diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::LastGoodRecovered
    }));
    assert_eq!(
        fs::read(pointer_path(directory.path())).unwrap(),
        pointer_before
    );

    fs::write(
        core_path(directory.path(), first.generation_id()),
        "also invalid\n",
    )
    .unwrap();
    let rejection = repository.load_active(None).unwrap_err();
    assert!(rejection.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::NoValidGeneration
    }));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    CreateDir,
    CreateStaging,
    Write,
    FileSync,
    Read,
    List,
    Rename,
    PointerCreate,
    PointerWrite,
    PointerSync,
    PointerPersist,
    Sync,
}

#[derive(Clone, Copy, Debug)]
struct FailurePoint {
    operation: Operation,
    occurrence: usize,
}

#[derive(Clone, Debug)]
struct FailingOperations {
    native: NativeCompositionFileOperations,
    failure: FailurePoint,
    counts: Arc<Mutex<[usize; 12]>>,
}

impl FailingOperations {
    fn new(failure: FailurePoint) -> Self {
        Self {
            native: NativeCompositionFileOperations,
            failure,
            counts: Arc::new(Mutex::new([0; 12])),
        }
    }

    fn fail(&self, operation: Operation) -> io::Result<()> {
        let index = operation as usize;
        let mut counts = self.counts.lock().unwrap();
        counts[index] += 1;
        if operation == self.failure.operation && counts[index] == self.failure.occurrence {
            return Err(io::Error::other(format!(
                "injected {operation:?} failure at occurrence {}",
                self.failure.occurrence
            )));
        }
        Ok(())
    }
}

impl CompositionFileOperations for FailingOperations {
    type ReplacementFile = NativeCompositionReplacementFile;

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.fail(Operation::CreateDir)?;
        self.native.create_dir_all(path)
    }

    fn create_staging_dir(&self, parent: &Path) -> io::Result<PathBuf> {
        self.fail(Operation::CreateStaging)?;
        self.native.create_staging_dir(parent)
    }

    fn write_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.fail(Operation::Write)?;
        self.native.write_file(path, bytes)
    }

    fn sync_file(&self, path: &Path) -> io::Result<()> {
        self.fail(Operation::FileSync)?;
        self.native.sync_file(path)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.fail(Operation::Read)?;
        self.native.read_to_string(path)
    }

    fn list_file_names(&self, path: &Path) -> io::Result<Vec<OsString>> {
        self.fail(Operation::List)?;
        self.native.list_file_names(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.fail(Operation::Rename)?;
        self.native.rename(from, to)
    }

    fn create_replacement_file(&self, parent: &Path) -> io::Result<Self::ReplacementFile> {
        self.fail(Operation::PointerCreate)?;
        self.native.create_replacement_file(parent)
    }

    fn write_replacement_file(
        &self,
        file: &mut Self::ReplacementFile,
        bytes: &[u8],
    ) -> io::Result<()> {
        self.fail(Operation::PointerWrite)?;
        self.native.write_replacement_file(file, bytes)
    }

    fn sync_replacement_file(&self, file: &Self::ReplacementFile) -> io::Result<()> {
        self.fail(Operation::PointerSync)?;
        self.native.sync_replacement_file(file)
    }

    fn persist_replacement_file(&self, file: Self::ReplacementFile, path: &Path) -> io::Result<()> {
        self.fail(Operation::PointerPersist)?;
        self.native.persist_replacement_file(file, path)
    }

    fn sync_dir(&self, path: &Path) -> io::Result<()> {
        self.fail(Operation::Sync)?;
        self.native.sync_dir(path)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        self.native.remove_dir_all(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.native.exists(path)
    }
}

#[test]
fn every_pre_pointer_failure_preserves_the_previous_active_generation() {
    let failure_points = [
        FailurePoint {
            operation: Operation::CreateDir,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::CreateDir,
            occurrence: 3,
        },
        FailurePoint {
            operation: Operation::CreateStaging,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::Write,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::Write,
            occurrence: 2,
        },
        FailurePoint {
            operation: Operation::Write,
            occurrence: 3,
        },
        FailurePoint {
            operation: Operation::FileSync,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::FileSync,
            occurrence: 2,
        },
        FailurePoint {
            operation: Operation::FileSync,
            occurrence: 3,
        },
        FailurePoint {
            operation: Operation::Read,
            occurrence: 2,
        },
        FailurePoint {
            operation: Operation::Read,
            occurrence: 3,
        },
        FailurePoint {
            operation: Operation::Read,
            occurrence: 4,
        },
        FailurePoint {
            operation: Operation::List,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::Sync,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::Sync,
            occurrence: 2,
        },
        FailurePoint {
            operation: Operation::Rename,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::Sync,
            occurrence: 3,
        },
        FailurePoint {
            operation: Operation::PointerCreate,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::PointerWrite,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::PointerSync,
            occurrence: 1,
        },
        FailurePoint {
            operation: Operation::PointerPersist,
            occurrence: 1,
        },
    ];

    for failure in failure_points {
        let directory = tempfile::tempdir().unwrap();
        let native = CompositionBundleRepository::new(directory.path());
        let first = candidate(1);
        let second = candidate(2);
        native.activate(&first, None).unwrap();
        let pointer_before = fs::read(pointer_path(directory.path())).unwrap();

        let failing = CompositionBundleRepository::with_operations(
            directory.path(),
            FailingOperations::new(failure),
        );
        assert!(
            failing
                .activate(&second, Some(first.generation_id()))
                .is_err(),
            "failure point did not trigger: {failure:?}"
        );
        assert_eq!(
            fs::read(pointer_path(directory.path())).unwrap(),
            pointer_before,
            "pointer changed after {failure:?}"
        );
        let loaded = native.load_active(None).unwrap();
        assert_eq!(loaded.bundle.generation_id(), first.generation_id());
    }
}

#[test]
fn post_commit_directory_sync_failure_returns_committed_activation_with_warning() {
    let directory = tempfile::tempdir().unwrap();
    let native = CompositionBundleRepository::new(directory.path());
    let first = candidate(1);
    let second = candidate(2);
    native.activate(&first, None).unwrap();

    let repository = CompositionBundleRepository::with_operations(
        directory.path(),
        FailingOperations::new(FailurePoint {
            operation: Operation::Sync,
            occurrence: 4,
        }),
    );
    let activation = repository
        .activate(&second, Some(first.generation_id()))
        .unwrap();
    assert_eq!(activation.active_generation, *second.generation_id());
    assert!(activation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::SyncFailed
    }));
    assert_eq!(
        native.load_active(None).unwrap().bundle.generation_id(),
        second.generation_id()
    );
}

#[test]
fn invalid_higher_precedence_scope_fails_closed() {
    let candidate = candidate(1);
    let loaded = LoadedCompositionBundle {
        bundle: candidate.validate(None).unwrap(),
        status: CompositionGenerationLoadStatus::Active,
        diagnostics: Vec::new(),
    };
    let selected = CompositionLayoutCatalog::default()
        .with_scope(
            CompositionLayoutScope::BuiltIn,
            CompositionScopeLoad::Valid(loaded.clone()),
        )
        .with_scope(
            CompositionLayoutScope::Project,
            CompositionScopeLoad::Missing,
        )
        .select()
        .unwrap()
        .unwrap();
    assert_eq!(selected.scope, CompositionLayoutScope::BuiltIn);

    let invalid =
        CompositionPersistenceRejection::single(CompositionPersistenceDiagnosticRecord::error(
            CompositionPersistenceDiagnosticCode::DigestMismatch,
            CompositionPersistenceDiagnosticStage::Scope,
            CompositionPersistenceDiagnosticSubject::General("user_layout".to_owned()),
            "Repair the user layout.",
        ));
    let rejection = CompositionLayoutCatalog::default()
        .with_scope(
            CompositionLayoutScope::BuiltIn,
            CompositionScopeLoad::Valid(loaded),
        )
        .with_scope(
            CompositionLayoutScope::User,
            CompositionScopeLoad::Invalid(invalid),
        )
        .select()
        .unwrap_err();
    assert!(rejection.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::ScopeSelectionFailed
    }));
}
