use crate::export::{ExportError, Tessellate};
use crate::store::{Geometry, GeometryId, GeometryKind, Node, NodeId, NodeKind, Store};
use crate::visitor::Visitor;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ObjExportOptions {
    pub include_normals: bool,
    pub group_bounding_boxes: bool,
    pub tolerance: f32,
}

impl Default for ObjExportOptions {
    fn default() -> Self {
        Self {
            include_normals: true,
            group_bounding_boxes: false,
            tolerance: 0.1,
        }
    }
}

pub struct ObjExporter {
    obj_file: BufWriter<File>,
    mtl_file: BufWriter<File>,
    defined_materials: HashSet<u64>,
    vertex_offset: u32,
    normal_offset: u32,
    /// Group names only (for "o" output, matching C++ behavior - no File/Model prefix)
    group_stack: Vec<String>,
    options: ObjExportOptions,
}

impl ObjExporter {
    pub fn new(obj_path: &str, options: ObjExportOptions) -> Result<Self, ExportError> {
        let obj_file = File::create(obj_path)?;

        // Create MTL file path
        let path = Path::new(obj_path);
        let mtl_path = path.with_extension("mtl");
        let mtl_name = mtl_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| ExportError::InvalidPath("Invalid MTL path".to_string()))?
            .to_string();

        let mtl_file = File::create(&mtl_path)?;

        let mut obj_writer = BufWriter::new(obj_file);
        let mtl_writer = BufWriter::new(mtl_file);

        // Write OBJ header
        writeln!(obj_writer, "# Exported from RVM")?;
        writeln!(obj_writer, "mtllib {}", mtl_name)?;
        writeln!(obj_writer)?;

