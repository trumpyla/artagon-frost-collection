//! Source-level discovery. Cargo supplies target roots; syn parses Rust items.
use crate::{Discovered, Json, rel};
use proc_macro2::{Delimiter, TokenStream, TokenTree};
use quote::ToTokens;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use syn::{Attribute, Item, Lit, Meta};

/// Runs `cargo metadata --no-deps` for one manifest under `root`.
fn metadata(root: &Path, manifest: &str) -> Result<Json, String> {
    let path = root.join(manifest);
    if !path.exists() {
        return Err(format!("required manifest missing: {manifest}"));
    }
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&path)
        .output()
        .map_err(|e| format!("cargo metadata {manifest}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata {manifest}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Json::parse(std::str::from_utf8(&output.stdout).map_err(|e| e.to_string())?)
}

/// Directories of the packages the root manifest names, relative to `root`.
/// A single-package root yields `[""]`; a virtual workspace yields one
/// directory per member.
pub(crate) fn member_dirs(root: &Path) -> Result<Vec<String>, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let value = metadata(&root, "Cargo.toml")?;
    let mut dirs = Vec::new();
    for package in value
        .get("packages")
        .and_then(Json::as_array)
        .ok_or("cargo metadata missing packages")?
    {
        let manifest = PathBuf::from(
            package
                .get("manifest_path")
                .and_then(Json::as_str)
                .ok_or("package manifest path missing")?,
        );
        let manifest = manifest
            .canonicalize()
            .map_err(|e| format!("{}: {e}", manifest.display()))?;
        let dir = manifest.parent().ok_or("manifest without parent")?;
        if !dir.starts_with(&root) {
            return Err(format!("package outside root: {}", dir.display()));
        }
        dirs.push(rel(&root, dir));
    }
    dirs.sort();
    Ok(dirs)
}

/// Discovers every target of the root manifest's packages and of each
/// isolated manifest. Paths in the result are relative to `root`.
pub(crate) fn discover(root: &Path, isolated: &[String]) -> Result<Vec<Discovered>, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let mut manifests = vec!["Cargo.toml".to_string()];
    manifests.extend(isolated.iter().cloned());
    let mut found = Vec::new();
    for manifest in &manifests {
        let value = metadata(&root, manifest)?;
        for package in value
            .get("packages")
            .and_then(Json::as_array)
            .ok_or("cargo metadata missing packages")?
        {
            discover_package(&root, package, &mut found)?;
        }
    }
    found.sort();
    found.dedup();
    Ok(found)
}

fn discover_package(
    root: &Path,
    package: &Json,
    found: &mut Vec<Discovered>,
) -> Result<(), String> {
    let available_features: Vec<String> = package
        .get("features")
        .and_then(Json::as_object)
        .unwrap_or_default()
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    for target in package
        .get("targets")
        .and_then(Json::as_array)
        .ok_or("cargo metadata missing targets")?
    {
        let kind = target
            .get("kind")
            .and_then(Json::as_array)
            .and_then(|a| a.first())
            .and_then(Json::as_str)
            .ok_or("target kind missing")?;
        if !["lib", "test", "bench", "example", "bin"].contains(&kind) {
            continue;
        }
        let path = PathBuf::from(
            target
                .get("src_path")
                .and_then(Json::as_str)
                .ok_or("target path missing")?,
        );
        let path = path
            .canonicalize()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !path.starts_with(root) {
            return Err(format!("target outside root: {}", path.display()));
        }
        let name = rel(root, &path);
        let features = target
            .get("required-features")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Json::as_str)
            .map(str::to_string)
            .collect();
        let target_source =
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let target_file =
            syn::parse_file(&target_source).map_err(|e| format!("{}: {e}", path.display()))?;
        let base = Discovered {
            target: name.clone(),
            file: name.clone(),
            selector: if kind == "bench" { "bench" } else { "target" }.to_string(),
            kind: kind.to_string(),
            features,
            available_features: available_features.clone(),
            cfg: cfgs(&target_file.attrs).join("; "),
        };
        if kind == "bench" {
            found.push(base.clone());
        } else {
            let before = found.len();
            let dir = path.parent().ok_or("target without parent")?.to_path_buf();
            let mut active = BTreeSet::new();
            scan_file(root, &path, &dir, &[], &[], &base, &mut active, found)?;
            if before == found.len() {
                found.push(base.clone());
            }
        }
    }
    Ok(())
}

fn cfgs(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
        .map(|a| a.meta.to_token_stream().to_string())
        .collect()
}

