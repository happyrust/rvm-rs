use crate::store::geometry::GeometryId;
use glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionFlags {
    None = 0,
    HasCircularSide = 1,
    HasRectangularSide = 2,
}

impl ConnectionFlags {
    pub fn has_circular_side(self) -> bool {
        matches!(self, ConnectionFlags::HasCircularSide)
    }

    pub fn has_rectangular_side(self) -> bool {
        matches!(self, ConnectionFlags::HasRectangularSide)
    }

    pub fn combine(self, other: ConnectionFlags) -> ConnectionFlags {
        match (self, other) {
            (ConnectionFlags::None, other) => other,
            (this, ConnectionFlags::None) => this,
            (ConnectionFlags::HasCircularSide, _) | (_, ConnectionFlags::HasCircularSide) => {
                ConnectionFlags::HasCircularSide
            }
            _ => ConnectionFlags::HasRectangularSide,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Connection {
    pub geometries: [GeometryId; 2],
    pub offsets: [usize; 2],
    pub position: Vec3,
    pub normal: Vec3,
    pub flags: ConnectionFlags,
}

impl Connection {
    pub fn new(
        geo1: GeometryId,
        geo2: GeometryId,
        offset1: usize,
        offset2: usize,
        position: Vec3,
        normal: Vec3,
        flags: ConnectionFlags,
    ) -> Self {
        Self {
            geometries: [geo1, geo2],
            offsets: [offset1, offset2],
            position,
            normal,
            flags,
        }
    }

    pub fn has_circular_side(&self) -> bool {
        self.flags.has_circular_side()
    }

    pub fn has_rectangular_side(&self) -> bool {
        self.flags.has_rectangular_side()
    }
}

