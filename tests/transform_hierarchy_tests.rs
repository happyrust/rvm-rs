use rvm_rs::math::BBox3;
use rvm_rs::store::geometry::Cylinder;
use rvm_rs::store::{
    Attribute, FileNode, GeometryKind, GroupNode, ModelNode, NodeId, NodeKind, Store,
};
use rvm_rs::transform::{discard_groups_from_file, flatten_keep_from_file, flatten_regex};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn build_sample_store() -> Store {
    let mut store = Store::new();

    let info = store.intern_string("info");
    let note = store.intern_string("note");
    let date = store.intern_string("date");
    let user = store.intern_string("user");
    let encoding = store.intern_string("utf8");
    let path = store.intern_string("sample.rvm");
    let file = store.new_node(NodeKind::File(FileNode {
        info,
        note,
        date,
        user,
        encoding,
        path,
    }));
    store.push_node_context(file);

    let project = store.intern_string("project");
    let model_name = store.intern_string("model");
    let model = store.new_node(NodeKind::Model(ModelNode {
        project,
        name: model_name,
        first_color: None,
    }));
    store.push_node_context(model);

    let root = new_group(&mut store, "ROOT");
    store.push_node_context(root);

    let match_group = new_group(&mut store, "MATCH");
    store.push_node_context(match_group);
    let keep_target = new_group(&mut store, "KEEP_TARGET");
    add_group_attribute(&mut store, keep_target, "k_keep", "v_keep");
    add_group_geometry(&mut store, keep_target, 1.0, 1.0);
    store.pop_node_context();
    store.pop_node_context();

    let drop_group = new_group(&mut store, "DROP");
    add_group_attribute(&mut store, drop_group, "k_drop", "v_drop");
    add_group_geometry(&mut store, drop_group, 2.0, 2.0);
    store.push_node_context(drop_group);
    let drop_leaf = new_group(&mut store, "DROP_LEAF");
    add_group_attribute(&mut store, drop_leaf, "k_leaf", "v_leaf");
    add_group_geometry(&mut store, drop_leaf, 3.0, 3.0);
    store.pop_node_context();

    store.pop_node_context();
    store.pop_node_context();
    store.pop_node_context();

    store
}

fn new_group(store: &mut Store, name: &str) -> NodeId {
    let name_id = store.intern_string(name);
    store.new_node(NodeKind::Group(GroupNode {
        name: name_id,
        translation: glam::Vec3::ZERO,
        material: 0,
        transparency: 0,
        id: -1,
        bbox_world: BBox3::new(),
        first_geometry: None,
        attributes: Vec::new(),
    }))
}

fn add_group_attribute(store: &mut Store, group_id: NodeId, key: &str, value: &str) {
    let key_id = store.intern_string(key);
    let value_id = store.intern_string(value);
    if let Some(node) = store.get_node_mut(group_id) {
        if let NodeKind::Group(group) = &mut node.kind {
            group.attributes.push(Attribute {
                key: key_id,
                value: value_id,
            });
        }
    }
}

fn add_group_geometry(store: &mut Store, group_id: NodeId, radius: f32, height: f32) {
    store.new_geometry(
        group_id,
        GeometryKind::Cylinder(Cylinder { radius, height }),
    );
}

fn temp_file_with_content(content: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rvm_rs_transform_tags_{unique}.txt"));
    std::fs::write(&path, content).expect("write temp tag file");
    path
}

fn group_exists(store: &Store, name: &str) -> bool {
    find_group(store, name).is_some()
}

fn find_group(store: &Store, name: &str) -> Option<NodeId> {
    for &root in store.roots() {
        if let Some(found) = find_group_recurse(store, root, name) {
            return Some(found);
        }
    }
    None
}

fn find_group_recurse(store: &Store, node_id: NodeId, name: &str) -> Option<NodeId> {
    let node = store.get_node(node_id)?;
    if let NodeKind::Group(group) = &node.kind {
        if store.get_string(group.name) == name {
            return Some(node_id);
        }
    }

    let mut child = node.first_child;
    while let Some(child_id) = child {
        if let Some(found) = find_group_recurse(store, child_id, name) {
            return Some(found);
        }
        child = store.get_node(child_id).and_then(|n| n.next);
    }
    None
}

fn group_geometry_count(store: &Store, group_id: NodeId) -> usize {
    let Some(node) = store.get_node(group_id) else {
        return 0;
    };
    let NodeKind::Group(group) = &node.kind else {
        return 0;
    };

    let mut count = 0usize;
    let mut geo = group.first_geometry;
    while let Some(geo_id) = geo {
        count += 1;
        geo = store.get_geometry(geo_id).and_then(|g| g.next);
    }
    count
}

fn group_attribute_count(store: &Store, group_id: NodeId) -> usize {
    let Some(node) = store.get_node(group_id) else {
        return 0;
    };
    let NodeKind::Group(group) = &node.kind else {
        return 0;
    };
    group.attributes.len()
}

#[test]
fn test_discard_groups_from_file_prunes_subtree() {
    let mut store = build_sample_store();
    let tags_file = temp_file_with_content("DROP\n");

    let result = discard_groups_from_file(&mut store, &tags_file).expect("discard should succeed");
    let _ = std::fs::remove_file(&tags_file);

    assert_eq!(result.tags_read, 1);
    assert_eq!(result.discarded_groups, 1);
    assert!(!group_exists(&store, "DROP"));
    assert!(!group_exists(&store, "DROP_LEAF"));
    assert!(group_exists(&store, "MATCH"));
}

#[test]
fn test_flatten_regex_moves_data_to_nearest_kept_ancestor() {
    let mut store = build_sample_store();
    let result = flatten_regex(&mut store, "^MATCH$").expect("flatten regex should succeed");

    assert!(result.nodes_after < result.nodes_before);
    assert!(group_exists(&store, "DROP"));
    assert!(!group_exists(&store, "DROP_LEAF"));

    let drop = find_group(&store, "DROP").expect("DROP should exist");
    assert!(group_geometry_count(&store, drop) >= 2);
    assert!(group_attribute_count(&store, drop) >= 2);
}

#[test]
fn test_flatten_keep_from_file_builds_pruned_copy() {
    let source = build_sample_store();
    let tags_file = temp_file_with_content("KEEP_TARGET\n");

    let (new_store, result) =
        flatten_keep_from_file(&source, &tags_file).expect("flatten keep should succeed");
    let _ = std::fs::remove_file(&tags_file);

    assert_eq!(result.tags_read, 1);
    assert_eq!(result.active_tags, 1);
    assert!(group_exists(&new_store, "KEEP_TARGET"));
    assert!(group_exists(&new_store, "DROP"));
    assert!(group_exists(&new_store, "DROP_LEAF"));

    let drop_group = find_group(&new_store, "DROP").expect("DROP should remain at level 1");
    let drop_leaf = find_group(&new_store, "DROP_LEAF").expect("DROP_LEAF should remain");
    assert_eq!(group_geometry_count(&new_store, drop_group), 1);
    assert_eq!(group_geometry_count(&new_store, drop_leaf), 1);
}
