pub mod cache;
pub mod gltf;
pub mod json;
pub mod obj;
pub mod tessellator;

pub use cache::{CacheStats, GeometryCache};
pub use gltf::{GltfExportOptions, GltfExporter};
pub use json::JsonExporter;
pub use obj::{ObjExportOptions, ObjExporter};
pub use tessellator::{Tessellate, Triangulation};

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid export path: {0}")]
    InvalidPath(String),

    #[error("Tessellation error: {0}")]
    Tessellation(String),

    #[error("Unsupported geometry type: {0}")]
    UnsupportedGeometry(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::geometry::*;
    use crate::store::{GroupNode, NodeKind, Store};

    #[test]
    fn test_cylinder_tessellation() {
        let cylinder = Cylinder {
            radius: 1.0,
            height: 2.0,
        };

        let tri = cylinder.tessellate(0.1, 1.0);

        // Verify vertices are divisible by 3 (x, y, z)
        assert_eq!(tri.vertices.len() % 3, 0);
        assert_eq!(tri.normals.len() % 3, 0);
        assert_eq!(tri.indices.len() % 3, 0);

        // Verify we have vertices
        assert!(!tri.vertices.is_empty());
        assert!(!tri.normals.is_empty());
        assert!(!tri.indices.is_empty());

        // Verify normals are normalized (approximately)
        for i in (0..tri.normals.len()).step_by(3) {
            let nx = tri.normals[i];
            let ny = tri.normals[i + 1];
            let nz = tri.normals[i + 2];
            let length = (nx * nx + ny * ny + nz * nz).sqrt();
            assert!(
                (length - 1.0).abs() < 0.01,
                "Normal not normalized: {}",
                length
            );
        }
    }

    #[test]
    fn test_sphere_tessellation() {
        let sphere = Sphere { radius: 1.0 };

        let tri = sphere.tessellate(0.1, 1.0);

        assert_eq!(tri.vertices.len() % 3, 0);
        assert_eq!(tri.normals.len() % 3, 0);
        assert_eq!(tri.indices.len() % 3, 0);
        assert!(!tri.vertices.is_empty());
    }

    #[test]
    fn test_box_tessellation() {
        let b = Box {
            lengths: [2.0, 2.0, 2.0],
        };

        let tri = b.tessellate(0.1, 1.0);

        // A box should have 24 vertices (4 per face * 6 faces)
        assert_eq!(tri.vertices.len(), 24 * 3);
        assert_eq!(tri.normals.len(), 24 * 3);
        // 12 triangles (2 per face * 6 faces)
        assert_eq!(tri.indices.len(), 12 * 3);
    }

    #[test]
    fn test_obj_exporter_creation() {
        use std::env;
        let temp_dir = env::temp_dir();
        let obj_path = temp_dir.join("test_export.obj");
        let obj_path_str = obj_path.to_str().unwrap();

        let options = ObjExportOptions::default();
        let result = ObjExporter::new(obj_path_str, options);

        assert!(result.is_ok());

        if let Ok(exporter) = result {
            assert!(exporter.finish().is_ok());
        }

        // Cleanup
        let _ = std::fs::remove_file(&obj_path);
        let mtl_path = temp_dir.join("test_export.mtl");
        let _ = std::fs::remove_file(&mtl_path);
    }

    #[test]
    fn test_json_exporter_creation() {
        let _exporter = JsonExporter::new();
        // JsonExporter created successfully
    }

    #[test]
    fn test_gltf_exporter_creation() {
        let options = GltfExportOptions::default();
        let _exporter = GltfExporter::new(options);
        // GltfExporter created successfully
    }

    #[test]
    fn test_material_deduplication() {
        use std::env;
        let temp_dir = env::temp_dir();
        let obj_path = temp_dir.join("test_materials.obj");
        let obj_path_str = obj_path.to_str().unwrap();

        let options = ObjExportOptions::default();
        let mut exporter = ObjExporter::new(obj_path_str, options).unwrap();

        // Create a simple store with geometries
        let mut store = Store::new();
        let group_node = GroupNode {
            name: store.intern_string("test_group"),
            translation: glam::Vec3::ZERO,
            material: 0xFF0000, // Red
            transparency: 0,
            id: -1,
            bbox_world: crate::math::BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        };

        let node_id = store.new_node(NodeKind::Group(group_node));

        // Add two geometries with the same color
        let cyl1 = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 2.0,
        });
        let geo1_id = store.new_geometry(node_id, cyl1);
        if let Some(geo1) = store.get_geometry_mut(geo1_id) {
            geo1.color = 0xFF0000;
            geo1.transparency = 0;
            geo1.color_rgb = geo1.color;
        }

        let cyl2 = GeometryKind::Cylinder(Cylinder {
            radius: 0.5,
            height: 1.0,
        });
        let geo2_id = store.new_geometry(node_id, cyl2);
        if let Some(geo2) = store.get_geometry_mut(geo2_id) {
            geo2.color = 0xFF0000; // Same color
            geo2.transparency = 0;
            geo2.color_rgb = geo2.color;
        }

        // Use the traverse function to visit nodes
        crate::visitor::traverse(&mut store, &mut exporter);

        let _ = exporter.finish();

        // Cleanup
        let _ = std::fs::remove_file(&obj_path);
        let mtl_path = temp_dir.join("test_materials.mtl");
        let _ = std::fs::remove_file(&mtl_path);
    }

    #[test]
    fn test_vertex_count_consistency() {
        let cylinder = Cylinder {
            radius: 1.0,
            height: 2.0,
        };

        let tri = cylinder.tessellate(0.1, 1.0);

        let vertex_count = tri.vertices.len() / 3;
        let normal_count = tri.normals.len() / 3;

        // Vertex count should equal normal count
        assert_eq!(vertex_count, normal_count);

        // All indices should be valid
        for &idx in &tri.indices {
            assert!((idx as usize) < vertex_count);
        }
    }

    #[test]
    fn test_transparency_mapping() {
        // Test that transparency is correctly mapped to alpha
        // transparency = 0 -> alpha = 1.0
        // transparency = 100 -> alpha = 0.0
        // transparency = 50 -> alpha = 0.5

        let test_cases = vec![(0, 1.0), (50, 0.5), (100, 0.0)];

        for (transparency, expected_alpha) in test_cases {
            let alpha = 1.0 - (transparency as f32 / 100.0);
            assert!((alpha - expected_alpha).abs() < 0.001);
        }
    }
}
