use crate::store::{NodeId, NodeKind, Store};
use crate::transform::{parse_tag_file, TransformError};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiscardResult {
    pub tags_read: usize,
    pub discarded_groups: usize,
}

pub fn discard_groups_from_file(
    store: &mut Store,
    tag_file: &Path,
) -> Result<DiscardResult, TransformError> {
    let tags = parse_tag_file(tag_file)?;
    let tag_set: HashSet<String> = tags.into_iter().collect();

    let mut discarded = 0usize;
    let roots: Vec<NodeId> = store.roots().to_vec();
    for root_id in roots {
        let models = collect_children(store, root_id);
        for model_id in models {
            discarded += prune_children(store, model_id, &tag_set);
        }
    }

    Ok(DiscardResult {
        tags_read: tag_set.len(),
        discarded_groups: discarded,
    })
}

fn prune_children(store: &mut Store, parent: NodeId, discard_tags: &HashSet<String>) -> usize {
    let mut discarded = 0usize;
    let children = store.take_children(parent);

    for child in children {
        let should_discard = match store.get_node(child) {
            Some(node) => match &node.kind {
                NodeKind::Group(group) => {
                    let name = store.get_string(group.name);
                    discard_tags.contains(name)
                }
                _ => false,
            },
            None => false,
        };

        if should_discard {
            discarded += 1;
            continue;
        }

        discarded += prune_children(store, child, discard_tags);
        store.append_child(parent, child);
    }

    discarded
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
