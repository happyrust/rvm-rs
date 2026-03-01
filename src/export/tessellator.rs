use crate::math::BBox3;
use crate::store::connection::{get_interface, Connection, Interface};
use crate::store::geometry::*;
use glam::Vec3;
use std::f32::consts::PI;

/// Options for geometry culling (skipping small geometries)
#[derive(Debug, Clone, Copy)]
pub struct CullingOptions {
    /// Cull geometries smaller than this threshold (in world units)
    /// Set to 0.0 to disable culling
    pub geometry_threshold: f32,
    /// Cull entire groups smaller than this threshold (in world units)
    /// Set to 0.0 to disable group culling
    pub group_threshold: f32,
}

impl CullingOptions {
    pub fn new(geometry_threshold: f32, group_threshold: f32) -> Self {
        Self {
            geometry_threshold,
            group_threshold,
        }
    }

    pub fn disabled() -> Self {
        Self {
            geometry_threshold: 0.0,
            group_threshold: 0.0,
        }
    }

    pub fn default_thresholds(tolerance: f32) -> Self {
        Self {
            geometry_threshold: tolerance * 10.0, // 10x tolerance
            group_threshold: tolerance * 5.0,     // 5x tolerance
        }
    }
}

impl Default for CullingOptions {
    fn default() -> Self {
        Self::disabled()
    }
}

/// Check if a geometry should be culled based on its bounding box
pub fn should_cull_geometry(bbox: &BBox3, options: &CullingOptions) -> bool {
    if options.geometry_threshold <= 0.0 {
        return false;
    }

    let diagonal = bbox.diagonal_length();
    diagonal < options.geometry_threshold
}

/// Get the error value for a culled geometry
pub fn culled_geometry_error(bbox: &BBox3) -> f32 {
    bbox.diagonal_length()
}

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
        let n = normal.normalize_or_zero();
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

/// Check which caps should be generated for a geometry based on connections
///
/// Returns a vector of booleans indicating whether each cap should be generated.
/// For geometries with 2 caps (Cylinder, Snout, etc.), returns [bottom, top].
pub fn check_caps_for_geometry(geometry: &Geometry, connections: &[Connection]) -> Vec<bool> {
    let cap_count = get_cap_count(&geometry.kind);
    let mut generate_caps = vec![true; cap_count];

    // Check each cap
    for cap_index in 0..cap_count {
        // Find connections involving this geometry and cap
        for conn in connections {
            // Check if this connection involves our geometry
            let (our_index, _their_index) = if conn.geometries[0] == geometry_id_placeholder() {
                if conn.offsets[0] == cap_index {
                    (0, 1)
                } else {
                    continue;
                }
            } else if conn.geometries[1] == geometry_id_placeholder() {
                if conn.offsets[1] == cap_index {
                    (1, 0)
                } else {
                    continue;
                }
            } else {
                continue;
            };

            // Get interfaces for both sides
            let our_interface = get_interface(geometry, conn.offsets[our_index]);
            // For the other interface, we would need the other geometry
            // For now, we'll use a simplified approach

            // Check if interfaces match
            // This is a simplified version - in practice we'd need access to both geometries
            // For now, we'll just check the connection flags
            if matches_connection_type(&our_interface, conn) {
                generate_caps[cap_index] = false;
                break;
            }
        }
    }

    generate_caps
}

/// Get the number of caps for a geometry type
fn get_cap_count(kind: &GeometryKind) -> usize {
    match kind {
        GeometryKind::Cylinder(_) => 2,
        GeometryKind::Snout(_) => 2,
        GeometryKind::CircularTorus(_) => 2,
        GeometryKind::RectangularTorus(_) => 2,
        GeometryKind::EllipticalDish(_) => 1,
        GeometryKind::SphericalDish(_) => 1,
        GeometryKind::Pyramid(_) => 6, // 4 sides + 2 ends
        GeometryKind::Box(_) => 6,
        _ => 0,
    }
}

/// Temporary placeholder for geometry ID
fn geometry_id_placeholder() -> crate::store::geometry::GeometryId {
    crate::store::geometry::GeometryId(0)
}

/// Check if an interface matches the connection type
fn matches_connection_type(interface: &Interface, conn: &Connection) -> bool {
    match interface {
        Interface::Circular { .. } => conn.flags.has_circular_side(),
        Interface::Square { .. } => conn.flags.has_rectangular_side(),
        Interface::Undefined => false,
    }
}

