//! Discovery and validation of the Frost test/benchmark metadata catalog.
//!
//! The catalog is the JSON sidecar `verification/metadata/catalog.json`. This
//! tool discovers every `#[test]` function under `tests/` and every benchmark
//! target under `benches/`, then checks that the catalog is complete and
//! internally consistent. It reports catalog counts separately from API
//! obligation coverage, which this tool does not measure: a cataloged case is
//! a traceability link, not evidence that an obligation is exercised.
//!
//! ```text
//! cargo run --locked --manifest-path verification/catalog/Cargo.toml -- check --root .
//! cargo run --locked --manifest-path verification/catalog/Cargo.toml -- scaffold --root .
//! ```

mod bench;
mod discovery;
mod json;

use json::Json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SCHEMA: &str = "frost-verification-catalog/1";
const CATALOG_PATH: &str = "verification/metadata/catalog.json";
const CASE_FIELDS: [&str; 16] = [
    "id",
    "file",
    "selector",
    "status",
    "kind",
    "tags",
    "features",
    "area",
    "provenance",
    "contracts",
    "oracle",
    "scenario",
    "seed",
    "target",
    "cfg",
    "cost",
];
const CASE_OPTIONAL_FIELDS: [&str; 2] = ["timing", "control"];
/// `curated`: semantics reviewed by a person. `unreviewed`: structural
/// metadata only; no semantic coverage claim.
const STATUSES: [&str; 2] = ["curated", "unreviewed"];
const TAGS: [&str; 24] = [
    "ordered",
    "hash",
    "vec",
    "adaptive",
    "inline",
    "integer",
    "bitmap",
    "roaring",
    "concurrent",
    "snapshot",
    "adapters",
    "backends",
    "freeze-thaw",
    "set-algebra",
    "map-entry",
    "iterator",
    "ownership",
    "factory",
    "catalog",
    "native-control",
    "integration",
    "benchmark",
    "root-contract",
    "unreviewed",
];
const AREAS: [&str; 8] = [
    "family",
    "adaptive",
    "integer",
    "backends",
    "adapters",
    "concurrent",
    "verification",
    "support",
];
const TIMINGS: [&str; 3] = ["operation", "destructive-with-setup", "aggregate-readers"];
const KINDS: [&str; 10] = [
    "differential",
    "exhaustive",
    "property",
    "regression",
    "model",
    "ownership",
    "contract",
    "benchmark",
    "example",
    "unreviewed",
];
const SEEDS: [&str; 5] = ["none", "fixed", "proptest", "factory", "unreviewed"];
const COSTS: [&str; 5] = ["fast", "bounded", "slow", "timed", "unreviewed"];

/// One discovered test function or benchmark target.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Discovered {
    /// Top-level Cargo target file, for example `tests/families.rs`.
    target: String,
    /// Source file that holds the selector (a module file for `#[path]` modules).
    file: String,
    selector: String,
    kind: String,
    features: Vec<String>,
    available_features: Vec<String>,
    cfg: String,
}

#[derive(Default)]
struct Report {
    errors: Vec<String>,
    discovered_tests: usize,
    discovered_benches: usize,
    cases: usize,
    curated: usize,
    scaffolded: usize,
    /// Measured benchmark cases registered in the inventory sidecar.
    measured_cases: usize,
    contracts: usize,
    contracts_without_curated_cases: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, root, extra) = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };
    match command.as_str() {
        "inventory" => {
            let [target, file] = extra.as_slice() else {
                eprintln!(
                    "usage: frost-catalog inventory <bench target path> <FROST_BENCH_LIST output file> [--root DIR]"
                );
                return ExitCode::from(2);
            };
            match inventory(&root, target, Path::new(file)) {
                Ok(errors) if errors.is_empty() => {
                    println!(
                        "inventory {target}: executed cases match {} exactly",
                        bench::SIDECAR_PATH
                    );
                    ExitCode::SUCCESS
                }
                Ok(errors) => {
                    for error in &errors {
                        println!("error: {error}");
                    }
                    println!("inventory {target}: {} error(s)", errors.len());
                    ExitCode::from(1)
                }
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::from(1)
                }
            }
        }
        "inventory-scaffold" => {
            let [target, file] = extra.as_slice() else {
                eprintln!(
                    "usage: frost-catalog inventory-scaffold <bench target path> <FROST_BENCH_LIST output file> [--root DIR]"
                );
                return ExitCode::from(2);
            };
            match read_listing(Path::new(file))
                .and_then(|executed| bench::scaffold(&root, target, &executed))
            {
                Ok(added) => {
                    println!(
                        "scaffolded {added} measured cases for {target} into {}",
                        bench::SIDECAR_PATH
                    );
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("{message}");
                    ExitCode::from(1)
                }
            }
        }
        "check" => {
            let report = check(&root);
            print_report(&report);
            if report.errors.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        "scaffold" => match scaffold(&root) {
            Ok(added) => {
                println!("scaffolded {added} new entries into {CATALOG_PATH}");
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("{message}");
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!(
                "usage: frost-catalog <check|scaffold|inventory <target> <file>|inventory-scaffold <target> <file>> [--root DIR]"
            );
            ExitCode::from(2)
        }
    }
}

fn parse_args(args: &[String]) -> Result<(String, PathBuf, Vec<String>), String> {
    let mut command = None;
    let mut root = PathBuf::from(".");
    let mut extra = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--root" {
            root = PathBuf::from(iter.next().ok_or("--root needs a directory")?);
        } else if command.is_none() {
            command = Some(arg.clone());
        } else {
            extra.push(arg.clone());
        }
    }
    Ok((command.unwrap_or_else(|| "check".to_string()), root, extra))
}

/// Read and parse a `FROST_BENCH_LIST=1` listing file.
fn read_listing(file: &Path) -> Result<Vec<bench::Executed>, String> {
    let text = fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    bench::parse_listing(&text, &TIMINGS).map_err(|errors| errors.join("\n"))
}

/// Exact comparison of one executed listing with the bench inventory sidecar.
fn inventory(root: &Path, target: &str, file: &Path) -> Result<Vec<String>, String> {
    let executed = read_listing(file)?;
    let sidecar = bench::load_sidecar(root)?;
    Ok(bench::compare(&sidecar, target, &executed))
}

