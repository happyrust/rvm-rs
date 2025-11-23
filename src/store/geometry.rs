use crate::math::BBox3;

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
    pub unknown1: f32,
    pub unknown2: f32,
    pub unknown3: f32,
    pub unknown4: f32,
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
pub struct Polygon {
    pub vertices: Vec<glam::Vec3>,
    pub normals: Vec<glam::Vec3>,
}
