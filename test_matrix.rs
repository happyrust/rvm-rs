use glam::{Affine3A, Mat3A, Vec3, Vec3A};

fn main() {
    // 模拟从文件读取的 12 个 float（按 C++ 的顺序）
    // data[0..11] = m00, m10, m20, m01, m11, m21, m02, m12, m22, m03, m13, m23
    let m00 = 2.0f32;
    let m10 = 0.0f32;
    let m20 = 0.0f32;
    
    let m01 = 0.0f32;
    let m11 = 3.0f32;
    let m21 = 0.0f32;
    
    let m02 = 0.0f32;
    let m12 = 0.0f32;
    let m22 = 4.0f32;
    
    let m03 = 10.0f32;
    let m13 = 20.0f32;
    let m23 = 30.0f32;

    // Rust 的构建方式
    let mat = Mat3A::from_cols(
        Vec3A::new(m00, m10, m20),
        Vec3A::new(m01, m11, m21),
        Vec3A::new(m02, m12, m22),
    );
    let translation = Vec3::new(m03, m13, m23);
    let transform = Affine3A::from_mat3_translation(mat.into(), translation);

    // 测试点变换
    let point = Vec3::new(1.0, 1.0, 1.0);
    let transformed = transform.transform_point3(point);
    
    println!("Original point: {:?}", point);
    println!("Transformed point: {:?}", transformed);
    println!("Expected: ({}, {}, {})", 
        m00 * 1.0 + m01 * 1.0 + m02 * 1.0 + m03,
        m10 * 1.0 + m11 * 1.0 + m12 * 1.0 + m13,
        m20 * 1.0 + m21 * 1.0 + m22 * 1.0 + m23
    );
    
    // 测试向量变换（不包括平移）
    let vector = Vec3::new(1.0, 0.0, 0.0);
    let transformed_vec = transform.transform_vector3(vector);
    println!("\nOriginal vector: {:?}", vector);
    println!("Transformed vector: {:?}", transformed_vec);
    println!("Expected: ({}, {}, {})", m00, m10, m20);
    
    // 提取缩放
    let col0_len = (m00 * m00 + m10 * m10 + m20 * m20).sqrt();
    let col1_len = (m01 * m01 + m11 * m11 + m21 * m21).sqrt();
    let col2_len = (m02 * m02 + m12 * m12 + m22 * m22).sqrt();
    
    println!("\nScale factors:");
    println!("X scale: {}", col0_len);
    println!("Y scale: {}", col1_len);
    println!("Z scale: {}", col2_len);
    println!("Max scale: {}", col0_len.max(col1_len).max(col2_len));
}
