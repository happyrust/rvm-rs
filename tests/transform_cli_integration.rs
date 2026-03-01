use rvm_rs::transform::{flatten_keep_from_file, flatten_regex};
use rvm_rs::visitor::stats::StatsVisitor;
use rvm_rs::{parse_rvm, traverse, Store};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ParsedStats {
    groups: usize,
    geometries: usize,
}

#[test]
fn cli_discard_groups_removes_target_name_from_json() {
    let source_store = parse_store_from_test_data("test.rvm");
    let groups = collect_group_names_from_store(&source_store);
    let discard_name = groups
        .first()
        .cloned()
        .expect("test data should contain at least one group");

    let tags_file = write_temp_file(&format!("{discard_name}\n"));
    let output_json = temp_path("discard_output.json");

    let args = vec![
        "tests/test-data/test.rvm".to_string(),
        format!("--discard-groups={}", tags_file.display()),
        format!("--export-json={}", output_json.display()),
    ];
    let output = run_cli(&args);
    let _ = std::fs::remove_file(&tags_file);
    assert!(
        output.status.success(),
        "cli failed:\n{}",
        merge_output(&output)
    );

    let json = read_json(&output_json);
    let _ = std::fs::remove_file(&output_json);
    let exported_names = collect_group_names_from_json(&json);
    assert!(
        !exported_names.contains(&discard_name),
        "discarded group should not appear in JSON: {discard_name}"
    );
}

#[test]
fn cli_keep_regex_stats_match_library_transform() {
    let mut expected_store = parse_store_from_test_data("test.rvm");
    flatten_regex(&mut expected_store, "^$").expect("library flatten_regex should succeed");
    let expected_stats = stats_from_store(&mut expected_store);

    let args = vec![
        "tests/test-data/test.rvm".to_string(),
        "--keep-regex=^$".to_string(),
    ];
    let output = run_cli(&args);
    assert!(
        output.status.success(),
        "cli failed:\n{}",
        merge_output(&output)
    );

    let cli_stats = parse_stats_from_output(&merge_output(&output));
    assert_eq!(cli_stats.groups, expected_stats.groups);
    assert_eq!(cli_stats.geometries, expected_stats.geometries);
}

#[test]
fn cli_keep_groups_stats_and_json_match_library_transform() {
    let source_store = parse_store_from_test_data("test.rvm");
    let (target_name, dropped_candidate) = choose_keep_and_drop_candidates(&source_store);

    let tags_file = write_temp_file(&format!("{target_name}\n"));
    let (mut expected_store, _) = flatten_keep_from_file(&source_store, &tags_file)
        .expect("library flatten_keep should succeed");
    let expected_stats = stats_from_store(&mut expected_store);
    let expected_names: HashSet<String> = collect_group_names_from_store(&expected_store)
        .into_iter()
        .collect();

    let output_json = temp_path("keep_output.json");
    let args = vec![
        "tests/test-data/test.rvm".to_string(),
        format!("--keep-groups={}", tags_file.display()),
        format!("--export-json={}", output_json.display()),
    ];
    let output = run_cli(&args);
    let _ = std::fs::remove_file(&tags_file);
    assert!(
        output.status.success(),
        "cli failed:\n{}",
        merge_output(&output)
    );

    let cli_stats = parse_stats_from_output(&merge_output(&output));
    assert_eq!(cli_stats.groups, expected_stats.groups);
    assert_eq!(cli_stats.geometries, expected_stats.geometries);

    let json = read_json(&output_json);
    let _ = std::fs::remove_file(&output_json);
    let exported_names = collect_group_names_from_json(&json);
    assert!(exported_names.contains(&target_name));
    if let Some(dropped_name) = dropped_candidate {
        if !expected_names.contains(&dropped_name) {
            assert!(!exported_names.contains(&dropped_name));
        }
    }
}

fn choose_keep_and_drop_candidates(store: &Store) -> (String, Option<String>) {
    let mut by_depth = Vec::new();
    for &root in store.roots() {
        collect_group_names_with_depth(store, root, 0, &mut by_depth);
    }

    let mut depth2_or_more = by_depth
        .iter()
        .filter(|(_, depth)| *depth >= 2)
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    depth2_or_more.sort();
    depth2_or_more.dedup();

    let target = depth2_or_more
        .first()
        .cloned()
        .or_else(|| by_depth.first().map(|(name, _)| name.clone()))
        .expect("expected at least one group in test data");

    let dropped = depth2_or_more.into_iter().find(|name| name != &target);
    (target, dropped)
}

