use std::collections::BTreeMap;

use syn::Item;

use super::dependency_boundaries::collect_dependencies;
use super::dependency_boundaries::sources::{Module, is_test_only};

fn fixture(source: &str) -> BTreeMap<String, Module> {
    fn add(namespace: String, items: Vec<Item>, modules: &mut BTreeMap<String, Module>) {
        for item in &items {
            if let Item::Mod(module) = item
                && !is_test_only(&module.attrs)
                && let Some((_, children)) = &module.content
            {
                add(
                    format!("{namespace}::{}", module.ident),
                    children.clone(),
                    modules,
                );
            }
        }
        modules.insert(
            namespace.clone(),
            Module {
                source: format!("{}.rs", namespace.replace("::", "/")),
                namespace,
                items,
                lines: 1,
            },
        );
    }
    let mut modules = BTreeMap::new();
    add(
        "crate".to_owned(),
        syn::parse_file(source).unwrap().items,
        &mut modules,
    );
    modules
}

#[test]
fn a_module_and_its_reexported_function_can_share_a_name() {
    let modules = fixture(
        r#"
        mod adapters { mod database { pub fn database() {} } pub use database::database; }
        mod application { fn open() { crate::adapters::database(); } }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 1);
    assert_eq!(
        dependencies.first().unwrap().target,
        "crate::adapters::database::database"
    );
}

#[test]
fn private_trait_imports_cannot_hide_method_dependencies() {
    let source = r#"
        mod adapters {
            pub trait NativeRead { fn read_native(&self); }
            impl NativeRead for str { fn read_native(&self) {} }
            pub mod exports { pub use super::NativeRead as Read; }
            pub mod prelude { pub use super::exports::*; }
        }
        mod domain {
            MODULE_IMPORT
            pub fn inspect(value: &str) { LOCAL_IMPORT value.read_native(); }
        }
    "#;
    for (module_import, local_import, symbol) in [
        ("use crate::adapters::NativeRead;", "", "import::NativeRead"),
        (
            "use crate::adapters::NativeRead as Read;",
            "",
            "import::Read",
        ),
        ("use crate::adapters::NativeRead as _;", "", "import::_"),
        ("use crate::adapters::exports::Read;", "", "import::Read"),
        ("use crate::adapters::prelude::*;", "", "import::*"),
        ("", "use crate::adapters::NativeRead as _;", "inspect"),
    ] {
        let modules = fixture(
            &source
                .replace("MODULE_IMPORT", module_import)
                .replace("LOCAL_IMPORT", local_import),
        );
        let dependencies = collect_dependencies(&modules);
        assert_eq!(dependencies.len(), 1, "{module_import} {local_import}");
        let dependency = dependencies.first().unwrap();
        assert_eq!(dependency.source, "crate/domain.rs");
        assert_eq!(dependency.symbol, format!("crate::domain::{symbol}"));
        assert_eq!(dependency.target, "crate::adapters::NativeRead");
        assert_eq!(dependency.rule, "outward-layer");
    }
}

#[test]
fn trait_imports_preserve_port_access_and_local_composition_ownership() {
    let modules = fixture(
        r#"
        mod ports {
            pub trait Read { fn read(&self); }
            impl Read for str { fn read(&self) {} }
        }
        mod adapters {
            pub trait NativeRead { fn read_native(&self); }
            impl NativeRead for str { fn read_native(&self) {} }
        }
        mod application {
            use crate::ports::Read;
            #[cfg(test)] use crate::adapters::NativeRead;
            fn inspect(value: &str) { value.read(); }
            fn factory(value: &str) {
                use crate::adapters::NativeRead as _;
                value.read_native();
            }
        }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 1);
    let dependency = dependencies.first().unwrap();
    assert_eq!(dependency.symbol, "crate::application::factory");
    assert_eq!(dependency.target, "crate::adapters::NativeRead");
    assert_eq!(dependency.rule, "concrete-adapter");
}

#[test]
fn grouped_aliases_and_transitive_exports_resolve_to_the_adapter() {
    let modules = fixture(
        r#"
        mod adapters { pub struct Database; }
        mod application {
            pub use crate::adapters::{Database as Store};
            mod task { use super::Store as Handle; fn run() { Handle::open(); } }
        }
        mod api { fn open() -> crate::application::Store { todo!() } }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 3);
    assert!(
        dependencies
            .iter()
            .all(|dependency| dependency.target == "crate::adapters::Database")
    );
    assert!(
        dependencies
            .iter()
            .any(|dependency| dependency.symbol == "crate::application::task::run")
    );
    assert!(
        dependencies
            .iter()
            .any(|dependency| dependency.rule == "outward-layer")
    );
}

#[test]
fn globs_keep_concrete_types_without_turning_primitives_into_dependencies() {
    let modules = fixture(
        r#"
        mod adapters { pub struct Database; }
        mod application {
            use crate::adapters::Database;
            mod nested { use super::*; fn open() { Database::open(); let _: Vec<u32> = vec![]; } }
        }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 1);
    assert_eq!(
        dependencies.first().unwrap().target,
        "crate::adapters::Database"
    );
}

