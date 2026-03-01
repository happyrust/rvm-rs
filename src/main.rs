use memmap2::Mmap;
use rvm_rs::transform::{discard_groups_from_file, flatten_keep_from_file, flatten_regex};
use rvm_rs::visitor::stats::StatsVisitor;
use rvm_rs::{parse_att, parse_rvm, traverse, Store};
use rvm_rs::{GltfExportOptions, GltfExporter, JsonExporter, ObjExportOptions, ObjExporter};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process;

#[derive(Debug, Default)]
struct CliOptions {
    input_files: Vec<String>,
    export_obj: Option<String>,
    export_json: Option<String>,
    export_gltf: Option<String>,
    center_model: bool,
    rotate_z_to_y: bool,
    include_attributes: bool,
    merge_geometries: bool,
    tolerance: f32,
    discard_groups: Option<String>,
    keep_regex: Option<String>,
    keep_groups: Option<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage(&args[0]);
        process::exit(0);
    }

    let options = parse_cli_options(&args).unwrap_or_else(|err| {
        eprintln!("Error: {err}");
        process::exit(1);
    });

    let mut store = Store::new();

    for file_path in &options.input_files {
        let lower = file_path.to_ascii_lowercase();
        if lower.ends_with(".rvm") {
            match parse_rvm_file(file_path, &mut store) {
                Ok(_) => println!("Successfully parsed RVM file: {}", file_path),
                Err(e) => {
                    eprintln!("Error parsing RVM file {}: {}", file_path, e);
                    process::exit(1);
                }
            }
        } else if lower.ends_with(".txt") || lower.ends_with(".att") {
            match parse_att_file(file_path, &mut store) {
                Ok(_) => println!("Successfully parsed attribute file: {}", file_path),
                Err(e) => {
                    eprintln!("Error parsing attribute file {}: {}", file_path, e);
                    process::exit(1);
                }
            }
        } else {
            eprintln!(
                "Error: Unsupported input file type for {} (expected .rvm, .txt, or .att)",
                file_path
            );
            process::exit(1);
        }
    }

    if let Some(discard_file) = &options.discard_groups {
        match discard_groups_from_file(&mut store, Path::new(discard_file)) {
            Ok(result) => {
                println!(
                    "DiscardGroups: Read {} tags, discarded {} groups.",
                    result.tags_read, result.discarded_groups
                );
            }
            Err(e) => {
                eprintln!("Error applying --discard-groups: {}", e);
                process::exit(1);
            }
        }
    }

    if let Some(pattern) = &options.keep_regex {
        match flatten_regex(&mut store, pattern) {
            Ok(result) => {
                println!(
                    "FlattenRegex: nodes {} -> {}, geometries {} -> {}",
                    result.nodes_before,
                    result.nodes_after,
                    result.geometries_before,
                    result.geometries_after
                );
            }
            Err(e) => {
                eprintln!("Error applying --keep-regex: {}", e);
                process::exit(1);
            }
        }
    }

    if let Some(keep_file) = &options.keep_groups {
        match flatten_keep_from_file(&store, Path::new(keep_file)) {
            Ok((new_store, result)) => {
                println!(
                    "FlattenKeep: read {} tags (active {}), nodes {} -> {}, geometries {} -> {}",
                    result.tags_read,
                    result.active_tags,
                    result.nodes_before,
                    result.nodes_after,
                    result.geometries_before,
                    result.geometries_after
                );
                store = new_store;
            }
            Err(e) => {
                eprintln!("Error applying --keep-groups: {}", e);
                process::exit(1);
            }
        }
    }

    let mut stats = StatsVisitor::new();
    traverse(&mut store, &mut stats);
    stats.print_stats();

    if let Some(obj_path) = options.export_obj {
        println!("Exporting to OBJ: {}", obj_path);
        let options = ObjExportOptions {
            include_normals: true,
            group_bounding_boxes: false,
            tolerance: options.tolerance,
        };
        match ObjExporter::new(&obj_path, options) {
            Ok(mut exporter) => {
                traverse(&mut store, &mut exporter);
                if let Err(e) = exporter.finish() {
                    eprintln!("Error finishing OBJ export: {}", e);
                    process::exit(1);
                }
                println!("Successfully exported to OBJ");
            }
            Err(e) => {
                eprintln!("Error creating OBJ exporter: {}", e);
                process::exit(1);
            }
        }
    }

    if let Some(json_path) = options.export_json {
        println!("Exporting to JSON: {}", json_path);
        let mut exporter = JsonExporter::new();
        traverse(&mut store, &mut exporter);
        if let Err(e) = exporter.write_to_file(&json_path) {
            eprintln!("Error exporting to JSON: {}", e);
            process::exit(1);
        }
        println!("Successfully exported to JSON");
    }

    if let Some(gltf_path) = options.export_gltf {
        println!("Exporting to GLTF/GLB: {}", gltf_path);
        let binary_format = gltf_path.to_ascii_lowercase().ends_with(".glb");
        let gltf_options = GltfExportOptions {
            center_model: options.center_model,
            rotate_z_to_y: options.rotate_z_to_y,
            include_attributes: options.include_attributes,
            merge_geometries: options.merge_geometries,
            binary_format,
            tolerance: options.tolerance,
        };
        let mut exporter = GltfExporter::new(gltf_options);
        traverse(&mut store, &mut exporter);
        if let Err(e) = exporter.write_to_file(&gltf_path) {
            eprintln!("Error exporting to GLTF/GLB: {}", e);
            process::exit(1);
        }
        println!("Successfully exported to GLTF/GLB");
    }
}