fn collect_group_names_with_depth(
    store: &Store,
    node_id: rvm_rs::store::NodeId,
    depth: usize,
    out: &mut Vec<(String, usize)>,
) {
    let Some(node) = store.get_node(node_id) else {
        return;
    };
    if let rvm_rs::store::NodeKind::Group(group) = &node.kind {
        out.push((store.get_string(group.name).to_string(), depth));
    }
    let mut child = node.first_child;
    while let Some(child_id) = child {
        collect_group_names_with_depth(store, child_id, depth + 1, out);
        child = store.get_node(child_id).and_then(|n| n.next);
    }
}

fn parse_store_from_test_data(file_name: &str) -> Store {
    let mut store = Store::new();
    let path = Path::new("tests").join("test-data").join(file_name);
    let data = std::fs::read(&path).expect("failed to read test data");
    parse_rvm(&data, &mut store).expect("parse_rvm should succeed");
    store
}

fn collect_group_names_from_store(store: &Store) -> Vec<String> {
    let mut names = Vec::new();
    for &root in store.roots() {
        collect_group_names_recurse(store, root, &mut names);
    }
    names.sort();
    names.dedup();
    names
}

fn collect_group_names_recurse(
    store: &Store,
    node_id: rvm_rs::store::NodeId,
    names: &mut Vec<String>,
) {
    let Some(node) = store.get_node(node_id) else {
        return;
    };
    if let rvm_rs::store::NodeKind::Group(group) = &node.kind {
        names.push(store.get_string(group.name).to_string());
    }
    let mut child = node.first_child;
    while let Some(child_id) = child {
        collect_group_names_recurse(store, child_id, names);
        child = store.get_node(child_id).and_then(|n| n.next);
    }
}

fn stats_from_store(store: &mut Store) -> ParsedStats {
    let mut visitor = StatsVisitor::new();
    traverse(store, &mut visitor);
    ParsedStats {
        groups: visitor.group_count,
        geometries: visitor.geometry_count,
    }
}

fn run_cli(args: &[String]) -> Output {
    let bin = rust_bin_path();
    Command::new(bin)
        .args(args)
        .output()
        .expect("failed to execute rust cli")
}

fn rust_bin_path() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_rvm-rs") {
        return PathBuf::from(path);
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_rvm_rs") {
        return PathBuf::from(path);
    }
    let runtime = std::env::var("CARGO_BIN_EXE_rvm-rs")
        .or_else(|_| std::env::var("CARGO_BIN_EXE_rvm_rs"))
        .expect("missing rust binary path env");
    PathBuf::from(runtime)
}

fn merge_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}\n{stderr}")
}

fn parse_stats_from_output(output: &str) -> ParsedStats {
    let mut groups = None;
    let mut geometries = None;
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((label, value)) = split_label_and_count(trimmed) {
            match label {
                "Groups" => groups = Some(value),
                "Geometries" => geometries = Some(value),
                _ => {}
            }
        }
    }
    ParsedStats {
        groups: groups.expect("missing Groups in output"),
        geometries: geometries.expect("missing Geometries in output"),
    }
}

fn split_label_and_count(line: &str) -> Option<(&str, usize)> {
    let first_digit_idx = line.find(|ch: char| ch.is_ascii_digit())?;
    let label = line[..first_digit_idx].trim().trim_end_matches(':').trim();
    if label.is_empty() {
        return None;
    }
    let count = parse_first_usize(&line[first_digit_idx..])?;
    Some((label, count))
}

fn parse_first_usize(input: &str) -> Option<usize> {
    let mut start = None;
    let mut end = 0usize;
    for (idx, ch) in input.char_indices() {
        if ch.is_ascii_digit() {
            if start.is_none() {
                start = Some(idx);
            }
            end = idx + ch.len_utf8();
        } else if start.is_some() {
            break;
        }
    }
    let start = start?;
    input[start..end].parse::<usize>().ok()
}

fn write_temp_file(content: &str) -> PathBuf {
    let path = temp_path("transform_tags.txt");
    std::fs::write(&path, content).expect("write temp file");
    path
}

fn temp_path(suffix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("rvm_rs_{unique}_{suffix}"))
}

fn read_json(path: &Path) -> Value {
    let content = std::fs::read_to_string(path).expect("read json");
    serde_json::from_str(&content).expect("parse json")
}

fn collect_group_names_from_json(json: &Value) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(nodes) = json.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            collect_group_names_from_json_node(node, &mut names);
        }
    }
    names
}

fn collect_group_names_from_json_node(node: &Value, names: &mut HashSet<String>) {
    if let Some(name) = node.get("name").and_then(|v| v.as_str()) {
        names.insert(name.to_string());
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            collect_group_names_from_json_node(child, names);
        }
    }
}
