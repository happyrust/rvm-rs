use crate::export::ExportError;
use crate::store::{Geometry, GeometryKind, Node, NodeKind, Store};
use crate::visitor::Visitor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonNode {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f32; 6]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<[f32; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparency: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<JsonNode>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub geometries: Vec<JsonGeometry>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonGeometry {
    pub kind: String,
    pub color: u32,
    pub transparency: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<[f32; 16]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox_local: Option<[f32; 6]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bbox_world: Option<[f32; 6]>,
}

pub struct JsonExporter {
    root_nodes: Vec<JsonNode>,
    node_stack: Vec<JsonNode>,
}

impl JsonExporter {
    pub fn new() -> Self {
        Self {
            root_nodes: Vec::new(),
            node_stack: Vec::new(),
        }
    }

    pub fn write_to_file(self, path: &str) -> Result<(), ExportError> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let output = serde_json::json!({
            "nodes": self.root_nodes,
        });

        serde_json::to_writer_pretty(writer, &output)?;
        Ok(())
    }

    fn bbox_to_array(bbox: &crate::math::BBox3) -> [f32; 6] {
        [
            bbox.min.x, bbox.min.y, bbox.min.z, bbox.max.x, bbox.max.y, bbox.max.z,
        ]
    }

    fn transform_to_array(transform: &glam::Affine3A) -> [f32; 16] {
        let mat = glam::Mat4::from(*transform);
        mat.to_cols_array()
    }
}

impl Default for JsonExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Visitor for JsonExporter {
    fn visit_node(&mut self, node: &Node, store: &Store) {
        let json_node = match &node.kind {
            NodeKind::File(file) => {
                let mut attributes = HashMap::new();
                attributes.insert("info".to_string(), store.get_string(file.info).to_string());
                attributes.insert("note".to_string(), store.get_string(file.note).to_string());
                attributes.insert("date".to_string(), store.get_string(file.date).to_string());
                attributes.insert("user".to_string(), store.get_string(file.user).to_string());
                attributes.insert(
                    "encoding".to_string(),
                    store.get_string(file.encoding).to_string(),
                );
                attributes.insert("path".to_string(), store.get_string(file.path).to_string());

                JsonNode {
                    name: "File".to_string(),
                    bbox: None,
                    attributes: Some(attributes),
                    translation: None,
                    material: None,
                    transparency: None,
                    children: Vec::new(),
                    geometries: Vec::new(),
                }
            }
            NodeKind::Model(model) => {
                let mut attributes = HashMap::new();
                attributes.insert(
                    "project".to_string(),
                    store.get_string(model.project).to_string(),
                );

                JsonNode {
                    name: store.get_string(model.name).to_string(),
                    bbox: None,
                    attributes: Some(attributes),
                    translation: None,
                    material: None,
                    transparency: None,
                    children: Vec::new(),
                    geometries: Vec::new(),
                }
            }
            NodeKind::Group(group) => {
                let mut attributes = HashMap::new();
                for attr in &group.attributes {
                    let key = store.get_string(attr.key).to_string();
                    let value = store.get_string(attr.value).to_string();
                    attributes.insert(key, value);
                }

                JsonNode {
                    name: store.get_string(group.name).to_string(),
                    bbox: Some(Self::bbox_to_array(&group.bbox_world)),
                    attributes: if attributes.is_empty() {
                        None
                    } else {
                        Some(attributes)
                    },
                    translation: Some([
                        group.translation.x,
                        group.translation.y,
                        group.translation.z,
                    ]),
                    material: Some(group.material),
                    transparency: Some(group.transparency),
                    children: Vec::new(),
                    geometries: Vec::new(),
                }
            }
        };

        self.node_stack.push(json_node);
    }

    fn visit_geometry(&mut self, geometry: &Geometry, _store: &Store) {
        let kind_name = match &geometry.kind {
            GeometryKind::Cylinder(_) => "Cylinder",
            GeometryKind::Sphere(_) => "Sphere",
            GeometryKind::Box(_) => "Box",
            GeometryKind::Pyramid(_) => "Pyramid",
            GeometryKind::CircularTorus(_) => "CircularTorus",
            GeometryKind::RectangularTorus(_) => "RectangularTorus",
            GeometryKind::EllipticalDish(_) => "EllipticalDish",
            GeometryKind::SphericalDish(_) => "SphericalDish",
            GeometryKind::Snout(_) => "Snout",
            GeometryKind::Line(_) => "Line",
            GeometryKind::FacetGroup(_) => "FacetGroup",
        };

        let json_geo = JsonGeometry {
            kind: kind_name.to_string(),
            color: geometry.color,
            transparency: geometry.transparency,
            transform: Some(Self::transform_to_array(&geometry.transform)),
            bbox_local: Some(Self::bbox_to_array(&geometry.bbox_local)),
            bbox_world: Some(Self::bbox_to_array(&geometry.bbox_world)),
        };

        if let Some(current_node) = self.node_stack.last_mut() {
            current_node.geometries.push(json_geo);
        }
    }

    fn leave_node(&mut self, _node: &Node, _store: &Store) {
        if let Some(node) = self.node_stack.pop() {
            if let Some(parent) = self.node_stack.last_mut() {
                parent.children.push(node);
            } else {
                self.root_nodes.push(node);
            }
        }
    }
}

// Helper to build the tree structure after traversal
pub fn finalize_json_tree(exporter: &mut JsonExporter) {
    // This is a simplified version - in practice, you'd need to track
    // the hierarchy during traversal and build the tree properly
    if let Some(root) = exporter.node_stack.pop() {
        exporter.root_nodes.push(root);
    }
}
