use crate::store::{Geometry, GeometryKind, Node, NodeKind, Store};
use crate::visitor::Visitor;
use std::collections::HashMap;

pub struct StatsVisitor {
    pub file_count: usize,
    pub model_count: usize,
    pub group_count: usize,
    pub geometry_count: usize,
    pub geometry_type_counts: HashMap<String, usize>,
}

impl StatsVisitor {
    pub fn new() -> Self {
        Self {
            file_count: 0,
            model_count: 0,
            group_count: 0,
            geometry_count: 0,
            geometry_type_counts: HashMap::new(),
        }
    }

    pub fn print_stats(&self) {
        println!("=== RVM File Statistics ===");
        println!("Files: {}", self.file_count);
        println!("Models: {}", self.model_count);
        println!("Groups: {}", self.group_count);
        println!("Geometries: {}", self.geometry_count);
        println!("\nGeometry Types:");

        let mut types: Vec<_> = self.geometry_type_counts.iter().collect();
        types.sort_by_key(|(name, _)| *name);

        for (name, count) in types {
            println!("  {}: {}", name, count);
        }
    }
}

impl Default for StatsVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for StatsVisitor {
    fn visit_node(&mut self, node: &Node, _store: &Store) {
        match &node.kind {
            NodeKind::File(_) => self.file_count += 1,
            NodeKind::Model(_) => self.model_count += 1,
            NodeKind::Group(_) => self.group_count += 1,
        }
    }

    fn visit_geometry(&mut self, geometry: &Geometry, _store: &Store) {
        self.geometry_count += 1;

        let type_name = match &geometry.kind {
            GeometryKind::Pyramid(_) => "Pyramid",
            GeometryKind::Box(_) => "Box",
            GeometryKind::RectangularTorus(_) => "RectangularTorus",
            GeometryKind::CircularTorus(_) => "CircularTorus",
            GeometryKind::EllipticalDish(_) => "EllipticalDish",
            GeometryKind::SphericalDish(_) => "SphericalDish",
            GeometryKind::Snout(_) => "Snout",
            GeometryKind::Cylinder(_) => "Cylinder",
            GeometryKind::Sphere(_) => "Sphere",
            GeometryKind::Line(_) => "Line",
            GeometryKind::FacetGroup(_) => "FacetGroup",
        };

        *self
            .geometry_type_counts
            .entry(type_name.to_string())
            .or_insert(0) += 1;
    }
}
