use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use serde::{Deserialize, Serialize};
use syn::buffer::{Cursor, TokenBuffer};
use syn::visit::{self, Visit};
use syn::{Attribute, Item, Path, Visibility};

use super::names::{Names, import_bindings};
use super::sources::{Module, is_test_only};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Dependency {
    pub source: String,
    pub symbol: String,
    pub target: String,
    pub rule: String,
}

pub fn collect_dependencies(modules: &BTreeMap<String, Module>) -> BTreeSet<Dependency> {
    let names = Names::from_modules(modules);
    let mut dependencies = BTreeSet::new();
    for module in modules.values() {
        let layer = module.namespace.split("::").nth(1).unwrap_or_default();
        if !matches!(layer, "domain" | "ports" | "application" | "api") {
            continue;
        }
        let mut collector = Collector {
            module,
            names: &names,
            dependencies: &mut dependencies,
            symbol: module.namespace.clone(),
            impl_owner: String::new(),
            locals: BTreeMap::new(),
        };
        for item in &module.items {
            collector.visit_item(item);
        }
    }
    dependencies
}

struct Collector<'a> {
    module: &'a Module,
    names: &'a Names,
    dependencies: &'a mut BTreeSet<Dependency>,
    symbol: String,
    impl_owner: String,
    locals: BTreeMap<String, String>,
}

fn forbidden_rule(
    namespace: &str,
    target: &str,
    is_module_reference: bool,
) -> Option<&'static str> {
    let layer = namespace.split("::").nth(1)?;
    if layer == "api" && target == "crate::frb_generated::StreamSink" {
        return None;
    }
    let target_layer = target
        .strip_prefix("crate::")
        .and_then(|path| path.split("::").next());
    let outward = match layer {
        "domain" => matches!(
            target_layer,
            Some("ports" | "application" | "adapters" | "api" | "frb_generated")
        ),
        "ports" => matches!(
            target_layer,
            Some("application" | "adapters" | "api" | "frb_generated")
        ),
        "application" => matches!(target_layer, Some("api" | "frb_generated")),
        "api" => matches!(target_layer, Some("adapters" | "frb_generated")),
        _ => false,
    };
    if outward {
        return Some("outward-layer");
    }
    if layer == "application" && target_layer == Some("adapters") {
        return Some("concrete-adapter");
    }
    let external = target.split("::").next().unwrap_or_default();
    if (target.contains("::") || is_module_reference)
        && matches!(
            external,
            "rusqlite"
                | "image"
                | "exif"
                | "jpeg_decoder"
                | "zune_jpeg"
                | "zune_core"
                | "ort"
                | "tokenizers"
                | "reqwest"
                | "windows"
                | "windows_sys"
                | "notify"
                | "directories"
        )
    {
        return Some("external-engine");
    }
    if layer != "api" && external == "flutter_rust_bridge" {
        return Some("bridge-runtime");
    }
    if matches!(layer, "domain" | "ports")
        && ["std::fs", "std::process", "std::net"]
            .iter()
            .any(|prefix| target == *prefix || target.starts_with(&format!("{prefix}::")))
    {
        return Some("platform-io");
    }
    None
}

