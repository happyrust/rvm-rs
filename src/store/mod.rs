pub mod arena;
pub mod geometry;
pub mod node;
pub mod strings;

pub use arena::Arena;
pub use geometry::{Geometry, GeometryId, GeometryKind, GeometryType};
pub use node::{Attribute, FileNode, GroupNode, ModelNode, Node, NodeId, NodeKind};
pub use strings::{StringId, StringInterner};

use crate::math::BBox3;

pub struct Store {
    nodes: Vec<Node>,
    geometries: Vec<Geometry>,
    strings: StringInterner,
    roots: Vec<NodeId>,
    node_count: usize,
    geometry_count: usize,
    current_node_stack: Vec<NodeId>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            geometries: Vec::new(),
            strings: StringInterner::new(),
            roots: Vec::new(),
            node_count: 0,
            geometry_count: 0,
            current_node_stack: Vec::new(),
        }
    }

    pub fn new_node(&mut self, kind: NodeKind) -> NodeId {
        let id = NodeId(self.node_count as u32);
        self.node_count += 1;

        let node = Node {
            kind,
            next: None,
            first_child: None,
            last_child: None,
        };

        self.nodes.push(node);

        // Handle parent-child relationships
        if let Some(&parent_id) = self.current_node_stack.last() {
            let parent_idx = parent_id.0 as usize;
            if self.nodes[parent_idx].first_child.is_none() {
                self.nodes[parent_idx].first_child = Some(id);
                self.nodes[parent_idx].last_child = Some(id);
            } else {
                let last_child_id = self.nodes[parent_idx].last_child.unwrap();
                self.nodes[last_child_id.0 as usize].next = Some(id);
                self.nodes[parent_idx].last_child = Some(id);
            }
        } else {
            self.roots.push(id);
        }

        id
    }

    pub fn new_geometry(&mut self, parent: NodeId, kind: GeometryKind) -> GeometryId {
        let id = GeometryId(self.geometry_count as u32);
        self.geometry_count += 1;

        let geometry = Geometry {
            kind,
            geo_type: GeometryType::Primitive,
            transform: glam::Affine3A::IDENTITY,
            bbox_local: BBox3::new(),
            bbox_world: BBox3::new(),
            color: 0,
            transparency: 0,
            next: None,
        };

        self.geometries.push(geometry);

        // Link to parent node
        if let Some(node) = self.nodes.get_mut(parent.0 as usize) {
            if let NodeKind::Group(ref mut group) = node.kind {
                if group.first_geometry.is_none() {
                    group.first_geometry = Some(id);
                } else {
                    // Find last geometry and link
                    let mut current = group.first_geometry;
                    while let Some(geo_id) = current {
                        if self.geometries[geo_id.0 as usize].next.is_none() {
                            self.geometries[geo_id.0 as usize].next = Some(id);
                            break;
                        }
                        current = self.geometries[geo_id.0 as usize].next;
                    }
                }
            }
        }

        id
    }

    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0 as usize)
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(id.0 as usize)
    }

    pub fn get_geometry(&self, id: GeometryId) -> Option<&Geometry> {
        self.geometries.get(id.0 as usize)
    }

    pub fn get_geometry_mut(&mut self, id: GeometryId) -> Option<&mut Geometry> {
        self.geometries.get_mut(id.0 as usize)
    }

    pub fn find_root_group(&self, name: &str) -> Option<NodeId> {
        for &root_id in &self.roots {
            if let Some(node) = self.get_node(root_id) {
                if let NodeKind::Group(ref group) = node.kind {
                    if self.strings.get(group.name) == name {
                        return Some(root_id);
                    }
                }
            }
        }
        None
    }

    pub fn intern_string(&mut self, s: &str) -> StringId {
        self.strings.intern(s)
    }

    pub fn get_string(&self, id: StringId) -> &str {
        self.strings.get(id)
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn geometry_count(&self) -> usize {
        self.geometry_count
    }

    pub fn push_node_context(&mut self, id: NodeId) {
        self.current_node_stack.push(id);
    }

    pub fn pop_node_context(&mut self) {
        self.current_node_stack.pop();
    }

    pub fn current_node(&self) -> Option<NodeId> {
        self.current_node_stack.last().copied()
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::geometry::Cylinder;

    #[test]
    fn test_store_creation() {
        let store = Store::new();
        assert_eq!(store.node_count(), 0);
        assert_eq!(store.geometry_count(), 0);
    }

    #[test]
    fn test_string_interning() {
        let mut store = Store::new();
        let id1 = store.intern_string("test");
        let id2 = store.intern_string("test");
        let id3 = store.intern_string("other");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(store.get_string(id1), "test");
        assert_eq!(store.get_string(id3), "other");
    }

    #[test]
    fn test_node_creation() {
        let mut store = Store::new();
        let file_node = FileNode {
            info: store.intern_string("info"),
            note: store.intern_string("note"),
            date: store.intern_string("date"),
            user: store.intern_string("user"),
            encoding: store.intern_string("utf8"),
            path: store.intern_string("/path"),
        };

        let node_id = store.new_node(NodeKind::File(file_node));
        assert_eq!(store.node_count(), 1);
        assert!(store.get_node(node_id).is_some());
    }

    #[test]
    fn test_geometry_creation() {
        let mut store = Store::new();
        let group_node = GroupNode {
            name: store.intern_string("group1"),
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        };

        let node_id = store.new_node(NodeKind::Group(group_node));
        let geo_kind = GeometryKind::Cylinder(Cylinder {
            radius: 1.0,
            height: 2.0,
        });
        let geo_id = store.new_geometry(node_id, geo_kind);

        assert_eq!(store.geometry_count(), 1);
        assert!(store.get_geometry(geo_id).is_some());
    }

    #[test]
    fn test_scene_graph_hierarchy() {
        let mut store = Store::new();

        // Create parent group
        let parent_group = GroupNode {
            name: store.intern_string("parent"),
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        };
        let parent_id = store.new_node(NodeKind::Group(parent_group));
        store.push_node_context(parent_id);

        // Create child group
        let child_group = GroupNode {
            name: store.intern_string("child"),
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        };
        let child_id = store.new_node(NodeKind::Group(child_group));

        store.pop_node_context();

        // Verify hierarchy
        let parent = store.get_node(parent_id).unwrap();
        assert_eq!(parent.first_child, Some(child_id));
        assert_eq!(store.node_count(), 2);
    }
}
