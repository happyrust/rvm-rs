use crate::export::{ExportError, Tessellate};
use crate::store::{Geometry, GeometryId, GeometryKind, Node, NodeId, NodeKind, Store};
use crate::visitor::Visitor;
use serde_json::{json, Map, Value as JsonValue};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Debug, Clone)]
pub struct GltfExportOptions {
    pub center_model: bool,
    pub rotate_z_to_y: bool,
    pub include_attributes: bool,
    pub merge_geometries: bool,
    pub binary_format: bool,
    pub tolerance: f32,
}

impl Default for GltfExportOptions {
    fn default() -> Self {
        Self {
            center_model: false,
            rotate_z_to_y: false,
            include_attributes: false,
            merge_geometries: false,
            binary_format: false,
            tolerance: 0.1,
        }
    }
}

pub struct GltfExporter {
    nodes: Vec<JsonValue>,
    meshes: Vec<JsonValue>,
    accessors: Vec<JsonValue>,
    buffer_views: Vec<JsonValue>,
    materials: Vec<JsonValue>,
    buffer_data: Vec<u8>,
    defined_materials: HashMap<u64, u32>,
    node_stack: Vec<usize>,
    scene_min: Option<glam::Vec3>,
    scene_max: Option<glam::Vec3>,
    position_slices: Vec<(usize, usize)>, // (byte_offset, f32_len)
    position_accessors: Vec<u32>,
    options: GltfExportOptions,
}

impl GltfExporter {
    pub fn new(options: GltfExportOptions) -> Self {
        Self {
            nodes: Vec::new(),
            meshes: Vec::new(),
            accessors: Vec::new(),
            buffer_views: Vec::new(),
            materials: Vec::new(),
            buffer_data: Vec::new(),
            defined_materials: HashMap::new(),
            node_stack: Vec::new(),
            scene_min: None,
            scene_max: None,
            position_slices: Vec::new(),
            position_accessors: Vec::new(),
            options,
        }
    }

    fn material_key(color: u32, transparency: u32) -> u64 {
        ((color as u64) << 32) | (transparency as u64)
    }

