//! File: domain/ui/ui_controls/src/catalog/index.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};

use crate::package::descriptor::ControlPackageDescriptor;

use super::{ControlCatalogEntryDescriptor, ControlCatalogQuery, ControlCatalogQueryResult};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCatalogIndex {
    pub entries: Vec<ControlCatalogEntryDescriptor>,
}

impl ControlCatalogIndex {
    pub fn from_packages<'a>(
        packages: impl IntoIterator<Item = &'a ControlPackageDescriptor>,
    ) -> Self {
        let mut entries = packages
            .into_iter()
            .flat_map(|package| {
                package.control_kinds.iter().map(move |kind| {
                    ControlCatalogEntryDescriptor::from_control_kind(package, kind)
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.package_id
                .cmp(&right.package_id)
                .then_with(|| left.control_kind_id.cmp(&right.control_kind_id))
        });
        Self { entries }
    }

    pub fn query(&self, query: &ControlCatalogQuery) -> ControlCatalogQueryResult {
        ControlCatalogQueryResult {
            entries: self
                .entries
                .iter()
                .filter(|entry| query.matches(entry))
                .cloned()
                .collect(),
        }
    }

    pub fn entry(&self, control_kind_id: &str) -> Option<&ControlCatalogEntryDescriptor> {
        self.entries
            .iter()
            .find(|entry| entry.control_kind_id == control_kind_id)
    }
}
