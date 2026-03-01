use crate::store::{
    Attribute, FileNode, Geometry, GeometryId, GroupNode, ModelNode, NodeId, NodeKind, Store,
};
use crate::transform::{parse_tag_file, TransformError};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlattenKeepResult {
    pub tags_read: usize,
    pub active_tags: usize,
    pub nodes_before: usize,
    pub nodes_after: usize,
    pub geometries_before: usize,
    pub geometries_after: usize,
}

pub fn flatten_keep_from_file(
    source_store: &Store,
    tag_file: &Path,
) -> Result<(Store, FlattenKeepResult), TransformError> {
    let tags = parse_tag_file(tag_file)?;
    let mut selected_tag_index: HashMap<String, i32> = HashMap::new();
    for (idx, tag) in tags.iter().enumerate() {
        selected_tag_index.insert(tag.clone(), idx as i32);
    }

    let source_group_names = collect_all_group_names(source_store);
    let active_tags = tags
        .iter()
        .filter(|name| source_group_names.contains((*name).as_str()))
        .count();

    let (nodes_before, geometries_before) = reachable_counts(source_store);
    let marks = mark_groups(source_store, &selected_tag_index);
    let mut dest_store = Store::new();
    copy_pruned_store(source_store, &mut dest_store, &marks);
    let (nodes_after, geometries_after) = reachable_counts(&dest_store);

    Ok((
        dest_store,
        FlattenKeepResult {
            tags_read: tags.len(),
            active_tags,
            nodes_before,
            nodes_after,
            geometries_before,
            geometries_after,
        },
    ))
}

fn mark_groups(source: &Store, selected_tag_index: &HashMap<String, i32>) -> HashMap<NodeId, i32> {
    let mut marks = HashMap::new();

    for root in source.roots() {
        let models = collect_children(source, *root);
        for model in models {
            let groups = collect_children(source, model);
            for group in groups {
                mark_group_recursive(source, group, -1, selected_tag_index, &mut marks);
            }
        }
    }

    marks
}

fn mark_group_recursive(
    source: &Store,
    group_id: NodeId,
    inherited_id: i32,
    selected_tag_index: &HashMap<String, i32>,
    marks: &mut HashMap<NodeId, i32>,
) {
    let Some(node) = source.get_node(group_id) else {
        return;
    };
    let NodeKind::Group(group) = &node.kind else {
        return;
    };

    let name = source.get_string(group.name);
    let current_id = selected_tag_index
        .get(name)
        .copied()
        .unwrap_or(inherited_id);
    marks.insert(group_id, current_id);

    let children = collect_children(source, group_id);
    for child in children {
        mark_group_recursive(source, child, current_id, selected_tag_index, marks);
    }
}

fn copy_pruned_store(source: &Store, dest: &mut Store, marks: &HashMap<NodeId, i32>) {
    for root in source.roots() {
        let root_kind = copy_node_kind(source, dest, *root);
        let dst_root = dest.new_node(root_kind);
        dest.push_node_context(dst_root);

        let models = collect_children(source, *root);
        for src_model in models {
            let model_kind = copy_node_kind(source, dest, src_model);
            let dst_model = dest.new_node(model_kind);
            dest.push_node_context(dst_model);

            let groups = collect_children(source, src_model);
            for src_group in groups {
                build_pruned_copy_recurse(source, dest, marks, src_group, dst_model, 0);
            }

            dest.pop_node_context();
        }

        dest.pop_node_context();
    }
}

fn build_pruned_copy_recurse(
    source: &Store,
    dest: &mut Store,
    marks: &HashMap<NodeId, i32>,
    src_group: NodeId,
    dst_parent: NodeId,
    level: usize,
) {
    let Some(src_node) = source.get_node(src_group) else {
        return;
    };
    let NodeKind::Group(_) = src_node.kind else {
        return;
    };

    let mut mark = marks.get(&src_group).copied().unwrap_or(-1);
    if mark == -1 && level < 2 {
        mark = -2;
    }

    let active_parent = if mark != -1 {
        let mut group_kind = copy_node_kind(source, dest, src_group);
        if let NodeKind::Group(group) = &mut group_kind {
            group.id = mark;
        }
        dest.push_node_context(dst_parent);
        let new_group = dest.new_node(group_kind);
        dest.pop_node_context();
        new_group
    } else {
        dst_parent
    };

    let geometries = collect_group_geometries(source, src_group);
    for src_geo in geometries {
        clone_geometry_between_stores(source, dest, src_geo, active_parent);
    }

    let children = collect_children(source, src_group);
    for src_child in children {
        build_pruned_copy_recurse(source, dest, marks, src_child, active_parent, level + 1);
    }
}