    fn create_material(&mut self, color: u32, transparency: u32) -> u32 {
        let key = Self::material_key(color, transparency);

        if let Some(&mat_idx) = self.defined_materials.get(&key) {
            return mat_idx;
        }

        let mat_idx = self.materials.len() as u32;
        self.defined_materials.insert(key, mat_idx);

        // Extract RGB from color
        let r = ((color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color >> 8) & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;

        // Calculate alpha
        let alpha = 1.0 - (transparency as f32 / 100.0);

        let material = json!({
            "name": format!("mat_{}_{}", color, transparency),
            "pbrMetallicRoughness": {
                "baseColorFactor": [r, g, b, alpha],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.9
            }
        });

        self.materials.push(material);
        mat_idx
    }

    fn align_buffer(buffer: &mut Vec<u8>, alignment: usize) {
        let remainder = buffer.len() % alignment;
        if remainder != 0 {
            let padding = alignment - remainder;
            buffer.resize(buffer.len() + padding, 0);
        }
    }

    fn create_accessor_vec3(&mut self, data: &[f32]) -> u32 {
        self.create_accessor_vec3_with_options(data, false)
    }

    fn create_accessor_vec3_with_options(&mut self, data: &[f32], is_position: bool) -> u32 {
        let accessor_idx = self.accessors.len() as u32;
        let buffer_view_idx = self.buffer_views.len() as u32;

        // Align to 4 bytes
        Self::align_buffer(&mut self.buffer_data, 4);

        let byte_offset = self.buffer_data.len();

        // Write data
        for &value in data {
            self.buffer_data.extend_from_slice(&value.to_le_bytes());
        }

        let byte_length = self.buffer_data.len() - byte_offset;

        // Calculate min/max
        let mut min = [f32::MAX, f32::MAX, f32::MAX];
        let mut max = [f32::MIN, f32::MIN, f32::MIN];

        for i in (0..data.len()).step_by(3) {
            for j in 0..3 {
                if i + j < data.len() {
                    min[j] = min[j].min(data[i + j]);
                    max[j] = max[j].max(data[i + j]);
                }
            }
        }

        // Create buffer view
        self.buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": byte_offset,
            "byteLength": byte_length,
            "target": 34962 // ARRAY_BUFFER
        }));

        // Create accessor
        self.accessors.push(json!({
            "bufferView": buffer_view_idx,
            "componentType": 5126, // FLOAT
            "count": data.len() / 3,
            "type": "VEC3",
            "min": min,
            "max": max
        }));

        if is_position {
            self.position_slices.push((byte_offset, data.len()));
            self.position_accessors.push(accessor_idx);
            self.update_scene_bounds(&min, &max);
        }

        accessor_idx
    }

    fn create_accessor_indices(&mut self, data: &[u32]) -> u32 {
        let accessor_idx = self.accessors.len() as u32;
        let buffer_view_idx = self.buffer_views.len() as u32;

        // Align to 4 bytes
        Self::align_buffer(&mut self.buffer_data, 4);

        let byte_offset = self.buffer_data.len();

        // Write data
        for &value in data {
            self.buffer_data.extend_from_slice(&value.to_le_bytes());
        }

        let byte_length = self.buffer_data.len() - byte_offset;

        // Calculate min/max
        let min = *data.iter().min().unwrap_or(&0);
        let max = *data.iter().max().unwrap_or(&0);

        // Create buffer view
        self.buffer_views.push(json!({
            "buffer": 0,
            "byteOffset": byte_offset,
            "byteLength": byte_length,
            "target": 34963 // ELEMENT_ARRAY_BUFFER
        }));

        // Create accessor
        self.accessors.push(json!({
            "bufferView": buffer_view_idx,
            "componentType": 5125, // UNSIGNED_INT
            "count": data.len(),
            "type": "SCALAR",
            "min": [min],
            "max": [max]
        }));

        accessor_idx
    }

    fn update_scene_bounds(&mut self, min: &[f32; 3], max: &[f32; 3]) {
        let min_v = glam::Vec3::new(min[0], min[1], min[2]);
        let max_v = glam::Vec3::new(max[0], max[1], max[2]);
        match (self.scene_min, self.scene_max) {
            (Some(cur_min), Some(cur_max)) => {
                self.scene_min = Some(cur_min.min(min_v));
                self.scene_max = Some(cur_max.max(max_v));
            }
            _ => {
                self.scene_min = Some(min_v);
                self.scene_max = Some(max_v);
            }
        }
    }

    fn center_model(&mut self) {
        if !self.options.center_model {
            return;
        }
        if let (Some(min), Some(max)) = (self.scene_min, self.scene_max) {
            let center = (min + max) * 0.5;
            self.apply_center_offset(center);
            self.scene_min = Some(min - center);
            self.scene_max = Some(max - center);
        }
    }

    fn apply_center_offset(&mut self, center: glam::Vec3) {
        for &(byte_offset, len) in &self.position_slices {
            for i in 0..len {
                let idx = byte_offset + i * 4;
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&self.buffer_data[idx..idx + 4]);
                let mut value = f32::from_le_bytes(buf);
                match i % 3 {
                    0 => value -= center.x,
                    1 => value -= center.y,
                    2 => value -= center.z,
                    _ => {}
                }
                self.buffer_data[idx..idx + 4].copy_from_slice(&value.to_le_bytes());
            }
        }

        // Update accessor min/max for positions
        for &accessor_idx in &self.position_accessors {
            if let Some(accessor) = self.accessors.get_mut(accessor_idx as usize) {
                if let Some(obj) = accessor.as_object_mut() {
                    if let Some(min_arr) = obj.get_mut("min").and_then(|v| v.as_array_mut()) {
                        for (i, val) in min_arr.iter_mut().enumerate().take(3) {
                            if let Some(num) = val.as_f64() {
                                let shifted = num as f32
                                    - match i {
                                        0 => center.x,
                                        1 => center.y,
                                        _ => center.z,
                                    };
                                *val = json!(shifted);
                            }
                        }
                    }
                    if let Some(max_arr) = obj.get_mut("max").and_then(|v| v.as_array_mut()) {
                        for (i, val) in max_arr.iter_mut().enumerate().take(3) {
                            if let Some(num) = val.as_f64() {
                                let shifted = num as f32
                                    - match i {
                                        0 => center.x,
                                        1 => center.y,
                                        _ => center.z,
                                    };
                                *val = json!(shifted);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn write_to_file(mut self, path: &str) -> Result<(), ExportError> {
        if self.options.binary_format || path.ends_with(".glb") {
            self.write_glb(path)
        } else {
            self.write_gltf(path)
        }
    }

    fn write_gltf(&mut self, path: &str) -> Result<(), ExportError> {
        // Finalize buffer
        self.center_model();
        let buffer_length = self.buffer_data.len();

        let gltf = json!({
            "asset": {
                "version": "2.0",
                "generator": "rvm-rs exporter"
            },
            "scene": 0,
            "scenes": [{
                "nodes": (0..self.nodes.len()).collect::<Vec<_>>()
            }],
            "nodes": self.nodes,
            "meshes": self.meshes,
            "materials": self.materials,
            "accessors": self.accessors,
            "bufferViews": self.buffer_views,
            "buffers": [{
                "byteLength": buffer_length,
                "uri": format!("data:application/octet-stream;base64,{}",
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &self.buffer_data))
            }]
        });

        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &gltf)?;

        Ok(())
    }

    fn write_glb(&mut self, path: &str) -> Result<(), ExportError> {
        // Align buffer data to 4 bytes
        Self::align_buffer(&mut self.buffer_data, 4);
        self.center_model();

        let buffer_length = self.buffer_data.len();

        let gltf = json!({
            "asset": {
                "version": "2.0",
                "generator": "rvm-rs exporter"
            },
            "scene": 0,
            "scenes": [{
                "nodes": (0..self.nodes.len()).collect::<Vec<_>>()
            }],
            "nodes": self.nodes,
            "meshes": self.meshes,
            "materials": self.materials,
            "accessors": self.accessors,
            "bufferViews": self.buffer_views,
            "buffers": [{
                "byteLength": buffer_length
            }]
        });

        let json_string = serde_json::to_string(&gltf)?;
        let mut json_bytes = json_string.into_bytes();

        // Align JSON to 4 bytes with spaces
        while json_bytes.len() % 4 != 0 {
            json_bytes.push(b' ');
        }

        let json_length = json_bytes.len() as u32;
        let bin_length = self.buffer_data.len() as u32;
        let total_length = 12 + 8 + json_length + 8 + bin_length;

        let mut file = File::create(path)?;

        // GLB header
        file.write_all(b"glTF")?; // magic
        file.write_all(&2u32.to_le_bytes())?; // version
        file.write_all(&total_length.to_le_bytes())?; // length

        // JSON chunk
        file.write_all(&json_length.to_le_bytes())?;
        file.write_all(b"JSON")?;
        file.write_all(&json_bytes)?;

        // BIN chunk
        file.write_all(&bin_length.to_le_bytes())?;
        file.write_all(b"BIN\0")?;
        file.write_all(&self.buffer_data)?;

        Ok(())
    }
}

impl Visitor for GltfExporter {
    fn visit_node(&mut self, _node_id: NodeId, node: &Node, store: &mut Store) {
        let node_idx = self.nodes.len();

        let mut gltf_node = match &node.kind {
            NodeKind::File(_) => {
                json!({
                    "name": "File"
                })
            }
            NodeKind::Model(model) => {
                json!({
                    "name": store.get_string(model.name)
                })
            }
            NodeKind::Group(group) => {
                let translation = group.translation;
                json!({
                    "name": store.get_string(group.name),
                    "translation": [translation.x, translation.y, translation.z]
                })
            }
        };

        if self.options.include_attributes {
            if let Some(obj) = gltf_node.as_object_mut() {
                let mut extras = Map::new();
                if let NodeKind::Group(group) = &node.kind {
                    for attr in &group.attributes {
                        let key = store.get_string(attr.key).to_string();
                        let value = store.get_string(attr.value).to_string();
                        extras.insert(key, json!(value));
                    }
                }
                if !extras.is_empty() {
                    obj.insert("extras".to_string(), JsonValue::Object(extras));
                }
            }
        }

        if let Some(&parent_idx) = self.node_stack.last() {
            if let Some(parent) = self.nodes.get_mut(parent_idx) {
                if let Some(obj) = parent.as_object_mut() {
                    let entry = obj
                        .entry("children".to_string())
                        .or_insert_with(|| json!([]));
                    if let Some(arr) = entry.as_array_mut() {
                        arr.push(json!(node_idx));
                    }
                }
            }
        }

        self.nodes.push(gltf_node);
        self.node_stack.push(node_idx);
    }

    fn visit_geometry(
        &mut self,
        _geometry_id: GeometryId,
        geometry: &Geometry,
        _store: &mut Store,
    ) {
        // Extract scale from transform matrix
        let scale = crate::export::tessellator::get_scale(&geometry.transform.matrix3.into());

        let tri = match &geometry.kind {
            GeometryKind::Cylinder(cyl) => cyl.tessellate(self.options.tolerance, scale),
            GeometryKind::Sphere(sphere) => sphere.tessellate(self.options.tolerance, scale),
            GeometryKind::Box(b) => b.tessellate(self.options.tolerance, scale),
            GeometryKind::Pyramid(pyr) => pyr.tessellate(self.options.tolerance, scale),
            GeometryKind::CircularTorus(torus) => torus.tessellate(self.options.tolerance, scale),
            GeometryKind::RectangularTorus(torus) => {
                torus.tessellate(self.options.tolerance, scale)
            }
            GeometryKind::EllipticalDish(dish) => dish.tessellate(self.options.tolerance, scale),
            GeometryKind::SphericalDish(dish) => dish.tessellate(self.options.tolerance, scale),
            GeometryKind::Snout(snout) => snout.tessellate(self.options.tolerance, scale),
            GeometryKind::Line(line) => line.tessellate(self.options.tolerance, scale),
            GeometryKind::FacetGroup(fg) => fg.tessellate(self.options.tolerance, scale),
        };

        let mut transform = geometry.transform;
        if self.options.rotate_z_to_y {
            let rotation = glam::Affine3A::from_mat3(glam::Mat3::from_rotation_x(
                -std::f32::consts::FRAC_PI_2,
            ));
            transform = rotation * transform;
        }

        // Transform vertices
        let mut transformed_vertices = Vec::new();
        for i in (0..tri.vertices.len()).step_by(3) {
            let v = glam::Vec3::new(tri.vertices[i], tri.vertices[i + 1], tri.vertices[i + 2]);
            let transformed = transform.transform_point3(v);
            transformed_vertices.push(transformed.x);
            transformed_vertices.push(transformed.y);
            transformed_vertices.push(transformed.z);
        }

        // Transform normals
        let mut transformed_normals = Vec::new();
        for i in (0..tri.normals.len()).step_by(3) {
            let n = glam::Vec3::new(tri.normals[i], tri.normals[i + 1], tri.normals[i + 2]);
            let transformed = transform.transform_vector3(n).normalize_or_zero();
            transformed_normals.push(transformed.x);
            transformed_normals.push(transformed.y);
            transformed_normals.push(transformed.z);
        }

        // Create accessors
        let position_accessor = self.create_accessor_vec3_with_options(&transformed_vertices, true);
        let normal_accessor = self.create_accessor_vec3(&transformed_normals);
        let indices_accessor = self.create_accessor_indices(&tri.indices);

        // Create material
        let material_idx = self.create_material(geometry.color, geometry.transparency);

        // Create mesh
        let mesh_idx = self.meshes.len();
        self.meshes.push(json!({
            "primitives": [{
                "attributes": {
                    "POSITION": position_accessor,
                    "NORMAL": normal_accessor
                },
                "indices": indices_accessor,
                "material": material_idx
            }]
        }));

        // Add mesh to current node
        if let Some(&node_idx) = self.node_stack.last() {
            if let Some(node) = self.nodes.get_mut(node_idx) {
                node["mesh"] = json!(mesh_idx);
            }
        }
    }

    fn leave_node(&mut self, _node_id: NodeId, _node: &Node, _store: &mut Store) {
        self.node_stack.pop();
    }
}

// Note: base64 encoding for embedded buffers
mod base64 {
    pub mod engine {
        pub mod general_purpose {
            pub struct StandardEngine;
            pub const STANDARD: StandardEngine = StandardEngine;
        }
    }

    pub trait Engine {
        fn encode(&self, data: &[u8]) -> String;
    }

    impl Engine for engine::general_purpose::StandardEngine {
        fn encode(&self, data: &[u8]) -> String {
            const CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut result = String::new();

            for chunk in data.chunks(3) {
                let mut buf = [0u8; 3];
                for (i, &byte) in chunk.iter().enumerate() {
                    buf[i] = byte;
                }

                let b1 = (buf[0] >> 2) as usize;
                let b2 = (((buf[0] & 0x03) << 4) | (buf[1] >> 4)) as usize;
                let b3 = (((buf[1] & 0x0F) << 2) | (buf[2] >> 6)) as usize;
                let b4 = (buf[2] & 0x3F) as usize;

                result.push(CHARS[b1] as char);
                result.push(CHARS[b2] as char);
                result.push(if chunk.len() > 1 {
                    CHARS[b3] as char
                } else {
                    '='
                });
                result.push(if chunk.len() > 2 {
                    CHARS[b4] as char
                } else {
                    '='
                });
            }

            result
        }
    }
}
