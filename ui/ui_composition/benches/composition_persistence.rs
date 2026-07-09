use std::hint::black_box;
use std::time::{Duration, Instant};

use ui_composition::*;

#[derive(Clone, Copy)]
struct BenchmarkShape {
    name: &'static str,
    regions: usize,
    units: usize,
    extensions: usize,
    extension_bytes: usize,
}

const SHAPES: [BenchmarkShape; 3] = [
    BenchmarkShape {
        name: "small",
        regions: 16,
        units: 16,
        extensions: 1,
        extension_bytes: 4 * 1024,
    },
    BenchmarkShape {
        name: "medium",
        regions: 512,
        units: 1_024,
        extensions: 4,
        extension_bytes: 256 * 1024,
    },
    BenchmarkShape {
        name: "large",
        regions: 4_096,
        units: 8_192,
        extensions: 16,
        extension_bytes: 1024 * 1024,
    },
];

fn main() {
    for shape in SHAPES {
        let candidate = candidate(shape);
        let core_source = CanonicalCompositionDocuments::core_envelope(candidate.core()).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let repository = CompositionBundleRepository::new(directory.path());
        repository.activate(&candidate, None).unwrap();

        let serialize = measure(|| {
            black_box(
                CanonicalCompositionDocuments::core_envelope(black_box(candidate.core())).unwrap(),
            );
        });
        let deserialize = measure(|| {
            black_box(
                CanonicalCompositionDocuments::decode_core_envelope(black_box(&core_source))
                    .unwrap(),
            );
        });
        let validate = measure(|| {
            black_box(candidate.validate(None).unwrap());
        });
        let read = measure(|| {
            black_box(repository.load_active(None).unwrap());
        });

        report(shape, "core_serialize", core_source.len(), &serialize);
        report(shape, "core_deserialize", core_source.len(), &deserialize);
        report(shape, "bundle_validate", shape.units, &validate);
        report(shape, "generation_read", shape.units, &read);

        if shape.name == "large" {
            assert!(percentile(&serialize, 95) <= Duration::from_millis(250));
            assert!(percentile(&deserialize, 95) <= Duration::from_millis(500));
        }
    }
}

fn measure(mut operation: impl FnMut()) -> Vec<Duration> {
    for _ in 0..10 {
        operation();
    }
    let mut samples = Vec::with_capacity(50);
    for _ in 0..50 {
        let started = Instant::now();
        operation();
        samples.push(started.elapsed());
    }
    samples.sort();
    samples
}

fn report(shape: BenchmarkShape, operation: &str, amount: usize, samples: &[Duration]) {
    let p50 = percentile(samples, 50);
    let p95 = percentile(samples, 95);
    let units_per_second = amount as f64 / p50.as_secs_f64();
    println!(
        "shape={} operation={} p50_us={} p95_us={} units_per_second={:.2}",
        shape.name,
        operation,
        p50.as_micros(),
        p95.as_micros(),
        units_per_second
    );
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    samples[(samples.len() - 1) * percentile / 100]
}

fn candidate(shape: BenchmarkShape) -> CompositionBundleCandidate {
    let base_stack_units = shape.units - (shape.regions - 2);
    let units = (1..=shape.units)
        .map(|raw| {
            MountedUnitDefinition::new(
                MountedUnitId::new(raw as u64),
                MountedContentRef::new(
                    ContentOwnerId::new("benchmark.owner").unwrap(),
                    ContentProfileId::new("benchmark.content").unwrap(),
                    ContentInstanceRef::new(format!("benchmark.unit-{raw}")).unwrap(),
                ),
                [],
                UnavailableContentPolicy::ShowFallback,
            )
        })
        .collect::<Vec<_>>();
    let mut regions = Vec::with_capacity(shape.regions);
    regions.push(RegionDefinition::new(
        RegionId::new(1),
        None,
        RegionKind::Overlay {
            base: RegionId::new(2),
            ordered_overlays: (3..=shape.regions)
                .map(|raw| RegionId::new(raw as u64))
                .collect(),
        },
    ));
    regions.push(RegionDefinition::new(
        RegionId::new(2),
        None,
        RegionKind::Stack {
            ordered_units: (1..=base_stack_units)
                .map(|raw| MountedUnitId::new(raw as u64))
                .collect(),
            active_unit: MountedUnitId::new(1),
        },
    ));
    for region_raw in 3..=shape.regions {
        let unit_raw = base_stack_units + region_raw - 2;
        regions.push(RegionDefinition::new(
            RegionId::new(region_raw as u64),
            None,
            RegionKind::MountPoint {
                mounted_unit: MountedUnitId::new(unit_raw as u64),
            },
        ));
    }
    let definition = CompositionDefinitionV1::new(
        CompositionDefinitionId::new(1),
        DefinitionRevision::new(1),
        vec![PresentationTargetDefinition::new(
            PresentationTargetId::new(1),
            TargetProfileId::new("benchmark.desktop").unwrap(),
        )],
        vec![CompositionRootDefinition::new(
            CompositionRootId::new(1),
            PresentationTargetId::new(1),
            RegionId::new(1),
            true,
        )],
        regions,
        units,
    );
    let bytes_per_extension = shape.extension_bytes / shape.extensions;
    let extensions = (0..shape.extensions)
        .map(|index| {
            CanonicalExtensionPayload::new(
                ExtensionProfileId::new(format!("benchmark.extension-{index}")).unwrap(),
                ExtensionSchemaVersion::new(1).unwrap(),
                format!(
                    "\"{}\"\n",
                    "x".repeat(bytes_per_extension.saturating_sub(3))
                ),
            )
            .unwrap()
        })
        .collect();
    CompositionBundleCandidate::form(
        definition,
        CompositionCompatibility::new(
            AppProfileId::new("benchmark.app").unwrap(),
            AppSchemaVersion::new(1).unwrap(),
            AppSchemaVersion::new(1).unwrap(),
        )
        .unwrap(),
        extensions,
    )
    .unwrap()
}
