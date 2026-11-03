use std::collections::{BTreeMap, BTreeSet};

use syn::{Item, UseTree};

use super::sources::{Module, is_test_only};

#[derive(Default)]
pub struct Names {
    aliases: BTreeMap<String, BTreeSet<String>>,
    globs: BTreeMap<String, BTreeSet<String>>,
    declarations: BTreeSet<String>,
    modules: BTreeSet<String>,
}

pub fn import_bindings(namespace: &str, tree: &UseTree) -> Vec<(String, String)> {
    fn flatten(prefix: Vec<String>, tree: &UseTree, result: &mut Vec<(String, String)>) {
        match tree {
            UseTree::Path(path) => {
                let mut nested = prefix;
                nested.push(path.ident.to_string());
                flatten(nested, &path.tree, result);
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    flatten(prefix.clone(), item, result);
                }
            }
            UseTree::Name(name) => {
                let mut target = prefix;
                if name.ident != "self" {
                    target.push(name.ident.to_string());
                }
                result.push((
                    target.last().expect("import name").clone(),
                    target.join("::"),
                ));
            }
            UseTree::Rename(rename) => {
                let mut target = prefix;
                if rename.ident != "self" {
                    target.push(rename.ident.to_string());
                }
                result.push((rename.rename.to_string(), target.join("::")));
            }
            UseTree::Glob(_) => result.push(("*".to_owned(), prefix.join("::"))),
        }
    }
    let mut result = Vec::new();
    flatten(Vec::new(), tree, &mut result);
    result
        .into_iter()
        .map(|(alias, path)| (alias, source_path(namespace, &path)))
        .collect()
}

fn source_path(namespace: &str, path: &str) -> String {
    if let Some(binary) = namespace.strip_prefix("binary::") {
        if let Some(tail) = path.strip_prefix("rust_lib_cedarflake_ame::") {
            return format!("crate::{tail}");
        }
        if path == "rust_lib_cedarflake_ame" {
            return "crate".to_owned();
        }
        let root = binary.split("::").next().expect("binary crate name");
        if let Some(tail) = path.strip_prefix("crate::") {
            return format!("binary::{root}::{tail}");
        }
        if path == "crate" {
            return format!("binary::{root}");
        }
    }
    anchor(namespace, path)
}

fn anchor(namespace: &str, path: &str) -> String {
    if path == "crate" || path.starts_with("crate::") {
        return path.to_owned();
    }
    let mut base = namespace.split("::").collect::<Vec<_>>();
    let mut parts = path.split("::").peekable();
    if parts.peek() == Some(&"self") {
        parts.next();
    } else if parts.peek() != Some(&"super") {
        return path.to_owned();
    }
    while parts.peek() == Some(&"super") {
        parts.next();
        let root_depth = if namespace.starts_with("binary::") {
            2
        } else {
            1
        };
        assert!(base.len() > root_depth, "super escaped the crate root");
        base.pop();
    }
    base.extend(parts);
    base.join("::")
}

impl Names {
    pub fn from_modules(modules: &BTreeMap<String, Module>) -> Self {
        let mut names = Self::default();
        for module in modules.values() {
            names.modules.insert(module.namespace.clone());
            for item in &module.items {
                let (ident, attrs) = match item {
                    Item::Fn(value) => (Some(&value.sig.ident), &value.attrs),
                    Item::Struct(value) => (Some(&value.ident), &value.attrs),
                    Item::Enum(value) => (Some(&value.ident), &value.attrs),
                    Item::Type(value) => (Some(&value.ident), &value.attrs),
                    Item::Trait(value) => (Some(&value.ident), &value.attrs),
                    Item::Const(value) => (Some(&value.ident), &value.attrs),
                    Item::Static(value) => (Some(&value.ident), &value.attrs),
                    Item::Union(value) => (Some(&value.ident), &value.attrs),
                    Item::Mod(_) => continue,
                    Item::Use(value) => {
                        if !is_test_only(&value.attrs) {
                            for (alias, target) in import_bindings(&module.namespace, &value.tree) {
                                if alias == "*" {
                                    names
                                        .globs
                                        .entry(module.namespace.clone())
                                        .or_default()
                                        .insert(target);
                                } else {
                                    names
                                        .aliases
                                        .entry(format!("{}::{alias}", module.namespace))
                                        .or_default()
                                        .insert(target);
                                }
                            }
                        }
                        continue;
                    }
                    _ => continue,
                };
                if !is_test_only(attrs)
                    && let Some(ident) = ident
                {
                    names
                        .declarations
                        .insert(format!("{}::{ident}", module.namespace));
                }
            }
        }
        names
    }

