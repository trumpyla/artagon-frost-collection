//! Executable benchmark case inventory.
//!
//! Benchmark case names are composed at run time (`format!("ordered.{name}.
//! frost_count")`), so source literals cannot enumerate them. Instead each
//! benchmark target is run with `FROST_BENCH_LIST=1`, which prints one
//! `case<TAB>name<TAB>timing` line per measured case and skips timing. The
//! sidecar `verification/metadata/bench-inventory.json` registers every exact
//! measured name with structural metadata; `compare` enforces an exact
//! two-way match (no new case, no stale entry, same timing).

use crate::json::Json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(crate) const SIDECAR_PATH: &str = "verification/metadata/bench-inventory.json";
pub(crate) const SIDECAR_SCHEMA: &str = "frost-bench-inventory/1";
const ENTRY_FIELDS: [&str; 12] = [
    "name",
    "timing",
    "status",
    "tags",
    "features",
    "scenario",
    "control",
    "area",
    "provenance",
    "seed",
    "cost",
    "contracts",
];
const CONTROL_MARKERS: [&str; 4] = ["std", "native", "btree", "raw"];

/// One executed case from a listing: exact name and timing kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Executed {
    pub(crate) name: String,
    pub(crate) timing: String,
}

/// Parse `FROST_BENCH_LIST=1` output. Non-case lines (the CSV header and
/// diagnostics) are ignored; a malformed case line, an unknown timing, a
/// duplicate name, or an empty inventory is an error.
pub(crate) fn parse_listing(text: &str, timings: &[&str]) -> Result<Vec<Executed>, Vec<String>> {
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut executed = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("case\t") else {
            continue;
        };
        let fields: Vec<&str> = rest.split('\t').collect();
        let [name, timing] = fields.as_slice() else {
            errors.push(format!(
                "listing line {}: expected case<TAB>name<TAB>timing",
                index + 1
            ));
            continue;
        };
        if name.is_empty() || name.contains(',') || name.contains(char::is_whitespace) {
            errors.push(format!(
                "listing line {}: malformed case name {name:?}",
                index + 1
            ));
            continue;
        }
        if !timings.contains(timing) {
            errors.push(format!(
                "listing line {}: timing {timing:?} not in {timings:?}",
                index + 1
            ));
            continue;
        }
        if !seen.insert((*name).to_string()) {
            errors.push(format!(
                "listing line {}: duplicate executed case {name}",
                index + 1
            ));
            continue;
        }
        executed.push(Executed {
            name: (*name).to_string(),
            timing: (*timing).to_string(),
        });
    }
    if executed.is_empty() && errors.is_empty() {
        errors.push("listing contains no case lines (was FROST_BENCH_LIST set?)".to_string());
    }
    if errors.is_empty() {
        Ok(executed)
    } else {
        Err(errors)
    }
}

pub(crate) fn load_sidecar(root: &Path) -> Result<Json, String> {
    let path = root.join(SIDECAR_PATH);
    let text = fs::read_to_string(&path).map_err(|e| format!("{SIDECAR_PATH}: {e}"))?;
    Json::parse(&text)
}

fn entries<'a>(sidecar: &'a Json, target: &str) -> &'a [Json] {
    sidecar
        .get("targets")
        .and_then(|t| t.get(target))
        .and_then(Json::as_array)
        .unwrap_or_default()
}