/// Tessellate a geometry with connection-aware cap generation
///
/// This is the high-level function that integrates connection detection with tessellation.
/// It checks which caps should be generated based on connections and calls the appropriate
/// tessellation method.
///
/// # Arguments
/// * `geometry` - The geometry to tessellate
/// * `tolerance` - Tessellation tolerance
/// * `scale` - Scale factor
/// * `connections` - List of connections involving this geometry
///
/// # Returns
/// Triangulation with optimized cap generation
pub fn tessellate_with_connections(
    geometry: &Geometry,
    tolerance: f32,
    scale: f32,
    connections: &[Connection],
) -> Triangulation {
    // Check which caps should be generated
    let generate_caps = check_caps_for_geometry(geometry, connections);

    // Call the appropriate tessellation method based on geometry type
    match &geometry.kind {
        GeometryKind::Cylinder(cyl) => cyl.tessellate_with_caps(tolerance, scale, &generate_caps),
        GeometryKind::Snout(snout) => snout.tessellate_with_caps(tolerance, scale, &generate_caps),
        GeometryKind::CircularTorus(torus) => {
            torus.tessellate_with_caps(tolerance, scale, &generate_caps)
        }
        GeometryKind::RectangularTorus(torus) => {
            torus.tessellate_with_caps(tolerance, scale, &generate_caps)
        }
        // For geometries without TessellateWithCaps implementation, use default
        GeometryKind::Pyramid(pyr) => pyr.tessellate(tolerance, scale),
        GeometryKind::Box(b) => b.tessellate(tolerance, scale),
        GeometryKind::EllipticalDish(dish) => dish.tessellate(tolerance, scale),
        GeometryKind::SphericalDish(dish) => dish.tessellate(tolerance, scale),
        GeometryKind::Sphere(sphere) => sphere.tessellate(tolerance, scale),
        GeometryKind::Line(line) => line.tessellate(tolerance, scale),
        GeometryKind::FacetGroup(group) => group.tessellate(tolerance, scale),
    }
}

pub trait Tessellate {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation;
}

/// Extended tessellation trait that supports conditional cap generation
pub trait TessellateWithCaps {
    /// Tessellate with control over which caps to generate
    ///
    /// # Arguments
    /// * `tolerance` - Tessellation tolerance
    /// * `scale` - Scale factor
    /// * `generate_caps` - Boolean flags for each cap (true = generate, false = skip)
    ///
    /// For geometries with 2 caps: [bottom/start, top/end]
    /// For geometries with no caps: empty slice
    fn tessellate_with_caps(
        &self,
        tolerance: f32,
        scale: f32,
        generate_caps: &[bool],
    ) -> Triangulation;
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
    samples
        .ceil()
        .max(min_samples as f32)
        .min(max_samples as f32) as usize
}

/// Unified sphere-based shape tessellation
/// This function generates sphere, elliptical dish, and spherical dish geometries
/// by parameterizing the sphere generation.
///
/// Parameters:
/// - radius: Base radius of the sphere
/// - arc: Arc angle in radians (π for full sphere, π/2 for hemisphere)
/// - shift_z: Z-axis shift for the sphere center
/// - scale_z: Z-axis scaling factor (for elliptical shapes)
/// - tolerance: Tessellation tolerance
/// - scale: Geometry scale factor
fn sphere_based_shape(
    radius: f32,
    arc: f32,
    shift_z: f32,
    scale_z: f32,
    tolerance: f32,
    scale: f32,
) -> Triangulation {
    let mut tri = Triangulation::new();

    // Check for valid scale_z
    let scale_z = if scale_z.is_finite() { scale_z } else { 0.0 };

    // Determine if this is a full sphere
    let is_sphere = arc >= (PI - 1e-3);
    let arc = if is_sphere { PI } else { arc };

    // Calculate segments for circumference
    let segments = sagitta_based_segment_count(2.0 * PI, radius * scale, tolerance, 8, 64);
    let samples = segments; // Closed loop

    // Calculate number of rings (latitude divisions)
    let min_rings = 3;
    let rings = (scale_z * samples as f32 * arc / (2.0 * PI))
        .max(min_rings as f32)
        .ceil() as usize;

    // Pre-compute ring parameters
    let mut ring_samples = Vec::with_capacity(rings);
    let theta_scale = arc / (rings - 1) as f32;

    for r in 0..rings {
        let theta = theta_scale * r as f32;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        // Adaptive sampling: fewer samples near poles, more near equator
        let samples_in_ring = if r == 0 {
            1 // Top pole
        } else if is_sphere && r == rings - 1 {
            1 // Bottom pole
        } else {
            (sin_theta * samples as f32).max(3.0).ceil() as usize
        };

        ring_samples.push((samples_in_ring, cos_theta, sin_theta));
    }

    // Generate vertices
    for (samples_in_ring, cos_theta, sin_theta) in &ring_samples {
        let nz = cos_theta;
        let z = radius * scale_z * nz + shift_z;
        let w = sin_theta;

        let phi_scale = 2.0 * PI / *samples_in_ring as f32;

        for i in 0..*samples_in_ring {
            let phi = phi_scale * i as f32;
            let nx = w * phi.cos();
            let ny = w * phi.sin();

            let pos = Vec3::new(radius * nx, z, radius * ny);
            let normal = Vec3::new(nx, nz / scale_z, ny).normalize_or_zero();

            tri.add_vertex(pos, normal);
        }
    }

    // Generate indices
    let mut vertex_offset = 0;
    for r in 0..rings - 1 {
        let n_current = ring_samples[r].0;
        let n_next = ring_samples[r + 1].0;
        let offset_next = vertex_offset + n_current;

        if n_current < n_next {
            // Current ring has fewer samples than next ring
            for i_next in 0..n_next {
                let ii_next = (i_next + 1) % n_next;
                let i_current = (n_current * (i_next + 1)) / n_next;
                let ii_current = (n_current * (ii_next + 1)) / n_next;

                let i_c = i_current % n_current;
                let ii_c = ii_current % n_current;

                if i_c != ii_c {
                    tri.add_triangle(
                        (vertex_offset + i_c) as u32,
                        (offset_next + ii_next) as u32,
                        (vertex_offset + ii_c) as u32,
                    );
                }

                tri.add_triangle(
                    (vertex_offset + i_c) as u32,
                    (offset_next + i_next) as u32,
                    (offset_next + ii_next) as u32,
                );
            }
        } else {
            // Current ring has more or equal samples than next ring
            for i_current in 0..n_current {
                let ii_current = (i_current + 1) % n_current;
                let i_next = (n_next * i_current) / n_current;
                let ii_next = (n_next * ii_current) / n_current;

                let i_n = i_next % n_next;
                let ii_n = ii_next % n_next;

                tri.add_triangle(
                    (vertex_offset + i_current) as u32,
                    (offset_next + ii_n) as u32,
                    (vertex_offset + ii_current) as u32,
                );

                if i_n != ii_n {
                    tri.add_triangle(
                        (vertex_offset + i_current) as u32,
                        (offset_next + i_n) as u32,
                        (offset_next + ii_n) as u32,
                    );
                }
            }
        }

        vertex_offset = offset_next;
    }

    tri.error = tolerance;
    tri
}

