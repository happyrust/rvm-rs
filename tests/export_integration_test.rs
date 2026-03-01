use rvm_rs::store::geometry::{Box as GeoBox, Cylinder, GeometryKind, Sphere};
use rvm_rs::store::{GroupNode, NodeKind};
use rvm_rs::{
    traverse, GltfExportOptions, GltfExporter, JsonExporter, ObjExportOptions, ObjExporter, Store,
};
use std::fs;

fn create_test_store() -> Store {
    let mut store = Store::new();

    // Create a simple scene with a group containing some geometries
    let group_node = GroupNode {
        name: store.intern_string("TestGroup"),
        translation: glam::Vec3::new(0.0, 0.0, 0.0),
        material: 0xFF0000,
        transparency: 0,
        id: -1,
        bbox_world: rvm_rs::math::BBox3::new(),
        first_geometry: None,
        attributes: vec![],
    };

    let node_id = store.new_node(NodeKind::Group(group_node));

    // Add a cylinder
    let cyl = GeometryKind::Cylinder(Cylinder {
        radius: 1.0,
        height: 2.0,
    });
    let geo_id = store.new_geometry(node_id, cyl);
    if let Some(geo) = store.get_geometry_mut(geo_id) {
        geo.color = 0xFF0000; // Red
        geo.transparency = 0;
        geo.color_rgb = geo.color;
        geo.transform = glam::Affine3A::from_translation(glam::Vec3::new(0.0, 0.0, 0.0));
    }

    // Add a sphere
    let sphere = GeometryKind::Sphere(Sphere { radius: 0.5 });
    let geo_id2 = store.new_geometry(node_id, sphere);
    if let Some(geo) = store.get_geometry_mut(geo_id2) {
        geo.color = 0x00FF00; // Green
        geo.transparency = 0;
        geo.color_rgb = geo.color;
        geo.transform = glam::Affine3A::from_translation(glam::Vec3::new(3.0, 0.0, 0.0));
    }

    // Add a box
    let b = GeometryKind::Box(GeoBox {
        lengths: [1.0, 1.0, 1.0],
    });
    let geo_id3 = store.new_geometry(node_id, b);
    if let Some(geo) = store.get_geometry_mut(geo_id3) {
        geo.color = 0x0000FF; // Blue
        geo.transparency = 50; // Semi-transparent
        geo.color_rgb = geo.color;
        geo.transform = glam::Affine3A::from_translation(glam::Vec3::new(-3.0, 0.0, 0.0));
    }

    store
}

#[test]
fn test_obj_export_integration() {
    let mut store = create_test_store();
    let temp_dir = std::env::temp_dir();
    let obj_path = temp_dir.join("test_integration.obj");
    let mtl_path = temp_dir.join("test_integration.mtl");

    // Export to OBJ
    let options = ObjExportOptions {
        include_normals: true,
        group_bounding_boxes: false,
        tolerance: 0.1,
    };

    let mut exporter = ObjExporter::new(obj_path.to_str().unwrap(), options).unwrap();
    traverse(&mut store, &mut exporter);
    exporter.finish().unwrap();

    // Verify files were created
    assert!(obj_path.exists(), "OBJ file should exist");
    assert!(mtl_path.exists(), "MTL file should exist");

    // Read and verify OBJ file contains expected content
    let obj_content = fs::read_to_string(&obj_path).unwrap();
    assert!(
        obj_content.contains("mtllib"),
        "OBJ should reference MTL file"
    );
    assert!(obj_content.contains("v "), "OBJ should contain vertices");
    assert!(obj_content.contains("vn "), "OBJ should contain normals");
    assert!(obj_content.contains("f "), "OBJ should contain faces");
    assert!(obj_content.contains("usemtl"), "OBJ should use materials");

    // Read and verify MTL file
    let mtl_content = fs::read_to_string(&mtl_path).unwrap();
    assert!(
        mtl_content.contains("newmtl"),
        "MTL should define materials"
    );
    assert!(mtl_content.contains("Kd"), "MTL should have diffuse color");
    assert!(mtl_content.contains("d "), "MTL should have transparency");

    // Cleanup
    let _ = fs::remove_file(&obj_path);
    let _ = fs::remove_file(&mtl_path);
}

#[test]
fn test_json_export_integration() {
    let mut store = create_test_store();
    let temp_dir = std::env::temp_dir();
    let json_path = temp_dir.join("test_integration.json");

    // Export to JSON
    let mut exporter = JsonExporter::new();
    traverse(&mut store, &mut exporter);
    exporter.write_to_file(json_path.to_str().unwrap()).unwrap();

    // Verify file was created
    assert!(json_path.exists(), "JSON file should exist");

    // Read and verify JSON content
    let json_content = fs::read_to_string(&json_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&json_content).unwrap();

    // Verify structure
    assert!(json.get("nodes").is_some(), "JSON should have nodes");

    // Cleanup
    let _ = fs::remove_file(&json_path);
}