/// Structural validation of the sidecar against the registered benchmark
/// targets: every target has a non-empty section, no section is unregistered,
/// and every entry carries valid metadata. This does not prove the entries
/// match what the benchmarks execute; `compare` does that per target.
pub(crate) fn validate_sidecar(
    sidecar: &Json,
    bench_targets: &BTreeSet<String>,
    timings: &[&str],
    statuses: &[&str],
    tags: &[&str],
) -> Vec<String> {
    let mut errors = Vec::new();
    if sidecar.get("schema").and_then(Json::as_str) != Some(SIDECAR_SCHEMA) {
        errors.push(format!("bench inventory schema must be {SIDECAR_SCHEMA:?}"));
    }
    if sidecar.as_object().is_none_or(|fields| {
        fields
            .iter()
            .any(|(key, _)| !["schema", "targets"].contains(&key.as_str()))
    }) {
        errors.push("bench inventory: invalid root object".to_string());
    }
    if sidecar.get("targets").and_then(Json::as_object).is_none() {
        errors.push("bench inventory: targets must be an object".into());
    }
    let sections = sidecar
        .get("targets")
        .and_then(Json::as_object)
        .unwrap_or_default();
    for (target, _) in sections {
        if !bench_targets.contains(target) {
            errors.push(format!(
                "bench inventory: {target} is not a registered benchmark target"
            ));
        }
    }
    for target in bench_targets {
        let list = entries(sidecar, target);
        if list.is_empty() {
            errors.push(format!(
                "bench inventory: {target} has no registered measured cases"
            ));
        }
        let mut names = BTreeSet::new();
        for entry in list {
            let name = entry
                .get("name")
                .and_then(Json::as_str)
                .unwrap_or("<missing name>");
            let label = format!("bench inventory {target} {name}");
            for (key, _) in entry.as_object().unwrap_or_default() {
                if !ENTRY_FIELDS.contains(&key.as_str()) {
                    errors.push(format!("{label}: unknown field {key:?}"));
                }
            }
            for field in ENTRY_FIELDS {
                if entry.get(field).is_none() {
                    errors.push(format!("{label}: missing field {field:?}"));
                }
            }
            if entry.as_object().is_none()
                || entry
                    .get("name")
                    .and_then(Json::as_str)
                    .is_none_or(|s| s.trim().is_empty())
                || !names.insert(name.to_string())
            {
                errors.push(format!("{label}: empty or duplicate name"));
            }
            if !entry
                .get("timing")
                .and_then(Json::as_str)
                .is_some_and(|t| timings.contains(&t))
            {
                errors.push(format!("{label}: timing must be one of {timings:?}"));
            }
            if !entry
                .get("status")
                .and_then(Json::as_str)
                .is_some_and(|s| statuses.contains(&s))
            {
                errors.push(format!("{label}: status must be one of {statuses:?}"));
            }
            match entry.get("tags").and_then(Json::as_array) {
                Some(list) if !list.is_empty() => {
                    for tag in list {
                        if !tag.as_str().is_some_and(|t| tags.contains(&t)) {
                            errors.push(format!(
                                "{label}: tag {tag:?} not in the controlled vocabulary"
                            ));
                        }
                    }
                }
                _ => errors.push(format!("{label}: tags must be a non-empty array")),
            }
            match entry.get("features") {
                Some(Json::Str(inherit)) if inherit == "inherit" => {}
                Some(Json::Array(list))
                    if list
                        .iter()
                        .all(|f| f.as_str().is_some_and(|s| !s.trim().is_empty())) => {}
                _ => errors.push(format!(
                    "{label}: features must be an array of strings or \"inherit\""
                )),
            }
            if entry
                .get("scenario")
                .and_then(Json::as_str)
                .is_none_or(|s| s.len() < 10)
            {
                errors.push(format!(
                    "{label}: scenario must describe the case (10+ chars)"
                ));
            }
            if !entry
                .get("area")
                .and_then(Json::as_str)
                .is_some_and(|a| crate::AREAS.contains(&a))
            {
                errors.push(format!("{label}: invalid area"));
            }
            if !entry
                .get("seed")
                .and_then(Json::as_str)
                .is_some_and(|s| crate::SEEDS.contains(&s))
            {
                errors.push(format!("{label}: invalid seed"));
            }
            if entry.get("cost").and_then(Json::as_str) != Some("timed") {
                errors.push(format!("{label}: cost must be timed"));
            }
            if entry
                .get("provenance")
                .and_then(Json::as_str)
                .is_none_or(|s| s.trim().is_empty())
            {
                errors.push(format!("{label}: missing provenance"));
            }
            if !entry
                .get("contracts")
                .and_then(Json::as_array)
                .is_some_and(|a| a.iter().all(|v| v.as_str().is_some_and(|s| !s.is_empty())))
            {
                errors.push(format!("{label}: contracts must be an array of strings"));
            }
            if let Some(control) = entry.get("control").and_then(Json::as_str) {
                if control.starts_with("unreviewed:")
                    && (control.trim() == "unreviewed:"
                        || entry.get("status").and_then(Json::as_str) != Some("unreviewed"))
                {
                    errors.push(format!(
                        "{label}: unresolved control requires a reason and unreviewed status"
                    ));
                }
                if control != "none"
                    && !control.starts_with("unreviewed: ")
                    && !list.iter().any(|candidate| {
                        candidate.get("name").and_then(Json::as_str) == Some(control)
                    })
                {
                    errors.push(format!("{label}: control does not name a sibling"));
                }
            }
            if entry
                .get("control")
                .and_then(Json::as_str)
                .is_none_or(str::is_empty)
            {
                errors.push(format!(
                    "{label}: control must name the native control case or \"none\""
                ));
            }
        }
    }
    errors
}