impl Tessellate for Cylinder {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        // Default: generate all caps
        self.tessellate_with_caps(tolerance, scale, &[true, true])
    }
}

impl TessellateWithCaps for Cylinder {
    fn tessellate_with_caps(
        &self,
        tolerance: f32,
        scale: f32,
        generate_caps: &[bool],
    ) -> Triangulation {
        let mut tri = Triangulation::new();

        // Determine which caps to generate
        let gen_bottom = generate_caps.get(0).copied().unwrap_or(true);
        let gen_top = generate_caps.get(1).copied().unwrap_or(true);

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

        // Conditionally add caps
        if gen_bottom {
            let center_bottom = tri.add_vertex(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
            for i in 0..segments {
                let base = (i * 2) as u32;
                tri.add_triangle(center_bottom, base + 2, base);
            }
        }

        if gen_top {
            let center_top =
                tri.add_vertex(Vec3::new(0.0, self.height, 0.0), Vec3::new(0.0, 1.0, 0.0));
            for i in 0..segments {
                let base = (i * 2) as u32;
                tri.add_triangle(center_top, base + 1, base + 3);
            }
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for Sphere {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        // Full sphere: arc = π, no shift, uniform scaling
        sphere_based_shape(
            self.radius, // radius
            PI,          // arc (full sphere)
            0.0,         // shift_z
            1.0,         // scale_z (uniform)
            tolerance,
            scale,
        )
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
        // Default: generate all caps
        self.tessellate_with_caps(tolerance, scale, &[true, true])
    }
}

impl TessellateWithCaps for CircularTorus {
    fn tessellate_with_caps(
        &self,
        tolerance: f32,
        scale: f32,
        generate_caps: &[bool],
    ) -> Triangulation {
        let mut tri = Triangulation::new();

        // Determine which caps to generate
        let gen_start = generate_caps.get(0).copied().unwrap_or(true);
        let gen_end = generate_caps.get(1).copied().unwrap_or(true);

        let major_radius = self.offset;
        let minor_radius = self.radius;
        let angle_range = self.angle;

        // Major direction (toroidal) - along the sweep
        let major_segments = sagitta_based_segment_count(
            angle_range,
            (major_radius + minor_radius) * scale,
            tolerance,
            8,
            64,
        );
        let major_samples = major_segments + 1; // Open sweep

        // Minor direction (poloidal) - circular cross-section
        let minor_segments =
            sagitta_based_segment_count(2.0 * PI, minor_radius * scale, tolerance, 8, 32);
        let minor_samples = minor_segments; // Closed loop

        // Generate shell vertices
        for i in 0..major_samples {
            let theta = angle_range * (i as f32) / major_segments as f32;
            let cos_theta = theta.cos();
            let sin_theta = theta.sin();

            for j in 0..minor_samples {
                let phi = 2.0 * PI * (j as f32) / minor_samples as f32;
                let cos_phi = phi.cos();
                let sin_phi = phi.sin();

                // Position: sweep a circle of radius 'minor_radius'
                // around a major circle of radius 'major_radius'
                let x = (major_radius + minor_radius * cos_phi) * cos_theta;
                let y = minor_radius * sin_phi;
                let z = (major_radius + minor_radius * cos_phi) * sin_theta;

                // Normal: points radially outward from the minor circle center
                let nx = cos_phi * cos_theta;
                let ny = sin_phi;
                let nz = cos_phi * sin_theta;

                tri.add_vertex(Vec3::new(x, y, z), Vec3::new(nx, ny, nz));
            }
        }

        // Generate shell triangles
        for i in 0..major_segments {
            for j in 0..minor_samples {
                let j_next = (j + 1) % minor_samples;

                let current = (i * minor_samples + j) as u32;
                let current_next = (i * minor_samples + j_next) as u32;
                let next = ((i + 1) * minor_samples + j) as u32;
                let next_next = ((i + 1) * minor_samples + j_next) as u32;

                tri.add_triangle(current, next, current_next);
                tri.add_triangle(current_next, next, next_next);
            }
        }

        let shell_verts = (major_samples * minor_samples) as u32;

        // Helper function to tessellate a circular cap
        fn tessellate_circle_cap(
            tri: &mut Triangulation,
            center_idx: u32,
            ring_start: u32,
            ring_count: usize,
            reverse: bool,
        ) {
            for i in 0..ring_count {
                let i_next = (i + 1) % ring_count;
                let v1 = ring_start + i as u32;
                let v2 = ring_start + i_next as u32;

                if reverse {
                    tri.add_triangle(center_idx, v2, v1);
                } else {
                    tri.add_triangle(center_idx, v1, v2);
                }
            }
        }

        // Conditionally generate start cap (at theta = 0)
        let mut start_cap_base = shell_verts;
        if gen_start {
            let theta_start: f32 = 0.0;
            let cos_start = theta_start.cos();
            let sin_start = theta_start.sin();

            // Add center vertex
            let start_center = tri.add_vertex(
                Vec3::new(major_radius * cos_start, 0.0, major_radius * sin_start),
                Vec3::new(0.0, -1.0, 0.0),
            );

            // Add ring vertices
            for j in 0..minor_samples {
                let phi = 2.0 * PI * (j as f32) / minor_samples as f32;
                let cos_phi = phi.cos();
                let sin_phi = phi.sin();

                let x = (major_radius + minor_radius * cos_phi) * cos_start;
                let y = minor_radius * sin_phi;
                let z = (major_radius + minor_radius * cos_phi) * sin_start;

                tri.add_vertex(Vec3::new(x, y, z), Vec3::new(0.0, -1.0, 0.0));
            }
            tessellate_circle_cap(
                &mut tri,
                start_center,
                start_cap_base + 1,
                minor_samples,
                true,
            );

            start_cap_base += 1 + minor_samples as u32;
        }

        // Conditionally generate end cap (at theta = angle_range)
        if gen_end {
            let end_cap_base = start_cap_base;
            let theta_end = angle_range;
            let cos_end = theta_end.cos();
            let sin_end = theta_end.sin();

            // Add center vertex
            let end_center = tri.add_vertex(
                Vec3::new(major_radius * cos_end, 0.0, major_radius * sin_end),
                Vec3::new(-sin_end, 0.0, cos_end),
            );

            // Add ring vertices
            for j in 0..minor_samples {
                let phi = 2.0 * PI * (j as f32) / minor_samples as f32;
                let cos_phi = phi.cos();
                let sin_phi = phi.sin();

                let x = (major_radius + minor_radius * cos_phi) * cos_end;
                let y = minor_radius * sin_phi;
                let z = (major_radius + minor_radius * cos_phi) * sin_end;

                tri.add_vertex(Vec3::new(x, y, z), Vec3::new(-sin_end, 0.0, cos_end));
            }
            tessellate_circle_cap(&mut tri, end_center, end_cap_base + 1, minor_samples, false);
        }

        tri.error = tolerance;
        tri
    }
}
impl Tessellate for RectangularTorus {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        // Default: generate all caps
        self.tessellate_with_caps(tolerance, scale, &[true, true])
    }
}

impl TessellateWithCaps for RectangularTorus {
    fn tessellate_with_caps(
        &self,
        tolerance: f32,
        scale: f32,
        generate_caps: &[bool],
    ) -> Triangulation {
        let mut tri = Triangulation::new();

        // Determine which caps to generate
        let gen_start = generate_caps.get(0).copied().unwrap_or(true);
        let gen_end = generate_caps.get(1).copied().unwrap_or(true);

        let inner_r = self.inner_radius;
        let outer_r = self.outer_radius;
        let height = self.height;
        let angle_range = self.angle;
        let h2 = height / 2.0;

        let segments = sagitta_based_segment_count(angle_range, outer_r * scale, tolerance, 8, 64);
        let samples = segments + 1; // Open sweep, need extra sample

        // Define rectangular cross-section corners (matching C++ implementation)
        // [radius, height_offset]
        let square = [
            [outer_r, -h2], // Outer, bottom
            [inner_r, -h2], // Inner, bottom
            [inner_r, h2],  // Inner, top
            [outer_r, h2],  // Outer, top
        ];

        // Pre-compute cos/sin for each sample along the sweep
        let mut angles = Vec::with_capacity(samples);
        for i in 0..samples {
            let theta = (angle_range / segments as f32) * i as f32;
            angles.push((theta.cos(), theta.sin()));
        }

        // Generate vertices for the shell (4 faces × 2 vertices per sample)
        for i in 0..samples {
            let (cos_t, sin_t) = angles[i];

            // Normal directions for each face
            let normals = [
                Vec3::new(0.0, 0.0, -1.0),      // Bottom face
                Vec3::new(-cos_t, -sin_t, 0.0), // Inner face
                Vec3::new(0.0, 0.0, 1.0),       // Top face
                Vec3::new(cos_t, sin_t, 0.0),   // Outer face
            ];

            // For each of the 4 faces, add 2 vertices (current corner and next corner)
            for k in 0..4 {
                let kk = (k + 1) % 4;

                // First vertex of the edge
                let pos1 = Vec3::new(square[k][0] * cos_t, square[k][1], square[k][0] * sin_t);
                tri.add_vertex(pos1, normals[k]);

                // Second vertex of the edge
                let pos2 = Vec3::new(square[kk][0] * cos_t, square[kk][1], square[kk][0] * sin_t);
                tri.add_vertex(pos2, normals[k]);
            }
        }

        // Generate indices for shell (4 faces, each swept along the arc)
        for i in 0..(samples - 1) {
            for k in 0..4 {
                let base = (i * 8 + k * 2) as u32;
                let next = ((i + 1) * 8 + k * 2) as u32;

                // Two triangles per quad
                tri.add_triangle(base, base + 1, next);
                tri.add_triangle(next, base + 1, next + 1);
            }
        }

        let shell_verts = (samples * 8) as u32;

        // Conditionally generate start cap (at angle = 0)
        if gen_start {
            let start_base = shell_verts;
            for k in 0..4 {
                let (cos_t, sin_t) = angles[0];
                let pos = Vec3::new(square[k][0] * cos_t, square[k][1], square[k][0] * sin_t);
                tri.add_vertex(pos, Vec3::new(0.0, -1.0, 0.0));
            }
            tri.add_triangle(start_base, start_base + 2, start_base + 1);
            tri.add_triangle(start_base + 2, start_base, start_base + 3);
        }

        // Conditionally generate end cap (at angle = angle_range)
        if gen_end {
            let end_base = if gen_start {
                shell_verts + 4
            } else {
                shell_verts
            };

            for k in 0..4 {
                let (cos_t, sin_t) = angles[samples - 1];
                let pos = Vec3::new(square[k][0] * cos_t, square[k][1], square[k][0] * sin_t);
                let normal = Vec3::new(-sin_t, 0.0, cos_t);
                tri.add_vertex(pos, normal);
            }
            tri.add_triangle(end_base, end_base + 1, end_base + 2);
            tri.add_triangle(end_base + 2, end_base + 3, end_base);
        }

        tri.error = tolerance;
        tri
    }
}

impl Tessellate for EllipticalDish {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        // Elliptical dish: quarter sphere with Z-axis scaling
        sphere_based_shape(
            self.base_radius,               // radius
            PI / 2.0,                       // arc (quarter sphere)
            0.0,                            // shift_z
            self.height / self.base_radius, // scale_z (elliptical)
            tolerance,
            scale,
        )
    }
}

impl Tessellate for SphericalDish {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        let r_circ = self.base_radius;
        let h = self.height;