#[test]
fn local_aliases_are_limited_to_their_lexical_block() {
    let modules = fixture(
        r#"
        mod adapters { pub struct Database; }
        mod application {
            fn open() { use crate::adapters::Database as Store; Store::open(); }
            fn pure() { let Store = 1; consume(Store); }
        }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 1);
    assert_eq!(
        dependencies.first().unwrap().symbol,
        "crate::application::open"
    );
}

#[test]
fn concrete_resource_fields_and_methods_have_separate_owners() {
    let modules = fixture(
        r#"
        mod adapters { pub struct Database; }
        mod application {
            use crate::adapters::Database;
            struct Host { database: Database }
            impl Host { fn open() -> Self { Self { database: Database::open() } } }
        }
    "#,
    );
    let symbols = collect_dependencies(&modules)
        .into_iter()
        .map(|dependency| dependency.symbol)
        .collect::<Vec<_>>();
    assert_eq!(
        symbols,
        ["crate::application::Host", "crate::application::Host::open"]
    );
}

#[test]
fn test_only_code_is_excluded_without_hiding_conditional_production_code() {
    let modules = fixture(
        r#"
        mod adapters { pub struct Database; }
        mod application {
            #[cfg(test)] fn test_helper() { crate::adapters::Database::open(); }
            #[cfg(all(test, windows))] mod tests { fn open() { crate::adapters::Database::open(); } }
            #[cfg(any(test, windows))] fn production() { crate::adapters::Database::open(); }
            #[cfg(not(test))] fn release() { crate::adapters::Database::open(); }
            fn neutral() { #[cfg(test)] { crate::adapters::Database::open(); } }
            fn neutral_branch() { #[cfg(test)] if true { crate::adapters::Database::open(); } }
            fn conditional_branch() { #[cfg(any(test, windows))] if true { crate::adapters::Database::open(); } }
        }
    "#,
    );
    let symbols = collect_dependencies(&modules)
        .into_iter()
        .map(|dependency| dependency.symbol)
        .collect::<Vec<_>>();
    assert_eq!(
        symbols,
        [
            "crate::application::conditional_branch",
            "crate::application::production",
            "crate::application::release"
        ]
    );
}

#[test]
fn macro_paths_are_checked_without_reading_string_contents() {
    let modules = fixture(
        r#"
        mod adapters { pub struct Database; }
        mod application {
            use crate::adapters::Database as Store;
            fn open() { wrap!(Store::open()); }
            fn pure() { println!("crate::adapters::Database"); }
        }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 1);
    assert_eq!(
        dependencies.first().unwrap().symbol,
        "crate::application::open"
    );
}

#[test]
fn external_crate_exports_are_distinct_from_ordinary_variable_names() {
    let modules = fixture(
        r#"
        mod domain { fn values(image: u32, directories: u32) -> u32 { image + directories } }
        mod application { pub use image as decoder; }
        mod api { fn stream(value: crate::frb_generated::StreamSink<u32>) {} }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies.first().unwrap().target, "image");
}

#[test]
fn domain_ports_and_bridge_accept_only_their_direction() {
    let modules = fixture(
        r#"
        mod domain { pub(crate) struct Value; fn bad() { std::fs::read("x"); } }
        mod ports {
            trait Port { fn value() -> crate::domain::Value; }
            fn bad() { crate::application::run(); }
            pub use crate as root;
        }
        mod application { pub fn run() {} }
        mod api { fn good() { crate::application::run(); } fn bad() { rusqlite::Connection::open("x"); } }
    "#,
    );
    let rules = collect_dependencies(&modules)
        .into_iter()
        .map(|dependency| dependency.rule)
        .collect::<Vec<_>>();
    assert_eq!(
        rules,
        [
            "external-engine",
            "platform-io",
            "outward-layer",
            "outward-layer"
        ]
    );

    for native in [
        "adapters",
        "journal_broker",
        "windows_usn",
        "classification_worker",
    ] {
        let source = r#"
            mod native_boundary { pub const VERSION: u16 = 5; }
            mod facade { pub use crate::native_boundary::VERSION as Current; }
            mod domain { const BAD: u16 = crate::facade::Current; }
            mod ports { const BAD: u16 = crate::facade::Current; }
            mod application { fn bad() -> u16 { crate::facade::Current } }
            mod api { fn bad() -> u16 { crate::facade::Current } }
        "#;
        let dependencies =
            collect_dependencies(&fixture(&source.replace("native_boundary", native)));
        assert_eq!(dependencies.len(), 5, "{native}");
        for dependency in dependencies {
            assert_eq!(dependency.target, format!("crate::{native}::VERSION"));
            assert_eq!(
                dependency.rule,
                if matches!(
                    dependency.symbol.as_str(),
                    "crate::application::bad" | "crate::facade::export::Current"
                ) {
                    "concrete-adapter"
                } else {
                    "outward-layer"
                },
            );
        }
    }
}

#[test]
#[should_panic(expected = "Included Rust code needs explicit boundary admission")]
fn included_code_cannot_hide_unparsed_dependencies() {
    collect_dependencies(&fixture(r#"mod application { include!("unparsed.rs"); }"#));
}

#[test]
fn adapters_can_implement_ports_and_use_domain_rules_and_external_engines() {
    let source = r#"
        mod domain { pub fn validate() {} }
        mod ports { pub trait Repository { fn write(&self); } }
        mod native_boundary {
            struct Database { connection: rusqlite::Connection }
            impl crate::ports::Repository for Database {
                fn write(&self) { crate::domain::validate(); }
            }
        }
    "#;
    for native in [
        "adapters",
        "journal_broker",
        "windows_usn",
        "classification_worker",
    ] {
        let modules = fixture(&source.replace("native_boundary", native));
        assert!(collect_dependencies(&modules).is_empty(), "{native}");
    }
}

#[test]
fn adapters_cannot_reach_application_policy_through_aliases_or_reexports() {
    let source = r#"
        mod application { pub fn validate() {} }
        mod native_boundary {
            mod imports { pub use crate::application::validate as shared; }
            mod database { use super::imports::shared as check; fn write() { check(); } }
        }
    "#;
    for native in [
        "adapters",
        "journal_broker",
        "windows_usn",
        "classification_worker",
    ] {
        let modules = fixture(&source.replace("native_boundary", native));
        let dependencies = collect_dependencies(&modules);
        assert_eq!(dependencies.len(), 2, "{native}");
        assert!(dependencies.iter().all(|dependency| {
            dependency.target == "crate::application::validate"
                && dependency.rule == "outward-layer"
        }));
    }
}

#[test]
fn conditional_adapters_cannot_depend_on_application_or_bridge_entrypoints() {
    let modules = fixture(
        r#"
        mod application { pub fn run() {} }
        mod api { pub fn run() {} }
        mod adapters {
            #[cfg(test)] fn helper() { crate::application::run(); }
            #[cfg(any(test, windows))] fn native() { crate::application::run(); }
            fn bridge() { crate::api::run(); }
            fn generated() -> crate::frb_generated::StreamSink<u32> { todo!() }
        }
    "#,
    );
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 3);
    assert!(
        dependencies
            .iter()
            .all(|dependency| dependency.rule == "outward-layer")
    );
    assert!(
        dependencies
            .iter()
            .all(|dependency| !dependency.symbol.ends_with("::helper"))
    );
}

#[test]
fn production_loading_checks_root_facades_and_binary_crates() {
    let repository = tempfile::tempdir().unwrap();
    let source = repository.path().join("rust/src");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::create_dir(source.join("tools")).unwrap();
    std::fs::write(
        source.parent().unwrap().join("Cargo.toml"),
        r#"
        [package]
        name = "rust_lib_cedarflake_ame"
        version = "0.1.0"
        edition = "2024"
        [features]
        worker = []
        [[bin]]
        name = "native_worker"
        path = "src/tools/worker.rs"
        required-features = ["worker"]
    "#,
    )
    .unwrap();
    for (file, content) in [
        (
            "lib.rs",
            "mod application; mod adapters; mod classification; mod frb_generated; fn open() { crate::adapters::write(); }",
        ),
        (
            "application.rs",
            "pub fn validate() { crate::adapters::write(); }",
        ),
        (
            "classification.rs",
            "pub use crate::application::validate as shared;",
        ),
        (
            "adapters.rs",
            "fn write() { crate::classification::shared(); }",
        ),
        (
            "tools/worker.rs",
            r#"
                use rust_lib_cedarflake_ame::classification::shared as Check;
                pub use rust_lib_cedarflake_ame::classification::shared as PublicCheck;
                mod local;
                fn main() { crate::local::run(); Check(); }
            "#,
        ),
        ("tools/local.rs", "pub fn run() {}"),
    ] {
        std::fs::write(source.join(file), content).unwrap();
    }
    let modules = super::dependency_boundaries::load_modules(repository.path());
    assert!(modules.contains_key("binary::native_worker::local"));
    let dependencies = collect_dependencies(&modules);
    assert_eq!(dependencies.len(), 5);
    for (symbol, target, rule) in [
        ("crate::open", "crate::adapters::write", "concrete-adapter"),
        (
            "crate::application::validate",
            "crate::adapters::write",
            "concrete-adapter",
        ),
        (
            "crate::adapters::write",
            "crate::application::validate",
            "outward-layer",
        ),
        (
            "binary::native_worker::export::PublicCheck",
            "crate::application::validate",
            "outward-layer",
        ),
        (
            "binary::native_worker::main",
            "crate::application::validate",
            "outward-layer",
        ),
    ] {
        assert!(
            dependencies.iter().any(|dependency| {
                dependency.symbol == symbol
                    && dependency.target == target
                    && dependency.rule == rule
            }),
            "missing {symbol} -> {target} ({rule})"
        );
    }
}
