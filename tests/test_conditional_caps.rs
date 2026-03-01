use rvm_rs::export::tessellator::{Tessellate, TessellateWithCaps};
use rvm_rs::store::geometry::{CircularTorus, Cylinder, RectangularTorus, Snout};

#[test]
fn test_cylinder_with_all_caps() {
    let cylinder = Cylinder {
        radius: 1.0,
        height: 2.0,
    };

    let tri = cylinder.tessellate(0.01, 1.0);

    // Should have shell + 2 caps
    assert!(tri.vertices.len() > 0);
    assert!(tri.indices.len() > 0);

    // Store counts for comparison
    let full_vertex_count = tri.vertices.len();
    let full_index_count = tri.indices.len();

    // Verify we have more than just the shell
    assert!(full_vertex_count > 100); // Shell + caps
    assert!(full_index_count > 100);
}

#[test]
fn test_cylinder_with_no_caps() {
    let cylinder = Cylinder {
        radius: 1.0,
        height: 2.0,
    };

    let tri_full = cylinder.tessellate(0.01, 1.0);
    let tri_no_caps = cylinder.tessellate_with_caps(0.01, 1.0, &[false, false]);

    // No caps should have fewer vertices and indices
    assert!(tri_no_caps.vertices.len() < tri_full.vertices.len());
    assert!(tri_no_caps.indices.len() < tri_full.indices.len());

    // Should still have shell vertices
    assert!(tri_no_caps.vertices.len() > 0);
    assert!(tri_no_caps.indices.len() > 0);
}

#[test]
fn test_cylinder_with_bottom_cap_only() {
    let cylinder = Cylinder {
        radius: 1.0,
        height: 2.0,
    };

    let tri_full = cylinder.tessellate(0.01, 1.0);
    let tri_bottom_only = cylinder.tessellate_with_caps(0.01, 1.0, &[true, false]);

    // Should have fewer vertices than full but more than no caps
    assert!(tri_bottom_only.vertices.len() < tri_full.vertices.len());
    assert!(tri_bottom_only.vertices.len() > 0);
}

#[test]
fn test_cylinder_with_top_cap_only() {
    let cylinder = Cylinder {
        radius: 1.0,
        height: 2.0,
    };

    let tri_full = cylinder.tessellate(0.01, 1.0);
    let tri_top_only = cylinder.tessellate_with_caps(0.01, 1.0, &[false, true]);

    // Should have fewer vertices than full but more than no caps
    assert!(tri_top_only.vertices.len() < tri_full.vertices.len());
    assert!(tri_top_only.vertices.len() > 0);
}

#[test]
fn test_snout_with_all_caps() {
    let snout = Snout {
        radius_bottom: 2.0,
        radius_top: 1.0,
        height: 3.0,
        offset_x: 0.0,
        offset_y: 0.0,
        bottom_shear_x: 0.0,
        bottom_shear_y: 0.0,
        top_shear_x: 0.0,
        top_shear_y: 0.0,
    };

    let tri = snout.tessellate(0.01, 1.0);

    // Should have shell + 2 caps
    assert!(tri.vertices.len() > 0);
    assert!(tri.indices.len() > 0);
}

#[test]
fn test_snout_with_no_caps() {
    let snout = Snout {
        radius_bottom: 2.0,
        radius_top: 1.0,
        height: 3.0,
        offset_x: 0.0,
        offset_y: 0.0,
        bottom_shear_x: 0.0,
        bottom_shear_y: 0.0,
        top_shear_x: 0.0,
        top_shear_y: 0.0,
    };

    let tri_full = snout.tessellate(0.01, 1.0);
    let tri_no_caps = snout.tessellate_with_caps(0.01, 1.0, &[false, false]);

    // No caps should have fewer vertices and indices
    assert!(tri_no_caps.vertices.len() < tri_full.vertices.len());
    assert!(tri_no_caps.indices.len() < tri_full.indices.len());

    // Should still have shell vertices
    assert!(tri_no_caps.vertices.len() > 0);
    assert!(tri_no_caps.indices.len() > 0);
}

#[test]
fn test_snout_with_bottom_cap_only() {
    let snout = Snout {
        radius_bottom: 2.0,
        radius_top: 1.0,
        height: 3.0,
        offset_x: 0.0,
        offset_y: 0.0,
        bottom_shear_x: 0.0,
        bottom_shear_y: 0.0,
        top_shear_x: 0.0,
        top_shear_y: 0.0,
    };

    let tri_full = snout.tessellate(0.01, 1.0);
    let tri_bottom_only = snout.tessellate_with_caps(0.01, 1.0, &[true, false]);

    // Should have fewer vertices than full
    assert!(tri_bottom_only.vertices.len() < tri_full.vertices.len());
    assert!(tri_bottom_only.vertices.len() > 0);
}