fn print_report(report: &Report) {
    println!("discovered test functions: {}", report.discovered_tests);
    println!(
        "discovered benchmark targets: {}",
        report.discovered_benches
    );
    println!(
        "catalog cases: {} (curated {}, scaffolded {})",
        report.cases, report.curated, report.scaffolded
    );
    println!(
        "registered measured benchmark cases: {} (structure only; run `inventory` per target with a fresh FROST_BENCH_LIST listing for exact execution agreement)",
        report.measured_cases
    );
    println!("contracts: {}", report.contracts);
    if report.contracts_without_curated_cases.is_empty() {
        println!("contracts without a curated case: none");
    } else {
        println!(
            "contracts without a curated case ({}): {}",
            report.contracts_without_curated_cases.len(),
            report.contracts_without_curated_cases.join(", ")
        );
    }
    println!(
        "API obligation coverage: UNMEASURED (catalog linkage only; execution and semantic coverage need separate evidence)"
    );
    for error in &report.errors {
        println!("error: {error}");
    }
    if report.errors.is_empty() {
        println!("catalog: ok");
    } else {
        println!("catalog: {} error(s)", report.errors.len());
    }
}

// ---------------------------------------------------------------- discovery

fn discover(root: &Path) -> Result<Vec<Discovered>, String> {
    discovery::discover(root)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

// --------------------------------------------------------------- validation

fn check(root: &Path) -> Report {
    let mut report = Report::default();
    let discovered = match discover(root) {
        Ok(found) => found,
        Err(message) => {
            report.errors.push(message);
            return report;
        }
    };
    report.discovered_tests = discovered
        .iter()
        .filter(|d| d.selector.starts_with("test:"))
        .count();
    report.discovered_benches = discovered.iter().filter(|d| d.selector == "bench").count();
    let catalog_path = root.join(CATALOG_PATH);
    let catalog = match fs::read_to_string(&catalog_path)
        .map_err(|e| format!("{CATALOG_PATH}: {e}"))
        .and_then(|text| Json::parse(&text))
    {
        Ok(catalog) => catalog,
        Err(message) => {
            report.errors.push(message);
            return report;
        }
    };
    validate(root, &catalog, &discovered, &mut report);
    // Structural check of the benchmark inventory sidecar: every discovered
    // bench target must register its measured cases with valid metadata.
    // Whether those names match what actually runs is checked per target by
    // `inventory`, which needs a fresh `FROST_BENCH_LIST=1` listing.
    let bench_targets: BTreeSet<String> = discovered
        .iter()
        .filter(|d| d.selector == "bench")
        .map(|d| d.target.clone())
        .collect();
    match bench::load_sidecar(root) {
        Ok(sidecar) => {
            report.measured_cases = bench_targets
                .iter()
                .map(|t| {
                    sidecar
                        .get("targets")
                        .and_then(|s| s.get(t))
                        .and_then(Json::as_array)
                        .map_or(0, <[Json]>::len)
                })
                .sum();
            report.errors.extend(bench::validate_sidecar(
                &sidecar,
                &bench_targets,
                &TIMINGS,
                &STATUSES,
                &TAGS,
            ));
            let contract_ids: BTreeSet<&str> = catalog
                .get("contracts")
                .and_then(Json::as_array)
                .unwrap_or_default()
                .iter()
                .filter_map(|c| c.get("id").and_then(Json::as_str))
                .collect();
            for target in &bench_targets {
                let Some(discovery) = discovered.iter().find(|d| &d.target == target) else {
                    continue;
                };
                for entry in sidecar
                    .get("targets")
                    .and_then(|t| t.get(target))
                    .and_then(Json::as_array)
                    .unwrap_or_default()
                {
                    if let Some(features) = entry.get("features").and_then(Json::as_array) {
                        if features.iter().any(|f| {
                            !f.as_str().is_some_and(|s| {
                                discovery.available_features.iter().any(|known| known == s)
                            })
                        }) {
                            report
                                .errors
                                .push(format!("bench inventory {target}: unknown feature"));
                        }
                    }
                    if let Some(contracts) = entry.get("contracts").and_then(Json::as_array) {
                        if contracts
                            .iter()
                            .any(|c| !c.as_str().is_some_and(|s| contract_ids.contains(s)))
                        {
                            report
                                .errors
                                .push(format!("bench inventory {target}: unknown contract"));
                        }
                    }
                }
            }
        }
        Err(message) => report.errors.push(message),
    }
    report
}

fn validate(root: &Path, catalog: &Json, discovered: &[Discovered], report: &mut Report) {
    let errors = &mut report.errors;
    if catalog.get("schema").and_then(Json::as_str) != Some(SCHEMA) {
        errors.push(format!("schema must be {SCHEMA:?}"));
    }
    for key in ["contracts", "targets", "cases"] {
        if catalog.get(key).and_then(Json::as_array).is_none() {
            errors.push(format!("{key} must be an array"));
        }
    }
    for (key, _) in catalog.as_object().unwrap_or_default() {
        if !["schema", "contracts", "targets", "cases"].contains(&key.as_str()) {
            errors.push(format!("unknown top-level field {key:?}"));
        }
    }

    // Contracts identify obligations; resolving their API semantics is out of scope.
    let mut contract_ids = BTreeSet::new();
    for contract in catalog
        .get("contracts")
        .and_then(Json::as_array)
        .unwrap_or_default()
    {
        let Some(id) = contract.get("id").and_then(Json::as_str) else {
            errors.push("contract without string id".to_string());
            continue;
        };
        if !id.starts_with("C-")
            || id.len() <= 2
            || !id[2..]
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
        {
            errors.push(format!("contract id {id:?} must match C-[A-Z0-9-]+"));
        }
        if !contract_ids.insert(id.to_string()) {
            errors.push(format!("duplicate contract id {id:?}"));
        }
        for field in ["title", "source"] {
            if contract
                .get(field)
                .and_then(Json::as_str)
                .is_none_or(str::is_empty)
            {
                errors.push(format!("contract {id}: missing field {field:?}"));
            }
        }
        if let Some(source) = contract.get("source").and_then(Json::as_str) {
            for part in source.split(';') {
                let path = part.split_whitespace().next().unwrap_or("");
                if !safe_relative(path) || !root.join(path).is_file() {
                    errors.push(format!(
                        "contract {id}: source path {path:?} does not exist"
                    ));
                }
            }
        }
        if !contract
            .get("obligations")
            .and_then(Json::as_array)
            .is_some_and(|a| {
                a.iter()
                    .all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
            })
        {
            errors.push(format!(
                "contract {id}: obligations must be an array of non-empty strings"
            ));
        }
        for obligation in contract
            .get("obligations")
            .and_then(Json::as_array)
            .unwrap_or_default()
        {
            let Some(symbol) = obligation.as_str() else {
                errors.push(format!("contract {id}: obligation must be a string"));
                continue;
            };
            let _ = symbol; // A name alone does not resolve a public API.
        }
    }
    report.contracts = contract_ids.len();

    // Targets: every discovered target registered once, every entry real.
    let discovered_targets: BTreeSet<&str> = discovered.iter().map(|d| d.target.as_str()).collect();
    let mut registered_targets: BTreeMap<String, String> = BTreeMap::new();
    for target in catalog
        .get("targets")
        .and_then(Json::as_array)
        .unwrap_or_default()
    {
        let Some(path) = target.get("path").and_then(Json::as_str) else {
            errors.push("target without string path".to_string());
            continue;
        };
        let kind = target.get("kind").and_then(Json::as_str).unwrap_or("");
        if !["test", "bench", "lib", "example", "bin"].contains(&kind) {
            errors.push(format!("target {path}: invalid kind"));
        }
        let actual = discovered.iter().find(|d| d.target == path);
        if actual.is_some_and(|d| d.kind != kind) {
            errors.push(format!("target {path}: kind differs from Cargo metadata"));
        }
        if !target
            .get("features")
            .and_then(Json::as_array)
            .is_some_and(|f| {
                f.iter().all(|v| {
                    v.as_str().is_some_and(|s| {
                        !s.is_empty()
                            && actual.is_some_and(|d| d.available_features.iter().any(|x| x == s))
                    })
                })
            })
        {
            errors.push(format!(
                "target {path}: features must be known package feature strings"
            ));
        }
        if let (Some(d), Some(features)) = (actual, target.get("features").and_then(Json::as_array))
        {
            if d.features
                .iter()
                .any(|required| !features.iter().any(|v| v.as_str() == Some(required)))
            {
                errors.push(format!("target {path}: missing Cargo required-features"));
            }
        }
        if !safe_relative(path) || !root.join(path).is_file() {
            errors.push(format!("target {path}: file does not exist"));
        } else if !discovered_targets.contains(path) {
            errors.push(format!("target {path}: not a discovered Cargo target"));
        }
        if registered_targets
            .insert(path.to_string(), kind.to_string())
            .is_some()
        {
            errors.push(format!("duplicate target {path}"));
        }
    }
    for target in &discovered_targets {
        if !registered_targets.contains_key(*target) {
            errors.push(format!("uncovered target {target}: register it in targets"));
        }
    }

    // Cases.
    let mut ids = BTreeSet::new();
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    let mut curated_contracts: BTreeSet<String> = BTreeSet::new();
    let mut file_cache: BTreeMap<String, Option<String>> = BTreeMap::new();
    for case in catalog
        .get("cases")
        .and_then(Json::as_array)
        .unwrap_or_default()
    {
        report.cases += 1;
        let id = case
            .get("id")
            .and_then(Json::as_str)
            .unwrap_or("<missing id>")
            .to_string();
        for (key, _) in case.as_object().unwrap_or_default() {
            if !CASE_FIELDS.contains(&key.as_str()) && !CASE_OPTIONAL_FIELDS.contains(&key.as_str())
            {
                errors.push(format!("case {id}: unknown field {key:?}"));
            }
        }
        for field in CASE_FIELDS {
            if case.get(field).is_none() {
                errors.push(format!("case {id}: missing field {field:?}"));
            }
        }
        if !valid_case_id(&id) {
            errors.push(format!(
                "case id {id:?} must match [A-Z]+-[A-Z0-9_]+-[0-9]+"
            ));
        }
        if !ids.insert(id.clone()) {
            errors.push(format!("duplicate case id {id:?}"));
        }
        let status = case.get("status").and_then(Json::as_str).unwrap_or("");
        if !STATUSES.contains(&status) {
            errors.push(format!("case {id}: status must be one of {STATUSES:?}"));
        }
        match status {
            "curated" => report.curated += 1,
            "unreviewed" => report.scaffolded += 1,
            _ => {}
        }
        let kind = case.get("kind").and_then(Json::as_str).unwrap_or("");
        match case.get("tags").and_then(Json::as_array) {
            Some(tags) if !tags.is_empty() => {
                for tag in tags {
                    if !tag.as_str().is_some_and(|t| TAGS.contains(&t)) {
                        errors.push(format!(
                            "case {id}: tag {:?} not in the controlled vocabulary",
                            tag.as_str().unwrap_or("<non-string>")
                        ));
                    }
                }
            }
            _ => errors.push(format!("case {id}: tags must be a non-empty array")),
        }
        let declared_target = case.get("target").and_then(Json::as_str).unwrap_or("");
        let actual_target = discovered.iter().find(|d| d.target == declared_target);
        match case.get("features") {
            Some(Json::Str(inherit)) if inherit == "inherit" => {}
            Some(Json::Array(features))
                if features.iter().all(|f| {
                    f.as_str().is_some_and(|s| {
                        !s.is_empty()
                            && actual_target
                                .is_some_and(|d| d.available_features.iter().any(|x| x == s))
                    })
                }) => {}
            _ => errors.push(format!(
                "case {id}: features must be an array of strings or \"inherit\""
            )),
        }
        if !case
            .get("area")
            .and_then(Json::as_str)
            .is_some_and(|a| AREAS.contains(&a))
        {
            errors.push(format!("case {id}: area must be one of {AREAS:?}"));
        }
        if case
            .get("provenance")
            .and_then(Json::as_str)
            .is_none_or(str::is_empty)
        {
            errors.push(format!("case {id}: provenance must be a non-empty string"));
        }
        if let Some(provenance) = case.get("provenance").and_then(Json::as_str) {
            if provenance.starts_with("discovery:") {
                if provenance != "discovery:scaffold" {
                    errors.push(format!("case {id}: unknown discovery provenance"));
                }
            } else if !provenance.starts_with("review ") {
                let path = provenance.split_whitespace().next().unwrap_or("");
                if !safe_relative(path) || !root.join(path).is_file() {
                    errors.push(format!(
                        "case {id}: provenance path {path:?} does not exist"
                    ));
                }
            }
        }
        if kind == "benchmark" {
            if !case
                .get("timing")
                .and_then(Json::as_str)
                .is_some_and(|t| TIMINGS.contains(&t))
            {
                errors.push(format!(
                    "case {id}: benchmark timing must be one of {TIMINGS:?}"
                ));
            }
            if case
                .get("control")
                .and_then(Json::as_str)
                .is_none_or(str::is_empty)
            {
                errors.push(format!(
                    "case {id}: benchmark control must name the native control or \"none\""
                ));
            }
        }
        if !KINDS.contains(&kind) {
            errors.push(format!("case {id}: kind must be one of {KINDS:?}"));
        }
        let seed = case.get("seed").and_then(Json::as_str).unwrap_or("");
        if !SEEDS.contains(&seed) {
            errors.push(format!("case {id}: seed must be one of {SEEDS:?}"));
        }
        if !case
            .get("cost")
            .and_then(Json::as_str)
            .is_some_and(|c| COSTS.contains(&c))
        {
            errors.push(format!("case {id}: cost must be one of {COSTS:?}"));
        }
        let scenario = case.get("scenario").and_then(Json::as_str).unwrap_or("");
        let oracle = case.get("oracle").and_then(Json::as_str).unwrap_or("");
        if case
            .get("scenario")
            .and_then(Json::as_str)
            .is_none_or(|s| s.trim().is_empty())
            || case
                .get("oracle")
                .and_then(Json::as_str)
                .is_none_or(|s| s.trim().is_empty())
        {
            errors.push(format!(
                "case {id}: scenario and oracle must be non-empty strings"
            ));
        }
        if !case
            .get("contracts")
            .and_then(Json::as_array)
            .is_some_and(|a| a.iter().all(|v| v.as_str().is_some_and(|s| !s.is_empty())))
        {
            errors.push(format!("case {id}: contracts must be an array of strings"));
        }
        let contracts: Vec<&str> = case
            .get("contracts")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Json::as_str)
            .collect();
        for contract in &contracts {
            if !contract_ids.contains(*contract) {
                errors.push(format!("case {id}: unknown contract {contract:?}"));
            }
        }
        if status == "curated" {
            if contracts.is_empty() {
                errors.push(format!("case {id}: curated case must reference a contract"));
            }
            if oracle.is_empty() || oracle == "unreviewed" {
                errors.push(format!("case {id}: curated case must name its oracle"));
            }
            if scenario.len() < 10 {
                errors.push(format!("case {id}: curated case needs a scenario"));
            }
            if kind == "unreviewed" || seed == "unreviewed" {
                errors.push(format!("case {id}: curated case cannot be unreviewed"));
            }
            for contract in &contracts {
                curated_contracts.insert((*contract).to_string());
            }
        }

        let file = case
            .get("file")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        let selector = case
            .get("selector")
            .and_then(Json::as_str)
            .unwrap_or("")
            .to_string();
        if !registered_targets.contains_key(declared_target) {
            errors.push(format!("case {id}: target must name a registered target"));
        }
        if (selector.starts_with("fn:") || selector.starts_with("literal:"))
            && declared_target != file
        {
            errors.push(format!(
                "case {id}: source selector must name its target file"
            ));
        }
        if (selector.starts_with("test:")
            || selector.starts_with("macro:")
            || selector.starts_with("fuzz:")
            || selector == "target"
            || selector == "bench")
            && !discovered
                .iter()
                .any(|d| d.target == declared_target && d.file == file && d.selector == selector)
        {
            errors.push(format!(
                "case {id}: selector does not belong to declared target"
            ));
        }
        let actual = discovered
            .iter()
            .find(|d| d.target == declared_target && d.file == file && d.selector == selector)
            .or_else(|| {
                discovered.iter().find(|d| {
                    d.target == declared_target && d.file == file && d.selector == "bench"
                })
            });
        if let Some(d) = actual {
            let expected = if d.cfg.is_empty() { "none" } else { &d.cfg };
            if case.get("cfg").and_then(Json::as_str) != Some(expected) {
                errors.push(format!(
                    "case {id}: cfg must match discovered conditions: {expected}"
                ));
            }
        } else if case
            .get("cfg")
            .and_then(Json::as_str)
            .is_none_or(str::is_empty)
        {
            errors.push(format!("case {id}: cfg must be a non-empty string"));
        }
        if !safe_relative(&file) {
            errors.push(format!(
                "case {id}: file must be a relative path inside the repository"
            ));
            continue;
        }
        let text = file_cache
            .entry(file.clone())
            .or_insert_with(|| fs::read_to_string(root.join(&file)).ok())
            .clone();
        let Some(text) = text else {
            errors.push(format!("case {id}: file {file:?} does not exist"));
            continue;
        };
        let target = discovered
            .iter()
            .find(|d| d.file == file && d.selector == selector)
            .map(|d| d.target.as_str())
            .or_else(|| {
                registered_targets
                    .get_key_value(&file)
                    .map(|(k, _)| k.as_str())
            });
        if target.is_none_or(|t| !registered_targets.contains_key(t)) {
            errors.push(format!(
                "case {id}: file {file:?} belongs to no registered target"
            ));
        }
        if !covered.insert((file.clone(), selector.clone())) {
            errors.push(format!(
                "case {id}: duplicate selector {selector:?} in {file}"
            ));
        }
        let resolved = match selector.split_once(':') {
            Some(("test", name)) => {
                discovered
                    .iter()
                    .any(|d| d.file == file && d.selector == selector)
                    && !name.is_empty()
            }
            Some(("fn", name)) => {
                text.contains(&format!("fn {name}(")) || text.contains(&format!("fn {name}<"))
            }
            Some(("literal", literal)) => text.contains(&format!("\"{literal}\"")),
            Some(("fuzz" | "macro", _)) => discovered
                .iter()
                .any(|d| d.file == file && d.selector == selector),
            _ if selector == "bench" || selector == "target" => discovered
                .iter()
                .any(|d| d.file == file && d.selector == selector),
            _ => false,
        };
        if !resolved {
            errors.push(format!(
                "case {id}: selector {selector:?} not found in {file}"
            ));
        }
    }

    for item in discovered {
        if !covered.contains(&(item.file.clone(), item.selector.clone())) {
            errors.push(format!(
                "uncovered case: {} {} (run `scaffold` to add a stub)",
                item.file, item.selector
            ));
        }
    }
    report.contracts_without_curated_cases = contract_ids
        .iter()
        .filter(|id| !curated_contracts.contains(*id))
        .cloned()
        .collect();
}

