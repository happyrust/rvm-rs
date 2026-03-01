use crate::math::BBox3;
use crate::store::{Geometry, GeometryId, Node, NodeId, NodeKind, Store};
use crate::visitor::Visitor;

pub struct AddGroupBBox {
    stack: Vec<NodeId>,
}

impl AddGroupBBox {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }
}

impl Default for AddGroupBBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for AddGroupBBox {
    fn visit_node(&mut self, node_id: NodeId, node: &Node, store: &mut Store) {
        if let NodeKind::Group(_) = node.kind {
            if let Some(node_mut) = store.get_node_mut(node_id) {
                if let NodeKind::Group(ref mut group) = node_mut.kind {
                    group.bbox_world = BBox3::new();
                }
            }
            self.stack.push(node_id);
        }
    }

    fn visit_geometry(&mut self, _geometry_id: GeometryId, geometry: &Geometry, store: &mut Store) {
        let Some(&group_id) = self.stack.last() else {
            return;
        };

        if let Some(node_mut) = store.get_node_mut(group_id) {
            if let NodeKind::Group(ref mut group) = node_mut.kind {
                group.bbox_world = group.bbox_world.union(&geometry.bbox_world);
            }
        }
    }

    fn leave_node(&mut self, _node_id: NodeId, node: &Node, store: &mut Store) {
        if let NodeKind::Group(_) = node.kind {
            let Some(group_id) = self.stack.pop() else {
                return;
            };

            let current_bbox = match store.get_node(group_id) {
                Some(Node {
                    kind: NodeKind::Group(group),
                    ..
                }) => group.bbox_world,
                _ => return,
            };

            if !current_bbox.is_valid() {
                return;
            }

            if let Some(&parent_id) = self.stack.last() {
                if let Some(parent_node) = store.get_node_mut(parent_id) {
                    if let NodeKind::Group(ref mut parent_group) = parent_node.kind {
                        parent_group.bbox_world = parent_group.bbox_world.union(&current_bbox);
                    }
                }
            }
        }
    }
}
