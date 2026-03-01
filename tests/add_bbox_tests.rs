use glam::Vec3;
use rvm_rs::math::BBox3;
use rvm_rs::store::geometry::{Cylinder, GeometryKind};
use rvm_rs::store::{GroupNode, NodeKind, Store};
use rvm_rs::visitor::add_bbox::AddGroupBBox;
use rvm_rs::visitor::traverse;

fn run_add_bbox(store: &mut Store) {
    let mut visitor = AddGroupBBox::new();
    traverse(store, &mut visitor);
}

#[test]
fn test_add_bbox_empty_group() {
    let mut store = Store::new();
    let group_node = GroupNode {
        name: store.intern_string("EmptyGroup"),
        translation: Vec3::ZERO,
        material: 0,
        transparency: 0,
        id: -1,
        bbox_world: BBox3::new(),
        first_geometry: None,
        attributes: Vec::new(),
    };
    let group_id = store.new_node(NodeKind::Group(group_node));

    run_add_bbox(&mut store);

    let node = store.get_node(group_id).unwrap();
    let NodeKind::Group(group) = &node.kind else {
        panic!("expected group node");
    };
    assert!(
        !group.bbox_world.is_valid(),
        "empty group should have empty bbox"
    );
}

#[test]
fn test_add_bbox_single_geometry_group() {
    let mut store = Store::new();
    let group_node = GroupNode {
        name: store.intern_string("GeoGroup"),
        translation: Vec3::ZERO,
        material: 0,
        transparency: 0,
        id: -1,
        bbox_world: BBox3::new(),
        first_geometry: None,
        attributes: Vec::new(),
    };
    let group_id = store.new_node(NodeKind::Group(group_node));

    let geo_id = store.new_geometry(
        group_id,
        GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 1.0,
        }),
    );
    let expected = BBox3::from_min_max(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(4.0, 5.0, 6.0));
    if let Some(geo) = store.get_geometry_mut(geo_id) {
        geo.bbox_world = expected;
    }

    run_add_bbox(&mut store);

    let node = store.get_node(group_id).unwrap();
    let NodeKind::Group(group) = &node.kind else {
        panic!("expected group node");
    };
    assert_eq!(group.bbox_world, expected);
}

#[test]
fn test_add_bbox_deep_hierarchy() {
    let mut store = Store::new();
    let root_node = GroupNode {
        name: store.intern_string("Root"),
        translation: Vec3::ZERO,
        material: 0,
        transparency: 0,
        id: -1,
        bbox_world: BBox3::new(),
        first_geometry: None,
        attributes: Vec::new(),
    };
    let root_id = store.new_node(NodeKind::Group(root_node));
    store.push_node_context(root_id);

    let mid_node = GroupNode {
        name: store.intern_string("Mid"),
        translation: Vec3::ZERO,
        material: 0,
        transparency: 0,
        id: -1,
        bbox_world: BBox3::new(),
        first_geometry: None,
        attributes: Vec::new(),
    };
    let mid_id = store.new_node(NodeKind::Group(mid_node));
    store.push_node_context(mid_id);

    let leaf_node = GroupNode {
        name: store.intern_string("Leaf"),
        translation: Vec3::ZERO,
        material: 0,
        transparency: 0,
        id: -1,
        bbox_world: BBox3::new(),
        first_geometry: None,
        attributes: Vec::new(),
    };
    let leaf_id = store.new_node(NodeKind::Group(leaf_node));

    let geo_id = store.new_geometry(
        leaf_id,
        GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 1.0,
        }),
    );
    let expected = BBox3::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 3.0));
    if let Some(geo) = store.get_geometry_mut(geo_id) {
        geo.bbox_world = expected;
    }

    run_add_bbox(&mut store);

    let root_node = store.get_node(root_id).unwrap();
    let mid_node = store.get_node(mid_id).unwrap();
    let leaf_node = store.get_node(leaf_id).unwrap();

    let NodeKind::Group(root_group) = &root_node.kind else {
        panic!("expected root group");
    };
    let NodeKind::Group(mid_group) = &mid_node.kind else {
        panic!("expected mid group");
    };
    let NodeKind::Group(leaf_group) = &leaf_node.kind else {
        panic!("expected leaf group");
    };

    assert_eq!(leaf_group.bbox_world, expected);
    assert_eq!(mid_group.bbox_world, expected);
    assert_eq!(root_group.bbox_world, expected);
}
