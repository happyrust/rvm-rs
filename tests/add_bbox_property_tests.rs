use glam::Vec3;
use proptest::prelude::*;
use rvm_rs::math::BBox3;
use rvm_rs::store::geometry::{Cylinder, GeometryKind};
use rvm_rs::store::{GroupNode, NodeId, NodeKind, Store};
use rvm_rs::visitor::add_bbox::AddGroupBBox;
use rvm_rs::visitor::traverse;

fn build_chain_store(group_bboxes: &[Vec<BBox3>], local_offset: Vec3) -> (Store, Vec<NodeId>) {
    let mut store = Store::new();
    let mut group_ids = Vec::new();

    for (idx, bboxes) in group_bboxes.iter().enumerate() {
        let group_node = GroupNode {
            name: store.intern_string(&format!("G{}", idx)),
            translation: Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        };
        let group_id = store.new_node(NodeKind::Group(group_node));
        group_ids.push(group_id);
        store.push_node_context(group_id);

        for bbox in bboxes {
            let geo_id = store.new_geometry(
                group_id,
                GeometryKind::Cylinder(Cylinder {
                    radius: 1.0,
                    height: 1.0,
                }),
            );
            if let Some(geo) = store.get_geometry_mut(geo_id) {
                geo.bbox_world = *bbox;
                geo.bbox_local =
                    BBox3::from_min_max(bbox.min + local_offset, bbox.max + local_offset);
            }
        }
    }

    (store, group_ids)
}

fn union_bboxes(bboxes: &[BBox3]) -> BBox3 {
    bboxes
        .iter()
        .fold(BBox3::new(), |acc, bbox| acc.union(bbox))
}

fn expected_chain_bboxes(group_bboxes: &[Vec<BBox3>]) -> Vec<BBox3> {
    let mut expected = vec![BBox3::new(); group_bboxes.len()];
    for i in (0..group_bboxes.len()).rev() {
        let mut bbox = union_bboxes(&group_bboxes[i]);
        if i + 1 < group_bboxes.len() {
            bbox = bbox.union(&expected[i + 1]);
        }
        expected[i] = bbox;
    }
    expected
}

fn group_geometry_bboxes(store: &Store, group_id: NodeId) -> Vec<BBox3> {
    let Some(node) = store.get_node(group_id) else {
        return Vec::new();
    };
    let NodeKind::Group(group) = &node.kind else {
        return Vec::new();
    };

    let mut result = Vec::new();
    let mut geo_id = group.first_geometry;
    while let Some(id) = geo_id {
        if let Some(geo) = store.get_geometry(id) {
            result.push(geo.bbox_world);
            geo_id = geo.next;
        } else {
            break;
        }
    }
    result
}

prop_compose! {
    fn bbox_strategy()
        (min in prop::array::uniform3(-10.0f32..10.0),
         size in prop::array::uniform3(0.1f32..5.0))
        -> BBox3 {
        let min_v = Vec3::new(min[0], min[1], min[2]);
        let max_v = Vec3::new(min[0] + size[0], min[1] + size[1], min[2] + size[2]);
        BBox3::from_min_max(min_v, max_v)
    }
}