#[allow(
    clippy::too_many_arguments,
    reason = "recursive module traversal passes explicit source and target context"
)]
fn scan_file(
    root: &Path,
    file: &Path,
    dir: &Path,
    modules: &[String],
    conditions: &[String],
    target: &Discovered,
    active: &mut BTreeSet<PathBuf>,
    out: &mut Vec<Discovered>,
) -> Result<(), String> {
    let file = file
        .canonicalize()
        .map_err(|e| format!("{}: {e}", file.display()))?;
    if !file.starts_with(root) {
        return Err(format!("module outside root: {}", file.display()));
    }
    if !active.insert(file.clone()) {
        return Err(format!("module cycle: {}", file.display()));
    }
    let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let parsed = syn::parse_file(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut inherited = conditions.to_vec();
    inherited.extend(cfgs(&parsed.attrs));
    scan_items(
        root,
        &file,
        dir,
        modules,
        &inherited,
        target,
        active,
        out,
        &parsed.items,
    )?;
    active.remove(&file);
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "recursive module traversal passes explicit source and target context"
)]
fn scan_items(
    root: &Path,
    file: &Path,
    dir: &Path,
    modules: &[String],
    conditions: &[String],
    target: &Discovered,
    active: &mut BTreeSet<PathBuf>,
    out: &mut Vec<Discovered>,
    items: &[Item],
) -> Result<(), String> {
    for item in items {
        match item {
            Item::Fn(f)
                if f.attrs.iter().any(|a| {
                    a.path().is_ident("test")
                        || a.path()
                            .segments
                            .iter()
                            .map(|s| s.ident.to_string())
                            .collect::<Vec<_>>()
                            == ["kani", "proof"]
                }) =>
            {
                let mut names = modules.to_vec();
                names.push(f.sig.ident.to_string());
                let mut all = conditions.to_vec();
                all.extend(cfgs(&f.attrs));
                let mut d = target.clone();
                d.file = rel(root, file);
                d.selector = format!("test:{}", names.join("::"));
                d.cfg = all.join("; ");
                out.push(d);
            }
            Item::Mod(m) => {
                let mut names = modules.to_vec();
                names.push(m.ident.to_string());
                let mut all = conditions.to_vec();
                all.extend(cfgs(&m.attrs));
                if let Some((_, nested)) = &m.content {
                    scan_items(
                        root,
                        file,
                        &dir.join(m.ident.to_string()),
                        &names,
                        &all,
                        target,
                        active,
                        out,
                        nested,
                    )?;
                } else {
                    let explicit = m.attrs.iter().find_map(|a| match &a.meta {
                        Meta::NameValue(nv) if nv.path.is_ident("path") => match &nv.value {
                            syn::Expr::Lit(e) => match &e.lit {
                                Lit::Str(s) => Some(s.value()),
                                _ => None,
                            },
                            _ => None,
                        },
                        _ => None,
                    });
                    let module = if let Some(p) = explicit {
                        // A path attribute in an external `foo.rs` module is
                        // relative to that source file, not its child-module
                        // search directory `foo/`.
                        if dir == file.with_extension("") {
                            file.parent().ok_or("module without parent")?.join(p)
                        } else {
                            dir.join(p)
                        }
                    } else {
                        let flat = dir.join(format!("{}.rs", m.ident));
                        if flat.exists() {
                            flat
                        } else {
                            dir.join(m.ident.to_string()).join("mod.rs")
                        }
                    };
                    let next_dir = if module.file_name().is_some_and(|n| n == "mod.rs") {
                        module.parent().unwrap().to_path_buf()
                    } else {
                        module.with_extension("")
                    };
                    scan_file(root, &module, &next_dir, &names, &all, target, active, out)?;
                }
            }
            Item::Macro(m)
                if m.mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "proptest" || s.ident == "prop_state_machine") =>
            {
                for (name, local_cfg) in proptest_items(m.mac.tokens.clone()) {
                    let mut names = modules.to_vec();
                    names.push(name);
                    let mut all = conditions.to_vec();
                    all.extend(cfgs(&m.attrs));
                    all.extend(local_cfg);
                    let mut d = target.clone();
                    d.file = rel(root, file);
                    d.selector = format!("test:{}", names.join("::"));
                    d.cfg = all.join("; ");
                    out.push(d);
                }
            }
            Item::Macro(m)
                if m.mac
                    .path
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "fuzz_target") =>
            {
                let mut all = conditions.to_vec();
                all.extend(cfgs(&m.attrs));
                let mut d = target.clone();
                d.file = rel(root, file);
                d.selector = "fuzz:fuzz_target".into();
                d.cfg = all.join("; ");
                out.push(d);
            }
            Item::Macro(m) if m.mac.path.is_ident("macro_rules") && m.ident.is_some() => {
                let macro_name = m.ident.as_ref().unwrap().to_string();
                for name in template_names(m.mac.tokens.clone()) {
                    let mut names = modules.to_vec();
                    names.push(macro_name.clone());
                    names.push(name);
                    let mut all = conditions.to_vec();
                    all.extend(cfgs(&m.attrs));
                    let mut d = target.clone();
                    d.file = rel(root, file);
                    d.selector = format!("macro:{}", names.join("::"));
                    d.cfg = all.join("; ");
                    out.push(d);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// Proptest's `name(arg in strategy)` and proptest-state-machine's
// `name(sequential 1..n => Test)` are not Rust function signatures. Inspect
// top-level macro tokens, never strings/comments or arbitrary macro expansions.
fn proptest_items(tokens: TokenStream) -> Vec<(String, Vec<String>)> {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    let mut result = Vec::new();
    let mut pending = false;
    let mut conditions = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if matches!(&tokens[i], TokenTree::Punct(p) if p.as_char() == '#') {
            if let Some(TokenTree::Group(g)) = tokens.get(i + 1) {
                if g.delimiter() == Delimiter::Bracket {
                    let value = g.stream().to_string();
                    if value == "test" {
                        pending = true;
                    }
                    if value.starts_with("cfg (") || value.starts_with("cfg_attr (") {
                        conditions.push(value);
                    }
                }
            }
        }
        if pending && matches!(&tokens[i], TokenTree::Ident(id) if id == "fn") {
            if let Some(TokenTree::Ident(id)) = tokens.get(i + 1) {
                result.push((id.to_string(), std::mem::take(&mut conditions)));
                pending = false;
            }
        }
        i += 1;
    }
    result
}

fn template_names(tokens: TokenStream) -> Vec<String> {
    let mut result: Vec<String> = proptest_items(tokens.clone())
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    for token in tokens {
        if let TokenTree::Group(group) = token {
            result.extend(template_names(group.stream()));
        }
    }
    result.sort();
    result.dedup();
    result
}
