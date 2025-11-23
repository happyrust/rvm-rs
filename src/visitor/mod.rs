pub mod stats;

use crate::store::{Geometry, Node, NodeId, Store};

pub trait Visitor {
    fn visit_node(&mut self, node: &Node, store: &Store);
    fn visit_geometry(&mut self, geometry: &Geometry, store: &Store);
    fn leave_node(&mut self, _node: &Node, _store: &Store) {}
}

pub fn traverse<V: Visitor>(store: &Store, visitor: &mut V) {
    for &root_id in store.roots() {
        traverse_node(store, root_id, visitor);
    }
}

fn traverse_node<V: Visitor>(store: &Store, node_id: NodeId, visitor: &mut V) {
    if let Some(node) = store.get_node(node_id) {
        visitor.visit_node(node, store);

        // Visit geometries if this is a group node
        if let crate::store::NodeKind::Group(ref group) = node.kind {
            let mut geo_id = group.first_geometry;
            while let Some(id) = geo_id {
                if let Some(geometry) = store.get_geometry(id) {
                    visitor.visit_geometry(geometry, store);
                    geo_id = geometry.next;
                }
            }
        }

        // Visit children
        let mut child_id = node.first_child;
        while let Some(id) = child_id {
            traverse_node(store, id, visitor);
            if let Some(child) = store.get_node(id) {
                child_id = child.next;
            } else {
                break;
            }
        }

        visitor.leave_node(node, store);
    }
}
