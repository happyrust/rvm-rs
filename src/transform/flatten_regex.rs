use crate::store::{NodeId, NodeKind, Store};
use crate::transform::TransformError;
use regex::Regex;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlattenRegexResult {
    pub nodes_before: usize,
    pub nodes_after: usize,
    pub geometries_before: usize,
    pub geometries_after: usize,
}

pub fn flatten_regex(
    store: &mut Store,
    pattern: &str,
) -> Result<FlattenRegexResult, TransformError> {
    let regex = Regex::new(pattern)?;
    let (nodes_before, geometries_before) = reachable_counts(store);

    let roots: Vec<NodeId> = store.roots().to_vec();
    for root_id in roots {
        let models = collect_children(store, root_id);
        for model_id in models {
            let groups = collect_children(store, model_id);
            for group_id in groups {
                handle_children(store, &regex, group_id, group_id);
            }
        }
    }

    let (nodes_after, geometries_after) = reachable_counts(store);
    Ok(FlattenRegexResult {
        nodes_before,
        nodes_after,
        geometries_before,
        geometries_after,
    })
}

fn handle_children(
    store: &mut Store,
    regex: &Regex,
    nearest_kept_ancestor: NodeId,
    parent: NodeId,
) {
    let children = store.take_children(parent);

    for child in children {
        if should_keep(store, child, regex) {
            store.append_child(nearest_kept_ancestor, child);
            handle_children(store, regex, child, child);
        } else {
            store.move_attributes(child, nearest_kept_ancestor);
            store.move_geometries(child, nearest_kept_ancestor);
            handle_children(store, regex, nearest_kept_ancestor, child);
        }
    }
}

fn should_keep(store: &Store, node_id: NodeId, regex: &Regex) -> bool {
    let Some(node) = store.get_node(node_id) else {
        return false;
    };
    let NodeKind::Group(group) = &node.kind else {
        return false;
    };
    let name = store.get_string(group.name);
    regex.is_match(name)
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