/// Exact two-way comparison of an executed listing with the sidecar section
/// of `target`: every executed case must be registered by exact name with the
/// same timing, and every registered case must have been executed.
pub(crate) fn compare(sidecar: &Json, target: &str, executed: &[Executed]) -> Vec<String> {
    let mut errors = Vec::new();
    let registered: BTreeMap<&str, &str> = entries(sidecar, target)
        .iter()
        .filter_map(|e| Some((e.get("name")?.as_str()?, e.get("timing")?.as_str()?)))
        .collect();
    let mut seen = BTreeSet::new();
    for case in executed {
        seen.insert(case.name.as_str());
        match registered.get(case.name.as_str()) {
            None => errors.push(format!(
                "executed benchmark case {} is not registered in {SIDECAR_PATH} for {target}",
                case.name
            )),
            Some(timing) if *timing != case.timing => errors.push(format!(
                "executed benchmark case {} has timing {} but is registered as {timing}",
                case.name, case.timing
            )),
            Some(_) => {}
        }
    }
    for name in registered.keys() {
        if !seen.contains(name) {
            errors.push(format!(
                "registered benchmark case {name} for {target} was not executed (stale entry)"
            ));
        }
    }
    errors
}

/// Derive structural metadata for a measured name from its dot-separated
/// segments. The result is honest scaffolding (`unreviewed`): tags come from
/// the family segment and native-control markers, the scenario names the
/// dataset and operation, and the control is a sibling case carrying a
/// native-control marker, or "none".
pub(crate) fn derive_entry(name: &str, timing: &str, target: &str, all_names: &[String]) -> Json {
    let segments: Vec<&str> = name.split('.').collect();
    let family = segments.first().copied().unwrap_or("");
    let operation = segments.last().copied().unwrap_or("");
    let mut tags: Vec<&str> = vec!["benchmark", "unreviewed"];
    let family_tag = match family {
        "ordered" | "btree" => Some("ordered"),
        "hash" => Some("hash"),
        "dense" => Some("bitmap"),
        "roaring" => Some("roaring"),
        "adaptive" => Some("adaptive"),
        "snapshot" => Some("snapshot"),
        "u32" | "u64" | "i64" | "user_id" => Some("integer"),
        _ => None,
    };
    if let Some(tag) = family_tag {
        tags.push(tag);
    }
    if target.contains("integer") && !tags.contains(&"integer") {
        tags.push("integer");
    }
    if is_control(name) {
        tags.push("native-control");
    }
    let dataset = if segments.len() > 2 {
        segments[1..segments.len() - 1].join(".")
    } else {
        String::new()
    };
    let scenario = if dataset.is_empty() {
        format!("family {family}; operation {operation}")
    } else {
        format!("family {family}; dataset {dataset}; operation {operation}")
    };
    let control = if is_reviewed_native(name, target) {
        "none".to_string()
    } else {
        control_for(name, target, all_names)
            .unwrap_or_else(|| "unreviewed: no equivalent native control established".to_string())
    };
    Json::Object(vec![
        ("name".to_string(), Json::Str(name.to_string())),
        ("timing".to_string(), Json::Str(timing.to_string())),
        ("status".to_string(), Json::Str("unreviewed".to_string())),
        (
            "tags".to_string(),
            Json::Array(tags.into_iter().map(|t| Json::Str(t.to_string())).collect()),
        ),
        ("features".to_string(), Json::Str("inherit".to_string())),
        ("scenario".to_string(), Json::Str(scenario)),
        ("control".to_string(), Json::Str(control)),
        (
            "area".to_string(),
            Json::Str(
                if target.contains("integer") {
                    "integer"
                } else if target.contains("snapshot") {
                    "concurrent"
                } else if target.contains("adaptive") {
                    "adaptive"
                } else {
                    "family"
                }
                .to_string(),
            ),
        ),
        (
            "provenance".to_string(),
            Json::Str("discovery:bench-list".to_string()),
        ),
        (
            "seed".to_string(),
            Json::Str(
                if name.contains("factory_") {
                    "factory"
                } else {
                    "unreviewed"
                }
                .to_string(),
            ),
        ),
        ("cost".to_string(), Json::Str("timed".to_string())),
        ("contracts".to_string(), Json::Array(Vec::new())),
    ])
}

