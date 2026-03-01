use rvm_rs::export::tessellator::Tessellate;
use rvm_rs::store::geometry::{EllipticalDish, Sphere, SphericalDish};

#[test]
fn test_sphere_basic() {
    let sphere = Sphere { radius: 1.0 };

    let tri = sphere.tessellate(0.01, 1.0);

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

    // Check that vertices are approximately on sphere surface
    for i in 0..tri.vertices.len() / 3 {
        let x = tri.vertices[3 * i];
        let y = tri.vertices[3 * i + 1];
        let z = tri.vertices[3 * i + 2];
        let dist = (x * x + y * y + z * z).sqrt();
        assert!(
            (dist - 1.0).abs() < 0.1,
            "Vertex not on sphere surface: distance = {}",
            dist
        );
    }
}

#[test]
fn test_elliptical_dish_basic() {
    let dish = EllipticalDish {
        base_radius: 2.0,
        height: 1.0,
    };

    let tri = dish.tessellate(0.01, 1.0);

    // Should have vertices
    assert!(tri.vertices.len() > 0);
    assert_eq!(tri.vertices.len(), tri.normals.len());
    assert!(tri.indices.len() > 0);

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

    // Check height range
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for i in 0..tri.vertices.len() / 3 {
        let y = tri.vertices[3 * i + 1];
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    // Height should be approximately 1.0
    assert!(
        (max_y - min_y - 1.0).abs() < 0.2,
        "Height range unexpected: {}",
        max_y - min_y
    );
}

#[test]
fn test_spherical_dish_basic() {
    let dish = SphericalDish {
        base_radius: 2.0,
        height: 1.0,
    };

    let tri = dish.tessellate(0.01, 1.0);

    // Should have vertices
    assert!(tri.vertices.len() > 0);
    assert_eq!(tri.vertices.len(), tri.normals.len());
    assert!(tri.indices.len() > 0);

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
fn test_sphere_adaptive_sampling() {
    let sphere = Sphere { radius: 1.0 };

    // Test with different tolerances
    let tri_coarse = sphere.tessellate(0.1, 1.0);
    let tri_fine = sphere.tessellate(0.01, 1.0);

    let verts_coarse = tri_coarse.vertices.len() / 3;
    let verts_fine = tri_fine.vertices.len() / 3;

    // Finer tolerance should produce more vertices
    assert!(
        verts_fine > verts_coarse,
        "Fine mesh should have more vertices: {} vs {}",
        verts_fine,
        verts_coarse
    );
}

#[test]
fn test_elliptical_dish_scaling() {
    // Test that elliptical dish properly scales in Z direction
    let dish = EllipticalDish {
        base_radius: 2.0,
        height: 4.0, // Height is 2x radius
    };

    let tri = dish.tessellate(0.01, 1.0);

    // Find the range of Y coordinates
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for i in 0..tri.vertices.len() / 3 {
        let y = tri.vertices[3 * i + 1];
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let height = max_y - min_y;

    // Height should be approximately 4.0
    assert!(
        (height - 4.0).abs() < 0.5,
        "Height should be ~4.0, got {}",
        height
    );
}

#[test]
fn test_spherical_dish_geometry() {
    // Test spherical dish with specific geometry
    let r_circ = 2.0;
    let h = 1.0;

    let dish = SphericalDish {
        base_radius: r_circ,
        height: h,
    };

    let tri = dish.tessellate(0.01, 1.0);

    // Calculate expected sphere radius
    let r_sphere = (r_circ * r_circ + h * h) / (2.0 * h);

    // Check that vertices are approximately on the sphere surface
    let center_y = h - r_sphere;

    let mut max_deviation: f32 = 0.0;
    for i in 0..tri.vertices.len() / 3 {
        let x = tri.vertices[3 * i];
        let y = tri.vertices[3 * i + 1];
        let z = tri.vertices[3 * i + 2];

        let dist = ((x * x + (y - center_y) * (y - center_y) + z * z).sqrt() - r_sphere).abs();
        max_deviation = max_deviation.max(dist);
    }

    // Deviation should be small (within tolerance)
    assert!(
        max_deviation < 0.2,
        "Max deviation from sphere surface: {}",
        max_deviation
    );
}

#[test]
fn test_all_shapes_have_valid_indices() {
    let shapes: Vec<Box<dyn Tessellate>> = vec![
        Box::new(Sphere { radius: 1.0 }),
        Box::new(EllipticalDish {
            base_radius: 2.0,
            height: 1.0,
        }),
        Box::new(SphericalDish {
            base_radius: 2.0,
            height: 1.0,
        }),
    ];

    for shape in shapes {
        let tri = shape.tessellate(0.01, 1.0);
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
}