        // Calculate sphere radius from base radius and height
        let r_sphere = (r_circ * r_circ + h * h) / (2.0 * h);

        // Calculate arc angle
        let sinval = (r_circ / r_sphere).clamp(-1.0, 1.0);
        let mut arc = sinval.asin();
        if r_circ < h {
            arc = PI - arc;
        }

        // Spherical dish: partial sphere with shift
        sphere_based_shape(
            r_sphere,     // radius
            arc,          // arc (partial sphere)
            h - r_sphere, // shift_z
            1.0,          // scale_z (uniform)
            tolerance,
            scale,
        )
    }
}

impl Tessellate for Snout {
    fn tessellate(&self, tolerance: f32, scale: f32) -> Triangulation {
        // Default: generate all caps
        self.tessellate_with_caps(tolerance, scale, &[true, true])
    }
}

impl TessellateWithCaps for Snout {
    fn tessellate_with_caps(
        &self,
        tolerance: f32,
        scale: f32,
        generate_caps: &[bool],
    ) -> Triangulation {
        let mut tri = Triangulation::new();

        // Determine which caps to generate
        let gen_bottom = generate_caps.get(0).copied().unwrap_or(true);
        let gen_top = generate_caps.get(1).copied().unwrap_or(true);

        let r_bottom = self.radius_bottom;
        let r_top = self.radius_top;
        let height = self.height;
        let h2 = height / 2.0;
        let ox = self.offset_x / 2.0;
        let oy = self.offset_y / 2.0;

        let radius_max = r_bottom.max(r_top);
        let segments = sagitta_based_segment_count(2.0 * PI, radius_max * scale, tolerance, 8, 64);

        // Convert shear angles to slopes (tan of angle)
        let mb_x = self.bottom_shear_x.tan();
        let mb_y = self.bottom_shear_y.tan();
        let mt_x = self.top_shear_x.tan();
        let mt_y = self.top_shear_y.tan();

        // Pre-compute angles
        let mut angles = Vec::with_capacity(segments);
        for i in 0..segments {
            let angle = 2.0 * PI * (i as f32) / (segments as f32);
            angles.push((angle.cos(), angle.sin()));
        }

        // Generate shell vertices
        for i in 0..segments {
            let (cos_a, sin_a) = angles[i];

            // Bottom circle positions (with shear)
            let bx = r_bottom * cos_a - ox;
            let by = r_bottom * sin_a - oy;
            let bz = -h2 + mb_x * r_bottom * cos_a + mb_y * r_bottom * sin_a;

            // Top circle positions (with shear and offset)
            let tx = r_top * cos_a + ox;
            let ty = r_top * sin_a + oy;
            let tz = h2 + mt_x * r_top * cos_a + mt_y * r_top * sin_a;

            // Calculate shell normal (considering taper and offset)
            let s = self.offset_x * cos_a + self.offset_y * sin_a;
            let nx = cos_a;
            let ny = sin_a;
            let nz = -(r_top - r_bottom + s) / height;
            let normal = Vec3::new(nx, ny, nz).normalize_or_zero();

            tri.add_vertex(Vec3::new(bx, bz, by), normal);
            tri.add_vertex(Vec3::new(tx, tz, ty), normal);
        }

        // Generate shell triangles
        for i in 0..segments {
            let i_next = (i + 1) % segments;
            let base = (i * 2) as u32;
            let next = (i_next * 2) as u32;

            tri.add_triangle(base, next, base + 1);
            tri.add_triangle(base + 1, next, next + 1);
        }

        let shell_verts = (segments * 2) as u32;

        // Conditionally generate bottom cap
        if gen_bottom {
            // Bottom cap normal (considering shear)
            let bottom_normal = Vec3::new(
                self.bottom_shear_x.sin() * self.bottom_shear_y.cos(),
                -self.bottom_shear_x.cos() * self.bottom_shear_y.cos(),
                self.bottom_shear_y.sin(),
            )
            .normalize_or_zero();

            // Add bottom cap vertices
            for i in 0..segments {
                let (cos_a, sin_a) = angles[i];
                let bx = r_bottom * cos_a - ox;
                let by = r_bottom * sin_a - oy;
                let bz = -h2 + mb_x * r_bottom * cos_a + mb_y * r_bottom * sin_a;
                tri.add_vertex(Vec3::new(bx, bz, by), bottom_normal);
            }

            // Tessellate bottom cap (fan triangulation, reversed winding)
            for i in 1..(segments - 1) {
                tri.add_triangle(
                    shell_verts,
                    shell_verts + (i + 1) as u32,
                    shell_verts + i as u32,
                );
            }
        }

        // Conditionally generate top cap
        if gen_top {
            let top_cap_base = if gen_bottom {
                shell_verts + segments as u32
            } else {
                shell_verts
            };

            // Top cap normal (considering shear)
            let top_normal = Vec3::new(
                -self.top_shear_x.sin() * self.top_shear_y.cos(),
                self.top_shear_x.cos() * self.top_shear_y.cos(),
                -self.top_shear_y.sin(),
            )
            .normalize_or_zero();

            // Add top cap vertices
            for i in 0..segments {
                let (cos_a, sin_a) = angles[i];
                let tx = r_top * cos_a + ox;
                let ty = r_top * sin_a + oy;
                let tz = h2 + mt_x * r_top * cos_a + mt_y * r_top * sin_a;
                tri.add_vertex(Vec3::new(tx, tz, ty), top_normal);
            }

            // Tessellate top cap (fan triangulation)
            for i in 1..(segments - 1) {
                tri.add_triangle(
                    top_cap_base,
                    top_cap_base + i as u32,
                    top_cap_base + (i + 1) as u32,
                );
            }
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
            tessellate_polygon(polygon, &mut tri);
        }

        tri.error = 0.0;
        tri
    }
}

