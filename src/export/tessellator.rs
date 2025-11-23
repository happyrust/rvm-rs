use crate::store::geometry::*;
use glam::Vec3;
use std::f32::consts::PI;

#[derive(Debug, Clone)]
pub struct Triangulation {
    pub vertices: Vec<f32>,
    pub normals: Vec<f32>,
    pub indices: Vec<u32>,
    pub error: f32,
}

impl Triangulation {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            error: 0.0,
        }
    }

    pub fn add_vertex(&mut self, pos: Vec3, normal: Vec3) -> u32 {
        let index = (self.vertices.len() / 3) as u32;
        self.vertices.extend_from_slice(&[pos.x, pos.y, pos.z]);
        let n = normal.normalize();
        self.normals.extend_from_slice(&[n.x, n.y, n.z]);
        index
    }

    pub fn add_triangle(&mut self, i0: u32, i1: u32, i2: u32) {
        self.indices.extend_from_slice(&[i0, i1, i2]);
    }
}

impl Default for Triangulation {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Tessellate {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation;
}

/// Extract the maximum scale factor from a 3x3 matrix
pub fn get_scale(mat: &glam::Mat3A) -> f32 {
    let sx = mat.x_axis.length();
    let sy = mat.y_axis.length();
    let sz = mat.z_axis.length();
    sx.max(sy).max(sz)
}

/// Calculate segment count based on sagitta (弦高) error
/// This matches the C++ implementation
fn sagitta_based_segment_count(
    arc: f32,
    radius: f32,
    tolerance: f32,
    min_samples: usize,
    max_samples: usize,
) -> usize {
    if radius <= 0.0 {
        return min_samples;
    }
    let ratio = tolerance / radius;
    let ratio_clamped = ratio.clamp(-1.0, 1.0);
    let samples = arc / (1.0 - ratio_clamped).acos();
    samples.ceil().max(min_samples as f32).min(max_samples as f32) as usize
}

impl Tessellate for Cylinder {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        // Calculate number of segments based on tolerance and scale
        let scaled_radius = self.radius * scale;
        let segments = sagitta_based_segment_count(2.0 * PI, scaled_radius, tolerance, 8, 64);

        // Generate vertices for top and bottom circles
        for i in 0..=segments {
            let angle = 2.0 * PI * (i as f32) / (segments as f32);
            let x = self.radius * angle.cos();
            let z = self.radius * angle.sin();

            // Bottom vertex
            let bottom_pos = Vec3::new(x, 0.0, z);
            let normal = Vec3::new(x, 0.0, z).normalize();
            tri.add_vertex(bottom_pos, normal);

            // Top vertex
            let top_pos = Vec3::new(x, self.height, z);
            tri.add_vertex(top_pos, normal);
        }

        // Generate side triangles
        for i in 0..segments {
            let base = (i * 2) as u32;
            tri.add_triangle(base, base + 2, base + 1);
            tri.add_triangle(base + 1, base + 2, base + 3);
        }

