use rvm_rs::export::tessellator::Tessellate;
use rvm_rs::store::geometry::Snout;

#[test]
fn test_snout_without_shear() {
    let snout = Snout {
        radius_bottom: 1.0,
        radius_top: 0.5,
        height: 2.0,
        offset_x: 0.0,
        offset_y: 0.0,
        bottom_shear_x: 0.0,
        bottom_shear_y: 0.0,
        top_shear_x: 0.0,
        top_shear_y: 0.0,
    };

    let tri = snout.tessellate(0.01, 1.0);

    // Should have vertices
    assert!(tri.vertices.len() > 0);
    assert_eq!(tri.vertices.len(), tri.normals.len());
    assert!(tri.indices.len() > 0);
    assert_eq!(tri.indices.len() % 3, 0);

    // All normals should be normalized
    for i in 0..tri.normals.len() / 3 {
        let nx = tri.normals[3 * i];
        let ny = tri.normals[3 * i + 1];
        let nz = tri.normals[3 * i + 2];
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!(
            (length - 1.0).abs() < 0.01,
            "Normal not normalized: {}",
            length
        );
    }
}

#[test]
fn test_snout_with_shear() {
    let snout = Snout {
        radius_bottom: 1.0,
        radius_top: 0.5,
        height: 2.0,
        offset_x: 0.2,
        offset_y: 0.1,
        bottom_shear_x: 0.1, // ~5.7 degrees
        bottom_shear_y: 0.05,
        top_shear_x: -0.1,
        top_shear_y: -0.05,
    };

    let tri = snout.tessellate(0.01, 1.0);

    // Should have vertices
    assert!(tri.vertices.len() > 0);
    assert_eq!(tri.vertices.len(), tri.normals.len());
    assert!(tri.indices.len() > 0);

    // Verify bottom and top are at different heights due to shear (snouts run along Z)
    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;

    for i in 0..tri.vertices.len() / 3 {
        let z = tri.vertices[3 * i + 2];
        min_z = min_z.min(z);
        max_z = max_z.max(z);
    }

    // Height should be approximately 2.0, but shear will affect it
    let height_range = max_z - min_z;
    assert!(
        height_range > 1.8 && height_range < 2.5,
        "Height range unexpected: {}",
        height_range
    );
}

#[test]
fn test_snout_vertex_count() {
    let snout = Snout {
        radius_bottom: 1.0,
        radius_top: 0.5,
        height: 2.0,
        offset_x: 0.0,
        offset_y: 0.0,
        bottom_shear_x: 0.0,
        bottom_shear_y: 0.0,
        top_shear_x: 0.0,
        top_shear_y: 0.0,
    };

    let tri = snout.tessellate(0.01, 1.0);

    // Calculate expected vertex count
    // Shell: segments * 2 (bottom and top)
    // Bottom cap: segments
    // Top cap: segments
    // Total: segments * 4

    let vertex_count = tri.vertices.len() / 3;
    assert!(
        vertex_count % 4 == 0,
        "Vertex count should be multiple of 4"
    );

    let segments = vertex_count / 4;
    assert!(segments >= 8, "Should have at least 8 segments");
}

#[test]
fn test_snout_indices_valid() {
    let snout = Snout {
        radius_bottom: 1.0,
        radius_top: 0.5,
        height: 2.0,
        offset_x: 0.0,
        offset_y: 0.0,
        bottom_shear_x: 0.0,
        bottom_shear_y: 0.0,
        top_shear_x: 0.0,
        top_shear_y: 0.0,
    };

    let tri = snout.tessellate(0.01, 1.0);
    let vertex_count = (tri.vertices.len() / 3) as u32;

    // All indices should be valid
    for &idx in &tri.indices {
        assert!(
            idx < vertex_count,
            "Invalid index {} (vertex count: {})",
            idx,
            vertex_count
        );
    }
}