/// Tessellate a single polygon (with possibly multiple contours) into the output Triangulation.
/// Matches C++ libtess2 behaviour:
///   - 1 contour, 3 verts → direct triangle
///   - 1 contour, 4 verts → quad split along best diagonal
///   - otherwise → spade CDT with inside/outside flood-fill
fn tessellate_polygon(polygon: &Polygon, tri: &mut Triangulation) {
    // Skip degenerate contours / validate finite vertex data
    let valid_contours: Vec<&Contour> = polygon
        .contours
        .iter()
        .filter(|c| {
            c.vertices.len() >= 3
                && c.vertices
                    .iter()
                    .all(|v| v.x.is_finite() && v.y.is_finite() && v.z.is_finite())
        })
        .collect();

    if valid_contours.is_empty() {
        return;
    }

    let total_verts: usize = valid_contours.iter().map(|c| c.vertices.len()).sum();
    if total_verts < 3 {
        return;
    }

    // Fast-paths for simple single-contour cases (matching C++)
    if valid_contours.len() == 1 {
        let cont = valid_contours[0];
        if cont.vertices.len() == 3 {
            tessellate_triangle(cont, tri);
            return;
        }
        if cont.vertices.len() == 4 {
            tessellate_quad(cont, tri);
            return;
        }
    }

    tessellate_complex_polygon(&valid_contours, tri);
}

