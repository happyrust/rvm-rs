use crate::math::BBox3;
use crate::store::strings::StringId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GeometryId(pub u32);

#[derive(Debug, Clone)]
pub enum GeometryKind {
    Pyramid(Pyramid),
    Box(Box),
    RectangularTorus(RectangularTorus),
    CircularTorus(CircularTorus),
    EllipticalDish(EllipticalDish),
    SphericalDish(SphericalDish),
    Snout(Snout),
    Cylinder(Cylinder),
    Sphere(Sphere),
    Line(Line),
    FacetGroup(FacetGroup),
}

#[derive(Debug, Clone)]
pub struct Geometry {
    pub kind: GeometryKind,
    pub geo_type: GeometryType,
    pub transform: glam::Affine3A,
    pub bbox_local: BBox3,
    pub bbox_world: BBox3,
    pub color: u32,
    pub transparency: u32,
    pub sample_start_angle: f32,
    pub color_name: Option<StringId>,
    pub color_rgb: u32,
    pub next: Option<GeometryId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometryType {
    Primitive,
    Obstruction,
    Insulation,
}

#[derive(Debug, Clone)]
pub struct Pyramid {
    pub bottom: [f32; 2],
    pub top: [f32; 2],
    pub offset: [f32; 2],
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct Box {
    pub lengths: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct RectangularTorus {
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub height: f32,
    pub angle: f32,
}

#[derive(Debug, Clone)]
pub struct CircularTorus {
    pub offset: f32,
    pub radius: f32,
    pub angle: f32,
}

#[derive(Debug, Clone)]
pub struct EllipticalDish {
    pub base_radius: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct SphericalDish {
    pub base_radius: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct Snout {
    pub radius_bottom: f32,
    pub radius_top: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub bottom_shear_x: f32, // bshear[0] - bottom shear angle in X direction
    pub bottom_shear_y: f32, // bshear[1] - bottom shear angle in Y direction
    pub top_shear_x: f32,    // tshear[0] - top shear angle in X direction
    pub top_shear_y: f32,    // tshear[1] - top shear angle in Y direction
}

#[derive(Debug, Clone)]
pub struct Cylinder {
    pub radius: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct Sphere {
    pub radius: f32,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub start_radius: f32,
    pub end_radius: f32,
}

#[derive(Debug, Clone)]
pub struct FacetGroup {
    pub polygons: Vec<Polygon>,
}

#[derive(Debug, Clone)]
pub struct Contour {
    pub vertices: Vec<glam::Vec3>,
    pub normals: Vec<glam::Vec3>,
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub contours: Vec<Contour>,
}

impl Polygon {
    /// Total vertex count across all contours.
    pub fn total_vertices(&self) -> usize {
        self.contours.iter().map(|c| c.vertices.len()).sum()
    }

    /// Convenience: single-contour polygon vertices.
    pub fn vertices(&self) -> &[glam::Vec3] {
        if self.contours.len() == 1 {
            &self.contours[0].vertices
        } else {
            &[]
        }
    }

    /// Convenience: single-contour polygon normals.
    pub fn normals(&self) -> &[glam::Vec3] {
        if self.contours.len() == 1 {
            &self.contours[0].normals
        } else {
            &[]
        }
    }
}
