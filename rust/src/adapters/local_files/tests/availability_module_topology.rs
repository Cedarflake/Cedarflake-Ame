use super::{
    CRATE_PARENT_MODULE_CONTRACTS, LOCAL_MODULE_CONTRACTS,
    assert_availability_exact_source_contract_with_environment,
};

const LOCAL_SOURCE: &str = include_str!("../../local_files.rs");
const DOMAIN_SOURCE: &str = include_str!("../../../domain/mod.rs");
const METADATA_SOURCE: &str = include_str!("../../../domain/library_metadata_inventory.rs");
const ADAPTERS_SOURCE: &str = include_str!("../../mod.rs");
const CRATE_SOURCE: &str = include_str!("../../../lib.rs");

#[test]
fn root_availability_admitted_modules_preserve_exact_source_loading_contracts() {
    for module_name in [
        "media_signature",
        "file_admission",
        "preview_cache_namespace",
    ] {
        let contracts = LOCAL_MODULE_CONTRACTS
            .iter()
            .filter(|contract| contract.name == module_name)
            .collect::<Vec<_>>();
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].visibility, "");
        assert!(contracts[0].attributes.is_empty());
        assert!(!contracts[0].is_inline);
    }
    let fixtures = CRATE_PARENT_MODULE_CONTRACTS
        .iter()
        .filter(|contract| contract.name == "media_fixtures")
        .collect::<Vec<_>>();
    assert_eq!(fixtures.len(), 1);
    assert_eq!(fixtures[0].visibility, "pub(crate)");
    assert_eq!(
        fixtures[0].attributes,
        ["cfg(test)", "path=\"../test_support/media_fixtures.rs\""]
    );
    assert!(!fixtures[0].is_inline);
    assert_contract(LOCAL_SOURCE, CRATE_SOURCE)
        .expect("the complete live source contract must accept the exact declarations");
}

#[test]
fn root_availability_admitted_modules_reject_alternate_generated_or_broader_loading() {
    let crate_source = CRATE_SOURCE.replace("\r\n", "\n");
    let signature = "mod media_signature;";
    let fixtures = concat!(
        "#[cfg(test)]\n",
        "#[path = \"../test_support/media_fixtures.rs\"]\n",
        "pub(crate) mod media_fixtures;"
    );
    for (source, declaration, replacements, source_key, module_name) in [
        (
            LOCAL_SOURCE,
            "mod preview_cache_namespace;",
            vec![
                "".to_owned(),
                "mod preview_cache_namespace;\nmod preview_cache_namespace;".to_owned(),
                "pub mod preview_cache_namespace;".to_owned(),
                "#[cfg(test)]\nmod preview_cache_namespace;".to_owned(),
                "#[path = \"alternate.rs\"]\nmod preview_cache_namespace;".to_owned(),
                "#[cfg_attr(not(test), path = \"alternate.rs\")]\nmod preview_cache_namespace;"
                    .to_owned(),
                "#[adversarial_loader]\nmod preview_cache_namespace;".to_owned(),
                "mod preview_cache_namespace { mod generated {} }".to_owned(),
                "include!(\"alternate.rs\");".to_owned(),
                "mod preview_cache_namespace;\nmod unknown_cleanup_namespace;".to_owned(),
            ],
            "local",
            "preview_cache_namespace",
        ),
        (
            LOCAL_SOURCE,
            "mod file_admission;",
            vec![
                "".to_owned(),
                "mod file_admission;\nmod file_admission;".to_owned(),
                "pub mod file_admission;".to_owned(),
                "#[cfg(test)]\nmod file_admission;".to_owned(),
                "#[path = \"alternate.rs\"]\nmod file_admission;".to_owned(),
                "#[cfg_attr(not(test), path = \"alternate.rs\")]\nmod file_admission;".to_owned(),
                "#[adversarial_loader]\nmod file_admission;".to_owned(),
                "mod file_admission { mod generated {} }".to_owned(),
                "include!(\"alternate.rs\");".to_owned(),
                "mod file_admission;\nmod unknown_file_admission;".to_owned(),
            ],
            "local",
            "file_admission",
        ),
        (
            LOCAL_SOURCE,
            signature,
            vec![
                "".to_owned(),
                format!("{signature}\n{signature}"),
                "pub mod media_signature;".to_owned(),
                "#[cfg(test)]\nmod media_signature;".to_owned(),
                "#[path = \"alternate.rs\"]\nmod media_signature;".to_owned(),
                "#[cfg_attr(not(test), path = \"alternate.rs\")]\nmod media_signature;".to_owned(),
                "#[adversarial_loader]\nmod media_signature;".to_owned(),
                "mod media_signature { mod generated {} }".to_owned(),
                "include!(\"alternate.rs\");".to_owned(),
                format!("{signature}\nmod unknown_media_signature;"),
            ],
            "local",
            "media_signature",
        ),
        (
            crate_source.as_str(),
            fixtures,
            vec![
                "".to_owned(),
                format!("{fixtures}\n{fixtures}"),
                fixtures.replace("pub(crate)", "pub"),
                fixtures.replace("#[cfg(test)]\n", ""),
                fixtures.replace("cfg(test)", "cfg(any(test, windows))"),
                fixtures.replace("../test_support/media_fixtures.rs", "alternate.rs"),
                fixtures.replace("#[path = \"../test_support/media_fixtures.rs\"]\n", ""),
                fixtures.replace(
                    "pub(crate) mod media_fixtures;",
                    "pub(crate) mod media_fixtures { mod generated {} }",
                ),
                format!("#[cfg_attr(test, path = \"alternate.rs\")]\n{fixtures}"),
                format!("#[adversarial_loader]\n{fixtures}"),
                "include!(\"alternate.rs\");".to_owned(),
                format!("{fixtures}\nmod unknown_media_fixtures;"),
            ],
            "crate-parent",
            "media_fixtures",
        ),
    ] {
        assert!(
            source.contains(declaration),
            "the exact {module_name} fixture must match the current source"
        );
        for replacement in replacements {
            let mutated = source.replacen(declaration, &replacement, 1);
            assert_ne!(mutated, source, "the {module_name} mutation must be real");
            let (local, parent) = if source_key == "local" {
                (mutated.as_str(), CRATE_SOURCE)
            } else {
                (LOCAL_SOURCE, mutated.as_str())
            };
            let error = assert_contract(local, parent)
                .expect_err("changing an admitted module must still fail closed");
            assert!(
                error.contains(&format!("availability module topology source {source_key}")),
                "the exact loading owner must reject {replacement:?}: {error}"
            );
        }
    }
}

fn assert_contract(local: &str, parent: &str) -> Result<(), String> {
    assert_availability_exact_source_contract_with_environment(
        local,
        Some(DOMAIN_SOURCE),
        Some(METADATA_SOURCE),
        Some(ADAPTERS_SOURCE),
        Some(parent),
    )
}