        // Add caps
        let center_bottom = tri.add_vertex(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let center_top = tri.add_vertex(Vec3::new(0.0, self.height, 0.0), Vec3::new(0.0, 1.0, 0.0));

        for i in 0..segments {
            let base = (i * 2) as u32;
            // Bottom cap
            tri.add_triangle(center_bottom, base + 2, base);
            // Top cap
            tri.add_triangle(center_top, base + 1, base + 3);
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for Sphere {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let scaled_radius = self.radius * scale;
        let segments = sagitta_based_segment_count(2.0 * PI, scaled_radius, tolerance, 8, 32);
        let rings = segments / 2;

        // Generate vertices
        for ring in 0..=rings {
            let phi = PI * (ring as f32) / (rings as f32);
            let y = self.radius * phi.cos();
            let ring_radius = self.radius * phi.sin();

            for seg in 0..=segments {
                let theta = 2.0 * PI * (seg as f32) / (segments as f32);
                let x = ring_radius * theta.cos();
                let z = ring_radius * theta.sin();

                let pos = Vec3::new(x, y, z);
                let normal = pos.normalize();
                tri.add_vertex(pos, normal);
            }
        }

        // Generate triangles
        for ring in 0..rings {
            for seg in 0..segments {
                let current = (ring * (segments + 1) + seg) as u32;
                let next = current + (segments + 1) as u32;

                tri.add_triangle(current, next, current + 1);
                tri.add_triangle(current + 1, next, next + 1);
            }
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for Box {
    fn tessellate(&self, _tolerance: f32, _scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let hx = self.lengths[0] / 2.0;
        let hy = self.lengths[1] / 2.0;
        let hz = self.lengths[2] / 2.0;

        // Front face (+Z)
        let v0 = tri.add_vertex(Vec3::new(-hx, -hy, hz), Vec3::new(0.0, 0.0, 1.0));
        let v1 = tri.add_vertex(Vec3::new(hx, -hy, hz), Vec3::new(0.0, 0.0, 1.0));
        let v2 = tri.add_vertex(Vec3::new(hx, hy, hz), Vec3::new(0.0, 0.0, 1.0));
        let v3 = tri.add_vertex(Vec3::new(-hx, hy, hz), Vec3::new(0.0, 0.0, 1.0));
        tri.add_triangle(v0, v1, v2);
        tri.add_triangle(v0, v2, v3);

        // Back face (-Z)
        let v4 = tri.add_vertex(Vec3::new(hx, -hy, -hz), Vec3::new(0.0, 0.0, -1.0));
        let v5 = tri.add_vertex(Vec3::new(-hx, -hy, -hz), Vec3::new(0.0, 0.0, -1.0));
        let v6 = tri.add_vertex(Vec3::new(-hx, hy, -hz), Vec3::new(0.0, 0.0, -1.0));
        let v7 = tri.add_vertex(Vec3::new(hx, hy, -hz), Vec3::new(0.0, 0.0, -1.0));
        tri.add_triangle(v4, v5, v6);
        tri.add_triangle(v4, v6, v7);

        // Right face (+X)
        let v8 = tri.add_vertex(Vec3::new(hx, -hy, hz), Vec3::new(1.0, 0.0, 0.0));
        let v9 = tri.add_vertex(Vec3::new(hx, -hy, -hz), Vec3::new(1.0, 0.0, 0.0));
        let v10 = tri.add_vertex(Vec3::new(hx, hy, -hz), Vec3::new(1.0, 0.0, 0.0));
        let v11 = tri.add_vertex(Vec3::new(hx, hy, hz), Vec3::new(1.0, 0.0, 0.0));
        tri.add_triangle(v8, v9, v10);
        tri.add_triangle(v8, v10, v11);

        // Left face (-X)
        let v12 = tri.add_vertex(Vec3::new(-hx, -hy, -hz), Vec3::new(-1.0, 0.0, 0.0));
        let v13 = tri.add_vertex(Vec3::new(-hx, -hy, hz), Vec3::new(-1.0, 0.0, 0.0));
        let v14 = tri.add_vertex(Vec3::new(-hx, hy, hz), Vec3::new(-1.0, 0.0, 0.0));
        let v15 = tri.add_vertex(Vec3::new(-hx, hy, -hz), Vec3::new(-1.0, 0.0, 0.0));
        tri.add_triangle(v12, v13, v14);
        tri.add_triangle(v12, v14, v15);

        // Top face (+Y)
        let v16 = tri.add_vertex(Vec3::new(-hx, hy, hz), Vec3::new(0.0, 1.0, 0.0));
        let v17 = tri.add_vertex(Vec3::new(hx, hy, hz), Vec3::new(0.0, 1.0, 0.0));
        let v18 = tri.add_vertex(Vec3::new(hx, hy, -hz), Vec3::new(0.0, 1.0, 0.0));
        let v19 = tri.add_vertex(Vec3::new(-hx, hy, -hz), Vec3::new(0.0, 1.0, 0.0));
        tri.add_triangle(v16, v17, v18);
        tri.add_triangle(v16, v18, v19);

        // Bottom face (-Y)
        let v20 = tri.add_vertex(Vec3::new(-hx, -hy, -hz), Vec3::new(0.0, -1.0, 0.0));
        let v21 = tri.add_vertex(Vec3::new(hx, -hy, -hz), Vec3::new(0.0, -1.0, 0.0));
        let v22 = tri.add_vertex(Vec3::new(hx, -hy, hz), Vec3::new(0.0, -1.0, 0.0));
        let v23 = tri.add_vertex(Vec3::new(-hx, -hy, hz), Vec3::new(0.0, -1.0, 0.0));
        tri.add_triangle(v20, v21, v22);
        tri.add_triangle(v20, v22, v23);

        tri.error = 0.0;
        tri
    }
}

impl Tessellate for Pyramid {
    fn tessellate(&self, _tolerance: f32, _scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let bx = self.bottom[0] / 2.0;
        let bz = self.bottom[1] / 2.0;
        let tx = self.top[0] / 2.0;
        let tz = self.top[1] / 2.0;
        let ox = self.offset[0];
        let oz = self.offset[1];

        // Bottom face
        let v0 = tri.add_vertex(Vec3::new(-bx, 0.0, -bz), Vec3::new(0.0, -1.0, 0.0));
        let v1 = tri.add_vertex(Vec3::new(bx, 0.0, -bz), Vec3::new(0.0, -1.0, 0.0));
        let v2 = tri.add_vertex(Vec3::new(bx, 0.0, bz), Vec3::new(0.0, -1.0, 0.0));
        let v3 = tri.add_vertex(Vec3::new(-bx, 0.0, bz), Vec3::new(0.0, -1.0, 0.0));
        tri.add_triangle(v0, v1, v2);
        tri.add_triangle(v0, v2, v3);

        // Top face
        let v4 = tri.add_vertex(
            Vec3::new(ox - tx, self.height, oz - tz),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let v5 = tri.add_vertex(
            Vec3::new(ox + tx, self.height, oz - tz),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let v6 = tri.add_vertex(
            Vec3::new(ox + tx, self.height, oz + tz),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let v7 = tri.add_vertex(
            Vec3::new(ox - tx, self.height, oz + tz),
            Vec3::new(0.0, 1.0, 0.0),
        );
        tri.add_triangle(v4, v6, v5);
        tri.add_triangle(v4, v7, v6);

        // Side faces (approximate normals)
        // Front
        let n_front = Vec3::new(0.0, 0.0, 1.0);
        let v8 = tri.add_vertex(Vec3::new(-bx, 0.0, bz), n_front);
        let v9 = tri.add_vertex(Vec3::new(bx, 0.0, bz), n_front);
        let v10 = tri.add_vertex(Vec3::new(ox + tx, self.height, oz + tz), n_front);
        let v11 = tri.add_vertex(Vec3::new(ox - tx, self.height, oz + tz), n_front);
        tri.add_triangle(v8, v9, v10);
        tri.add_triangle(v8, v10, v11);

        // Back
        let n_back = Vec3::new(0.0, 0.0, -1.0);
        let v12 = tri.add_vertex(Vec3::new(bx, 0.0, -bz), n_back);
        let v13 = tri.add_vertex(Vec3::new(-bx, 0.0, -bz), n_back);
        let v14 = tri.add_vertex(Vec3::new(ox - tx, self.height, oz - tz), n_back);
        let v15 = tri.add_vertex(Vec3::new(ox + tx, self.height, oz - tz), n_back);
        tri.add_triangle(v12, v13, v14);
        tri.add_triangle(v12, v14, v15);

        // Right
        let n_right = Vec3::new(1.0, 0.0, 0.0);
        let v16 = tri.add_vertex(Vec3::new(bx, 0.0, -bz), n_right);
        let v17 = tri.add_vertex(Vec3::new(bx, 0.0, bz), n_right);
        let v18 = tri.add_vertex(Vec3::new(ox + tx, self.height, oz + tz), n_right);
        let v19 = tri.add_vertex(Vec3::new(ox + tx, self.height, oz - tz), n_right);
        tri.add_triangle(v16, v17, v18);
        tri.add_triangle(v16, v18, v19);

        // Left
        let n_left = Vec3::new(-1.0, 0.0, 0.0);
        let v20 = tri.add_vertex(Vec3::new(-bx, 0.0, bz), n_left);
        let v21 = tri.add_vertex(Vec3::new(-bx, 0.0, -bz), n_left);
        let v22 = tri.add_vertex(Vec3::new(ox - tx, self.height, oz - tz), n_left);
        let v23 = tri.add_vertex(Vec3::new(ox - tx, self.height, oz + tz), n_left);
        tri.add_triangle(v20, v21, v22);
        tri.add_triangle(v20, v22, v23);

        tri.error = 0.0;
        tri
    }
}

impl Tessellate for CircularTorus {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let major_radius = self.offset;
        let minor_radius = self.radius;
        let angle_range = self.angle;

        let major_segments = sagitta_based_segment_count(
            angle_range,
            (major_radius + minor_radius) * scale,
            tolerance,
            8,
            64,
        );
        let minor_segments =
            sagitta_based_segment_count(2.0 * PI, minor_radius * scale, tolerance, 8, 32);

        for i in 0..=major_segments {
            let theta = angle_range * (i as f32) / (major_segments as f32);
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            for j in 0..=minor_segments {
                let phi = 2.0 * PI * (j as f32) / (minor_segments as f32);
                let cos_phi = phi.cos();
                let sin_phi = phi.sin();

                let x = (major_radius + minor_radius * cos_phi) * cos_theta;
                let y = minor_radius * sin_phi;
                let z = (major_radius + minor_radius * cos_phi) * sin_theta;

                let nx = cos_phi * cos_theta;
                let ny = sin_phi;
                let nz = cos_phi * sin_theta;

                tri.add_vertex(Vec3::new(x, y, z), Vec3::new(nx, ny, nz));
            }
        }

        for i in 0..major_segments {
            for j in 0..minor_segments {
                let current = (i * (minor_segments + 1) + j) as u32;
                let next = current + (minor_segments + 1) as u32;

                tri.add_triangle(current, next, current + 1);
                tri.add_triangle(current + 1, next, next + 1);
            }
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for RectangularTorus {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let inner_r = self.inner_radius;
        let outer_r = self.outer_radius;
        let height = self.height;
        let angle_range = self.angle;

        let segments =
            sagitta_based_segment_count(angle_range, outer_r * scale, tolerance, 8, 64);

        // Generate vertices for inner and outer arcs at bottom and top
        for i in 0..=segments {
            let theta = angle_range * (i as f32) / (segments as f32);
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            // Inner arc
            let inner_x = inner_r * cos_theta;
            let inner_z = inner_r * sin_theta;

            // Outer arc
            let outer_x = outer_r * cos_theta;
            let outer_z = outer_r * sin_theta;

            // Bottom vertices
            tri.add_vertex(
                Vec3::new(inner_x, 0.0, inner_z),
                Vec3::new(-cos_theta, 0.0, -sin_theta),
            );
            tri.add_vertex(
                Vec3::new(outer_x, 0.0, outer_z),
                Vec3::new(cos_theta, 0.0, sin_theta),
            );

            // Top vertices
            tri.add_vertex(
                Vec3::new(inner_x, height, inner_z),
                Vec3::new(-cos_theta, 0.0, -sin_theta),
            );
            tri.add_vertex(
                Vec3::new(outer_x, height, outer_z),
                Vec3::new(cos_theta, 0.0, sin_theta),
            );
        }

        // Generate triangles
        for i in 0..segments {
            let base = (i * 4) as u32;

            // Inner wall
            tri.add_triangle(base, base + 2, base + 4);
            tri.add_triangle(base + 4, base + 2, base + 6);

            // Outer wall
            tri.add_triangle(base + 1, base + 5, base + 3);
            tri.add_triangle(base + 5, base + 7, base + 3);

            // Bottom face
            tri.add_triangle(base, base + 1, base + 4);
            tri.add_triangle(base + 4, base + 1, base + 5);

            // Top face
            tri.add_triangle(base + 2, base + 6, base + 3);
            tri.add_triangle(base + 6, base + 7, base + 3);
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for EllipticalDish {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let radius = self.base_radius;
        let height = self.height;

        let radial_segments =
            sagitta_based_segment_count(2.0 * PI, radius * scale, tolerance, 8, 64);
        let height_segments = ((height * scale / tolerance).ceil() as usize).clamp(4, 32);

        // Center point at top (unused for now)
        let _center = tri.add_vertex(Vec3::new(0.0, height, 0.0), Vec3::new(0.0, 1.0, 0.0));

        for h in 0..=height_segments {
            let t = (h as f32) / (height_segments as f32);
            let y = height * (1.0 - t);
            let r = radius * t;

            for seg in 0..=radial_segments {
                let theta = 2.0 * PI * (seg as f32) / (radial_segments as f32);
                let x = r * theta.cos();
                let z = r * theta.sin();

                let normal = Vec3::new(x, height - y, z).normalize();
                tri.add_vertex(Vec3::new(x, y, z), normal);
            }
        }

        // Generate triangles
        for h in 0..height_segments {
            for seg in 0..radial_segments {
                let current = 1 + h * (radial_segments + 1) + seg;
                let next = current + (radial_segments + 1);

                tri.add_triangle(current as u32, (current + 1) as u32, next as u32);
                tri.add_triangle((current + 1) as u32, (next + 1) as u32, next as u32);
            }
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for SphericalDish {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let radius = self.base_radius;
        let height = self.height;

        // Calculate sphere radius from base radius and height
        let sphere_radius = (radius * radius + height * height) / (2.0 * height);

        let radial_segments =
            sagitta_based_segment_count(2.0 * PI, radius * scale, tolerance, 8, 64);
        let height_segments = ((height * scale / tolerance).ceil() as usize).clamp(4, 32);

        for h in 0..=height_segments {
            let t = (h as f32) / (height_segments as f32);
            let phi = (PI / 2.0) * t;
            let y = sphere_radius * (1.0 - phi.cos());
            let r = sphere_radius * phi.sin();

            for seg in 0..=radial_segments {
                let theta = 2.0 * PI * (seg as f32) / (radial_segments as f32);
                let x = r * theta.cos();
                let z = r * theta.sin();

                let pos = Vec3::new(x, y, z);
                let center = Vec3::new(0.0, sphere_radius, 0.0);
                let normal = (pos - center).normalize();
                tri.add_vertex(pos, normal);
            }
        }

        for h in 0..height_segments {
            for seg in 0..radial_segments {
                let current = (h * (radial_segments + 1) + seg) as u32;
                let next = current + (radial_segments + 1) as u32;

                tri.add_triangle(current, current + 1, next);
                tri.add_triangle(current + 1, next + 1, next);
            }
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for Snout {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let r_bottom = self.radius_bottom;
        let r_top = self.radius_top;
        let height = self.height;
        let ox = self.offset_x;
        let oy = self.offset_y;

        let radius_max = r_bottom.max(r_top);
        let segments = sagitta_based_segment_count(2.0 * PI, radius_max * scale, tolerance, 8, 64);

        // Generate vertices
        for i in 0..=segments {
            let angle = 2.0 * PI * (i as f32) / (segments as f32);
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Bottom circle
            let bx = r_bottom * cos_a;
            let bz = r_bottom * sin_a;
            let bottom_pos = Vec3::new(bx, 0.0, bz);
            let bottom_normal = Vec3::new(cos_a, 0.0, sin_a);
            tri.add_vertex(bottom_pos, bottom_normal);

            // Top circle (offset)
            let tx = ox + r_top * cos_a;
            let tz = oy + r_top * sin_a;
            let top_pos = Vec3::new(tx, height, tz);
            let top_normal = Vec3::new(cos_a, 0.0, sin_a);
            tri.add_vertex(top_pos, top_normal);
        }

        // Generate side triangles
        for i in 0..segments {
            let base = (i * 2) as u32;
            tri.add_triangle(base, base + 2, base + 1);
            tri.add_triangle(base + 1, base + 2, base + 3);
        }

        // Add caps
        let center_bottom = tri.add_vertex(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let center_top = tri.add_vertex(Vec3::new(ox, height, oy), Vec3::new(0.0, 1.0, 0.0));

        for i in 0..segments {
            let base = (i * 2) as u32;
            tri.add_triangle(center_bottom, base + 2, base);
            tri.add_triangle(center_top, base + 1, base + 3);
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for Line {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        let r_start = self.start_radius;
        let r_end = self.end_radius;
        let length = 1.0; // Line goes from (0,0,0) to (0,1,0)

        let radius_max = r_start.max(r_end);
        let segments = sagitta_based_segment_count(2.0 * PI, radius_max * scale, tolerance, 8, 64);

        for i in 0..=segments {
            let angle = 2.0 * PI * (i as f32) / (segments as f32);
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Start circle
            let sx = r_start * cos_a;
            let sz = r_start * sin_a;
            let start_pos = Vec3::new(sx, 0.0, sz);
            let start_normal = Vec3::new(cos_a, 0.0, sin_a);
            tri.add_vertex(start_pos, start_normal);

            // End circle
            let ex = r_end * cos_a;
            let ez = r_end * sin_a;
            let end_pos = Vec3::new(ex, length, ez);
            let end_normal = Vec3::new(cos_a, 0.0, sin_a);
            tri.add_vertex(end_pos, end_normal);
        }

        // Generate triangles
        for i in 0..segments {
            let base = (i * 2) as u32;
            tri.add_triangle(base, base + 2, base + 1);
            tri.add_triangle(base + 1, base + 2, base + 3);
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for FacetGroup {
    fn tessellate(&self, _tolerance: f32, _scale: f32) -> Triangulation {
        let mut tri = Triangulation::new();

        for polygon in &self.polygons {
            if polygon.vertices.len() < 3 {
                continue;
            }

            // 使用每个顶点携带的法线；若缺失，则使用平均法线
            let fallback_normal = if polygon.normals.len() >= 3 {
                let a = polygon.vertices[1] - polygon.vertices[0];
                let b = polygon.vertices[2] - polygon.vertices[0];
                a.cross(b).normalize_or_zero()
            } else {
                glam::Vec3::Z
            };

            let normal_for = |idx: usize, normals: &Vec<glam::Vec3>| -> glam::Vec3 {
                normals.get(idx).copied().unwrap_or(fallback_normal)
            };

            // 简单扇形三角化
            let v0 = tri.add_vertex(polygon.vertices[0], normal_for(0, &polygon.normals));
            for i in 1..polygon.vertices.len() - 1 {
                let v1 = tri.add_vertex(polygon.vertices[i], normal_for(i, &polygon.normals));
                let v2 = tri.add_vertex(
                    polygon.vertices[i + 1],
                    normal_for(i + 1, &polygon.normals),
                );
                tri.add_triangle(v0, v1, v2);
            }
        }

        tri.error = 0.0;
        tri
    }
}
