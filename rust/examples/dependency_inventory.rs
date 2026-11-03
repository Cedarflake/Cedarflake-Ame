#[path = "../test_support/dependency_boundaries/mod.rs"]
mod dependency_boundaries;

fn main() {
    let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let modules = dependency_boundaries::load_modules(repository);
    let dependencies = dependency_boundaries::collect_dependencies(&modules);
    let source_sizes = modules
        .values()
        .map(|module| (&module.source, module.lines))
        .collect::<std::collections::BTreeMap<_, _>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "dependencies": dependencies,
            "sourceSizes": source_sizes,
        }))
        .expect("serialize dependency inventory")
    );
}
