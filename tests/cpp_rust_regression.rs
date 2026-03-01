use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const CPP_BIN_ENV: &str = "RVMPARSER_CPP_BIN";
const RUST_BIN_ENV: &str = "CARGO_BIN_EXE_rvm-rs";
const GEOMETRY_KINDS: [&str; 11] = [
    "Pyramid",
    "Box",
    "RectangularTorus",
    "CircularTorus",
    "EllipticalDish",
    "SphericalDish",
    "Snout",
    "Cylinder",
    "Sphere",
    "FacetGroup",
    "Line",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct StatsSnapshot {
    file_count: Option<usize>,
    model_count: Option<usize>,
    group_count: usize,
    geometry_count: usize,
    geometry_type_counts: BTreeMap<String, usize>,
}

#[derive(Debug)]
struct CollectedStats {
    snapshot: StatsSnapshot,
    raw_output: String,
}

#[test]
fn test_cpp_rust_consistency_test_rvm() {
    assert_cpp_rust_consistency("test.rvm");
}

#[test]
fn test_cpp_rust_consistency_wall_rvm() {
    assert_cpp_rust_consistency("wall.rvm");
}

#[test]
fn test_cpp_rust_consistency_test_rvm_with_keep_regex() {
    assert_cpp_rust_consistency_with_args("test.rvm", &["--keep-regex=^$".to_string()]);
}

#[test]
fn test_cpp_rust_consistency_test_rvm_with_discard_groups() {
    let tags_file = temp_tag_file("NOT_EXISTING_GROUP_NAME_12345\n");
    let arg = format!("--discard-groups={}", tags_file.display());
    assert_cpp_rust_consistency_with_args("test.rvm", &[arg]);
    let _ = std::fs::remove_file(tags_file);
}

fn assert_cpp_rust_consistency(file_name: &str) {
    assert_cpp_rust_consistency_with_args(file_name, &[]);
}

fn assert_cpp_rust_consistency_with_args(file_name: &str, extra_args: &[String]) {
    let Some(cpp_bin) = optional_bin_path(CPP_BIN_ENV) else {
        eprintln!(
            "skip cpp/rust regression: env var `{}` is unset or invalid",
            CPP_BIN_ENV
        );
        return;
    };

    let file_path = regression_file(file_name);
    assert!(
        file_path.exists(),
        "regression input does not exist: {}",
        file_path.display()
    );

    let cpp = run_cpp_and_collect(&cpp_bin, &file_path, extra_args);
    let rust = run_rust_and_collect(&file_path, extra_args);

    assert_snapshots_equal(file_name, &cpp, &rust);
}

fn regression_file(file_name: &str) -> PathBuf {
    Path::new("tests").join("test-data").join(file_name)
}

fn run_cpp_and_collect(cpp_bin: &Path, file_path: &Path, extra_args: &[String]) -> CollectedStats {
    let output = Command::new(cpp_bin)
        .arg(file_path)
        .args(extra_args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run C++ parser at {}: {err}", cpp_bin.display()));

    let raw_output = merge_output(&output);
    if !output.status.success() {
        panic!(
            "C++ parser failed for {} with status {:?}\n{}",
            file_path.display(),
            output.status.code(),
            raw_output
        );
    }

    let snapshot = parse_cpp_stats(&raw_output).unwrap_or_else(|err| {
        panic!(
            "failed to parse C++ stats for {}: {err}\n{}",
            file_path.display(),
            raw_output
        )
    });

    CollectedStats {
        snapshot,
        raw_output,
    }
}

fn run_rust_and_collect(file_path: &Path, extra_args: &[String]) -> CollectedStats {
    let rust_bin = rust_bin_path();
    let output = Command::new(&rust_bin)
        .arg(file_path)
        .args(extra_args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run Rust parser at {}: {err}", rust_bin.display()));

    let raw_output = merge_output(&output);
    if !output.status.success() {
        panic!(
            "Rust parser failed for {} with status {:?}\n{}",
            file_path.display(),
            output.status.code(),
            raw_output
        );
    }

    let snapshot = parse_rust_stats(&raw_output).unwrap_or_else(|err| {
        panic!(
            "failed to parse Rust stats for {}: {err}\n{}",
            file_path.display(),
            raw_output
        )
    });

    CollectedStats {
        snapshot,
        raw_output,
    }
}

fn required_bin_path(env_name: &str) -> PathBuf {
    let raw = env::var(env_name).unwrap_or_else(|_| {
        panic!(
            "required env var `{}` is not set. Please set it to an absolute executable path.",
            env_name
        )
    });
    let path = PathBuf::from(raw);
    assert!(
        path.exists(),
        "path from `{}` does not exist: {}",
        env_name,
        path.display()
    );
    path
}

fn optional_bin_path(env_name: &str) -> Option<PathBuf> {
    let raw = env::var(env_name).ok()?;
    let path = PathBuf::from(raw);
    if path.exists() { Some(path) } else { None }
}

fn rust_bin_path() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_rvm-rs") {
        let path = PathBuf::from(path);
        assert!(
            path.exists(),
            "binary from compile-time env `CARGO_BIN_EXE_rvm-rs` does not exist: {}",
            path.display()
        );
        return path;
    }

    if let Some(path) = option_env!("CARGO_BIN_EXE_rvm_rs") {
        let path = PathBuf::from(path);
        assert!(
            path.exists(),
            "binary from compile-time env `CARGO_BIN_EXE_rvm_rs` does not exist: {}",
            path.display()
        );
        return path;
    }

    required_bin_path(RUST_BIN_ENV)
}

fn merge_output(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{stdout}\n{stderr}")
}

fn parse_cpp_stats(output: &str) -> Result<StatsSnapshot, String> {
    let mut in_stats = false;
    let mut group_count = None;
    let mut geometry_count = None;
    let mut geometry_type_counts = BTreeMap::new();

    for line in output.lines() {
        let normalized = strip_logger_prefix(line);
        if normalized == "Stats:" {
            in_stats = true;
            continue;
        }
        if !in_stats {
            continue;
        }

        let Some((label, count)) = split_label_and_count(normalized) else {
            continue;
        };
        let Some(kind) = normalize_label(label) else {
            continue;
        };

        match kind {
            "Groups" => group_count = Some(count),
            "Geometries" => geometry_count = Some(count),
            "Pyramid" | "Box" | "RectangularTorus" | "CircularTorus" | "EllipticalDish"
            | "SphericalDish" | "Snout" | "Cylinder" | "Sphere" | "FacetGroup" | "Line" => {
                geometry_type_counts.insert(kind.to_string(), count);
            }
            _ => {}
        }
    }

    let group_count = group_count.ok_or_else(|| "missing `Groups` in C++ output".to_string())?;
    let geometry_count =
        geometry_count.ok_or_else(|| "missing `Geometries` in C++ output".to_string())?;

    Ok(StatsSnapshot {
        file_count: None,
        model_count: None,
        group_count,
        geometry_count,
        geometry_type_counts,
    })
}

fn parse_rust_stats(output: &str) -> Result<StatsSnapshot, String> {
    let mut file_count = None;
    let mut model_count = None;
    let mut group_count = None;
    let mut geometry_count = None;
    let mut geometry_type_counts = BTreeMap::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some((label, count)) = split_label_and_count(trimmed) else {
            continue;
        };
        let Some(kind) = normalize_label(label) else {
            continue;
        };

        match kind {
            "Files" => file_count = Some(count),
            "Models" => model_count = Some(count),
            "Groups" => group_count = Some(count),
            "Geometries" => geometry_count = Some(count),
            "Pyramid" | "Box" | "RectangularTorus" | "CircularTorus" | "EllipticalDish"
            | "SphericalDish" | "Snout" | "Cylinder" | "Sphere" | "FacetGroup" | "Line" => {
                geometry_type_counts.insert(kind.to_string(), count);
            }
            _ => {}
        }
    }

    let file_count = file_count.ok_or_else(|| "missing `Files` in Rust output".to_string())?;
    let model_count = model_count.ok_or_else(|| "missing `Models` in Rust output".to_string())?;
    let group_count = group_count.ok_or_else(|| "missing `Groups` in Rust output".to_string())?;
    let geometry_count =
        geometry_count.ok_or_else(|| "missing `Geometries` in Rust output".to_string())?;

    Ok(StatsSnapshot {
        file_count: Some(file_count),
        model_count: Some(model_count),
        group_count,
        geometry_count,
        geometry_type_counts,
    })
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

fn strip_logger_prefix(line: &str) -> &str {
    let trimmed = line.trim();
    for prefix in ["[I]", "[W]", "[E]"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.trim_start();
        }
    }
    trimmed
}