impl Collector<'_> {
    fn record(&mut self, path: &str, is_module_reference: bool) {
        for target in self
            .names
            .resolve(&self.module.namespace, path, &self.locals)
        {
            if let Some(rule) = forbidden_rule(&self.module.namespace, &target, is_module_reference)
            {
                self.dependencies.insert(Dependency {
                    source: self.module.source.clone(),
                    symbol: self.symbol.clone(),
                    target: self.names.dependency_owner(&target),
                    rule: rule.to_owned(),
                });
            }
        }
    }

    fn tokens(&mut self, mut cursor: Cursor<'_>) {
        while !cursor.eof() {
            if let Some((inside, _, _, after)) = cursor.any_group() {
                self.tokens(inside);
                cursor = after;
                continue;
            }
            if let Some((ident, mut after)) = cursor.ident() {
                let mut path = ident.to_string();
                while let Some((first, next)) = after.punct() {
                    if first.as_char() != ':' {
                        break;
                    }
                    let Some((second, next)) = next.punct() else {
                        break;
                    };
                    if second.as_char() != ':' {
                        break;
                    }
                    let Some((part, next)) = next.ident() else {
                        break;
                    };
                    path.push_str("::");
                    path.push_str(&part.to_string());
                    after = next;
                }
                self.record(&path, false);
                cursor = after;
            } else if let Some((_, after)) = cursor.token_tree() {
                cursor = after;
            } else {
                break;
            }
        }
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        let (name, attrs): (String, &[Attribute]) = match item {
            Item::Fn(value) => (value.sig.ident.to_string(), &value.attrs),
            Item::Struct(value) => (value.ident.to_string(), &value.attrs),
            Item::Enum(value) => (value.ident.to_string(), &value.attrs),
            Item::Type(value) => (value.ident.to_string(), &value.attrs),
            Item::Trait(value) => (value.ident.to_string(), &value.attrs),
            Item::Union(value) => (value.ident.to_string(), &value.attrs),
            Item::Static(value) => (value.ident.to_string(), &value.attrs),
            Item::Const(value) => (value.ident.to_string(), &value.attrs),
            Item::Impl(value) => (
                value.self_ty.to_token_stream().to_string().replace(' ', ""),
                &value.attrs,
            ),
            Item::Macro(value) => (
                value
                    .ident
                    .as_ref()
                    .map_or_else(|| "macro".to_owned(), ToString::to_string),
                &value.attrs,
            ),
            Item::Use(value) => {
                if is_test_only(&value.attrs) {
                    return;
                }
                if !matches!(value.vis, Visibility::Inherited) {
                    let previous = self.symbol.clone();
                    for (alias, target) in import_bindings(&self.module.namespace, &value.tree) {
                        self.symbol = format!("{}::export::{alias}", self.module.namespace);
                        self.record(&target, true);
                    }
                    self.symbol = previous;
                }
                return;
            }
            Item::Mod(_) => return,
            _ => return,
        };
        if is_test_only(attrs) {
            return;
        }
        let previous = std::mem::replace(
            &mut self.symbol,
            format!("{}::{name}", self.module.namespace),
        );
        visit::visit_item(self, item);
        self.symbol = previous;
    }

    fn visit_item_impl(&mut self, value: &'ast syn::ItemImpl) {
        let previous = std::mem::replace(&mut self.impl_owner, self.symbol.clone());
        visit::visit_item_impl(self, value);
        self.impl_owner = previous;
    }

    fn visit_impl_item_fn(&mut self, value: &'ast syn::ImplItemFn) {
        if is_test_only(&value.attrs) {
            return;
        }
        let previous = std::mem::replace(
            &mut self.symbol,
            format!("{}::{}", self.impl_owner, value.sig.ident),
        );
        visit::visit_impl_item_fn(self, value);
        self.symbol = previous;
    }

    fn visit_trait_item_fn(&mut self, value: &'ast syn::TraitItemFn) {
        if is_test_only(&value.attrs) {
            return;
        }
        let previous = self.symbol.clone();
        self.symbol = format!("{previous}::{}", value.sig.ident);
        visit::visit_trait_item_fn(self, value);
        self.symbol = previous;
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let previous = self.locals.clone();
        for statement in &block.stmts {
            if let syn::Stmt::Item(Item::Use(value)) = statement {
                if is_test_only(&value.attrs) {
                    continue;
                }
                for (alias, target) in import_bindings(&self.module.namespace, &value.tree) {
                    assert_ne!(
                        alias, "*",
                        "Block-local glob imports need an explicit boundary owner: {}",
                        self.symbol
                    );
                    self.locals.insert(alias, target);
                }
            }
        }
        visit::visit_block(self, block);
        self.locals = previous;
    }

    fn visit_local(&mut self, value: &'ast syn::Local) {
        if !is_test_only(&value.attrs) {
            visit::visit_local(self, value);
        }
    }

    fn visit_expr_block(&mut self, value: &'ast syn::ExprBlock) {
        if !is_test_only(&value.attrs) {
            visit::visit_expr_block(self, value);
        }
    }

    fn visit_path(&mut self, path: &'ast Path) {
        let path_text = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        self.record(&path_text, false);
        visit::visit_path(self, path);
    }

    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if !attribute.path().is_ident("cfg") && !attribute.path().is_ident("cfg_attr") {
            let buffer = TokenBuffer::new2(attribute.meta.to_token_stream());
            self.tokens(buffer.begin());
        }
    }

    fn visit_macro(&mut self, value: &'ast syn::Macro) {
        assert!(
            !value.path.is_ident("include"),
            "Included Rust code needs explicit boundary admission: {}",
            self.symbol
        );
        self.visit_path(&value.path);
        let buffer = TokenBuffer::new2(value.tokens.clone());
        self.tokens(buffer.begin());
    }
}