fn is_control(name: &str) -> bool {
    name.split('.')
        .any(|segment| CONTROL_MARKERS.iter().any(|m| segment.contains(m)))
}

fn is_reviewed_native(name: &str, target: &str) -> bool {
    let (prefix, op) = name.rsplit_once('.').unwrap_or(("", ""));
    (target == "benches/ordered_sets.rs" && ["std_count", "std_subset"].contains(&op))
        || (target == "benches/integer_sets.rs"
            && name.contains("factory_")
            && prefix.rsplit('.').next() == Some("btree")
            && ["intersection_count", "intersection_materialize"].contains(&op))
}

/// Only map operations whose native operation is explicitly equivalent.
fn control_for(name: &str, target: &str, all_names: &[String]) -> Option<String> {
    let (prefix, operation) = name.rsplit_once('.')?;
    let candidate = match (target, operation) {
        ("benches/ordered_sets.rs", "frost_count") => format!("{prefix}.std_count"),
        ("benches/ordered_sets.rs", "frost_subset") => format!("{prefix}.std_subset"),
        ("benches/integer_sets.rs", "intersection_count" | "intersection_materialize")
            if name.contains("factory_") =>
        {
            let (dataset, profile) = prefix.rsplit_once('.')?;
            if ![
                "auto",
                "sorted",
                "dense",
                "roaring",
                "chunked",
                "frozen_ordered",
            ]
            .contains(&profile)
                && !profile.starts_with("mixed_")
            {
                return None;
            }
            format!("{dataset}.btree.{operation}")
        }
        _ => return None,
    };
    all_names.iter().find(|other| **other == candidate).cloned()
}

