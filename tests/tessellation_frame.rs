//! Every primitive's tessellation must live in the same local frame as the RVM
//! record it came from: the PRIM record carries its own local bbox, and the
//! exporters apply the record transform to the tessellated vertices verbatim.
//! If the tessellator emits a cylinder along +Y while the record describes a
//! cylinder centred along Z, glTF/OBJ output shows it rotated by 90 degrees.
//!
//! The check is data-driven: parse each bundled RVM, tessellate every primitive
//! and compare the tessellation's AABB against `Geometry::bbox_local`.

use std::collections::BTreeMap;
use std::path::Path;

use rvm_rs::export::tessellator::get_scale;
use rvm_rs::export::Tessellate;
use rvm_rs::parse_rvm;
use rvm_rs::store::geometry::{GeometryId, GeometryKind};
use rvm_rs::store::Store;

#[derive(Default, Debug)]
struct KindTally {
    checked: usize,
    mismatched: usize,
    sample: Option<String>,
}

fn aabb(vertices: &[f32]) -> Option<([f32; 3], [f32; 3])> {
    let points = vertices.as_chunks::<3>().0;
    let mut lo = *points.first()?;
    let mut hi = lo;
    for point in points {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(point[axis]);
            hi[axis] = hi[axis].max(point[axis]);
        }
    }
    Some((lo, hi))
}

fn kind_name(kind: &GeometryKind) -> &'static str {
    match kind {
        GeometryKind::Pyramid(_) => "Pyramid",
        GeometryKind::Box(_) => "Box",
        GeometryKind::RectangularTorus(_) => "RectangularTorus",
        GeometryKind::CircularTorus(_) => "CircularTorus",
        GeometryKind::EllipticalDish(_) => "EllipticalDish",
        GeometryKind::SphericalDish(_) => "SphericalDish",
        GeometryKind::Snout(_) => "Snout",
        GeometryKind::Cylinder(_) => "Cylinder",
        GeometryKind::Sphere(_) => "Sphere",
        GeometryKind::Line(_) => "Line",
        GeometryKind::FacetGroup(_) => "FacetGroup",
    }
}

fn tessellate(
    kind: &GeometryKind,
    tolerance: f32,
    scale: f32,
) -> Option<rvm_rs::export::Triangulation> {
    Some(match kind {
        // How far a sheared or offset cap reaches is a question about the shear/offset
        // model, not about the frame: in shilou.rvm the record bboxes of sheared snouts
        // disagree with r * tan(shear) by hundreds of mm and those of offset snouts are
        // symmetric around the axis, so both are left out of this check.
        GeometryKind::Snout(g)
            if [
                g.bottom_shear_x,
                g.bottom_shear_y,
                g.top_shear_x,
                g.top_shear_y,
                g.offset_x,
                g.offset_y,
            ]
            .iter()
            .any(|value| value.abs() > 1e-6) =>
        {
            return None;
        }
        GeometryKind::Cylinder(g) => g.tessellate(tolerance, scale),
        GeometryKind::Sphere(g) => g.tessellate(tolerance, scale),
        GeometryKind::Box(g) => g.tessellate(tolerance, scale),
        GeometryKind::Pyramid(g) => g.tessellate(tolerance, scale),
        GeometryKind::CircularTorus(g) => g.tessellate(tolerance, scale),
        GeometryKind::RectangularTorus(g) => g.tessellate(tolerance, scale),
        GeometryKind::EllipticalDish(g) => g.tessellate(tolerance, scale),
        GeometryKind::SphericalDish(g) => g.tessellate(tolerance, scale),
        GeometryKind::Snout(g) => g.tessellate(tolerance, scale),
        // Lines have no volume and facet groups are stored in record space already.
        GeometryKind::Line(_) | GeometryKind::FacetGroup(_) => return None,
    })
}

/// Tolerance per axis: 1 % of the record's extent on that axis plus a floor that
/// absorbs the sagitta of the coarsest (8-segment) circle, r(1 - cos(pi/8)) ~ 7.6 %.
fn within(tess: ([f32; 3], [f32; 3]), record: ([f32; 3], [f32; 3])) -> bool {
    (0..3).all(|axis| {
        let extent = (record.1[axis] - record.0[axis]).abs();
        let tolerance = 1e-3 + 0.08 * extent;
        (tess.0[axis] - record.0[axis]).abs() <= tolerance
            && (tess.1[axis] - record.1[axis]).abs() <= tolerance
    })
}