fn normal_for_vertex(cont: &Contour, idx: usize, fallback: Vec3) -> Vec3 {
    cont.normals.get(idx).copied().unwrap_or(fallback)
}

fn compute_contour_fallback_normal(cont: &Contour) -> Vec3 {
    if cont.vertices.len() >= 3 {
        let a = cont.vertices[1] - cont.vertices[0];
        let b = cont.vertices[2] - cont.vertices[0];
        let n = a.cross(b);
        if n.length_squared() > 1e-20 {
            return n.normalize();
        }
    }
    Vec3::Z
}

fn tessellate_triangle(cont: &Contour, tri: &mut Triangulation) {
    let fb = compute_contour_fallback_normal(cont);
    let v0 = tri.add_vertex(cont.vertices[0], normal_for_vertex(cont, 0, fb));
    let v1 = tri.add_vertex(cont.vertices[1], normal_for_vertex(cont, 1, fb));
    let v2 = tri.add_vertex(cont.vertices[2], normal_for_vertex(cont, 2, fb));
    tri.add_triangle(v0, v1, v2);
}

/// Split a quad along the least-folding diagonal (matching C++ logic).
fn tessellate_quad(cont: &Contour, tri: &mut Triangulation) {
    let fb = compute_contour_fallback_normal(cont);
    let vo = (tri.vertices.len() / 3) as u32;

    for i in 0..4 {
        tri.add_vertex(cont.vertices[i], normal_for_vertex(cont, i, fb));
    }

    let v = &cont.vertices;
    let v01 = v[1] - v[0];
    let v12 = v[2] - v[1];
    let v23 = v[3] - v[2];
    let v30 = v[0] - v[3];

    let n0 = v01.cross(v30);
    let n1 = v12.cross(v01);
    let n2 = v23.cross(v12);
    let n3 = v30.cross(v23);

    if n0.dot(n2) < n1.dot(n3) {
        // Split along 0-2
        tri.add_triangle(vo, vo + 1, vo + 2);
        tri.add_triangle(vo + 2, vo + 3, vo);
    } else {
        // Split along 1-3
        tri.add_triangle(vo + 3, vo, vo + 1);
        tri.add_triangle(vo + 1, vo + 2, vo + 3);
    }
}

