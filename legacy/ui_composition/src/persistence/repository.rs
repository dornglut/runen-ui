use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::{
    CanonicalCompositionDocuments, CompositionBundleCandidate, CompositionCompatibilityRequirement,
    CompositionExtensionIdentity, CompositionGenerationId, CompositionGenerationLoadStatus,
    CompositionGenerationPointerV1, CompositionPersistenceDiagnosticCode as Code,
    CompositionPersistenceDiagnosticRecord as Record,
    CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection,
    ValidatedCompositionBundle,
};

const POINTER_FILE: &str = "active-generation.ron";
const GENERATIONS_DIR: &str = "generations";
const CORE_FILE: &str = "core.ron";
const EXTENSIONS_DIR: &str = "extensions";

pub trait CompositionFileOperations: Clone {
    type ReplacementFile;

    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn create_staging_dir(&self, parent: &Path) -> io::Result<PathBuf>;
    fn write_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn sync_file(&self, path: &Path) -> io::Result<()>;
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn list_file_names(&self, path: &Path) -> io::Result<Vec<OsString>>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn create_replacement_file(&self, parent: &Path) -> io::Result<Self::ReplacementFile>;
    fn write_replacement_file(
        &self,
        file: &mut Self::ReplacementFile,
        bytes: &[u8],
    ) -> io::Result<()>;
    fn sync_replacement_file(&self, file: &Self::ReplacementFile) -> io::Result<()>;
    fn persist_replacement_file(&self, file: Self::ReplacementFile, path: &Path) -> io::Result<()>;
    fn sync_dir(&self, path: &Path) -> io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeCompositionFileOperations;

#[doc(hidden)]
pub struct NativeCompositionReplacementFile(tempfile::NamedTempFile);

impl CompositionFileOperations for NativeCompositionFileOperations {
    type ReplacementFile = NativeCompositionReplacementFile;

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn create_staging_dir(&self, parent: &Path) -> io::Result<PathBuf> {
        tempfile::Builder::new()
            .prefix(".composition-staging-")
            .tempdir_in(parent)
            .map(|directory| directory.keep())
    }

    fn write_file(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(bytes)
    }

