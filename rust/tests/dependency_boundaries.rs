#[path = "../test_support/dependency_boundaries/mod.rs"]
mod dependency_boundaries;

#[path = "../test_support/dependency_boundaries/tests.rs"]
mod parser_tests;

#[path = "../test_support/dependency_boundaries/policy.rs"]
mod policy;

#[path = "../test_support/dependency_boundaries/policy_tests.rs"]
mod policy_tests;

#[test]
fn repository_dependencies_match_the_fixed_inventory() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let modules = dependency_boundaries::load_modules(repository);
    assert!(modules.values().all(|module| module.lines > 0));
    let dependencies = dependency_boundaries::collect_dependencies(&modules);
    let inventory_path = "tool/dependency-boundaries/legacy_rust_dependencies.json";
    let hash = std::process::Command::new("git")
        .current_dir(repository)
        .args([
            "hash-object",
            &format!("--path={inventory_path}"),
            inventory_path,
        ])
        .output()
        .expect("hash frozen inventory");
    assert!(hash.status.success());
    assert_eq!(
        String::from_utf8(hash.stdout).unwrap().trim(),
        "b46bdae35521d0153bb315a911f3882c429d1787",
        "The baseline is immutable; remove resolved IDs from the active policy instead"
    );
    let inventory: policy::Inventory =
        serde_json::from_str(&std::fs::read_to_string(repository.join(inventory_path)).unwrap())
            .unwrap();
    assert_eq!(
        inventory.baseline,
        "f74146a4e88d72325c576504825399df109774aa"
    );
    let policy: policy::Policy = serde_json::from_str(
        &std::fs::read_to_string(
            repository.join("tool/dependency-boundaries/rust_boundary_policy.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let errors = policy::compare(&dependencies, &inventory, &policy);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    for entry in inventory
        .entries
        .iter()
        .filter(|entry| policy.active_ids.contains(&entry.id))
    {
        for coverage in &inventory.owners[&entry.owner].coverage {
            assert!(
                repository.join(coverage).is_file(),
                "Lost coverage owner for {}: {coverage}",
                entry.id
            );
        }
    }
}
