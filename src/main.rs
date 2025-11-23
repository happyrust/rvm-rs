use memmap2::Mmap;
use rvm_rs::visitor::stats::StatsVisitor;
use rvm_rs::{parse_att, parse_rvm, traverse, Store};
use rvm_rs::{GltfExportOptions, GltfExporter, JsonExporter, ObjExportOptions, ObjExporter};
use std::env;
use std::fs::File;
use std::io::Read;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    // Check for help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage(&args[0]);
        process::exit(0);
    }

    let file_path = &args[1];

    // Parse export options
    let mut export_obj: Option<String> = None;
    let mut export_json: Option<String> = None;
    let mut export_gltf: Option<String> = None;
    let mut center_model = false;
    let mut rotate_z_to_y = false;
    let mut include_attributes = false;
    let mut merge_geometries = false;
    let mut tolerance = 0.1;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--export-obj" => {
                if i + 1 < args.len() {
                    export_obj = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --export-obj requires a path argument");
                    process::exit(1);
                }
            }
            "--export-json" => {
                if i + 1 < args.len() {
                    export_json = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --export-json requires a path argument");
                    process::exit(1);
                }
            }
            "--export-gltf" => {
                if i + 1 < args.len() {
                    export_gltf = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --export-gltf requires a path argument");
                    process::exit(1);
                }
            }
            "--center" => {
                center_model = true;
                i += 1;
            }
            "--rotate-z-to-y" => {
                rotate_z_to_y = true;
                i += 1;
            }
            "--include-attributes" => {
                include_attributes = true;
                i += 1;
            }
            "--merge-geometries" => {
                merge_geometries = true;
                i += 1;
            }
            "--tolerance" => {
                if i + 1 < args.len() {
                    tolerance = args[i + 1].parse().unwrap_or(0.1);
                    i += 2;
                } else {
                    eprintln!("Error: --tolerance requires a numeric argument");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Error: Unknown option: {}", args[i]);
                process::exit(1);
            }
        }
    }

    // Determine file type by extension
    let is_rvm = file_path.ends_with(".rvm") || file_path.ends_with(".RVM");
    let is_att = file_path.ends_with(".txt") || file_path.ends_with(".att");

    if !is_rvm && !is_att {
        eprintln!("Error: Unsupported file type. Expected .rvm, .txt, or .att file");
        process::exit(1);
    }

    let mut store = Store::new();

    if is_rvm {
        // Parse RVM file using memory mapping
        match parse_rvm_file(file_path, &mut store) {
            Ok(_) => {
                println!("Successfully parsed RVM file: {}", file_path);
            }
            Err(e) => {
                eprintln!("Error parsing RVM file: {}", e);
                process::exit(1);
            }
        }
    } else if is_att {
        // Parse attribute file
        match parse_att_file(file_path, &mut store) {
            Ok(_) => {
                println!("Successfully parsed attribute file: {}", file_path);
            }
            Err(e) => {
                eprintln!("Error parsing attribute file: {}", e);
                process::exit(1);
            }
        }
    }

    // Collect and display statistics
    let mut stats = StatsVisitor::new();
    traverse(&store, &mut stats);
    stats.print_stats();

    // Perform exports
    if let Some(obj_path) = export_obj {
        println!("Exporting to OBJ: {}", obj_path);
        let options = ObjExportOptions {
            include_normals: true,
            group_bounding_boxes: false,
            tolerance,
        };
        match ObjExporter::new(&obj_path, options) {
            Ok(mut exporter) => {
                traverse(&store, &mut exporter);
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

    if let Some(json_path) = export_json {
        println!("Exporting to JSON: {}", json_path);
        let mut exporter = JsonExporter::new();
        traverse(&store, &mut exporter);
        if let Err(e) = exporter.write_to_file(&json_path) {
            eprintln!("Error exporting to JSON: {}", e);
            process::exit(1);
        }
        println!("Successfully exported to JSON");
    }

    if let Some(gltf_path) = export_gltf {
        println!("Exporting to GLTF/GLB: {}", gltf_path);
        let binary_format = gltf_path.ends_with(".glb");
        let options = GltfExportOptions {
            center_model,
            rotate_z_to_y,
            include_attributes,
            merge_geometries,
            binary_format,
            tolerance,
        };
        let mut exporter = GltfExporter::new(options);
        traverse(&store, &mut exporter);
        if let Err(e) = exporter.write_to_file(&gltf_path) {
            eprintln!("Error exporting to GLTF/GLB: {}", e);
            process::exit(1);
        }
        println!("Successfully exported to GLTF/GLB");
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
    println!("Usage: {} <file_path> [OPTIONS]", program_name);
    println!();
    println!("Arguments:");
    println!("  <file_path>              Path to RVM file (.rvm) or attribute file (.txt, .att)");
    println!();
    println!("Export Options:");
    println!("  --export-obj <path>      Export to OBJ format (creates .obj and .mtl files)");
    println!("  --export-json <path>     Export to JSON format");
    println!("  --export-gltf <path>     Export to GLTF/GLB format (.gltf or .glb)");
    println!();
    println!("Export Settings:");
    println!("  --center                 Center the model at origin");
    println!("  --rotate-z-to-y          Rotate Z-axis to Y-axis (coordinate system conversion)");
    println!("  --include-attributes     Include node attributes in export");
    println!("  --merge-geometries       Merge geometries with same material");
    println!("  --tolerance <value>      Tessellation tolerance (default: 0.1)");
    println!();
    println!("General Options:");
    println!("  --help, -h               Display this help message");
    println!();
    println!("Examples:");
    println!("  {} model.rvm", program_name);
    println!("  {} model.rvm --export-obj output.obj", program_name);
    println!("  {} model.rvm --export-json output.json", program_name);
    println!("  {} model.rvm --export-gltf output.gltf", program_name);
    println!(
        "  {} model.rvm --export-gltf output.glb --center",
        program_name
    );
    println!(
        "  {} model.rvm --export-obj out.obj --export-json out.json",
        program_name
    );
}