    fn sync_file(&self, path: &Path) -> io::Result<()> {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?
            .sync_all()
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn list_file_names(&self, path: &Path) -> io::Result<Vec<OsString>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn create_replacement_file(&self, parent: &Path) -> io::Result<Self::ReplacementFile> {
        tempfile::NamedTempFile::new_in(parent).map(NativeCompositionReplacementFile)
    }

    fn write_replacement_file(
        &self,
        file: &mut Self::ReplacementFile,
        bytes: &[u8],
    ) -> io::Result<()> {
        file.0.write_all(bytes)
    }

    fn sync_replacement_file(&self, file: &Self::ReplacementFile) -> io::Result<()> {
        file.0.as_file().sync_all()
    }

    fn persist_replacement_file(&self, file: Self::ReplacementFile, path: &Path) -> io::Result<()> {
        file.0.persist(path).map_err(|error| error.error)?;
        Ok(())
    }

    fn sync_dir(&self, path: &Path) -> io::Result<()> {
        #[cfg(windows)]
        {
            let _ = path;
            return Ok(());
        }
        #[cfg(not(windows))]
        File::open(path)?.sync_all()
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::remove_dir_all(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionActivation {
    pub active_generation: CompositionGenerationId,
    pub previous_generation: Option<CompositionGenerationId>,
    pub diagnostics: Vec<Record>,
}

#[derive(Clone, Debug)]
pub struct LoadedCompositionBundle {
    pub bundle: ValidatedCompositionBundle,
    pub status: CompositionGenerationLoadStatus,
    pub diagnostics: Vec<Record>,
}

#[derive(Clone, Debug)]
pub struct CompositionBundleRepository<O = NativeCompositionFileOperations> {
    root: PathBuf,
    operations: O,
}

impl CompositionBundleRepository<NativeCompositionFileOperations> {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::with_operations(root, NativeCompositionFileOperations)
    }
}

impl<O: CompositionFileOperations> CompositionBundleRepository<O> {
    pub fn with_operations(root: impl Into<PathBuf>, operations: O) -> Self {
        Self {
            root: root.into(),
            operations,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn current_pointer(
        &self,
    ) -> Result<Option<CompositionGenerationPointerV1>, CompositionPersistenceRejection> {
        let path = self.root.join(POINTER_FILE);
        if !self.operations.exists(&path) {
            return Ok(None);
        }
        let source = self.operations.read_to_string(&path).map_err(|error| {
            storage_error(
                Code::ReadbackFailed,
                Stage::Storage,
                &path,
                "Read the active composition generation pointer.",
                error,
            )
        })?;
        let pointer = CanonicalCompositionDocuments::decode_generation_pointer(&source)?;
        if pointer.pointer_schema_version != CompositionGenerationPointerV1::POINTER_SCHEMA_VERSION
        {
            return Err(super::diagnostic::rejection(
                Code::UnsupportedSchema,
                Stage::Storage,
                Subject::Path(path.display().to_string()),
                "Use active generation pointer schema version 1.",
            ));
        }
        Ok(Some(pointer))
    }

    pub fn activate(
        &self,
        candidate: &CompositionBundleCandidate,
        expected_active_generation: Option<&CompositionGenerationId>,
    ) -> Result<CompositionActivation, CompositionPersistenceRejection> {
        candidate.validate(None)?;
        self.operations
            .create_dir_all(&self.root)
            .map_err(|error| {
                storage_error(
                    Code::StagingFailed,
                    Stage::Storage,
                    &self.root,
                    "Create the composition persistence scope root.",
                    error,
                )
            })?;
        let current = self.current_pointer()?;
        let actual = current.as_ref().map(|pointer| &pointer.active);
        if actual != expected_active_generation {
            return Err(super::diagnostic::rejection(
                Code::StaleGeneration,
                Stage::Storage,
                Subject::General("active_generation".to_owned()),
                "Reload the active generation and retry the compare-and-swap activation.",
            ));
        }
        if actual == Some(candidate.generation_id()) {
            return Ok(CompositionActivation {
                active_generation: candidate.generation_id().clone(),
                previous_generation: current.and_then(|pointer| pointer.previous),
                diagnostics: Vec::new(),
            });
        }

        let generations = self.root.join(GENERATIONS_DIR);
        self.operations
            .create_dir_all(&generations)
            .map_err(|error| {
                storage_error(
                    Code::StagingFailed,
                    Stage::Storage,
                    &generations,
                    "Create the composition generation directory.",
                    error,
                )
            })?;
        let staging = self
            .operations
            .create_staging_dir(&generations)
            .map_err(|error| {
                storage_error(
                    Code::StagingFailed,
                    Stage::Storage,
                    &generations,
                    "Create a same-filesystem staging generation.",
                    error,
                )
            })?;
        let result = self.write_validate_and_commit_generation(candidate, &staging, &generations);
        if result.is_err() && self.operations.exists(&staging) {
            let _ = self.operations.remove_dir_all(&staging);
        }
        result?;

        let previous = current.map(|pointer| pointer.active);
        let pointer = CompositionGenerationPointerV1::new(
            candidate.generation_id().clone(),
            previous.clone(),
        );
        let pointer_source = CanonicalCompositionDocuments::generation_pointer(&pointer)?;
        let pointer_path = self.root.join(POINTER_FILE);
        let mut replacement = self
            .operations
            .create_replacement_file(&self.root)
            .map_err(|error| {
                storage_error(
                    Code::PointerWriteFailed,
                    Stage::Storage,
                    &pointer_path,
                    "Create the same-directory active generation pointer replacement file.",
                    error,
                )
            })?;
        self.operations
            .write_replacement_file(&mut replacement, pointer_source.as_bytes())
            .map_err(|error| {
                storage_error(
                    Code::PointerWriteFailed,
                    Stage::Storage,
                    &pointer_path,
                    "Write the active composition generation pointer replacement file.",
                    error,
                )
            })?;
        self.operations
            .sync_replacement_file(&replacement)
            .map_err(|error| {
                storage_error(
                    Code::SyncFailed,
                    Stage::Storage,
                    &pointer_path,
                    "Sync the active composition generation pointer replacement file.",
                    error,
                )
            })?;
        self.operations
            .persist_replacement_file(replacement, &pointer_path)
            .map_err(|error| {
                storage_error(
                    Code::PointerCommitFailed,
                    Stage::Storage,
                    &pointer_path,
                    "Atomically replace the active composition generation pointer.",
                    error,
                )
            })?;
        let diagnostics = self
            .operations
            .sync_dir(&self.root)
            .err()
            .map(|error| {
                Record::warning(
                    Code::SyncFailed,
                    Stage::Storage,
                    Subject::Path(self.root.display().to_string()),
                    "The pointer replacement committed, but its directory durability sync failed; treat the new generation as active and retry a durability sync.",
                )
                .with_context("source_error", error.to_string())
            })
            .into_iter()
            .collect();
        Ok(CompositionActivation {
            active_generation: pointer.active,
            previous_generation: pointer.previous,
            diagnostics,
        })
    }

    pub fn load_active(
        &self,
        requirement: Option<&CompositionCompatibilityRequirement>,
    ) -> Result<LoadedCompositionBundle, CompositionPersistenceRejection> {
        let Some(pointer) = self.current_pointer()? else {
            return Err(super::diagnostic::rejection(
                Code::NoValidGeneration,
                Stage::Recovery,
                Subject::Path(self.root.display().to_string()),
                "Create or select a valid composition layout generation.",
            ));
        };
        match self.read_generation(&pointer.active, requirement) {
            Ok(bundle) => Ok(LoadedCompositionBundle {
                bundle,
                status: CompositionGenerationLoadStatus::Active,
                diagnostics: Vec::new(),
            }),
            Err(active_rejection) => {
                let Some(previous) = pointer.previous else {
                    return Err(active_rejection);
                };
                match self.read_generation(&previous, requirement) {
                    Ok(bundle) => Ok(LoadedCompositionBundle {
                        bundle,
                        status: CompositionGenerationLoadStatus::RecoveredLastGood,
                        diagnostics: vec![Record::warning(
                            Code::LastGoodRecovered,
                            Stage::Recovery,
                            Subject::Generation(previous.to_string()),
                            "The active generation was invalid; using the explicit previous valid generation without rewriting the pointer.",
                        )],
                    }),
                    Err(previous_rejection) => {
                        let mut diagnostics = active_rejection.diagnostics().to_vec();
                        diagnostics.extend(previous_rejection.diagnostics().iter().cloned());
                        diagnostics.push(Record::error(
                            Code::NoValidGeneration,
                            Stage::Recovery,
                            Subject::Path(self.root.display().to_string()),
                            "Neither active nor previous composition generation is valid; select or create a valid layout.",
                        ));
                        Err(CompositionPersistenceRejection::new(diagnostics))
                    }
                }
            }
        }
    }

    pub fn read_generation(
        &self,
        generation: &CompositionGenerationId,
        requirement: Option<&CompositionCompatibilityRequirement>,
    ) -> Result<ValidatedCompositionBundle, CompositionPersistenceRejection> {
        self.read_generation_at(
            &self
                .root
                .join(GENERATIONS_DIR)
                .join(generation.directory_name()),
            Some(generation),
            requirement,
        )
    }

    fn write_validate_and_commit_generation(
        &self,
        candidate: &CompositionBundleCandidate,
        staging: &Path,
        generations: &Path,
    ) -> Result<(), CompositionPersistenceRejection> {
        let extension_dir = staging.join(EXTENSIONS_DIR);
        self.operations
            .create_dir_all(&extension_dir)
            .map_err(|error| {
                storage_error(
                    Code::StagingFailed,
                    Stage::Storage,
                    &extension_dir,
                    "Create the staging extension directory.",
                    error,
                )
            })?;
        let core_source = CanonicalCompositionDocuments::core_envelope(candidate.core())?;
        self.write_file(&staging.join(CORE_FILE), core_source.as_bytes())?;
        for extension in candidate.extensions() {
            let source = CanonicalCompositionDocuments::extension_envelope(extension)?;
            self.write_file(
                &extension_dir.join(extension_file_name(&extension.link.identity)),
                source.as_bytes(),
            )?;
        }
        self.operations.sync_dir(&extension_dir).map_err(|error| {
            storage_error(
                Code::SyncFailed,
                Stage::Storage,
                &extension_dir,
                "Sync the staging extension directory.",
                error,
            )
        })?;
        self.operations.sync_dir(staging).map_err(|error| {
            storage_error(
                Code::SyncFailed,
                Stage::Storage,
                staging,
                "Sync the staging composition generation.",
                error,
            )
        })?;
        let readback = self.read_generation_at(staging, None, None)?;
        if readback.generation_id() != candidate.generation_id() {
            return Err(super::diagnostic::rejection(
                Code::DigestMismatch,
                Stage::Storage,
                Subject::Generation(candidate.generation_id().to_string()),
                "Reject the staging generation because readback changed its digest.",
            ));
        }
        let final_path = generations.join(candidate.generation_id().directory_name());
        if self.operations.exists(&final_path) {
            let existing =
                self.read_generation_at(&final_path, Some(candidate.generation_id()), None)?;
            if existing != readback {
                return Err(super::diagnostic::rejection(
                    Code::DigestMismatch,
                    Stage::Storage,
                    Subject::Generation(candidate.generation_id().to_string()),
                    "An existing generation with this digest has different canonical content.",
                ));
            }
            self.operations.remove_dir_all(staging).map_err(|error| {
                storage_error(
                    Code::GenerationCommitFailed,
                    Stage::Storage,
                    staging,
                    "Remove the duplicate staging generation after validating the immutable generation.",
                    error,
                )
            })?;
        } else {
            self.operations
                .rename(staging, &final_path)
                .map_err(|error| {
                    storage_error(
                        Code::GenerationCommitFailed,
                        Stage::Storage,
                        &final_path,
                        "Commit the validated immutable composition generation.",
                        error,
                    )
                })?;
            self.operations.sync_dir(generations).map_err(|error| {
                storage_error(
                    Code::SyncFailed,
                    Stage::Storage,
                    generations,
                    "Sync the generation directory after committing the immutable generation.",
                    error,
                )
            })?;
        }
        Ok(())
    }

    fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), CompositionPersistenceRejection> {
        self.operations.write_file(path, bytes).map_err(|error| {
            storage_error(
                Code::WriteFailed,
                Stage::Storage,
                path,
                "Write the canonical composition generation file.",
                error,
            )
        })?;
        self.operations.sync_file(path).map_err(|error| {
            storage_error(
                Code::SyncFailed,
                Stage::Storage,
                path,
                "Sync the canonical composition generation file.",
                error,
            )
        })
    }

    fn read_generation_at(
        &self,
        path: &Path,
        expected_generation: Option<&CompositionGenerationId>,
        requirement: Option<&CompositionCompatibilityRequirement>,
    ) -> Result<ValidatedCompositionBundle, CompositionPersistenceRejection> {
        let core_path = path.join(CORE_FILE);
        let core_source = self
            .operations
            .read_to_string(&core_path)
            .map_err(|error| {
                storage_error(
                    Code::ReadbackFailed,
                    Stage::Storage,
                    &core_path,
                    "Read the canonical core composition envelope.",
                    error,
                )
            })?;
        let core = CanonicalCompositionDocuments::decode_core_envelope(&core_source)?;
        let extension_dir = path.join(EXTENSIONS_DIR);
        let actual_names = self
            .operations
            .list_file_names(&extension_dir)
            .map_err(|error| {
                storage_error(
                    Code::ReadbackFailed,
                    Stage::Storage,
                    &extension_dir,
                    "List the exact composition extension document set.",
                    error,
                )
            })?;
        let expected_names = core
            .extension_links
            .iter()
            .map(|link| OsString::from(extension_file_name(&link.identity)))
            .collect::<std::collections::BTreeSet<_>>();
        let actual_names = actual_names
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        if actual_names != expected_names {
            return Err(super::diagnostic::rejection(
                Code::ExtraExtension,
                Stage::Bundle,
                Subject::Path(extension_dir.display().to_string()),
                "Make the on-disk extension document set match the core envelope links exactly.",
            ));
        }
        let mut extensions = Vec::with_capacity(core.extension_links.len());
        for link in &core.extension_links {
            let extension_path = extension_dir.join(extension_file_name(&link.identity));
            let source = self
                .operations
                .read_to_string(&extension_path)
                .map_err(|error| {
                    storage_error(
                        Code::ReadbackFailed,
                        Stage::Storage,
                        &extension_path,
                        "Read the canonical composition extension envelope.",
                        error,
                    )
                })?;
            extensions.push(CanonicalCompositionDocuments::decode_extension_envelope(
                &source,
            )?);
        }
        let bundle = ValidatedCompositionBundle::from_envelopes(core, extensions, requirement)?;
        if let Some(expected) = expected_generation
            && bundle.generation_id() != expected
        {
            return Err(super::diagnostic::rejection(
                Code::DigestMismatch,
                Stage::Storage,
                Subject::Generation(expected.to_string()),
                "Match the generation directory identity to the complete canonical bundle digest.",
            ));
        }
        Ok(bundle)
    }
}

fn extension_file_name(identity: &CompositionExtensionIdentity) -> String {
    format!(
        "{}.v{}.ron",
        identity.profile.as_str(),
        identity.schema_version.get()
    )
}

fn storage_error(
    code: Code,
    stage: Stage,
    path: &Path,
    message: &'static str,
    error: io::Error,
) -> CompositionPersistenceRejection {
    CompositionPersistenceRejection::single(
        Record::error(
            code,
            stage,
            Subject::Path(path.display().to_string()),
            message,
        )
        .with_context("source_error", error.to_string()),
    )
}