fn parse_cli_options(args: &[String]) -> Result<CliOptions, String> {
    let mut options = CliOptions {
        tolerance: 0.1,
        ..CliOptions::default()
    };

    let mut i = 1usize;
    while i < args.len() {
        let arg = &args[i];

        if !arg.starts_with("--") {
            options.input_files.push(arg.clone());
            i += 1;
            continue;
        }

        let (key, inline_value) = split_option(arg);
        match key {
            "--export-obj" => {
                options.export_obj = Some(read_option_value(args, &mut i, key, inline_value)?)
            }
            "--export-json" => {
                options.export_json = Some(read_option_value(args, &mut i, key, inline_value)?)
            }
            "--export-gltf" => {
                options.export_gltf = Some(read_option_value(args, &mut i, key, inline_value)?)
            }
            "--discard-groups" => {
                options.discard_groups = Some(read_option_value(args, &mut i, key, inline_value)?)
            }
            "--keep-regex" => {
                options.keep_regex = Some(read_option_value(args, &mut i, key, inline_value)?)
            }
            "--keep-groups" => {
                options.keep_groups = Some(read_option_value(args, &mut i, key, inline_value)?)
            }
            "--tolerance" => {
                let value = read_option_value(args, &mut i, key, inline_value)?;
                options.tolerance = value
                    .parse::<f32>()
                    .map_err(|_| format!("{key} requires a numeric value, got `{value}`"))?;
            }
            "--center" => {
                options.center_model = parse_optional_bool_flag(inline_value)?;
                i += 1;
            }
            "--rotate-z-to-y" => {
                options.rotate_z_to_y = parse_optional_bool_flag(inline_value)?;
                i += 1;
            }
            "--include-attributes" => {
                options.include_attributes = parse_optional_bool_flag(inline_value)?;
                i += 1;
            }
            "--merge-geometries" => {
                options.merge_geometries = parse_optional_bool_flag(inline_value)?;
                i += 1;
            }
            "--help" | "-h" => {
                i += 1;
            }
            _ => {
                return Err(format!("Unknown option: {key}"));
            }
        }
    }

    if options.input_files.is_empty() {
        return Err("at least one input file is required".to_string());
    }

    Ok(options)
}

fn split_option(arg: &str) -> (&str, Option<String>) {
    if let Some((key, value)) = arg.split_once('=') {
        (key, Some(value.to_string()))
    } else {
        (arg, None)
    }
}

fn read_option_value(
    args: &[String],
    index: &mut usize,
    key: &str,
    inline_value: Option<String>,
) -> Result<String, String> {
    if let Some(value) = inline_value {
        if value.is_empty() {
            return Err(format!("{key} requires a non-empty value"));
        }
        *index += 1;
        return Ok(value);
    }

    if *index + 1 >= args.len() {
        return Err(format!("{key} requires a value"));
    }

    let value = args[*index + 1].clone();
    if value.starts_with("--") {
        return Err(format!("{key} requires a value"));
    }
    *index += 2;
    Ok(value)
}

fn parse_optional_bool_flag(inline_value: Option<String>) -> Result<bool, String> {
    let Some(value) = inline_value else {
        return Ok(true);
    };
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" => Ok(true),
        "0" | "false" | "no" => Ok(false),
        _ => Err(format!(
            "invalid boolean value `{}` (expected true/false/1/0/yes/no)",
            value
        )),
    }
}

fn parse_rvm_file(path: &str, store: &mut Store) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    parse_rvm(&mmap, store)?;
    Ok(())
}

fn parse_att_file(path: &str, store: &mut Store) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    parse_att(&content, store)?;
    Ok(())
}

fn print_usage(program_name: &str) {
    println!("RVM Parser - Rust implementation");
    println!();
    println!("Usage: {} <input files...> [OPTIONS]", program_name);
    println!();
    println!("Input Files:");
    println!("  .rvm                   Geometry file");
    println!("  .txt / .att            Attribute file");
    println!();
    println!("Hierarchy Options:");
    println!("  --discard-groups <file>  Discard groups from tag file");
    println!("  --keep-regex <regex>     Flatten hierarchy by regex (keep matching names)");
    println!("  --keep-groups <file>     Keep groups from tag file (pruned copy)");
    println!();
    println!("Export Options:");
    println!("  --export-obj <path>      Export to OBJ format (creates .obj and .mtl files)");
    println!("  --export-json <path>     Export to JSON format");
    println!("  --export-gltf <path>     Export to GLTF/GLB format (.gltf or .glb)");
    println!();
    println!("Export Settings:");
    println!("  --center[=bool]          Center the model at origin");
    println!("  --rotate-z-to-y[=bool]   Rotate Z-axis to Y-axis");
    println!("  --include-attributes[=bool] Include node attributes in GLTF");
    println!("  --merge-geometries[=bool]   Merge geometries with same material");
    println!("  --tolerance <value>      Tessellation tolerance (default: 0.1)");
    println!();
    println!("General Options:");
    println!("  --help, -h               Display this help message");
    println!();
    println!("Examples:");
    println!("  {} model.rvm", program_name);
    println!(
        "  {} model.rvm model.att --discard-groups=discard.txt --export-json out.json",
        program_name
    );
    println!(
        "  {} model.rvm --keep-regex \"^/SITE/.*\" --export-obj out.obj",
        program_name
    );
    println!(
        "  {} model.rvm --keep-groups keep.txt --export-gltf out.glb",
        program_name
    );
}