prop_compose! {
    fn group_bboxes_strategy()(group_count in 1usize..6)
        (group_bboxes in prop::collection::vec(prop::collection::vec(bbox_strategy(), 0..4), group_count),
         group_count in Just(group_count))
        -> (usize, Vec<Vec<BBox3>>) {
        (group_count, group_bboxes)
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 29: BBox Bottom-Up Traversal
    #[test]
    fn prop_add_bbox_bottom_up((_, group_bboxes) in group_bboxes_strategy()) {
        let (mut store, group_ids) = build_chain_store(&group_bboxes, Vec3::ZERO);
        let mut visitor = AddGroupBBox::new();
        traverse(&mut store, &mut visitor);

        let expected = expected_chain_bboxes(&group_bboxes);
        for (idx, group_id) in group_ids.iter().enumerate() {
            let node = store.get_node(*group_id).unwrap();
            let NodeKind::Group(group) = &node.kind else {
                continue;
            };
            prop_assert_eq!(group.bbox_world, expected[idx]);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 30: BBox Geometry Inclusion
    #[test]
    fn prop_add_bbox_geometry_inclusion((_, group_bboxes) in group_bboxes_strategy()) {
        let (mut store, group_ids) = build_chain_store(&group_bboxes, Vec3::ZERO);
        let mut visitor = AddGroupBBox::new();
        traverse(&mut store, &mut visitor);

        for group_id in group_ids {
            let node = store.get_node(group_id).unwrap();
            let NodeKind::Group(group) = &node.kind else {
                continue;
            };
            for geo_bbox in group_geometry_bboxes(&store, group_id) {
                prop_assert!(group.bbox_world.contains(geo_bbox.min));
                prop_assert!(group.bbox_world.contains(geo_bbox.max));
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 31: BBox Hierarchy Propagation
    #[test]
    fn prop_add_bbox_hierarchy_propagation((_, group_bboxes) in group_bboxes_strategy()) {
        let (mut store, group_ids) = build_chain_store(&group_bboxes, Vec3::ZERO);
        let mut visitor = AddGroupBBox::new();
        traverse(&mut store, &mut visitor);

        for window in group_ids.windows(2) {
            let parent_id = window[0];
            let child_id = window[1];

            let parent_node = store.get_node(parent_id).unwrap();
            let child_node = store.get_node(child_id).unwrap();
            let NodeKind::Group(parent) = &parent_node.kind else {
                continue;
            };
            let NodeKind::Group(child) = &child_node.kind else {
                continue;
            };

            if !child.bbox_world.is_valid() {
                continue;
            }

            prop_assert!(parent.bbox_world.contains(child.bbox_world.min));
            prop_assert!(parent.bbox_world.contains(child.bbox_world.max));
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 32: BBox Storage
    #[test]
    fn prop_add_bbox_storage((_, group_bboxes) in group_bboxes_strategy()) {
        let (mut store, group_ids) = build_chain_store(&group_bboxes, Vec3::ZERO);
        let mut visitor = AddGroupBBox::new();
        traverse(&mut store, &mut visitor);

        for (idx, group_id) in group_ids.iter().enumerate() {
            let node = store.get_node(*group_id).unwrap();
            let NodeKind::Group(group) = &node.kind else {
                continue;
            };

            let subtree_has_geometry = group_bboxes[idx..]
                .iter()
                .any(|bboxes| !bboxes.is_empty());

            if subtree_has_geometry {
                prop_assert!(group.bbox_world.is_valid());
            } else {
                prop_assert!(!group.bbox_world.is_valid());
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]
    // Feature: auxiliary-tools-implementation, Property 33: BBox World Space
    #[test]
    fn prop_add_bbox_world_space((_, group_bboxes) in group_bboxes_strategy()) {
        prop_assume!(group_bboxes.iter().any(|bboxes| !bboxes.is_empty()));

        let local_offset = Vec3::new(5.0, -3.0, 2.0);
        let (mut store, group_ids) = build_chain_store(&group_bboxes, local_offset);
        let mut visitor = AddGroupBBox::new();
        traverse(&mut store, &mut visitor);

        let expected_world = expected_chain_bboxes(&group_bboxes);
        for (idx, group_id) in group_ids.iter().enumerate() {
            let node = store.get_node(*group_id).unwrap();
            let NodeKind::Group(group) = &node.kind else {
                continue;
            };
            prop_assert_eq!(group.bbox_world, expected_world[idx]);

            if expected_world[idx].is_valid() {
                let expected_local = BBox3::from_min_max(
                    expected_world[idx].min + local_offset,
                    expected_world[idx].max + local_offset,
                );
                prop_assert_ne!(group.bbox_world, expected_local);
            }
        }
    }
}