fn normalize_label(label: &str) -> Option<&'static str> {
    let compact = label
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();

    match compact.as_str() {
        "files" => Some("Files"),
        "models" => Some("Models"),
        "groups" => Some("Groups"),
        "geometries" => Some("Geometries"),
        "pyramid" | "pyramids" => Some("Pyramid"),
        "box" | "boxes" => Some("Box"),
        "rectangulartorus" | "rectangulartori" => Some("RectangularTorus"),
        "circulartorus" | "circulartori" => Some("CircularTorus"),
        "ellipticaldish" | "ellipticaldishes" => Some("EllipticalDish"),
        "sphericaldish" | "sphericaldishes" => Some("SphericalDish"),
        "snout" | "snouts" => Some("Snout"),
        "cylinder" | "cylinders" => Some("Cylinder"),
        "sphere" | "spheres" => Some("Sphere"),
        "line" | "lines" => Some("Line"),
        "facetgroup" | "facetgroups" => Some("FacetGroup"),
        _ => None,
    }
}

fn assert_snapshots_equal(file_name: &str, cpp: &CollectedStats, rust: &CollectedStats) {
    let mut mismatches = Vec::new();

    if let (Some(cpp_files), Some(rust_files)) = (cpp.snapshot.file_count, rust.snapshot.file_count)
    {
        if cpp_files != rust_files {
            mismatches.push(format!(
                "Files mismatch: cpp={}, rust={}",
                cpp_files, rust_files
            ));
        }
    }

    if let (Some(cpp_models), Some(rust_models)) =
        (cpp.snapshot.model_count, rust.snapshot.model_count)
    {
        if cpp_models != rust_models {
            mismatches.push(format!(
                "Models mismatch: cpp={}, rust={}",
                cpp_models, rust_models
            ));
        }
    }

    if cpp.snapshot.group_count != rust.snapshot.group_count {
        mismatches.push(format!(
            "Groups mismatch: cpp={}, rust={}",
            cpp.snapshot.group_count, rust.snapshot.group_count
        ));
    }

    if cpp.snapshot.geometry_count != rust.snapshot.geometry_count {
        mismatches.push(format!(
            "Geometries mismatch: cpp={}, rust={}",
            cpp.snapshot.geometry_count, rust.snapshot.geometry_count
        ));
    }

    let cpp_geometry_counts = with_zero_geometry_kinds(&cpp.snapshot.geometry_type_counts);
    let rust_geometry_counts = with_zero_geometry_kinds(&rust.snapshot.geometry_type_counts);
    if cpp_geometry_counts != rust_geometry_counts {
        mismatches.push(format!(
            "Geometry type counts mismatch: cpp={:?}, rust={:?}",
            cpp_geometry_counts, rust_geometry_counts
        ));
    }

    if !mismatches.is_empty() {
        panic!(
            "C++/Rust regression mismatch for {file_name}\n{}\n\ncpp snapshot: {:#?}\nrust snapshot: {:#?}\n\ncpp output:\n{}\n\nrust output:\n{}",
            mismatches.join("\n"),
            cpp.snapshot,
            rust.snapshot,
            cpp.raw_output,
            rust.raw_output
        );
    }
}

fn with_zero_geometry_kinds(input: &BTreeMap<String, usize>) -> BTreeMap<String, usize> {
    let mut result = BTreeMap::new();
    for kind in GEOMETRY_KINDS {
        result.insert(kind.to_string(), *input.get(kind).unwrap_or(&0));
    }
    result
}

fn temp_tag_file(content: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rvm_rs_cpp_rust_tags_{unique}.txt"));
    std::fs::write(&path, content).expect("write temp tag file");
    path
}
