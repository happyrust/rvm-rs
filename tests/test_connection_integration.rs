use glam::{Affine3A, Vec3};
use rvm_rs::export::tessellator::{tessellate_with_connections, Tessellate};
use rvm_rs::math::BBox3;
use rvm_rs::store::connection::{Connection, ConnectionFlags};
use rvm_rs::store::geometry::{Cylinder, Geometry, GeometryId, GeometryKind, GeometryType};

fn create_test_cylinder(radius: f32, height: f32) -> Geometry {
    Geometry {
        kind: GeometryKind::Cylinder(Cylinder { radius, height }),
        geo_type: GeometryType::Primitive,
        transform: Affine3A::IDENTITY,
        bbox_local: BBox3::new(),
        bbox_world: BBox3::new(),
        color: 0,
        transparency: 0,
        sample_start_angle: 0.0,
        color_name: None,
        color_rgb: 0,
        next: None,
    }
}

#[test]
fn test_tessellate_with_no_connections() {
    let geo = create_test_cylinder(1.0, 2.0);
    let connections = vec![];

    let tri = tessellate_with_connections(&geo, 0.01, 1.0, &connections);

    // Should generate all caps (same as default tessellation)
    assert!(tri.vertices.len() > 0);
    assert!(tri.indices.len() > 0);

    // Compare with default tessellation
    if let GeometryKind::Cylinder(cyl) = &geo.kind {
        let tri_default = cyl.tessellate(0.01, 1.0);
        assert_eq!(tri.vertices.len(), tri_default.vertices.len());
        assert_eq!(tri.indices.len(), tri_default.indices.len());
    }
}

#[test]
fn test_tessellate_with_connections_basic() {
    let geo = create_test_cylinder(1.0, 2.0);

    // Create a mock connection (simplified - doesn't actually match interfaces)
    let mut conn = Connection::new(GeometryId(0), 0, GeometryId(1), 0, Vec3::ZERO, Vec3::Z);
    conn.flags.set_circular_side();

    let connections = vec![conn];

    let tri = tessellate_with_connections(&geo, 0.01, 1.0, &connections);

    // Should still generate geometry (connection check is simplified in current implementation)
    assert!(tri.vertices.len() > 0);
    assert!(tri.indices.len() > 0);
}

#[test]
fn test_tessellate_different_geometry_types() {
    use rvm_rs::store::geometry::{Box, Sphere};

    // Test Box
    let box_geo = Geometry {
        kind: GeometryKind::Box(Box {
            lengths: [2.0, 2.0, 2.0],
        }),
        geo_type: GeometryType::Primitive,
        transform: Affine3A::IDENTITY,
        bbox_local: BBox3::new(),
        bbox_world: BBox3::new(),
        color: 0,
        transparency: 0,
        sample_start_angle: 0.0,
        color_name: None,
        color_rgb: 0,
        next: None,
    };

    let tri_box = tessellate_with_connections(&box_geo, 0.01, 1.0, &[]);
    assert!(tri_box.vertices.len() > 0);

    // Test Sphere
    let sphere_geo = Geometry {
        kind: GeometryKind::Sphere(Sphere { radius: 1.0 }),
        geo_type: GeometryType::Primitive,
        transform: Affine3A::IDENTITY,
        bbox_local: BBox3::new(),
        bbox_world: BBox3::new(),
        color: 0,
        transparency: 0,
        sample_start_angle: 0.0,
        color_name: None,
        color_rgb: 0,
        next: None,
    };

    let tri_sphere = tessellate_with_connections(&sphere_geo, 0.01, 1.0, &[]);
    assert!(tri_sphere.vertices.len() > 0);
}

#[test]
fn test_connection_flags() {
    let mut flags = ConnectionFlags::new();
    assert!(!flags.has_circular_side());
    assert!(!flags.has_rectangular_side());

    flags.set_circular_side();
    assert!(flags.has_circular_side());

    flags.set_rectangular_side();
    assert!(flags.has_rectangular_side());
}

#[test]
fn test_tessellate_with_connections_preserves_quality() {
    let geo = create_test_cylinder(1.0, 2.0);
    let connections = vec![];

    let tri = tessellate_with_connections(&geo, 0.01, 1.0, &connections);

    // Verify normals are normalized
    for i in 0..(tri.normals.len() / 3) {
        let nx = tri.normals[i * 3];
        let ny = tri.normals[i * 3 + 1];
        let nz = tri.normals[i * 3 + 2];
        let length = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!(
            (length - 1.0).abs() < 0.01,
            "Normal not normalized at index {}",
            i
        );
    }

    // Verify indices are valid
    let vertex_count = (tri.vertices.len() / 3) as u32;
    for i in 0..(tri.indices.len()) {
        assert!(
            tri.indices[i] < vertex_count,
            "Invalid index {} at position {}",
            tri.indices[i],
            i
        );
    }
}