#[test]
fn test_gltf_export_integration() {
    let mut store = create_test_store();
    let temp_dir = std::env::temp_dir();
    let gltf_path = temp_dir.join("test_integration.gltf");

    // Export to GLTF
    let options = GltfExportOptions {
        center_model: false,
        rotate_z_to_y: false,
        include_attributes: false,
        merge_geometries: false,
        binary_format: false,
        tolerance: 0.1,
    };

    let mut exporter = GltfExporter::new(options);
    traverse(&mut store, &mut exporter);
    exporter.write_to_file(gltf_path.to_str().unwrap()).unwrap();

    // Verify file was created
    assert!(gltf_path.exists(), "GLTF file should exist");

    // Read and verify GLTF content
    let gltf_content = fs::read_to_string(&gltf_path).unwrap();
    let gltf: serde_json::Value = serde_json::from_str(&gltf_content).unwrap();

    // Verify GLTF structure
    assert!(gltf.get("asset").is_some(), "GLTF should have asset");
    assert!(gltf.get("scenes").is_some(), "GLTF should have scenes");
    assert!(gltf.get("nodes").is_some(), "GLTF should have nodes");
    assert!(gltf.get("meshes").is_some(), "GLTF should have meshes");
    assert!(
        gltf.get("materials").is_some(),
        "GLTF should have materials"
    );
    assert!(
        gltf.get("accessors").is_some(),
        "GLTF should have accessors"
    );
    assert!(
        gltf.get("bufferViews").is_some(),
        "GLTF should have bufferViews"
    );
    assert!(gltf.get("buffers").is_some(), "GLTF should have buffers");

    // Verify version
    let version = gltf["asset"]["version"].as_str().unwrap();
    assert_eq!(version, "2.0", "GLTF version should be 2.0");

    // Cleanup
    let _ = fs::remove_file(&gltf_path);
}

#[test]
fn test_glb_export_integration() {
    let mut store = create_test_store();
    let temp_dir = std::env::temp_dir();
    let glb_path = temp_dir.join("test_integration.glb");

    // Export to GLB
    let options = GltfExportOptions {
        center_model: false,
        rotate_z_to_y: false,
        include_attributes: false,
        merge_geometries: false,
        binary_format: true,
        tolerance: 0.1,
    };

    let mut exporter = GltfExporter::new(options);
    traverse(&mut store, &mut exporter);
    exporter.write_to_file(glb_path.to_str().unwrap()).unwrap();

    // Verify file was created
    assert!(glb_path.exists(), "GLB file should exist");

    // Read and verify GLB binary header
    let glb_data = fs::read(&glb_path).unwrap();
    assert!(glb_data.len() >= 12, "GLB should have at least header");

    // Verify magic number "glTF"
    assert_eq!(
        &glb_data[0..4],
        b"glTF",
        "GLB should start with magic number"
    );

    // Verify version (should be 2)
    let version = u32::from_le_bytes([glb_data[4], glb_data[5], glb_data[6], glb_data[7]]);
    assert_eq!(version, 2, "GLB version should be 2");

    // Cleanup
    let _ = fs::remove_file(&glb_path);
}

#[test]
fn test_multiple_export_formats() {
    let mut store = create_test_store();
    let temp_dir = std::env::temp_dir();

    let obj_path = temp_dir.join("test_multi.obj");
    let json_path = temp_dir.join("test_multi.json");
    let gltf_path = temp_dir.join("test_multi.gltf");

    // Export to all formats
    let mut obj_exporter =
        ObjExporter::new(obj_path.to_str().unwrap(), ObjExportOptions::default()).unwrap();
    traverse(&mut store, &mut obj_exporter);
    obj_exporter.finish().unwrap();

    let mut json_exporter = JsonExporter::new();
    traverse(&mut store, &mut json_exporter);
    json_exporter
        .write_to_file(json_path.to_str().unwrap())
        .unwrap();

    let mut gltf_exporter = GltfExporter::new(GltfExportOptions::default());
    traverse(&mut store, &mut gltf_exporter);
    gltf_exporter
        .write_to_file(gltf_path.to_str().unwrap())
        .unwrap();

    // Verify all files exist
    assert!(obj_path.exists());
    assert!(json_path.exists());
    assert!(gltf_path.exists());

    // Cleanup
    let _ = fs::remove_file(&obj_path);
    let _ = fs::remove_file(temp_dir.join("test_multi.mtl"));
    let _ = fs::remove_file(&json_path);
    let _ = fs::remove_file(&gltf_path);
}