fn clone_geometry_between_stores(
    source: &Store,
    dest: &mut Store,
    source_geometry: GeometryId,
    parent: NodeId,
) {
    let Some(geo) = source.get_geometry(source_geometry) else {
        return;
    };
    let source_geo: Geometry = geo.clone();
    let new_geo = dest.new_geometry(parent, source_geo.kind.clone());

    let color_name = source_geo.color_name.map(|id| {
        let name = source.get_string(id);
        dest.intern_string(name)
    });

    if let Some(geo_mut) = dest.get_geometry_mut(new_geo) {
        geo_mut.geo_type = source_geo.geo_type;
        geo_mut.transform = source_geo.transform;
        geo_mut.bbox_local = source_geo.bbox_local;
        geo_mut.bbox_world = source_geo.bbox_world;
        geo_mut.color = source_geo.color;
        geo_mut.transparency = source_geo.transparency;
        geo_mut.sample_start_angle = source_geo.sample_start_angle;
        geo_mut.color_name = color_name;
        geo_mut.color_rgb = source_geo.color_rgb;
    }
}

fn copy_node_kind(source: &Store, dest: &mut Store, node_id: NodeId) -> NodeKind {
    let node = source
        .get_node(node_id)
        .unwrap_or_else(|| panic!("source node not found: {:?}", node_id));
    match &node.kind {
        NodeKind::File(file) => NodeKind::File(FileNode {
            info: dest.intern_string(source.get_string(file.info)),
            note: dest.intern_string(source.get_string(file.note)),
            date: dest.intern_string(source.get_string(file.date)),
            user: dest.intern_string(source.get_string(file.user)),
            encoding: dest.intern_string(source.get_string(file.encoding)),
            path: dest.intern_string(source.get_string(file.path)),
        }),
        NodeKind::Model(model) => NodeKind::Model(ModelNode {
            project: dest.intern_string(source.get_string(model.project)),
            name: dest.intern_string(source.get_string(model.name)),
            first_color: model.first_color,
        }),
        NodeKind::Group(group) => {
            let attributes = group
                .attributes
                .iter()
                .map(|att| Attribute {
                    key: dest.intern_string(source.get_string(att.key)),
                    value: dest.intern_string(source.get_string(att.value)),
                })
                .collect::<Vec<_>>();
            NodeKind::Group(GroupNode {
                name: dest.intern_string(source.get_string(group.name)),
                translation: group.translation,
                material: group.material,
                transparency: group.transparency,
                id: group.id,
                bbox_world: group.bbox_world,
                first_geometry: None,
                attributes,
            })
        }
    }
}

fn collect_children(store: &Store, parent: NodeId) -> Vec<NodeId> {
    let mut children = Vec::new();
    let mut current = store.get_node(parent).and_then(|n| n.first_child);
    while let Some(id) = current {
        children.push(id);
        current = store.get_node(id).and_then(|n| n.next);
    }
    children
}

fn collect_group_geometries(store: &Store, group_id: NodeId) -> Vec<GeometryId> {
    let mut geometries = Vec::new();
    let mut current = match store.get_node(group_id) {
        Some(node) => match &node.kind {
            NodeKind::Group(group) => group.first_geometry,
            _ => None,
        },
        None => None,
    };

    while let Some(geo_id) = current {
        geometries.push(geo_id);
        current = store.get_geometry(geo_id).and_then(|g| g.next);
    }

    geometries
}

fn collect_all_group_names(store: &Store) -> HashSet<String> {
    let mut names = HashSet::new();
    for root in store.roots() {
        collect_group_names_recurse(store, *root, &mut names);
    }
    names
}

fn collect_group_names_recurse(store: &Store, node_id: NodeId, names: &mut HashSet<String>) {
    let Some(node) = store.get_node(node_id) else {
        return;
    };
    if let NodeKind::Group(group) = &node.kind {
        names.insert(store.get_string(group.name).to_string());
    }
    let mut child = node.first_child;
    while let Some(child_id) = child {
        collect_group_names_recurse(store, child_id, names);
        child = store.get_node(child_id).and_then(|n| n.next);
    }
}

fn reachable_counts(store: &Store) -> (usize, usize) {
    let mut node_count = 0usize;
    let mut geometry_count = 0usize;

    for root_id in store.roots() {
        count_node_recursive(store, *root_id, &mut node_count, &mut geometry_count);
    }

    (node_count, geometry_count)
}

fn count_node_recursive(
    store: &Store,
    node_id: NodeId,
    node_count: &mut usize,
    geometry_count: &mut usize,
) {
    let Some(node) = store.get_node(node_id) else {
        return;
    };
    *node_count += 1;

    if let NodeKind::Group(group) = &node.kind {
        let mut geo = group.first_geometry;
        while let Some(geo_id) = geo {
            *geometry_count += 1;
            geo = store.get_geometry(geo_id).and_then(|g| g.next);
        }
    }

    let mut child = node.first_child;
    while let Some(child_id) = child {
        count_node_recursive(store, child_id, node_count, geometry_count);
        child = store.get_node(child_id).and_then(|n| n.next);
    }
}