fn valid_case_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('-').collect();
    parts.len() == 3
        && !parts[0].is_empty()
        && parts[0].chars().all(|c| c.is_ascii_uppercase())
        && !parts[1].is_empty()
        && parts[1]
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        && !parts[2].is_empty()
        && parts[2].chars().all(|c| c.is_ascii_digit())
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}

// ----------------------------------------------------------------- scaffold

/// Add `scaffolded` stubs for every uncovered target and case to the catalog
/// (creating it if absent). Existing entries are preserved verbatim.
fn scaffold(root: &Path) -> Result<usize, String> {
    let discovered = discover(root)?;
    let path = root.join(CATALOG_PATH);
    let mut catalog = match fs::read_to_string(&path) {
        Ok(text) => Json::parse(&text)?,
        Err(_) => Json::Object(vec![
            ("schema".to_string(), Json::Str(SCHEMA.to_string())),
            ("contracts".to_string(), Json::Array(Vec::new())),
            ("targets".to_string(), Json::Array(Vec::new())),
            ("cases".to_string(), Json::Array(Vec::new())),
        ]),
    };
    let Json::Object(fields) = &mut catalog else {
        return Err("catalog root must be an object".to_string());
    };
    // Preserve IDs when an old unqualified selector has one unambiguous
    // structural match in the same file. Ambiguities remain errors for review.
    if let Some((_, Json::Array(cases))) = fields.iter_mut().find(|(k, _)| k == "cases") {
        for case in cases {
            let Some(file) = case.get("file").and_then(Json::as_str).map(str::to_string) else {
                continue;
            };
            let Some(selector) = case
                .get("selector")
                .and_then(Json::as_str)
                .map(str::to_string)
            else {
                continue;
            };
            if let Some(name) = selector.strip_prefix("test:") {
                if !name.contains("::") {
                    let matches: Vec<_> = discovered
                        .iter()
                        .filter(|d| {
                            d.file == file
                                && (d.selector.starts_with("test:")
                                    || d.selector.starts_with("macro:"))
                                && d.selector.rsplit("::").next() == Some(name)
                        })
                        .collect();
                    if let Some(d) = matches.first().filter(|_| {
                        matches.len() == 1
                            || case.get("status").and_then(Json::as_str) == Some("unreviewed")
                    }) {
                        if let Json::Object(case_fields) = case {
                            if let Some((_, value)) =
                                case_fields.iter_mut().find(|(k, _)| k == "selector")
                            {
                                *value = Json::Str(d.selector.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    let mut added = 0;
    let existing_targets: BTreeSet<String> = field_array(fields, "targets")
        .iter()
        .filter_map(|t| t.get("path").and_then(Json::as_str).map(str::to_string))
        .collect();
    let mut covered: BTreeSet<(String, String)> = field_array(fields, "cases")
        .iter()
        .filter_map(|c| {
            Some((
                c.get("file")?.as_str()?.to_string(),
                c.get("selector")?.as_str()?.to_string(),
            ))
        })
        .collect();
    let mut counters: BTreeMap<String, usize> = BTreeMap::new();
    for case in field_array(fields, "cases") {
        if let Some(id) = case.get("id").and_then(Json::as_str) {
            if let Some((prefix, number)) = id.rsplit_once('-') {
                let n = number.parse().unwrap_or(0);
                let entry = counters.entry(prefix.to_string()).or_insert(0);
                *entry = (*entry).max(n);
            }
        }
    }
    let mut new_targets = Vec::new();
    let mut new_cases = Vec::new();
    for item in &discovered {
        if !existing_targets.contains(&item.target)
            && !new_targets
                .iter()
                .any(|t: &Json| t.get("path").and_then(Json::as_str) == Some(item.target.as_str()))
        {
            new_targets.push(Json::Object(vec![
                ("path".to_string(), Json::Str(item.target.clone())),
                ("kind".to_string(), Json::Str(item.kind.clone())),
                (
                    "features".to_string(),
                    Json::Array(item.features.iter().map(|f| Json::Str(f.clone())).collect()),
                ),
            ]));
            added += 1;
        }
        if covered.insert((item.file.clone(), item.selector.clone())) {
            let stem = Path::new(&item.target)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("UNKNOWN")
                .to_ascii_uppercase();
            let prefix = format!(
                "{}-{stem}",
                if item.selector == "bench" { "B" } else { "T" }
            );
            let counter = counters.entry(prefix.clone()).or_insert(0);
            *counter += 1;
            let selector_text = item.selector.trim_start_matches("test:").replace('_', " ");
            new_cases.push(Json::Object(vec![
                (
                    "id".to_string(),
                    Json::Str(format!("{prefix}-{counter:03}")),
                ),
                ("file".to_string(), Json::Str(item.file.clone())),
                ("selector".to_string(), Json::Str(item.selector.clone())),
                ("status".to_string(), Json::Str("unreviewed".to_string())),
                (
                    "tags".to_string(),
                    Json::Array(vec![
                        Json::Str("unreviewed".to_string()),
                        Json::Str(
                            if item.selector == "bench" {
                                "benchmark"
                            } else {
                                "integration"
                            }
                            .to_string(),
                        ),
                    ]),
                ),
                ("features".to_string(), Json::Str("inherit".to_string())),
                (
                    "area".to_string(),
                    Json::Str(area_of(&item.file).to_string()),
                ),
                (
                    "provenance".to_string(),
                    Json::Str("discovery:scaffold".to_string()),
                ),
                ("kind".to_string(), Json::Str("unreviewed".to_string())),
                ("contracts".to_string(), Json::Array(Vec::new())),
                ("oracle".to_string(), Json::Str("unreviewed".to_string())),
                (
                    "scenario".to_string(),
                    Json::Str(format!("scaffold: {selector_text}")),
                ),
                ("seed".to_string(), Json::Str("unreviewed".to_string())),
            ]));
            added += 1;
        }
    }
    push_all(fields, "targets", new_targets);
    push_all(fields, "cases", new_cases);
    if let Some((_, Json::Array(cases))) = fields.iter_mut().find(|(k, _)| k == "cases") {
        for case in cases {
            let Some(file) = case.get("file").and_then(Json::as_str).map(str::to_string) else {
                continue;
            };
            let Some(selector) = case
                .get("selector")
                .and_then(Json::as_str)
                .map(str::to_string)
            else {
                continue;
            };
            let found = discovered
                .iter()
                .find(|d| d.file == file && d.selector == selector)
                .or_else(|| {
                    discovered
                        .iter()
                        .find(|d| d.target == file && d.selector == "bench")
                });
            let Some(found) = found else {
                continue;
            };
            if let Json::Object(fields) = case {
                if !fields.iter().any(|(k, _)| k == "target") {
                    fields.push(("target".into(), Json::Str(found.target.clone())));
                }
                if !fields.iter().any(|(k, _)| k == "cfg") {
                    fields.push((
                        "cfg".into(),
                        Json::Str(if found.cfg.is_empty() {
                            "none".into()
                        } else {
                            found.cfg.clone()
                        }),
                    ));
                }
                if !fields.iter().any(|(k, _)| k == "cost") {
                    fields.push(("cost".into(), Json::Str("unreviewed".into())));
                }
            }
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, catalog.to_pretty()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(added)
}

/// Owner area guessed from the file stem for scaffolded entries; curation
/// corrects it. The vocabulary is `AREAS`.
fn area_of(file: &str) -> &'static str {
    let stem = file.to_ascii_lowercase();
    if stem.contains("integer") {
        "integer"
    } else if stem.contains("concurrent")
        || stem.contains("snapshot")
        || stem.contains("publication")
    {
        "concurrent"
    } else if stem.contains("adaptive") || stem.contains("inline") {
        "adaptive"
    } else if stem.contains("adapter") || stem.contains("ecosystem") || stem.contains("rayon") {
        "adapters"
    } else if stem.contains("bitmap")
        || stem.contains("backend")
        || stem.contains("advanced")
        || stem.contains("specialized")
    {
        "backends"
    } else if stem.contains("metadata") || stem.contains("catalog") {
        "support"
    } else {
        "family"
    }
}

fn field_array<'a>(fields: &'a [(String, Json)], key: &str) -> &'a [Json] {
    fields
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, v)| v.as_array())
        .unwrap_or_default()
}

fn push_all(fields: &mut Vec<(String, Json)>, key: &str, items: Vec<Json>) {
    if let Some((_, Json::Array(existing))) = fields.iter_mut().find(|(k, _)| k == key) {
        existing.extend(items);
    } else {
        fields.push((key.to_string(), Json::Array(items)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let root =
                std::env::temp_dir().join(format!("frost-catalog-{}-{n}", std::process::id()));
            fs::create_dir_all(root.join("tests/a")).unwrap();
            fs::create_dir_all(root.join("tests/support")).unwrap();
            fs::create_dir_all(root.join("benches")).unwrap();
            fs::create_dir_all(root.join("src")).unwrap();
            fs::create_dir_all(root.join("docs")).unwrap();
            fs::write(root.join("docs/x.md"), "fixture contract\n").unwrap();
            fs::create_dir_all(root.join("verification/metadata")).unwrap();
            fs::write(root.join("Cargo.toml"), "[package]\nname = \"catalog-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[features]\nstd = []\n").unwrap();
            fs::write(
                root.join("Cargo.lock"),
                "version = 4\n[[package]]\nname = \"catalog-fixture\"\nversion = \"0.1.0\"\n",
            )
            .unwrap();
            fs::write(
                root.join("tests/a.rs"),
                "#[path = \"a/live.rs\"]\nmod live;\n#[test]\nfn first() {}\n\nproptest! {\n    #[test]\n    fn second(x in 0..3) {}\n}\n// #[test] in a comment is ignored\n",
            )
            .unwrap();
            fs::write(
                root.join("tests/a/live.rs"),
                "#[test]\n#[should_panic]\nfn third() {}\n",
            )
            .unwrap();
            fs::write(
                root.join("tests/support/mod.rs"),
                "#[test]\nfn not_a_target() {}\n",
            )
            .unwrap();
            fs::write(
                root.join("benches/b.rs"),
                "fn case(name: &str) {}\nfn main() { case(\"balanced\"); }\n",
            )
            .unwrap();
            fs::write(
                root.join("src/lib.rs"),
                "pub struct Thing;\nimpl Thing { pub fn probe(&self) {} }\n",
            )
            .unwrap();
            fs::write(
                root.join(bench::SIDECAR_PATH),
                "{\"schema\": \"frost-bench-inventory/1\", \"targets\": {\"benches/b.rs\": [\n  {\"name\": \"balanced.frost_count\", \"timing\": \"operation\", \"status\": \"unreviewed\", \"tags\": [\"benchmark\", \"unreviewed\"], \"features\": \"inherit\", \"scenario\": \"dataset balanced; operation frost_count\", \"control\": \"none\", \"area\": \"family\", \"provenance\": \"fixture\", \"seed\": \"unreviewed\", \"cost\": \"timed\", \"contracts\": []}\n]}}\n",
            )
            .unwrap();
            Self { root }
        }

        fn write_catalog(&self, text: &str) {
            fs::write(self.root.join(CATALOG_PATH), text).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn valid_catalog() -> String {
        r#"{
  "schema": "frost-verification-catalog/1",
  "contracts": [
    {"id": "C-THING", "title": "Thing probes", "source": "docs/x.md", "obligations": ["Thing::probe", "Thing"]}
  ],
  "targets": [
    {"path": "tests/a.rs", "kind": "test", "features": ["std"]},
    {"path": "src/lib.rs", "kind": "lib", "features": []},
    {"path": "benches/b.rs", "kind": "bench", "features": []}
  ],
  "cases": [
    {"id": "T-A-001", "target": "tests/a.rs", "cfg": "none", "file": "tests/a.rs", "selector": "test:first", "status": "curated", "kind": "contract", "tags": ["integration"], "features": "inherit", "area": "family", "provenance": "docs/x.md", "contracts": ["C-THING"], "oracle": "hand-expected", "scenario": "first probes the thing", "seed": "none", "cost": "fast"},
    {"id": "T-A-002", "target": "tests/a.rs", "cfg": "none", "cost": "unreviewed", "file": "tests/a.rs", "selector": "test:second", "status": "unreviewed", "kind": "unreviewed", "tags": ["unreviewed"], "features": "inherit", "area": "family", "provenance": "discovery:scaffold", "contracts": [], "oracle": "unreviewed", "scenario": "scaffold", "seed": "unreviewed"},
    {"id": "T-A-003", "target": "tests/a.rs", "cfg": "none", "cost": "unreviewed", "file": "tests/a/live.rs", "selector": "test:live::third", "status": "curated", "kind": "regression", "tags": ["integration"], "features": ["std"], "area": "concurrent", "provenance": "docs/x.md", "contracts": ["C-THING"], "oracle": "hand-expected", "scenario": "module file case resolves", "seed": "none"},
    {"id": "T-LIB-001", "target": "src/lib.rs", "cfg": "none", "cost": "unreviewed", "file": "src/lib.rs", "selector": "target", "status": "unreviewed", "kind": "unreviewed", "tags": ["unreviewed"], "features": "inherit", "area": "family", "provenance": "discovery:scaffold", "contracts": [], "oracle": "unreviewed", "scenario": "target scaffold", "seed": "unreviewed"},
    {"id": "B-B-001", "target": "benches/b.rs", "cfg": "none", "file": "benches/b.rs", "selector": "bench", "status": "curated", "kind": "benchmark", "tags": ["benchmark"], "features": "inherit", "area": "family", "provenance": "docs/x.md", "contracts": ["C-THING"], "oracle": "BTreeSet before timing", "scenario": "benchmark target registered", "seed": "fixed", "cost": "timed", "timing": "operation", "control": "std_count"},
    {"id": "B-B-002", "target": "benches/b.rs", "cfg": "none", "file": "benches/b.rs", "selector": "literal:balanced", "status": "curated", "kind": "benchmark", "tags": ["benchmark", "native-control"], "features": "inherit", "area": "family", "provenance": "docs/x.md", "contracts": ["C-THING"], "oracle": "BTreeSet before timing", "scenario": "balanced case literal", "seed": "fixed", "cost": "timed", "timing": "operation", "control": "std_count"}
  ]
}
"#
        .to_string()
    }

    fn errors_for(fixture: &Fixture, catalog: &str) -> Vec<String> {
        fixture.write_catalog(catalog);
        check(&fixture.root).errors
    }

    #[test]
    fn executed_benchmark_inventory_rejects_an_omitted_case() {
        let fixture = Fixture::new();
        fixture.write_catalog(&valid_catalog());
        let listing = fixture.root.join("inventory.txt");
        fs::write(
            &listing,
            "case,iterations\ncase\tbalanced.frost_count\toperation\n",
        )
        .unwrap();
        assert_eq!(
            inventory(&fixture.root, "benches/b.rs", &listing).unwrap(),
            Vec::<String>::new()
        );
        // A new operation under the already registered `balanced` dataset.
        fs::write(
            &listing,
            "case,iterations\ncase\tbalanced.frost_count\toperation\ncase\tbalanced.new_operation\toperation\n",
        )
        .unwrap();
        let errors = inventory(&fixture.root, "benches/b.rs", &listing).unwrap();
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("balanced.new_operation is not registered"));
        // Empty and malformed listings are errors, not silent passes.
        fs::write(&listing, "case,iterations\n").unwrap();
        assert!(
            inventory(&fixture.root, "benches/b.rs", &listing)
                .unwrap_err()
                .contains("no case lines")
        );
        fs::write(&listing, "case\tbalanced.frost_count\tguess\n").unwrap();
        assert!(
            inventory(&fixture.root, "benches/b.rs", &listing)
                .unwrap_err()
                .contains("timing \"guess\"")
        );
        // The structural check also rejects a sidecar entry with bad metadata.
        fs::write(
            fixture.root.join(bench::SIDECAR_PATH),
            "{\"schema\": \"frost-bench-inventory/1\", \"targets\": {\"benches/b.rs\": [\n  {\"name\": \"balanced.frost_count\", \"timing\": \"operation\", \"status\": \"unreviewed\", \"tags\": [], \"features\": \"inherit\", \"scenario\": \"short\", \"control\": \"\", \"area\": \"family\", \"provenance\": \"fixture\", \"seed\": \"unreviewed\", \"cost\": \"timed\", \"contracts\": []}\n]}}\n",
        )
        .unwrap();
        let report = check(&fixture.root);
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("tags must be a non-empty array")),
            "{:?}",
            report.errors
        );
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("scenario must describe")),
            "{:?}",
            report.errors
        );
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("control must name")),
            "{:?}",
            report.errors
        );
    }

    #[test]
    fn discovery_finds_tests_in_targets_and_module_files_only() {
        let fixture = Fixture::new();
        let found = discover(&fixture.root).unwrap();
        let selectors: Vec<String> = found
            .iter()
            .map(|d| format!("{} {}", d.file, d.selector))
            .collect();
        assert_eq!(
            selectors,
            [
                "benches/b.rs bench",
                "src/lib.rs target",
                "tests/a.rs test:first",
                "tests/a.rs test:second",
                "tests/a/live.rs test:live::third"
            ]
        );
        assert!(found.iter().all(|d| d.target == "tests/a.rs"
            || d.target == "benches/b.rs"
            || d.target == "src/lib.rs"));
    }

    #[test]
    fn discovery_handles_directory_target_paths_names_and_non_code() {
        let fixture = Fixture::new();
        fs::create_dir_all(fixture.root.join("tests/nested/odd")).unwrap();
        fs::write(
            fixture.root.join("tests/nested/main.rs"),
            r##"
            #[path = "odd/elsewhere.rs"] mod outside;
            mod left { #[test] fn same() {} }
            mod right { #[test] fn same() {} }
            proptest! { #[cfg(feature = "std")] #[test] fn property(x in 0..2) {} }
            prop_state_machine! { #[test] fn machine(sequential 1..4 => Machine); }
            #[kani::proof] fn proof() {}
            #[cfg(feature = "std")] macro_rules! has_one { () => { #[test] fn generated() {} }; }
            const TEXT: &str = r#"#[test] fn fake() {}"#;
            // #[test] fn comment_only() {}
        "##,
        )
        .unwrap();
        fs::write(
            fixture.root.join("tests/nested/odd/elsewhere.rs"),
            "#[path = \"sibling.rs\"] mod child;\n#[test] fn from_path() {}\n",
        )
        .unwrap();
        fs::write(
            fixture.root.join("tests/nested/odd/sibling.rs"),
            "#[test] fn sibling() {}\n",
        )
        .unwrap();
        let found = discover(&fixture.root).unwrap();
        let selectors: BTreeSet<_> = found
            .iter()
            .filter(|d| d.target == "tests/nested/main.rs")
            .map(|d| d.selector.as_str())
            .collect();
        assert_eq!(
            selectors,
            [
                "macro:has_one::generated",
                "test:left::same",
                "test:machine",
                "test:outside::child::sibling",
                "test:outside::from_path",
                "test:proof",
                "test:property",
                "test:right::same"
            ]
            .into_iter()
            .collect()
        );
        assert!(
            found
                .iter()
                .any(|d| d.file == "tests/nested/odd/elsewhere.rs")
        );
        assert!(found.iter().any(|d| d.selector == "test:property"
            && d.cfg.contains("feature")
            && d.cfg.contains("std")));
        assert!(
            found
                .iter()
                .any(|d| d.selector == "macro:has_one::generated" && d.cfg.contains("std"))
        );
    }

    #[test]
    fn validation_rejects_wrong_collection_types_and_missing_cases() {
        let fixture = Fixture::new();
        let broken = valid_catalog().replace("\"contracts\": [\"C-THING\"]", "\"contracts\": [1]");
        let errors = errors_for(&fixture, &broken);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("contracts must be an array of strings")),
            "{errors:?}"
        );
        let broken = valid_catalog().replace("\"tags\": [\"integration\"]", "\"tags\": []");
        let errors = errors_for(&fixture, &broken);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("tags must be a non-empty array")),
            "{errors:?}"
        );
        let broken = valid_catalog().replace("test:second", "test:stale");
        let errors = errors_for(&fixture, &broken);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("selector \"test:stale\" not found")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("uncovered case") && e.contains("test:second")),
            "{errors:?}"
        );
        let broken = valid_catalog().replace("\"features\": [\"std\"]", "\"features\": [17]");
        let errors = errors_for(&fixture, &broken);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("features must be known package feature strings")),
            "{errors:?}"
        );
        let broken =
            valid_catalog().replace("\"cfg\": \"none\"", "\"cfg\": \"cfg (feature = xyz)\"");
        let errors = errors_for(&fixture, &broken);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("cfg must match discovered conditions")),
            "{errors:?}"
        );
    }

    #[test]
    fn valid_catalog_passes_and_reports_gaps_honestly() {
        let fixture = Fixture::new();
        fixture.write_catalog(&valid_catalog());
        let report = check(&fixture.root);
        assert_eq!(report.errors, Vec::<String>::new());
        assert_eq!((report.discovered_tests, report.discovered_benches), (3, 1));
        assert_eq!((report.cases, report.curated, report.scaffolded), (6, 4, 2));
        assert!(report.contracts_without_curated_cases.is_empty());
    }

    #[test]
    fn duplicate_ids_and_selectors_are_rejected() {
        let fixture = Fixture::new();
        let dup_id = valid_catalog().replace("\"id\": \"T-A-002\"", "\"id\": \"T-A-001\"");
        let errors = errors_for(&fixture, &dup_id);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("duplicate case id \"T-A-001\"")),
            "{errors:?}"
        );
        let dup_selector = valid_catalog().replace(
            "\"selector\": \"test:second\"",
            "\"selector\": \"test:first\"",
        );
        let errors = errors_for(&fixture, &dup_selector);
        assert!(
            errors.iter().any(|e| e.contains("duplicate selector")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("uncovered case: tests/a.rs test:second")),
            "{errors:?}"
        );
        let dup_contract = valid_catalog().replace(
            "\"contracts\": [\n    {",
            "\"contracts\": [\n    {\"id\": \"C-THING\", \"title\": \"t\", \"source\": \"s\", \"obligations\": []},\n    {",
        );
        let errors = errors_for(&fixture, &dup_contract);
        assert!(
            errors.iter().any(|e| e.contains("duplicate contract id")),
            "{errors:?}"
        );
    }

    #[test]
    fn broken_paths_selectors_and_obligations_are_rejected() {
        let fixture = Fixture::new();
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace(
                "\"file\": \"tests/a/live.rs\"",
                "\"file\": \"tests/missing.rs\"",
            ),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("file \"tests/missing.rs\" does not exist")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace("test:live::third", "test:live::fourth"),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("selector \"test:live::fourth\" not found")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace("literal:balanced", "literal:skewed"),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("selector \"literal:skewed\" not found")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace("\"Thing::probe\"", "\"\""),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("obligations must be an array of non-empty strings")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace(
                "{\"path\": \"benches/b.rs\", \"kind\": \"bench\", \"features\": []}",
                "{\"path\": \"benches/ghost.rs\", \"kind\": \"bench\", \"features\": []}",
            ),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("target benches/ghost.rs: file does not exist")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("uncovered target benches/b.rs")),
            "{errors:?}"
        );
    }

    #[test]
    fn missing_unknown_and_unreviewed_fields_are_rejected() {
        let fixture = Fixture::new();
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace("\"scenario\": \"first probes the thing\", ", ""),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("case T-A-001: missing field \"scenario\"")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace(
                "\"cost\": \"fast\"",
                "\"cost\": \"fast\", \"owner\": \"me\"",
            ),
        );
        assert!(
            errors.iter().any(|e| e.contains("unknown field \"owner\"")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace("\"status\": \"unreviewed\"", "\"status\": \"curated\""),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("T-A-002: curated case must reference a contract")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("T-A-002: curated case must name its oracle")),
            "{errors:?}"
        );
        let errors = errors_for(&fixture, &valid_catalog().replace("\"contracts\": [\"C-THING\"], \"oracle\": \"hand-expected\", \"scenario\": \"first", "\"contracts\": [\"C-NOPE\"], \"oracle\": \"hand-expected\", \"scenario\": \"first"));
        assert!(
            errors
                .iter()
                .any(|e| e.contains("unknown contract \"C-NOPE\"")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace("frost-verification-catalog/1", "other/9"),
        );
        assert!(
            errors.iter().any(|e| e.contains("schema must be")),
            "{errors:?}"
        );
        let errors = errors_for(&fixture, "{\"schema\": \"frost-verification-catalog/1\",}");
        assert!(errors.iter().any(|e| e.starts_with("json:")), "{errors:?}");
        // Required provenance metadata: empty tags, a tag outside the
        // vocabulary, a malformed features type, and an unknown area.
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace(
                "\"tags\": [\"integration\"], \"features\": \"inherit\", \"area\": \"family\"",
                "\"tags\": [], \"features\": 7, \"area\": \"kitchen\"",
            ),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("T-A-001: tags must be a non-empty array")),
            "{errors:?}"
        );
        assert!(errors.iter().any(|e| e.contains("T-A-001: features must be an array of strings or \"inherit\"")), "{errors:?}");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("T-A-001: area must be one of")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace("\"tags\": [\"unreviewed\"]", "\"tags\": [\"made-up\"]"),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("T-A-002: tag \"made-up\" not in the controlled vocabulary")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace(
                "\"provenance\": \"discovery:scaffold\"",
                "\"provenance\": \"\"",
            ),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("T-A-002: provenance must be a non-empty string")),
            "{errors:?}"
        );
        let errors = errors_for(
            &fixture,
            &valid_catalog().replace(
                "\"timing\": \"operation\", \"control\": \"std_count\"}",
                "\"timing\": \"guess\"}",
            ),
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("B-B-002: benchmark timing must be one of")),
            "{errors:?}"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("B-B-002: benchmark control must name")),
            "{errors:?}"
        );
    }

    #[test]
    fn scaffold_adds_only_uncovered_entries_and_check_then_passes() {
        let fixture = Fixture::new();
        fixture.write_catalog(
            r#"{
  "schema": "frost-verification-catalog/1",
  "contracts": [],
  "targets": [{"path": "tests/a.rs", "kind": "test", "features": []}],
  "cases": [
    {"id": "T-A-007", "file": "tests/a.rs", "selector": "test:first", "status": "unreviewed", "kind": "unreviewed", "tags": ["unreviewed"], "features": "inherit", "area": "family", "provenance": "discovery:scaffold", "contracts": [], "oracle": "unreviewed", "scenario": "scaffold", "seed": "unreviewed"}
  ]
}
"#,
        );
        let before = check(&fixture.root);
        assert!(before.errors.iter().any(|e| e.contains("uncovered case")));
        assert_eq!(scaffold(&fixture.root).unwrap(), 6);
        let after = check(&fixture.root);
        assert_eq!(after.errors, Vec::<String>::new());
        assert_eq!((after.cases, after.scaffolded), (5, 5));
        let text = fs::read_to_string(fixture.root.join(CATALOG_PATH)).unwrap();
        assert!(text.contains("\"T-A-008\""), "continues numbering: {text}");
        assert!(text.contains("\"B-B-001\""));
        assert_eq!(scaffold(&fixture.root).unwrap(), 0);
    }
}
