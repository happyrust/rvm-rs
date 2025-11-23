use crate::parser::ParseError;
use crate::store::node::{Attribute, NodeId, NodeKind};
use crate::store::strings::StringId;
use crate::store::Store;
use std::collections::HashSet;

pub fn parse_att(input: &str, store: &mut Store) -> Result<(), ParseError> {
    let lines: Vec<&str> = input.lines().collect();
    let mut node_stack: Vec<(usize, NodeId)> = Vec::new(); // (indent_level, node_id)
    let mut used_nodes: HashSet<NodeId> = HashSet::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        let indent = count_indentation(line);
        let trimmed = line.trim();

        if trimmed.starts_with("NEW") {
            // Extract name from NEW line
            let name = trimmed
                .strip_prefix("NEW")
                .unwrap_or("")
                .trim()
                .trim_matches(|c| c == '\'' || c == '"');

            // Pop stack until we find the right parent level
            while let Some(&(parent_indent, _)) = node_stack.last() {
                if parent_indent < indent {
                    break;
                }
                node_stack.pop();
            }

            // Locate the matching node in the existing store (prefer child of current stack top)
            let target = if let Some(&(_, parent_id)) = node_stack.last() {
                find_child_group_by_name(store, parent_id, name, &used_nodes)
            } else {
                find_root_group_by_name(store, name, &used_nodes)
            };

            if let Some(node_id) = target {
                used_nodes.insert(node_id);
                node_stack.push((indent, node_id));
            }
        } else if trimmed == "END" {
            // Pop the current node from stack
            node_stack.pop();
        } else if trimmed.contains(":=") {
            // Parse attribute line
            let attributes = parse_attribute_line(trimmed)?;

            // Add attributes to the current node
            if let Some(&(_, node_id)) = node_stack.last() {
                // First, intern all strings
                let interned_attrs: Vec<(StringId, StringId)> = attributes
                    .iter()
                    .map(|(k, v)| (store.intern_string(k), store.intern_string(v)))
                    .collect();

                // Then, add them to the node
                if let Some(node) = store.get_node_mut(node_id) {
                    if let NodeKind::Group(ref mut group) = node.kind {
                        for (key_id, value_id) in interned_attrs {
                            group.attributes.push(Attribute {
                                key: key_id,
                                value: value_id,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn count_indentation(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn find_root_group_by_name(store: &Store, name: &str, used: &HashSet<NodeId>) -> Option<NodeId> {
    store.roots().iter().copied().find(|id| {
        !used.contains(id)
            && store
                .get_node(*id)
                .and_then(|node| {
                    if let NodeKind::Group(group) = &node.kind {
                        Some(store.get_string(group.name) == name)
                    } else {
                        None
                    }
                })
                .unwrap_or(false)
    })
}

fn find_child_group_by_name(
    store: &Store,
    parent: NodeId,
    name: &str,
    used: &HashSet<NodeId>,
) -> Option<NodeId> {
    let mut child = store
        .get_node(parent)
        .and_then(|node| node.first_child);

    while let Some(id) = child {
        if !used.contains(&id) {
            if let Some(node) = store.get_node(id) {
                if let NodeKind::Group(group) = &node.kind {
                    if store.get_string(group.name) == name {
                        return Some(id);
                    }
                }
                child = node.next;
                continue;
            }
        }
        child = store.get_node(id).and_then(|n| n.next);
    }
    None
}

fn parse_attribute_line(line: &str) -> Result<Vec<(&str, &str)>, ParseError> {
    let mut attributes = Vec::new();

    // Split by &end& separator
    let parts: Vec<&str> = line.split("&end&").collect();

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if let Some(pos) = part.find(":=") {
            let key = part[..pos].trim();
            let value = part[pos + 2..].trim();

            // Remove quotes from value
            let value = value.trim_matches(|c| c == '\'' || c == '"');

            attributes.push((key, value));
        }
    }

    Ok(attributes)
}
