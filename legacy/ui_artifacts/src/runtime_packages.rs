use super::*;

pub(crate) fn runtime_packages(program: &UiProgram) -> Vec<RuntimePackageRecord> {
    let mut packages: BTreeMap<String, RuntimePackageRecord> = BTreeMap::new();
    for node in &program.graphs.control.nodes {
        let package_id = node.package_id.as_str().to_owned();
        let package =
            packages
                .entry(package_id.to_owned())
                .or_insert_with(|| RuntimePackageRecord {
                    package_id,
                    ..RuntimePackageRecord::default()
                });
        push_unique(&mut package.control_node_ids, node.node_id.as_str());
        push_unique(&mut package.control_kind_ids, node.control_kind.as_str());
    }

    for constraint in &program.graphs.layout.constraints {
        if let Some(kernel) = constraint.layout_kernel.as_ref() {
            push_kernel_for_package(&mut packages, kernel.as_str());
        }
    }
    for operator in &program.graphs.visual.operators {
        push_kernel_for_package(&mut packages, operator.visual_kernel.as_str());
    }

    packages.into_values().collect()
}

pub(crate) fn push_kernel_for_package(
    packages: &mut BTreeMap<String, RuntimePackageRecord>,
    kernel: &str,
) {
    if let Some(package_id) = owning_package_id(packages.keys(), kernel) {
        if let Some(package) = packages.get_mut(&package_id) {
            push_unique(&mut package.kernel_ids, kernel);
        }
    }
}

fn owning_package_id<'a>(
    package_ids: impl Iterator<Item = &'a String>,
    namespaced_id: &str,
) -> Option<String> {
    package_ids
        .filter(|package_id| {
            namespaced_id == package_id.as_str()
                || namespaced_id
                    .strip_prefix(package_id.as_str())
                    .is_some_and(|suffix| suffix.starts_with('.'))
        })
        .max_by_key(|package_id| package_id.len())
        .map(ToOwned::to_owned)
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}