/// Complex polygon tessellation using spade CDT (Constrained Delaunay Triangulation).
/// Projects 3D contours to a 2D plane, runs CDT with constraint edges along contour
/// boundaries, then uses flood-fill from the convex hull exterior to determine
/// inside/outside faces (WINDING_ODD semantics, matching libtess2).
fn tessellate_complex_polygon(contours: &[&Contour], tri: &mut Triangulation) {
    use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation as SpadeTriangulation};

    // 1. Collect all vertices and compute a robust polygon normal for 2D projection
    let mut all_verts: Vec<Vec3> = Vec::new();
    let mut all_normals: Vec<Vec3> = Vec::new();
    let mut contour_ranges: Vec<std::ops::Range<usize>> = Vec::new();

    for cont in contours {
        let start = all_verts.len();
        all_verts.extend_from_slice(&cont.vertices);
        all_normals.extend_from_slice(&cont.normals);
        let end = all_verts.len();
        contour_ranges.push(start..end);
    }

    if all_verts.len() < 3 {
        return;
    }

    // Compute center and polygon normal via Newell's method
    let center: Vec3 =
        all_verts.iter().copied().sum::<Vec3>() * (1.0 / all_verts.len() as f32);
    let poly_normal = newell_normal(&all_verts);
    if poly_normal.length_squared() < 1e-20 {
        return;
    }
    let poly_normal = poly_normal.normalize();

    // 2. Build local 2D coordinate frame on the polygon plane
    let (u_axis, v_axis) = build_tangent_frame(poly_normal);

    // 3. Project to 2D (centered to improve numerical stability, matching C++)
    let pts_2d: Vec<[f64; 2]> = all_verts
        .iter()
        .map(|v| {
            let d = *v - center;
            [d.dot(u_axis) as f64, d.dot(v_axis) as f64]
        })
        .collect();

    // 4. Build CDT
    let mut cdt = ConstrainedDelaunayTriangulation::<Point2<f64>>::new();

    // Insert all vertices, collecting handles
    let mut handles = Vec::with_capacity(pts_2d.len());
    for pt in &pts_2d {
        match cdt.insert(Point2::new(pt[0], pt[1])) {
            Ok(h) => handles.push(h),
            Err(_) => {
                // Duplicate point — find existing handle via locate
                let existing = cdt.locate(Point2::new(pt[0], pt[1]));
                if let spade::PositionInTriangulation::OnVertex(h) = existing {
                    handles.push(h);
                } else {
                    // fallback — should not happen
                    return;
                }
            }
        }
    }

    // Add constraint edges along each contour
    for range in &contour_ranges {
        let n = range.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let from = handles[range.start + i];
            let to = handles[range.start + (i + 1) % n];
            if from != to {
                let _ = cdt.try_add_constraint(from, to);
            }
        }
    }

    // 5. Classify each CDT face as inside/outside using ray-casting
    //    on the face centroid against all polygon contours.
    //    This implements TESS_WINDING_ODD semantics robustly,
    //    even when some constraint edges fail due to near-degenerate projections.
    use std::collections::HashMap;

    let num_faces = cdt.num_inner_faces();
    if num_faces == 0 {
        return;
    }

    let inner_faces: Vec<_> = cdt.inner_faces().collect();

    // Build 2D contour edge list for winding-number test
    let contour_edges_2d: Vec<Vec<([f64; 2], [f64; 2])>> = contour_ranges
        .iter()
        .map(|range| {
            let n = range.len();
            (0..n)
                .map(|i| (pts_2d[range.start + i], pts_2d[range.start + (i + 1) % n]))
                .collect()
        })
        .collect();

    // 6. Emit inside triangles
    let mut handle_to_out: HashMap<spade::handles::FixedVertexHandle, u32> = HashMap::new();
    for (i, &h) in handles.iter().enumerate() {
        handle_to_out.entry(h).or_insert_with(|| {
            let fb = all_normals.get(i).copied().unwrap_or(poly_normal);
            tri.add_vertex(all_verts[i], fb)
        });
    }
    // Handle any Steiner points (not expected, but safe)
    for v in cdt.vertices() {
        let h = v.fix();
        handle_to_out.entry(h).or_insert_with(|| {
            let p = v.position();
            let pos_3d = center + u_axis * (p.x as f32) + v_axis * (p.y as f32);
            tri.add_vertex(pos_3d, poly_normal)
        });
    }

    for face in &inner_faces {
        let verts = face.vertices();
        let p0 = verts[0].position();
        let p1 = verts[1].position();
        let p2 = verts[2].position();
        let cx = (p0.x + p1.x + p2.x) / 3.0;
        let cy = (p0.y + p1.y + p2.y) / 3.0;

        if point_in_polygon_odd([cx, cy], &contour_edges_2d) {
            let i0 = handle_to_out[&verts[0].fix()];
            let i1 = handle_to_out[&verts[1].fix()];
            let i2 = handle_to_out[&verts[2].fix()];
            tri.add_triangle(i0, i1, i2);
        }
    }
}