    pub fn resolve(
        &self,
        namespace: &str,
        path: &str,
        locals: &BTreeMap<String, String>,
    ) -> BTreeSet<String> {
        self.resolve_anchored(namespace, &source_path(namespace, path), locals)
    }

    pub fn resolve_export(&self, namespace: &str, target: &str) -> BTreeSet<String> {
        self.resolve_anchored(namespace, target, &BTreeMap::new())
    }

    fn resolve_anchored(
        &self,
        namespace: &str,
        anchored: &str,
        locals: &BTreeMap<String, String>,
    ) -> BTreeSet<String> {
        let (first, suffix) = anchored.split_once("::").unwrap_or((anchored, ""));
        let tail = if suffix.is_empty() {
            String::new()
        } else {
            format!("::{suffix}")
        };
        if let Some(target) = locals.get(first) {
            return self.expand(namespace, &format!("{target}{tail}"), &mut BTreeSet::new());
        }
        if first != "crate" {
            let qualified = format!("{namespace}::{first}");
            if self.aliases.contains_key(&qualified)
                || self.declarations.contains(&qualified)
                || self.modules.contains(&qualified)
            {
                return self.expand(
                    namespace,
                    &format!("{qualified}{tail}"),
                    &mut BTreeSet::new(),
                );
            }
            let mut from_globs = BTreeSet::new();
            for glob in self.globs.get(namespace).into_iter().flatten() {
                let target = anchor(namespace, glob);
                let candidate = format!("{target}::{anchored}");
                if self.known(&candidate) {
                    from_globs.extend(self.expand(namespace, &candidate, &mut BTreeSet::new()));
                }
            }
            if !from_globs.is_empty() {
                return from_globs;
            }
        }
        self.expand(namespace, anchored, &mut BTreeSet::new())
    }

    fn known(&self, path: &str) -> bool {
        self.modules.contains(path)
            || self.declarations.contains(path)
            || self.aliases.contains_key(path)
            || self
                .declarations
                .iter()
                .chain(self.aliases.keys())
                .any(|declaration| path.starts_with(&format!("{declaration}::")))
    }

    fn expand(
        &self,
        namespace: &str,
        path: &str,
        visited: &mut BTreeSet<String>,
    ) -> BTreeSet<String> {
        if !visited.insert(path.to_owned()) {
            return BTreeSet::new();
        }
        let segments = path.split("::").collect::<Vec<_>>();
        for length in (1..=segments.len()).rev() {
            let prefix = segments[..length].join("::");
            if length < segments.len() && self.modules.contains(&prefix) {
                continue;
            }
            if let Some(targets) = self.aliases.get(&prefix) {
                if !visited.insert(format!("alias:{prefix}")) {
                    return BTreeSet::new();
                }
                let parent = prefix
                    .rsplit_once("::")
                    .map_or(namespace, |(parent, _)| parent);
                let mut result = BTreeSet::new();
                for target in targets {
                    let mut target = anchor(parent, target);
                    if !target.contains("::") || !target.starts_with("crate::") {
                        let local = format!("{parent}::{target}");
                        if self.known(&local) || self.aliases.contains_key(&local) {
                            target = local;
                        }
                    }
                    if length < segments.len() {
                        target.push_str(&format!("::{}", segments[length..].join("::")));
                    }
                    result.extend(self.expand(parent, &target, &mut visited.clone()));
                }
                return result;
            }
        }
        for length in (1..segments.len()).rev() {
            let prefix = segments[..length].join("::");
            let member = segments[length..].join("::");
            let mut result = BTreeSet::new();
            for glob in self.globs.get(&prefix).into_iter().flatten() {
                let candidate = format!("{glob}::{member}");
                if self.known(&candidate) {
                    result.extend(self.expand(&prefix, &candidate, &mut visited.clone()));
                }
            }
            if !result.is_empty() {
                return result;
            }
        }
        BTreeSet::from([path.to_owned()])
    }

    pub fn dependency_owner(&self, path: &str) -> String {
        self.declarations
            .iter()
            .filter(|declaration| {
                path == declaration.as_str() || path.starts_with(&format!("{declaration}::"))
            })
            .max_by_key(|declaration| declaration.len())
            .cloned()
            .unwrap_or_else(|| path.to_owned())
    }
}