/// Add unreviewed entries for executed cases missing from the sidecar section
/// of `target`, creating the sidecar when absent. Existing entries are kept.
pub(crate) fn scaffold(root: &Path, target: &str, executed: &[Executed]) -> Result<usize, String> {
    let path = root.join(SIDECAR_PATH);
    let mut sidecar = match fs::read_to_string(&path) {
        Ok(text) => Json::parse(&text)?,
        Err(_) => Json::Object(vec![
            ("schema".to_string(), Json::Str(SIDECAR_SCHEMA.to_string())),
            ("targets".to_string(), Json::Object(Vec::new())),
        ]),
    };
    let existing: BTreeSet<String> = entries(&sidecar, target)
        .iter()
        .filter_map(|e| e.get("name").and_then(Json::as_str).map(str::to_string))
        .collect();
    let all_names: Vec<String> = executed.iter().map(|e| e.name.clone()).collect();
    let new_entries: Vec<Json> = executed
        .iter()
        .filter(|e| !existing.contains(&e.name))
        .map(|e| derive_entry(&e.name, &e.timing, target, &all_names))
        .collect();
    let added = new_entries.len();
    let Json::Object(fields) = &mut sidecar else {
        return Err("bench inventory root must be an object".to_string());
    };
    if !fields
        .iter()
        .any(|(k, v)| k == "targets" && matches!(v, Json::Object(_)))
    {
        fields.retain(|(k, _)| k != "targets");
        fields.push(("targets".to_string(), Json::Object(Vec::new())));
    }
    let Some((_, Json::Object(targets))) = fields.iter_mut().find(|(k, _)| k == "targets") else {
        unreachable!("targets object ensured above")
    };
    match targets.iter_mut().find(|(k, _)| k == target) {
        Some((_, Json::Array(list))) => list.extend(new_entries),
        Some(_) => return Err(format!("bench inventory section {target} must be an array")),
        None => targets.push((target.to_string(), Json::Array(new_entries))),
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, sidecar.to_pretty()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMINGS: [&str; 3] = ["operation", "destructive-with-setup", "aggregate-readers"];
    const STATUSES: [&str; 2] = ["curated", "unreviewed"];
    const TAGS: [&str; 6] = [
        "benchmark",
        "unreviewed",
        "ordered",
        "native-control",
        "snapshot",
        "integer",
    ];

    fn listing() -> &'static str {
        "case,iterations\nnoise line\ncase\tordered.balanced.frost_count\toperation\ncase\tordered.balanced.std_count\toperation\ncase\tsnapshot.owned_load.1_readers.aggregate_ns_per_read\taggregate-readers\ncase\tsnapshot.raw_arc_swap_guard.1_readers.aggregate_ns_per_read\taggregate-readers\n"
    }

    fn sidecar_for(listing_text: &str, target: &str) -> Json {
        let executed = parse_listing(listing_text, &TIMINGS).unwrap();
        let names: Vec<String> = executed.iter().map(|e| e.name.clone()).collect();
        Json::Object(vec![
            ("schema".to_string(), Json::Str(SIDECAR_SCHEMA.to_string())),
            (
                "targets".to_string(),
                Json::Object(vec![(
                    target.to_string(),
                    Json::Array(
                        executed
                            .iter()
                            .map(|e| derive_entry(&e.name, &e.timing, target, &names))
                            .collect(),
                    ),
                )]),
            ),
        ])
    }

    #[test]
    fn listing_parser_rejects_empty_malformed_unknown_timing_and_duplicates() {
        assert!(
            parse_listing("case,iterations\n", &TIMINGS).unwrap_err()[0].contains("no case lines")
        );
        let errors = parse_listing("case\tonly_two_fields\ncase\ta b\toperation\ncase\tx\tguess\ncase\ty\toperation\ncase\ty\toperation\n", &TIMINGS).unwrap_err();
        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(errors[0].contains("expected case<TAB>name<TAB>timing"));
        assert!(errors[1].contains("malformed case name"));
        assert!(errors[2].contains("timing \"guess\""));
        assert!(errors[3].contains("duplicate executed case y"));
        let executed = parse_listing(listing(), &TIMINGS).unwrap();
        assert_eq!(executed.len(), 4);
        assert_eq!(executed[2].timing, "aggregate-readers");
    }

    #[test]
    fn derived_metadata_is_structural_and_names_controls() {
        let names = [
            "ordered.example.frost_subset",
            "ordered.example.frost_disjoint",
            "ordered.example.std_count",
            "ordered.example.std_subset",
        ]
        .map(str::to_string);
        let subset = derive_entry(&names[0], "operation", "benches/ordered_sets.rs", &names);
        assert_eq!(
            subset.get("control").and_then(Json::as_str),
            Some("ordered.example.std_subset")
        );
        let disjoint = derive_entry(&names[1], "operation", "benches/ordered_sets.rs", &names);
        assert_eq!(
            disjoint.get("control").and_then(Json::as_str),
            Some("unreviewed: no equivalent native control established")
        );
        let sidecar = sidecar_for(listing(), "benches/ordered_sets.rs");
        let list = entries(&sidecar, "benches/ordered_sets.rs");
        let frost = &list[0];
        assert_eq!(
            frost.get("control").unwrap().as_str(),
            Some("ordered.balanced.std_count")
        );
        assert_eq!(
            frost.get("scenario").unwrap().as_str(),
            Some("family ordered; dataset balanced; operation frost_count")
        );
        assert!(
            list[1]
                .get("tags")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .any(|t| t.as_str() == Some("native-control"))
        );
        assert_eq!(
            list[2].get("control").unwrap().as_str(),
            Some("unreviewed: no equivalent native control established")
        );
        let targets: BTreeSet<String> = ["benches/ordered_sets.rs".to_string()]
            .into_iter()
            .collect();
        assert_eq!(
            validate_sidecar(&sidecar, &targets, &TIMINGS, &STATUSES, &TAGS),
            Vec::<String>::new()
        );
        let missing: BTreeSet<String> = [
            "benches/ordered_sets.rs".to_string(),
            "benches/y.rs".to_string(),
        ]
        .into_iter()
        .collect();
        let errors = validate_sidecar(&sidecar, &missing, &TIMINGS, &STATUSES, &TAGS);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("benches/y.rs has no registered measured cases")),
            "{errors:?}"
        );
    }

    #[test]
    fn compare_rejects_new_operation_under_existing_prefix_and_stale_entries() {
        let sidecar = sidecar_for(listing(), "benches/x.rs");
        let executed = parse_listing(listing(), &TIMINGS).unwrap();
        assert_eq!(
            compare(&sidecar, "benches/x.rs", &executed),
            Vec::<String>::new()
        );
        // A new operation under the registered `ordered.balanced` prefix.
        let with_new = format!(
            "{}case\tordered.balanced.new_operation\toperation\n",
            listing()
        );
        let executed = parse_listing(&with_new, &TIMINGS).unwrap();
        let errors = compare(&sidecar, "benches/x.rs", &executed);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(errors[0].contains("ordered.balanced.new_operation is not registered"));
        // A registered case that no longer runs, and a timing change.
        let fewer = "case\tordered.balanced.frost_count\tdestructive-with-setup\n";
        let executed = parse_listing(fewer, &TIMINGS).unwrap();
        let errors = compare(&sidecar, "benches/x.rs", &executed);
        assert!(
            errors
                .iter()
                .any(|e| e
                    .contains("has timing destructive-with-setup but is registered as operation")),
            "{errors:?}"
        );
        assert_eq!(
            errors.iter().filter(|e| e.contains("stale entry")).count(),
            3,
            "{errors:?}"
        );
    }

    #[test]
    fn scaffold_adds_only_missing_entries() {
        let root =
            std::env::temp_dir().join(format!("frost-bench-inventory-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let executed = parse_listing(listing(), &TIMINGS).unwrap();
        assert_eq!(scaffold(&root, "benches/x.rs", &executed).unwrap(), 4);
        assert_eq!(scaffold(&root, "benches/x.rs", &executed).unwrap(), 0);
        let with_new = format!(
            "{}case\tordered.balanced.new_operation\toperation\n",
            listing()
        );
        let executed = parse_listing(&with_new, &TIMINGS).unwrap();
        assert_eq!(scaffold(&root, "benches/x.rs", &executed).unwrap(), 1);
        let sidecar = load_sidecar(&root).unwrap();
        assert_eq!(entries(&sidecar, "benches/x.rs").len(), 5);
        assert_eq!(
            compare(&sidecar, "benches/x.rs", &executed),
            Vec::<String>::new()
        );
        let _ = fs::remove_dir_all(&root);
    }
}