#[test]
fn test_snout_with_shear_no_caps() {
    let snout = Snout {
        radius_bottom: 2.0,
        radius_top: 1.0,
        height: 3.0,
        offset_x: 0.1,
        offset_y: 0.1,
        bottom_shear_x: 0.1,
        bottom_shear_y: 0.05,
        top_shear_x: -0.1,
        top_shear_y: -0.05,
    };

    let tri_full = snout.tessellate(0.01, 1.0);
    let tri_no_caps = snout.tessellate_with_caps(0.01, 1.0, &[false, false]);

    // Verify shear doesn't break cap generation logic
    assert!(tri_no_caps.vertices.len() < tri_full.vertices.len());
    assert!(tri_no_caps.vertices.len() > 0);

    // Verify normals are normalized
    for i in 0..(tri_no_caps.normals.len() / 3) {
        let nx = tri_no_caps.normals[i * 3];
        let ny = tri_no_caps.normals[i * 3 + 1];
        let nz = tri_no_caps.normals[i * 3 + 2];
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!((length - 1.0).abs() < 0.01, "Normal not normalized");
    }
}

#[test]
fn test_cylinder_cap_reduction_percentage() {
    let cylinder = Cylinder {
        radius: 1.0,
        height: 2.0,
    };

    let tri_full = cylinder.tessellate(0.01, 1.0);
    let tri_no_caps = cylinder.tessellate_with_caps(0.01, 1.0, &[false, false]);

    let vertex_reduction = (tri_full.vertices.len() - tri_no_caps.vertices.len()) as f32
        / tri_full.vertices.len() as f32;
    let index_reduction =
        (tri_full.indices.len() - tri_no_caps.indices.len()) as f32 / tri_full.indices.len() as f32;

    // Caps should account for some portion of the geometry
    // For a cylinder with height=2*radius, caps are roughly 2-5% of vertices
    assert!(
        vertex_reduction > 0.01,
        "Vertex reduction too small: {}",
        vertex_reduction
    );
    assert!(
        index_reduction > 0.01,
        "Index reduction too small: {}",
        index_reduction
    );

    println!("Cylinder cap reduction:");
    println!("  Vertices: {:.1}%", vertex_reduction * 100.0);
    println!("  Indices: {:.1}%", index_reduction * 100.0);
}

#[test]
fn test_circular_torus_with_all_caps() {
    let torus = CircularTorus {
        offset: 3.0,
        radius: 0.5,
        angle: std::f32::consts::PI, // 180 degrees
    };

    let tri = torus.tessellate(0.01, 1.0);

    // Should have shell + 2 caps
    assert!(tri.vertices.len() > 0);
    assert!(tri.indices.len() > 0);
}

#[test]
fn test_circular_torus_with_no_caps() {
    let torus = CircularTorus {
        offset: 3.0,
        radius: 0.5,
        angle: std::f32::consts::PI,
    };

    let tri_full = torus.tessellate(0.01, 1.0);
    let tri_no_caps = torus.tessellate_with_caps(0.01, 1.0, &[false, false]);

    // No caps should have fewer vertices and indices
    assert!(tri_no_caps.vertices.len() < tri_full.vertices.len());
    assert!(tri_no_caps.indices.len() < tri_full.indices.len());

    // Should still have shell vertices
    assert!(tri_no_caps.vertices.len() > 0);
    assert!(tri_no_caps.indices.len() > 0);
}

#[test]
fn test_circular_torus_with_start_cap_only() {
    let torus = CircularTorus {
        offset: 3.0,
        radius: 0.5,
        angle: std::f32::consts::PI,
    };

    let tri_full = torus.tessellate(0.01, 1.0);
    let tri_start_only = torus.tessellate_with_caps(0.01, 1.0, &[true, false]);

    // Should have fewer vertices than full
    assert!(tri_start_only.vertices.len() < tri_full.vertices.len());
    assert!(tri_start_only.vertices.len() > 0);
}

