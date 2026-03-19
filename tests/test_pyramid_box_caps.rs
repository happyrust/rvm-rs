// Test conditional cap generation for Pyramid and Box geometries

use rvm_rs::export::tessellator::Tessellate;
use rvm_rs::store::geometry::{Box, Pyramid};

#[test]
fn test_pyramid_cap_analysis() {
    let pyramid = Pyramid {
        bottom: [2.0, 2.0],
        top: [1.0, 1.0],
        offset: [0.0, 0.0],
        height: 3.0,
    };

    // Generate with all caps
    let tri_all = pyramid.tessellate(0.01, 1.0);

    println!("Pyramid with all caps:");
    println!("  Vertices: {}", tri_all.vertices.len() / 3);
    println!("  Triangles: {}", tri_all.indices.len() / 3);

    // Pyramid has 6 faces: 2 end caps (top/bottom) + 4 side faces
    // Each end cap has 2 triangles (4 vertices)
    // Total cap vertices: 8 (4 for bottom + 4 for top)
    // Total vertices: 24 (4 corners × 6 faces)

    let total_verts = tri_all.vertices.len() / 3;
    let cap_verts = 8; // 2 caps × 4 vertices each
    let cap_percentage = (cap_verts as f32 / total_verts as f32) * 100.0;

    println!("  Cap percentage: {:.1}%", cap_percentage);

    // Cap percentage should be around 33% (8/24)
    assert!(cap_percentage > 25.0 && cap_percentage < 40.0);
}

#[test]
fn test_box_cap_analysis() {
    let b = Box {
        lengths: [2.0, 2.0, 2.0],
    };

    // Generate with all caps
    let tri_all = b.tessellate(0.01, 1.0);

    println!("Box with all caps:");
    println!("  Vertices: {}", tri_all.vertices.len() / 3);
    println!("  Triangles: {}", tri_all.indices.len() / 3);

    // Box has 6 faces, each face has 4 vertices
    // Total vertices: 24 (6 faces × 4 vertices)
    // All faces are "caps" in a sense, but typically we consider
    // the two end faces (perpendicular to main axis) as caps

    let total_verts = tri_all.vertices.len() / 3;
    println!("  Total vertices: {}", total_verts);

    // For a box, all 6 faces are equal, so "cap" percentage is 33% (2/6 faces)
    assert_eq!(total_verts, 24);
}

#[test]
fn test_pyramid_geometry_structure() {
    let pyramid = Pyramid {
        bottom: [2.0, 2.0],
        top: [1.0, 1.0],
        offset: [0.1, 0.1],
        height: 3.0,
    };

    let tri = pyramid.tessellate(0.01, 1.0);

    // Pyramid should have:
    // - 1 bottom face (4 vertices, 2 triangles)
    // - 1 top face (4 vertices, 2 triangles)
    // - 4 side faces (8 vertices, 8 triangles)
    // Total: 24 vertices, 12 triangles

    assert_eq!(tri.vertices.len() / 3, 24);
    assert_eq!(tri.indices.len() / 3, 12);

    // Verify all indices are valid
    for &idx in tri.indices.iter() {
        assert!((idx as usize) < tri.vertices.len() / 3);
    }
}

#[test]
fn test_box_geometry_structure() {
    let b = Box {
        lengths: [2.0, 3.0, 4.0],
    };

    let tri = b.tessellate(0.01, 1.0);

    // Box should have:
    // - 6 faces, each with 4 vertices and 2 triangles
    // Total: 24 vertices, 12 triangles

    assert_eq!(tri.vertices.len() / 3, 24);
    assert_eq!(tri.indices.len() / 3, 12);

    // Verify all indices are valid
    for &idx in tri.indices.iter() {
        assert!((idx as usize) < tri.vertices.len() / 3);
    }
}

#[test]
fn test_pyramid_normals() {
    let pyramid = Pyramid {
        bottom: [2.0, 2.0],
        top: [1.0, 1.0],
        offset: [0.0, 0.0],
        height: 3.0,
    };

    let tri = pyramid.tessellate(0.01, 1.0);

    // Verify all normals are unit length
    for i in 0..(tri.normals.len() / 3) {
        let nx = tri.normals[i * 3];
        let ny = tri.normals[i * 3 + 1];
        let nz = tri.normals[i * 3 + 2];
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!(
            (length - 1.0).abs() < 0.01,
            "Normal {} not unit length: {}",
            i,
            length
        );
    }
}

#[test]
fn test_box_normals() {
    let b = Box {
        lengths: [2.0, 2.0, 2.0],
    };

    let tri = b.tessellate(0.01, 1.0);

    // Verify all normals are unit length
    for i in 0..(tri.normals.len() / 3) {
        let nx = tri.normals[i * 3];
        let ny = tri.normals[i * 3 + 1];
        let nz = tri.normals[i * 3 + 2];
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!(
            (length - 1.0).abs() < 0.01,
            "Normal {} not unit length: {}",
            i,
            length
        );
    }
}
