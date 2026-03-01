pub mod add_bbox;
pub mod colorizer;
pub mod dump_names;
pub mod stats;

use crate::store::{Geometry, GeometryId, Node, NodeId, Store};

pub trait Visitor {
    fn visit_node(&mut self, node_id: NodeId, node: &Node, store: &mut Store);
    fn visit_geometry(&mut self, geometry_id: GeometryId, geometry: &Geometry, store: &mut Store);
    fn leave_node(&mut self, _node_id: NodeId, _node: &Node, _store: &mut Store) {}
}

pub fn traverse<V: Visitor>(store: &mut Store, visitor: &mut V) {
    let root_ids: Vec<NodeId> = store.roots().to_vec();
    for root_id in root_ids {
        traverse_node(store, root_id, visitor);
    }
}

fn traverse_node<V: Visitor>(store: &mut Store, node_id: NodeId, visitor: &mut V) {
    let node = match store.get_node(node_id) {
        Some(node) => node.clone(),
        None => return,
    };

    visitor.visit_node(node_id, &node, store);

    if let crate::store::NodeKind::Group(ref group) = node.kind {
        let mut geo_id = group.first_geometry;
        while let Some(id) = geo_id {
            let (geometry, next_id) = match store.get_geometry(id) {
                Some(geometry) => (geometry.clone(), geometry.next),
                None => break,
            };
            visitor.visit_geometry(id, &geometry, store);
            geo_id = next_id;
        }
    }

    let mut child_id = node.first_child;
    while let Some(id) = child_id {
        traverse_node(store, id, visitor);
        child_id = store.get_node(id).and_then(|child| child.next);
    }

    visitor.leave_node(node_id, &node, store);
}
