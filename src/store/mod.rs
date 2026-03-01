pub mod arena;
pub mod connection;
pub mod geometry;
pub mod node;
pub mod strings;

pub use arena::Arena;
pub use connection::{interfaces_match, Connection, ConnectionFlags, Interface, InterfaceKind};
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
        let id = self.new_node_detached(kind);

        // Handle parent-child relationships
        if let Some(&parent_id) = self.current_node_stack.last() {
            self.append_child(parent_id, id);
        } else {
            self.roots.push(id);
        }

        id
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        let parent_idx = parent.0 as usize;
        let child_idx = child.0 as usize;
        if parent_idx >= self.nodes.len() || child_idx >= self.nodes.len() {
            return;
        }

        self.nodes[child_idx].next = None;

        if self.nodes[parent_idx].first_child.is_none() {
            self.nodes[parent_idx].first_child = Some(child);
            self.nodes[parent_idx].last_child = Some(child);
            return;
        }

        if let Some(last_child) = self.nodes[parent_idx].last_child {
            self.nodes[last_child.0 as usize].next = Some(child);
        } else {
            self.nodes[parent_idx].first_child = Some(child);
        }
        self.nodes[parent_idx].last_child = Some(child);
    }

    pub fn take_children(&mut self, parent: NodeId) -> Vec<NodeId> {
        let parent_idx = parent.0 as usize;
        if parent_idx >= self.nodes.len() {
            return Vec::new();
        }

        let mut children = Vec::new();
        let mut current = self.nodes[parent_idx].first_child;
        self.nodes[parent_idx].first_child = None;
        self.nodes[parent_idx].last_child = None;

        while let Some(child_id) = current {
            let child_idx = child_id.0 as usize;
            let next = self.nodes[child_idx].next;
            self.nodes[child_idx].next = None;
            children.push(child_id);
            current = next;
        }

        children
    }

    pub fn remove_child_node(&mut self, parent: NodeId, child: NodeId) -> bool {
        let parent_idx = parent.0 as usize;
        let child_idx = child.0 as usize;
        if parent_idx >= self.nodes.len() || child_idx >= self.nodes.len() {
            return false;
        }

        let mut prev: Option<NodeId> = None;
        let mut current = self.nodes[parent_idx].first_child;
        while let Some(current_id) = current {
            if current_id == child {
                let next = self.nodes[child_idx].next;
                if let Some(prev_id) = prev {
                    self.nodes[prev_id.0 as usize].next = next;
                } else {
                    self.nodes[parent_idx].first_child = next;
                }

                if self.nodes[parent_idx].last_child == Some(child) {
                    self.nodes[parent_idx].last_child = prev;
                }

                if self.nodes[parent_idx].first_child.is_none() {
                    self.nodes[parent_idx].last_child = None;
                }

                self.nodes[child_idx].next = None;
                return true;
            }

            prev = Some(current_id);
            current = self.nodes[current_id.0 as usize].next;
        }

        false
    }

    pub fn move_attributes(&mut self, from: NodeId, to: NodeId) -> usize {
        if from == to {
            return 0;
        }

        let from_idx = from.0 as usize;
        let to_idx = to.0 as usize;
        if from_idx >= self.nodes.len() || to_idx >= self.nodes.len() {
            return 0;
        }

        if from_idx == to_idx {
            return 0;
        }

        let (from_node, to_node) = if from_idx < to_idx {
            let (left, right) = self.nodes.split_at_mut(to_idx);
            (&mut left[from_idx], &mut right[0])
        } else {
            let (left, right) = self.nodes.split_at_mut(from_idx);
            (&mut right[0], &mut left[to_idx])
        };

        match (&mut from_node.kind, &mut to_node.kind) {
            (NodeKind::Group(from_group), NodeKind::Group(to_group)) => {
                let moved = from_group.attributes.len();
                to_group.attributes.append(&mut from_group.attributes);
                moved
            }
            _ => 0,
        }
    }

    pub fn move_geometries(&mut self, from: NodeId, to: NodeId) -> usize {
        if from == to {
            return 0;
        }

        let from_idx = from.0 as usize;
        let to_idx = to.0 as usize;
        if from_idx >= self.nodes.len() || to_idx >= self.nodes.len() {
            return 0;
        }

        let from_head = match self.nodes[from_idx].kind {
            NodeKind::Group(ref mut group) => group.first_geometry.take(),
            _ => return 0,
        };
        let Some(from_head) = from_head else {
            return 0;
        };

        let mut moved = 0usize;
        let mut from_tail = from_head;
        loop {
            moved += 1;
            let next = self.geometries[from_tail.0 as usize].next;
            match next {
                Some(next_id) => from_tail = next_id,
                None => break,
            }
        }

        let to_first = match self.nodes[to_idx].kind {
            NodeKind::Group(ref group) => group.first_geometry,
            _ => return 0,
        };

        if let Some(to_head) = to_first {
            let mut to_tail = to_head;
            loop {
                let next = self.geometries[to_tail.0 as usize].next;
                match next {
                    Some(next_id) => to_tail = next_id,
                    None => break,
                }
            }
            self.geometries[to_tail.0 as usize].next = Some(from_head);
        } else if let NodeKind::Group(ref mut group) = self.nodes[to_idx].kind {
            group.first_geometry = Some(from_head);
        }

        self.geometries[from_tail.0 as usize].next = None;
        moved
    }

    pub fn clone_node(&mut self, parent: NodeId, source: NodeId) -> NodeId {
        let mut source_kind = self
            .get_node(source)
            .map(|node| node.kind.clone())
            .unwrap_or_else(|| panic!("clone_node source not found: {:?}", source));
        if let NodeKind::Group(group) = &mut source_kind {
            group.first_geometry = None;
        }
        let new_id = self.new_node_detached(source_kind);
        self.append_child(parent, new_id);
        new_id
    }

    pub fn clone_geometry(&mut self, parent: NodeId, source: GeometryId) -> GeometryId {
        let source_geo = self
            .get_geometry(source)
            .cloned()
            .unwrap_or_else(|| panic!("clone_geometry source not found: {:?}", source));

        let id = GeometryId(self.geometry_count as u32);
        self.geometry_count += 1;

        let mut geometry = source_geo;
        geometry.next = None;
        self.geometries.push(geometry);
        self.append_geometry(parent, id);
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
            sample_start_angle: 0.0,
            color_name: None,
            color_rgb: 0,
            next: None,
        };

        self.geometries.push(geometry);
        self.append_geometry(parent, id);

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

    fn append_geometry(&mut self, parent: NodeId, geometry: GeometryId) {
        let parent_idx = parent.0 as usize;
        if parent_idx >= self.nodes.len() {
            return;
        }
        let geometry_idx = geometry.0 as usize;
        if geometry_idx >= self.geometries.len() {
            return;
        }
        self.geometries[geometry_idx].next = None;

        if let NodeKind::Group(ref mut group) = self.nodes[parent_idx].kind {
            if group.first_geometry.is_none() {
                group.first_geometry = Some(geometry);
            } else {
                let mut tail = group.first_geometry;
                while let Some(tail_id) = tail {
                    let next = self.geometries[tail_id.0 as usize].next;
                    if next.is_none() {
                        self.geometries[tail_id.0 as usize].next = Some(geometry);
                        break;
                    }
                    tail = next;
                }
            }
        }
    }

    fn new_node_detached(&mut self, kind: NodeKind) -> NodeId {
        let id = NodeId(self.node_count as u32);
        self.node_count += 1;
        self.nodes.push(Node {
            kind,
            next: None,
            first_child: None,
            last_child: None,
        });
        id
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

    fn geometry_chain_len(store: &Store, group_id: NodeId) -> usize {
        let mut count = 0usize;
        let mut geo = match store.get_node(group_id) {
            Some(Node {
                kind: NodeKind::Group(group),
                ..
            }) => group.first_geometry,
            _ => None,
        };

        while let Some(geo_id) = geo {
            count += 1;
            geo = store.get_geometry(geo_id).and_then(|g| g.next);
        }
        count
    }

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
            id: -1,
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
            id: -1,
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
            id: -1,
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

    #[test]
    fn test_take_children_and_append_child() {
        let mut store = Store::new();
        let parent_name = store.intern_string("parent");
        let parent = store.new_node(NodeKind::Group(GroupNode {
            name: parent_name,
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        }));

        store.push_node_context(parent);
        let child1_name = store.intern_string("child1");
        let child1 = store.new_node(NodeKind::Group(GroupNode {
            name: child1_name,
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        }));
        let child2_name = store.intern_string("child2");
        let child2 = store.new_node(NodeKind::Group(GroupNode {
            name: child2_name,
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        }));
        store.pop_node_context();

        let taken = store.take_children(parent);
        assert_eq!(taken, vec![child1, child2]);
        let parent_node = store.get_node(parent).unwrap();
        assert!(parent_node.first_child.is_none());
        assert!(parent_node.last_child.is_none());

        store.append_child(parent, child2);
        store.append_child(parent, child1);
        let parent_node = store.get_node(parent).unwrap();
        assert_eq!(parent_node.first_child, Some(child2));
        assert_eq!(parent_node.last_child, Some(child1));
    }

    #[test]
    fn test_move_attributes_and_geometries() {
        let mut store = Store::new();
        let g1_name = store.intern_string("g1");
        let g1 = store.new_node(NodeKind::Group(GroupNode {
            name: g1_name,
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        }));
        let g2_name = store.intern_string("g2");
        let g2 = store.new_node(NodeKind::Group(GroupNode {
            name: g2_name,
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        }));

        let attr_key = store.intern_string("k");
        let attr_value = store.intern_string("v");
        if let Some(Node {
            kind: NodeKind::Group(group),
            ..
        }) = store.get_node_mut(g1)
        {
            group.attributes.push(Attribute {
                key: attr_key,
                value: attr_value,
            });
        }
        store.new_geometry(
            g1,
            GeometryKind::Cylinder(Cylinder {
                radius: 1.0,
                height: 1.0,
            }),
        );
        store.new_geometry(
            g1,
            GeometryKind::Cylinder(Cylinder {
                radius: 2.0,
                height: 2.0,
            }),
        );

        assert_eq!(store.move_attributes(g1, g2), 1);
        assert_eq!(store.move_geometries(g1, g2), 2);

        let g1_node = store.get_node(g1).unwrap();
        let g2_node = store.get_node(g2).unwrap();
        if let NodeKind::Group(group) = &g1_node.kind {
            assert!(group.attributes.is_empty());
            assert!(group.first_geometry.is_none());
        } else {
            panic!("g1 should be group");
        }
        if let NodeKind::Group(group) = &g2_node.kind {
            assert_eq!(group.attributes.len(), 1);
            assert_eq!(geometry_chain_len(&store, g2), 2);
        } else {
            panic!("g2 should be group");
        }
    }

    #[test]
    fn test_clone_node_and_geometry() {
        let mut store = Store::new();
        let parent_name = store.intern_string("parent");
        let parent = store.new_node(NodeKind::Group(GroupNode {
            name: parent_name,
            translation: glam::Vec3::ZERO,
            material: 0,
            transparency: 0,
            id: -1,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: Vec::new(),
        }));
        store.push_node_context(parent);
        let source_name = store.intern_string("source");
        let attr_key = store.intern_string("Color");
        let attr_value = store.intern_string("Blue");
        let source = store.new_node(NodeKind::Group(GroupNode {
            name: source_name,
            translation: glam::Vec3::new(1.0, 2.0, 3.0),
            material: 7,
            transparency: 12,
            id: 11,
            bbox_world: BBox3::new(),
            first_geometry: None,
            attributes: vec![Attribute {
                key: attr_key,
                value: attr_value,
            }],
        }));
        store.pop_node_context();
        let source_geo = store.new_geometry(
            source,
            GeometryKind::Cylinder(Cylinder {
                radius: 3.0,
                height: 4.0,
            }),
        );

        let cloned = store.clone_node(parent, source);
        store.clone_geometry(cloned, source_geo);

        let cloned_node = store.get_node(cloned).unwrap();
        match &cloned_node.kind {
            NodeKind::Group(group) => {
                assert_eq!(store.get_string(group.name), "source");
                assert_eq!(group.material, 7);
                assert_eq!(group.attributes.len(), 1);
                assert_eq!(geometry_chain_len(&store, cloned), 1);
            }
            _ => panic!("cloned node should be group"),
        }
    }
}