#[test]
fn test_circular_torus_with_end_cap_only() {
    let torus = CircularTorus {
        offset: 3.0,
        radius: 0.5,
        angle: std::f32::consts::PI,
    };

    let tri_full = torus.tessellate(0.01, 1.0);
    let tri_end_only = torus.tessellate_with_caps(0.01, 1.0, &[false, true]);

    // Should have fewer vertices than full
    assert!(tri_end_only.vertices.len() < tri_full.vertices.len());
    assert!(tri_end_only.vertices.len() > 0);
}

#[test]
fn test_circular_torus_cap_reduction() {
    let torus = CircularTorus {
        offset: 3.0,
        radius: 0.5,
        angle: std::f32::consts::PI,
    };

    let tri_full = torus.tessellate(0.01, 1.0);
    let tri_no_caps = torus.tessellate_with_caps(0.01, 1.0, &[false, false]);

    let vertex_reduction = (tri_full.vertices.len() - tri_no_caps.vertices.len()) as f32
        / tri_full.vertices.len() as f32;
    let index_reduction =
        (tri_full.indices.len() - tri_no_caps.indices.len()) as f32 / tri_full.indices.len() as f32;

    // Caps should account for some portion
    assert!(
        vertex_reduction > 0.01,
        "Vertex reduction too small: {}",
        vertex_reduction
    );
    assert!(
        index_reduction > 0.01,
        "Index reduction too small: {}",
        index_reduction
    );

    println!("CircularTorus cap reduction:");
    println!("  Vertices: {:.1}%", vertex_reduction * 100.0);
    println!("  Indices: {:.1}%", index_reduction * 100.0);
}

#[test]
fn test_rectangular_torus_with_all_caps() {
    let torus = RectangularTorus {
        inner_radius: 2.0,
        outer_radius: 3.0,
        height: 1.0,
        angle: std::f32::consts::PI / 2.0, // 90 degrees
    };

    let tri = torus.tessellate(0.01, 1.0);

    // Should have shell + 2 caps
    assert!(tri.vertices.len() > 0);
    assert!(tri.indices.len() > 0);
}

#[test]
fn test_rectangular_torus_with_no_caps() {
    let torus = RectangularTorus {
        inner_radius: 2.0,
        outer_radius: 3.0,
        height: 1.0,
        angle: std::f32::consts::PI / 2.0,
    };

    let tri_full = torus.tessellate(0.01, 1.0);
    let tri_no_caps = torus.tessellate_with_caps(0.01, 1.0, &[false, false]);

    // No caps should have fewer vertices and indices
    assert!(tri_no_caps.vertices.len() < tri_full.vertices.len());
    assert!(tri_no_caps.indices.len() < tri_full.indices.len());

    // Should still have shell vertices
    assert!(tri_no_caps.vertices.len() > 0);
    assert!(tri_no_caps.indices.len() > 0);
}

#[test]
fn test_rectangular_torus_with_start_cap_only() {
    let torus = RectangularTorus {
        inner_radius: 2.0,
        outer_radius: 3.0,
        height: 1.0,
        angle: std::f32::consts::PI / 2.0,
    };

    let tri_full = torus.tessellate(0.01, 1.0);
    let tri_start_only = torus.tessellate_with_caps(0.01, 1.0, &[true, false]);

    // Should have fewer vertices than full
    assert!(tri_start_only.vertices.len() < tri_full.vertices.len());
    assert!(tri_start_only.vertices.len() > 0);
}

#[test]
fn test_rectangular_torus_cap_reduction() {
    let torus = RectangularTorus {
        inner_radius: 2.0,
        outer_radius: 3.0,
        height: 1.0,
        angle: std::f32::consts::PI / 2.0,
    };

    let tri_full = torus.tessellate(0.01, 1.0);
    let tri_no_caps = torus.tessellate_with_caps(0.01, 1.0, &[false, false]);

    let vertex_reduction = (tri_full.vertices.len() - tri_no_caps.vertices.len()) as f32
        / tri_full.vertices.len() as f32;
    let index_reduction =
        (tri_full.indices.len() - tri_no_caps.indices.len()) as f32 / tri_full.indices.len() as f32;

    // Caps should account for some portion
    assert!(
        vertex_reduction > 0.005,
        "Vertex reduction too small: {}",
        vertex_reduction
    );
    assert!(
        index_reduction > 0.005,
        "Index reduction too small: {}",
        index_reduction
    );

    println!("RectangularTorus cap reduction:");
    println!("  Vertices: {:.1}%", vertex_reduction * 100.0);
    println!("  Indices: {:.1}%", index_reduction * 100.0);
}