        Ok(Self {
            obj_file: obj_writer,
            mtl_file: mtl_writer,
            defined_materials: HashSet::new(),
            vertex_offset: 1, // OBJ uses 1-based indexing
            normal_offset: 1,
            group_stack: Vec::new(),
            options,
        })
    }

    fn material_key(color: u32, transparency: u32) -> u64 {
        ((color as u64) << 32) | (transparency as u64)
    }

    fn write_material(&mut self, color: u32, transparency: u32) -> Result<(), ExportError> {
        let key = Self::material_key(color, transparency);

        if self.defined_materials.contains(&key) {
            return Ok(());
        }

        self.defined_materials.insert(key);

        // Extract RGB from color
        let r = ((color >> 16) & 0xFF) as f32 / 255.0;
        let g = ((color >> 8) & 0xFF) as f32 / 255.0;
        let b = (color & 0xFF) as f32 / 255.0;

        // Calculate alpha
        let alpha = 1.0 - (transparency as f32 / 100.0);

        writeln!(self.mtl_file, "newmtl mat_{}_{}", color, transparency)?;
        writeln!(self.mtl_file, "Ka {} {} {}", r, g, b)?;
        writeln!(self.mtl_file, "Kd {} {} {}", r, g, b)?;
        writeln!(self.mtl_file, "Ks 0.5 0.5 0.5")?;
        writeln!(self.mtl_file, "Ns 32.0")?;
        writeln!(self.mtl_file, "d {}", alpha)?;
        writeln!(self.mtl_file)?;

        Ok(())
    }

    fn write_triangulation(
        &mut self,
        tri: &crate::export::tessellator::Triangulation,
        transform: &glam::Affine3A,
        color: u32,
        transparency: u32,
    ) -> Result<(), ExportError> {
        // Ensure material is defined
        self.write_material(color, transparency)?;

        // Write vertices
        for i in (0..tri.vertices.len()).step_by(3) {
            let v = glam::Vec3::new(tri.vertices[i], tri.vertices[i + 1], tri.vertices[i + 2]);
            let transformed = transform.transform_point3(v);
            writeln!(
                self.obj_file,
                "v {} {} {}",
                transformed.x, transformed.y, transformed.z
            )?;
        }

        // Write normals
        if self.options.include_normals {
            for i in (0..tri.normals.len()).step_by(3) {
                let n = glam::Vec3::new(tri.normals[i], tri.normals[i + 1], tri.normals[i + 2]);
                let transformed = transform.transform_vector3(n).normalize_or_zero();
                writeln!(
                    self.obj_file,
                    "vn {} {} {}",
                    transformed.x, transformed.y, transformed.z
                )?;
            }
        }

        // Use material
        writeln!(self.obj_file, "usemtl mat_{}_{}", color, transparency)?;

        // Write faces
        for i in (0..tri.indices.len()).step_by(3) {
            let i0 = self.vertex_offset + tri.indices[i];
            let i1 = self.vertex_offset + tri.indices[i + 1];
            let i2 = self.vertex_offset + tri.indices[i + 2];

            if self.options.include_normals {
                let n0 = self.normal_offset + tri.indices[i];
                let n1 = self.normal_offset + tri.indices[i + 1];
                let n2 = self.normal_offset + tri.indices[i + 2];
                writeln!(
                    self.obj_file,
                    "f {}//{} {}//{} {}//{}",
                    i0, n0, i1, n1, i2, n2
                )?;
            } else {
                writeln!(self.obj_file, "f {} {} {}", i0, i1, i2)?;
            }
        }

        writeln!(self.obj_file)?;

        // Update offsets
        let vertex_count = (tri.vertices.len() / 3) as u32;
        self.vertex_offset += vertex_count;
        if self.options.include_normals {
            self.normal_offset += vertex_count;
        }

        Ok(())
    }

    /// Export Line as OBJ `l` (line) primitive - matches C++ behavior (centerline, not tessellated cylinder).
    fn write_line(
        &mut self,
        line: &crate::store::geometry::Line,
        transform: &glam::Affine3A,
        color: u32,
        transparency: u32,
    ) -> Result<(), ExportError> {
        self.write_material(color, transparency)?;

        // C++: canonical line from (a,0,0) to (b,0,0) in model space, then transformed
        let a = glam::Vec3::new(line.start_radius, 0.0, 0.0);
        let b = glam::Vec3::new(line.end_radius, 0.0, 0.0);
        let a_world = transform.transform_point3(a);
        let b_world = transform.transform_point3(b);

        writeln!(self.obj_file, "usemtl mat_{}_{}", color, transparency)?;
        writeln!(self.obj_file, "v {} {} {}", a_world.x, a_world.y, a_world.z)?;
        writeln!(self.obj_file, "v {} {} {}", b_world.x, b_world.y, b_world.z)?;
        writeln!(self.obj_file, "l -1 -2")?;
        writeln!(self.obj_file)?;

        self.vertex_offset += 2;
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), ExportError> {
        self.obj_file.flush()?;
        self.mtl_file.flush()?;
        Ok(())
    }
}

impl Visitor for ObjExporter {
    fn visit_node(&mut self, _node_id: NodeId, node: &Node, store: &mut Store) {
        match &node.kind {
            NodeKind::Group(group) => {
                let name = store.get_string(group.name);
                self.group_stack.push(name.to_string());

                // Write "o" with group path only (matches C++: stack[0]/stack[1]/...)
                let full_name = self.group_stack.join("/");
                let _ = writeln!(self.obj_file, "o {}", full_name);
            }
            NodeKind::Model(_) | NodeKind::File(_) => {}
        }
    }

    fn visit_geometry(
        &mut self,
        _geometry_id: GeometryId,
        geometry: &Geometry,
        _store: &mut Store,
    ) {
        // Line: export as OBJ `l` primitive (centerline only) - matches C++ behavior
        if let GeometryKind::Line(line) = &geometry.kind {
            let _ = self.write_line(line, &geometry.transform, geometry.color, geometry.transparency);
            return;
        }

        // Extract scale from transform matrix for tessellation
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
            GeometryKind::FacetGroup(fg) => fg.tessellate(self.options.tolerance, scale),
            GeometryKind::Line(_) => unreachable!(), // handled above
        };

        let _ = self.write_triangulation(
            &tri,
            &geometry.transform,
            geometry.color,
            geometry.transparency,
        );
    }

    fn leave_node(&mut self, _node_id: NodeId, node: &Node, _store: &mut Store) {
        if matches!(node.kind, NodeKind::Group(_)) && !self.group_stack.is_empty() {
            self.group_stack.pop();
        }
    }
}
