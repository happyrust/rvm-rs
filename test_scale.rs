use glam::{Affine3A, Mat3A, Vec3, Vec3A};

fn get_scale(mat: &Mat3A) -> f32 {
    let sx = mat.x_axis.length();
    let sy = mat.y_axis.length();
    let sz = mat.z_axis.length();
    sx.max(sy).max(sz)
}

fn main() {
    // Test 1: Identity matrix (no scale)
    let mat = Mat3A::IDENTITY;
    let scale = get_scale(&mat);
    println!("Test 1 - Identity matrix:");
    println!("  Scale: {} (expected: 1.0)", scale);
    assert!((scale - 1.0).abs() < 0.001);

    // Test 2: Uniform scale
    let mat = Mat3A::from_cols(
        Vec3A::new(2.0, 0.0, 0.0),
        Vec3A::new(0.0, 2.0, 0.0),
        Vec3A::new(0.0, 0.0, 2.0),
    );
    let scale = get_scale(&mat);
    println!("\nTest 2 - Uniform scale 2.0:");
    println!("  Scale: {} (expected: 2.0)", scale);
    assert!((scale - 2.0).abs() < 0.001);

    // Test 3: Non-uniform scale (should return max)
    let mat = Mat3A::from_cols(
        Vec3A::new(1.0, 0.0, 0.0),
        Vec3A::new(0.0, 3.0, 0.0),
        Vec3A::new(0.0, 0.0, 2.0),
    );
    let scale = get_scale(&mat);
    println!("\nTest 3 - Non-uniform scale (1, 3, 2):");
    println!("  Scale: {} (expected: 3.0)", scale);
    assert!((scale - 3.0).abs() < 0.001);

    // Test 4: Scale with rotation
    let angle = std::f32::consts::PI / 4.0; // 45 degrees
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let scale_factor = 2.5;
    
    // Rotation around Z axis with scale
    let mat = Mat3A::from_cols(
        Vec3A::new(scale_factor * cos_a, scale_factor * sin_a, 0.0),
        Vec3A::new(-scale_factor * sin_a, scale_factor * cos_a, 0.0),
        Vec3A::new(0.0, 0.0, scale_factor),
    );
    let scale = get_scale(&mat);
    println!("\nTest 4 - Rotation with scale 2.5:");
    println!("  Scale: {} (expected: 2.5)", scale);
    assert!((scale - 2.5).abs() < 0.001);

    // Test 5: From RVM file format (column-major)
    // Simulating: m00=2, m10=0, m20=0, m01=0, m11=3, m21=0, m02=0, m12=0, m22=4
    let m00 = 2.0f32;
    let m10 = 0.0f32;
    let m20 = 0.0f32;
    let m01 = 0.0f32;
    let m11 = 3.0f32;
    let m21 = 0.0f32;
    let m02 = 0.0f32;
    let m12 = 0.0f32;
    let m22 = 4.0f32;
    
    let mat = Mat3A::from_cols(
        Vec3A::new(m00, m10, m20),
        Vec3A::new(m01, m11, m21),
        Vec3A::new(m02, m12, m22),
    );
    let scale = get_scale(&mat);
    println!("\nTest 5 - RVM format matrix (scale 2, 3, 4):");
    println!("  Scale: {} (expected: 4.0)", scale);
    assert!((scale - 4.0).abs() < 0.001);

    println!("\n✓ All scale tests passed!");
}
