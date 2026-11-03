use std::collections::BTreeMap;
use std::path::Path;

use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, Item, Meta, Token};

pub struct Module {
    pub source: String,
    pub namespace: String,
    pub items: Vec<Item>,
    pub lines: usize,
}

pub fn is_test_only(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if attribute.path().is_ident("test") {
            return true;
        }
        if !attribute.path().is_ident("cfg") {
            return false;
        }
        attribute
            .parse_args::<Meta>()
            .is_ok_and(|predicate| without_tests(&predicate) == Possibility::Never)
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Possibility {
    Always,
    Never,
    Unknown,
}

fn without_tests(predicate: &Meta) -> Possibility {
    match predicate {
        Meta::Path(path) if path.is_ident("test") => Possibility::Never,
        Meta::Path(_) | Meta::NameValue(_) => Possibility::Unknown,
        Meta::List(list) => {
            let arguments = Punctuated::<Meta, Token![,]>::parse_terminated
                .parse2(list.tokens.clone())
                .expect("valid cfg predicate");
            let values = arguments.iter().map(without_tests).collect::<Vec<_>>();
            if list.path.is_ident("not") {
                assert_eq!(values.len(), 1, "not requires one cfg predicate");
                return match values[0] {
                    Possibility::Always => Possibility::Never,
                    Possibility::Never => Possibility::Always,
                    Possibility::Unknown => Possibility::Unknown,
                };
            }
            let (deciding, identity) = if list.path.is_ident("all") {
                (Possibility::Never, Possibility::Always)
            } else if list.path.is_ident("any") {
                (Possibility::Always, Possibility::Never)
            } else {
                return Possibility::Unknown;
            };
            if values.contains(&deciding) {
                deciding
            } else if values.contains(&Possibility::Unknown) {
                Possibility::Unknown
            } else {
                identity
            }
        }
    }
}

pub fn load_modules(repository: &Path) -> BTreeMap<String, Module> {
    let mut loader = Loader {
        repository,
        modules: BTreeMap::new(),
    };
    loader.load_file(&repository.join("rust/src/lib.rs"), "crate".to_owned());
    loader.modules
}

struct Loader<'a> {
    repository: &'a Path,
    modules: BTreeMap<String, Module>,
}

impl Loader<'_> {
    fn load_file(&mut self, path: &Path, namespace: String) {
        let text = std::fs::read_to_string(path).expect("read owned Rust source as UTF-8");
        let ast = syn::parse_file(&text).expect("parse owned Rust source");
        let parent = path.parent().expect("source parent");
        let directory = if matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some("lib.rs" | "mod.rs")
        ) {
            parent.to_owned()
        } else {
            path.with_extension("")
        };
        let source = path
            .strip_prefix(self.repository)
            .expect("repository-owned source")
            .to_str()
            .expect("UTF-8 source name")
            .replace('\\', "/");
        self.load_items(
            Module {
                source,
                namespace,
                items: ast.items,
                lines: text.lines().count(),
            },
            &directory,
            parent,
        );
    }

    fn load_items(&mut self, owner: Module, directory: &Path, path_base: &Path) {
        for item in &owner.items {
            let Item::Mod(module) = item else { continue };
            if is_test_only(&module.attrs) {
                continue;
            }
            let namespace = format!("{}::{}", owner.namespace, module.ident);
            if owner.namespace == "crate"
                && !matches!(
                    module.ident.to_string().as_str(),
                    "domain" | "ports" | "application" | "api" | "adapters"
                )
            {
                continue;
            }
            let nested_directory = directory.join(module.ident.to_string());
            if let Some((_, nested)) = &module.content {
                self.load_items(
                    Module {
                        source: owner.source.clone(),
                        namespace,
                        items: nested.clone(),
                        lines: owner.lines,
                    },
                    &nested_directory,
                    &nested_directory,
                );
                continue;
            }
            let custom_path = module
                .attrs
                .iter()
                .find(|attribute| attribute.path().is_ident("path"));
            let path = if let Some(attribute) = custom_path {
                let Meta::NameValue(value) = &attribute.meta else {
                    panic!("literal module path required")
                };
                let syn::Expr::Lit(value) = &value.value else {
                    panic!("literal module path required")
                };
                let syn::Lit::Str(value) = &value.lit else {
                    panic!("literal module path required")
                };
                let relative = Path::new(&value.value()).to_owned();
                assert!(
                    relative
                        .components()
                        .all(|part| matches!(part, std::path::Component::Normal(_))),
                    "Module paths must stay within owned source"
                );
                path_base.join(relative)
            } else {
                let direct = directory.join(format!("{}.rs", module.ident));
                if direct.is_file() {
                    direct
                } else {
                    nested_directory.join("mod.rs")
                }
            };
            self.load_file(&path, namespace);
        }
        assert!(
            self.modules
                .insert(owner.namespace.clone(), owner)
                .is_none(),
            "A production module namespace was declared twice"
        );
    }
}