fn audit(path: &Path) -> BTreeMap<&'static str, KindTally> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut store = Store::new();
    parse_rvm(&bytes, &mut store).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));

    let mut tally: BTreeMap<&'static str, KindTally> = BTreeMap::new();
    for index in 0..store.geometry_count() {
        let Some(geometry) = store.get_geometry(GeometryId(index as u32)) else {
            continue;
        };
        let Some(tri) = tessellate(&geometry.kind, 1e-3, get_scale(&geometry.transform.matrix3))
        else {
            continue;
        };
        let Some(tess) = aabb(&tri.vertices) else {
            continue;
        };
        let record = (
            geometry.bbox_local.min.to_array(),
            geometry.bbox_local.max.to_array(),
        );
        // A degenerate or inverted record bbox (negative radii occur in the wild) says
        // nothing about the frame.
        if (0..3).any(|axis| {
            !(record.1[axis] - record.0[axis]).is_finite() || record.1[axis] < record.0[axis]
        }) || record == ([0.0; 3], [0.0; 3])
        {
            continue;
        }
        let entry = tally.entry(kind_name(&geometry.kind)).or_default();
        entry.checked += 1;
        if !within(tess, record) {
            entry.mismatched += 1;
            entry.sample.get_or_insert_with(|| {
                format!(
                    "{:?}: tessellated {:?}..{:?} vs record {:?}..{:?}",
                    geometry.kind, tess.0, tess.1, record.0, record.1
                )
            });
        }
    }
    tally
}

fn assert_frames_match(file: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/test-data")
        .join(file);
    let tally = audit(&path);
    let mut failures = Vec::new();
    for (kind, t) in &tally {
        println!(
            "{file}: {kind:<16} checked {:>5} mismatched {:>5}",
            t.checked, t.mismatched
        );
        if t.mismatched > 0 {
            failures.push(format!(
                "{kind}: {}/{} off-frame, e.g. {}",
                t.mismatched,
                t.checked,
                t.sample.as_deref().unwrap_or("-")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{file}: tessellation frame disagrees with PRIM record bbox:\n  {}",
        failures.join("\n  ")
    );
}

fn aabb_of(tri: &rvm_rs::export::Triangulation) -> ([f32; 3], [f32; 3]) {
    aabb(&tri.vertices).expect("tessellation has vertices")
}

fn assert_close(
    actual: ([f32; 3], [f32; 3]),
    expected: ([f32; 3], [f32; 3]),
    tolerance: f32,
    what: &str,
) {
    for axis in 0..3 {
        assert!(
            (actual.0[axis] - expected.0[axis]).abs() <= tolerance
                && (actual.1[axis] - expected.1[axis]).abs() <= tolerance,
            "{what}: axis {axis} got {:?}..{:?}, expected {:?}..{:?}",
            actual.0,
            actual.1,
            expected.0,
            expected.1
        );
    }
}

#[test]
fn cylinder_runs_along_z_and_is_centred() {
    let tri = rvm_rs::store::geometry::Cylinder {
        radius: 30.0,
        height: 800.0,
    }
    .tessellate(1e-3, 1.0);
    assert_close(
        aabb_of(&tri),
        ([-30.0, -30.0, -400.0], [30.0, 30.0, 400.0]),
        0.5,
        "cylinder",
    );
}

#[test]
fn circular_torus_sweeps_from_x_towards_y_with_the_tube_standing_in_z() {
    let torus = rvm_rs::store::geometry::CircularTorus {
        offset: 76.0,
        radius: 30.0,
        angle: std::f32::consts::FRAC_PI_2,
    };
    let tri = torus.tessellate(1e-3, 1.0);
    assert_close(
        aabb_of(&tri),
        ([0.0, 0.0, -30.0], [106.0, 106.0, 30.0]),
        0.5,
        "circular torus",
    );
}

#[test]
fn pyramid_runs_along_z_and_is_centred() {
    let pyramid = rvm_rs::store::geometry::Pyramid {
        bottom: [200.0, 100.0],
        top: [200.0, 100.0],
        offset: [0.0, 0.0],
        height: 50.0,
    };
    let tri = pyramid.tessellate(1e-3, 1.0);
    assert_close(
        aabb_of(&tri),
        ([-100.0, -50.0, -25.0], [100.0, 50.0, 25.0]),
        1e-3,
        "pyramid",
    );
}

#[test]
fn dishes_rise_from_the_base_plane_along_z() {
    let dish = rvm_rs::store::geometry::EllipticalDish {
        base_radius: 135.0,
        height: 104.0,
    }
    .tessellate(1e-3, 1.0);
    assert_close(
        aabb_of(&dish),
        ([-135.0, -135.0, 0.0], [135.0, 135.0, 104.0]),
        1.0,
        "elliptical dish",
    );
}

#[test]
fn box_frame_is_untouched() {
    let tri = rvm_rs::store::geometry::Box {
        lengths: [10.0, 20.0, 30.0],
    }
    .tessellate(1e-3, 1.0);
    assert_close(
        aabb_of(&tri),
        ([-5.0, -10.0, -15.0], [5.0, 10.0, 15.0]),
        1e-6,
        "box",
    );
}

#[test]
fn tessellation_frame_matches_record_bbox_test_rvm() {
    assert_frames_match("test.rvm");
}

#[test]
fn tessellation_frame_matches_record_bbox_wall_rvm() {
    assert_frames_match("wall.rvm");
}

#[test]
fn tessellation_frame_matches_record_bbox_shilou_rvm() {
    assert_frames_match("shilou.rvm");
}
