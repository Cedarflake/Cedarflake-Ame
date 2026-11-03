use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::dependency_boundaries::references::Dependency;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Inventory {
    pub baseline: String,
    pub owners: BTreeMap<String, Owner>,
    pub entries: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Owner {
    pub responsibility: String,
    pub reason: String,
    pub lifetime: String,
    pub disposition: String,
    pub coverage: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub owner: String,
    pub dependency: Dependency,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub active_ids: Vec<String>,
}

pub fn compare(
    current: &BTreeSet<Dependency>,
    inventory: &Inventory,
    policy: &Policy,
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut entries = BTreeMap::new();
    let mut unique_dependencies = BTreeSet::new();
    for entry in &inventory.entries {
        if entries.insert(&entry.id, entry).is_some() {
            errors.push(format!("Duplicate inventory ID: {}", entry.id));
        }
        if !unique_dependencies.insert(&entry.dependency) {
            errors.push(format!("Duplicate dependency: {}", entry.id));
        }
        if !inventory.owners.contains_key(&entry.owner) {
            errors.push(format!("Missing responsibility: {}", entry.id));
        }
        if !entry.dependency.source.ends_with(".rs")
            || !entry.dependency.symbol.starts_with("crate::")
            || entry.dependency.target.contains('*')
        {
            errors.push(format!(
                "An exception must name an exact source, symbol and target: {}",
                entry.id
            ));
        }
    }
    for (name, owner) in &inventory.owners {
        if [
            &owner.responsibility,
            &owner.reason,
            &owner.lifetime,
            &owner.disposition,
        ]
        .iter()
        .any(|text| text.trim().is_empty())
            || owner.coverage.is_empty()
        {
            errors.push(format!("Incomplete responsibility metadata: {name}"));
        }
    }
    let mut active = BTreeSet::new();
    let mut admitted = BTreeSet::new();
    for id in &policy.active_ids {
        if !active.insert(id) {
            errors.push(format!("Duplicate active ID: {id}"));
        }
        let Some(entry) = entries.get(id) else {
            errors.push(format!("Unknown active ID: {id}"));
            continue;
        };
        admitted.insert(entry.dependency.clone());
        if !current.contains(&entry.dependency) {
            errors.push(format!(
                "Stale exception {id}: {} -> {}",
                entry.dependency.symbol, entry.dependency.target
            ));
        }
    }
    for dependency in current.difference(&admitted) {
        errors.push(format!(
            "{}: {} -> {} [{}]",
            dependency.source, dependency.symbol, dependency.target, dependency.rule
        ));
    }
    errors
}
