use std::collections::BTreeSet;

use super::dependency_boundaries::references::Dependency;
use super::policy::{Inventory, Policy, compare};

fn fixture() -> (Inventory, Policy, BTreeSet<Dependency>) {
    let inventory: Inventory = serde_json::from_value(serde_json::json!({
        "baseline": "fixed",
        "owners": {"catalog": {
            "responsibility": "Catalog construction",
            "reason": "Named runtime factory",
            "lifetime": "One session per path",
            "disposition": "Retained composition",
            "coverage": ["catalog_test.rs"]
        }},
        "entries": [{"id": "R001", "owner": "catalog", "dependency": {
            "source": "rust/src/application/catalog.rs",
            "symbol": "crate::application::catalog::open",
            "target": "crate::adapters::Database",
            "rule": "concrete-adapter"
        }}]
    }))
    .unwrap();
    let current = BTreeSet::from([inventory.entries[0].dependency.clone()]);
    (
        inventory,
        Policy {
            active_ids: vec!["R001".to_owned()],
            retained_owners: Default::default(),
            retained: Vec::new(),
        },
        current,
    )
}

#[test]
fn exact_named_composition_passes() {
    let (inventory, policy, current) = fixture();
    assert!(compare(&current, &inventory, &policy).is_empty());
}

#[test]
fn a_second_function_in_the_same_file_is_not_admitted() {
    let (inventory, policy, mut current) = fixture();
    let mut additional = inventory.entries[0].dependency.clone();
    additional.symbol = "crate::application::catalog::business_policy".to_owned();
    current.insert(additional);
    let errors = compare(&current, &inventory, &policy);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("business_policy"));
}

#[test]
fn removed_edges_require_removing_their_active_ids() {
    let (inventory, mut policy, _) = fixture();
    assert!(compare(&BTreeSet::new(), &inventory, &policy)[0].contains("Stale exception"));
    policy.active_ids.clear();
    assert!(compare(&BTreeSet::new(), &inventory, &policy).is_empty());
}

#[test]
fn duplicate_unknown_and_unclassified_entries_fail() {
    let (mut inventory, mut policy, current) = fixture();
    policy
        .active_ids
        .extend(["R001".to_owned(), "unknown".to_owned()]);
    inventory.entries[0].owner = "missing".to_owned();
    let errors = compare(&current, &inventory, &policy);
    assert_eq!(errors.len(), 3);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("Missing responsibility"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("Duplicate active ID"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("Unknown active ID"))
    );
}

#[test]
fn a_folder_exception_cannot_replace_an_exact_symbol() {
    let (mut inventory, policy, current) = fixture();
    inventory.entries[0].dependency.source = "rust/src/application/".to_owned();
    let errors = compare(&current, &inventory, &policy);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("exact source, symbol and target"))
    );
}

fn retained_fixture() -> (Inventory, Policy, BTreeSet<Dependency>) {
    let (inventory, _, current) = fixture();
    let policy = serde_json::from_value(serde_json::json!({
        "activeIds": [],
        "retainedOwners": {"catalog": {
            "responsibility": "Catalog construction",
            "reason": "Named session factory",
            "lifetime": "Caller owns the lane-scoped connection",
            "replacement": "Replace the factory while preserving its repository port",
            "decision": "docs/architecture/composition.md#catalog",
            "coverage": ["catalog_test.rs"]
        }},
        "retained": [{"id": "catalog-open", "owner": "catalog", "dependency": {
            "source": "rust/src/application/catalog.rs",
            "symbol": "crate::application::catalog::open",
            "target": "crate::adapters::Database",
            "rule": "concrete-adapter"
        }}]
    }))
    .unwrap();
    (inventory, policy, current)
}

#[test]
fn a_reviewed_boundary_replaces_only_its_exact_migration_edge() {
    let (inventory, policy, mut current) = retained_fixture();
    assert!(compare(&current, &inventory, &policy).is_empty());
    let mut additional = inventory.entries[0].dependency.clone();
    additional.symbol = "crate::application::catalog::business_policy".to_owned();
    current.insert(additional);
    let errors = compare(&current, &inventory, &policy);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("business_policy"));
}

#[test]
fn retained_boundaries_require_a_decision_lifetime_and_replacement() {
    let (inventory, mut policy, current) = retained_fixture();
    let owner = policy.retained_owners.get_mut("catalog").unwrap();
    owner.decision.clear();
    owner.lifetime.clear();
    owner.replacement.clear();
    let errors = compare(&current, &inventory, &policy);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("Incomplete retained boundary metadata"));
}

#[test]
fn retained_boundaries_cannot_hide_stale_or_duplicate_dependencies() {
    let (inventory, mut policy, current) = retained_fixture();
    assert!(compare(&BTreeSet::new(), &inventory, &policy)[0].contains("Stale retained"));
    policy.active_ids.push("R001".to_owned());
    assert!(compare(&current, &inventory, &policy)[0].contains("Duplicate admitted"));
}

#[test]
fn retained_boundaries_reject_wildcard_symbols_and_missing_owners() {
    let (inventory, mut policy, current) = retained_fixture();
    policy.retained[0].dependency.symbol = "crate::application::catalog::*".to_owned();
    policy.retained[0].owner = "missing".to_owned();
    let errors = compare(&current, &inventory, &policy);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("exact source, symbol and target"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("Missing retained responsibility"))
    );
}