/// Ray-casting point-in-polygon test with WINDING_ODD semantics.
/// Returns true if a ray from `point` crosses the contour boundaries an odd number of times.
fn point_in_polygon_odd(point: [f64; 2], contour_edges: &[Vec<([f64; 2], [f64; 2])>]) -> bool {
    let mut crossings = 0usize;
    let (px, py) = (point[0], point[1]);

    for contour in contour_edges {
        for &(a, b) in contour {
            let (ax, ay) = (a[0], a[1]);
            let (bx, by) = (b[0], b[1]);

            // Standard ray-casting: horizontal ray to +x
            if (ay <= py && by > py) || (by <= py && ay > py) {
                let t = (py - ay) / (by - ay);
                if px < ax + t * (bx - ax) {
                    crossings += 1;
                }
            }
        }
    }

    crossings % 2 == 1
}

/// Newell's method for computing a robust polygon normal from a vertex list.
fn newell_normal(verts: &[Vec3]) -> Vec3 {
    let mut n = Vec3::ZERO;
    let len = verts.len();
    for i in 0..len {
        let cur = verts[i];
        let next = verts[(i + 1) % len];
        n.x += (cur.y - next.y) * (cur.z + next.z);
        n.y += (cur.z - next.z) * (cur.x + next.x);
        n.z += (cur.x - next.x) * (cur.y + next.y);
    }
    n
}

/// Build an orthonormal tangent frame (u, v) from a given normal.
fn build_tangent_frame(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.y.abs() < 0.9 {
        Vec3::Y
    } else {
        Vec3::X
    };
    let u = up.cross(normal).normalize();
    let v = normal.cross(u).normalize();
    (u, v)
}
