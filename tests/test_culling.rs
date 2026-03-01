use glam::Vec3;
use rvm_rs::export::tessellator::{culled_geometry_error, should_cull_geometry, CullingOptions};
use rvm_rs::math::BBox3;

#[test]
fn test_culling_disabled() {
    let options = CullingOptions::disabled();

    let bbox = BBox3::from_min_max(Vec3::ZERO, Vec3::splat(0.001));

    // Should not cull when disabled
    assert!(!should_cull_geometry(&bbox, &options));
}

#[test]
fn test_culling_small_geometry() {
    let options = CullingOptions::new(0.1, 0.05);

    // Very small geometry (diagonal < 0.1)
    let small_bbox = BBox3::from_min_max(Vec3::ZERO, Vec3::splat(0.01));
    assert!(should_cull_geometry(&small_bbox, &options));

    // Large geometry (diagonal > 0.1)
    let large_bbox = BBox3::from_min_max(Vec3::ZERO, Vec3::splat(1.0));
    assert!(!should_cull_geometry(&large_bbox, &options));
}

#[test]
fn test_culling_threshold() {
    let options = CullingOptions::new(1.0, 0.5);

    // Diagonal = sqrt(3) ≈ 1.732 > 1.0
    let bbox1 = BBox3::from_min_max(Vec3::ZERO, Vec3::ONE);
    assert!(!should_cull_geometry(&bbox1, &options));

    // Diagonal = sqrt(0.03) ≈ 0.173 < 1.0
    let bbox2 = BBox3::from_min_max(Vec3::ZERO, Vec3::splat(0.1));
    assert!(should_cull_geometry(&bbox2, &options));
}

#[test]
fn test_default_thresholds() {
    let tolerance = 0.01;
    let options = CullingOptions::default_thresholds(tolerance);

    assert!((options.geometry_threshold - 0.1).abs() < 0.0001);
    assert!((options.group_threshold - 0.05).abs() < 0.0001);
}

#[test]
fn test_culled_geometry_error() {
    let bbox = BBox3::from_min_max(Vec3::ZERO, Vec3::ONE);
    let error = culled_geometry_error(&bbox);

    // Diagonal of unit cube = sqrt(3)
    assert!((error - 3.0_f32.sqrt()).abs() < 0.001);
}

#[test]
fn test_bbox_diagonal_length() {
    // Unit cube
    let bbox1 = BBox3::from_min_max(Vec3::ZERO, Vec3::ONE);
    assert!((bbox1.diagonal_length() - 3.0_f32.sqrt()).abs() < 0.001);

    // 2x2x2 cube
    let bbox2 = BBox3::from_min_max(Vec3::ZERO, Vec3::splat(2.0));
    assert!((bbox2.diagonal_length() - (12.0_f32.sqrt())).abs() < 0.001);

    // Flat rectangle (1x1x0)
    let bbox3 = BBox3::from_min_max(Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0));
    assert!((bbox3.diagonal_length() - 2.0_f32.sqrt()).abs() < 0.001);
}

#[test]
fn test_bbox_size() {
    let bbox = BBox3::from_min_max(Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 6.0, 8.0));
    let size = bbox.size();

    assert_eq!(size.x, 3.0);
    assert_eq!(size.y, 4.0);
    assert_eq!(size.z, 5.0);
}

#[test]
fn test_bbox_center() {
    let bbox = BBox3::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 4.0, 6.0));
    let center = bbox.center();

    assert_eq!(center.x, 1.0);
    assert_eq!(center.y, 2.0);
    assert_eq!(center.z, 3.0);
}

#[test]
fn test_culling_with_realistic_values() {
    // Typical tolerance for visualization
    let tolerance = 0.01;
    let options = CullingOptions::default_thresholds(tolerance);

    // Small bolt (1cm cube)
    let bolt = BBox3::from_min_max(Vec3::ZERO, Vec3::splat(0.01));
    let should_cull = should_cull_geometry(&bolt, &options);
    // Diagonal ≈ 0.017 < 0.1, should cull
    assert!(should_cull);

    // Medium pipe (10cm diameter, 1m long)
    let pipe = BBox3::from_min_max(Vec3::ZERO, Vec3::new(0.1, 1.0, 0.1));
    let should_cull = should_cull_geometry(&pipe, &options);
    // Diagonal ≈ 1.01 > 0.1, should not cull
    assert!(!should_cull);

    // Large tank (2m cube)
    let tank = BBox3::from_min_max(Vec3::ZERO, Vec3::splat(2.0));
    let should_cull = should_cull_geometry(&tank, &options);
    // Diagonal ≈ 3.46 > 0.1, should not cull
    assert!(!should_cull);
}

#[test]
fn test_culling_options_creation() {
    let opt1 = CullingOptions::new(0.5, 0.25);
    assert_eq!(opt1.geometry_threshold, 0.5);
    assert_eq!(opt1.group_threshold, 0.25);

    let opt2 = CullingOptions::disabled();
    assert_eq!(opt2.geometry_threshold, 0.0);
    assert_eq!(opt2.group_threshold, 0.0);

    let opt3 = CullingOptions::default();
    assert_eq!(opt3.geometry_threshold, 0.0);
    assert_eq!(opt3.group_threshold, 0.0);
}
