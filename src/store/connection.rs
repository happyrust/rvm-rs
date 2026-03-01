use crate::store::geometry::{Box, Geometry, GeometryId, GeometryKind, Pyramid, RectangularTorus};
use glam::{Affine3A, Vec3};

/// Connection flags indicating the type of interface
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionFlags(u8);

impl ConnectionFlags {
    pub const NONE: Self = Self(0);
    pub const HAS_CIRCULAR_SIDE: Self = Self(1 << 0);
    pub const HAS_RECTANGULAR_SIDE: Self = Self(1 << 1);

    pub fn new() -> Self {
        Self::NONE
    }

    pub fn has_circular_side(&self) -> bool {
        self.0 & Self::HAS_CIRCULAR_SIDE.0 != 0
    }

    pub fn has_rectangular_side(&self) -> bool {
        self.0 & Self::HAS_RECTANGULAR_SIDE.0 != 0
    }

    pub fn set_circular_side(&mut self) {
        self.0 |= Self::HAS_CIRCULAR_SIDE.0;
    }

    pub fn set_rectangular_side(&mut self) {
        self.0 |= Self::HAS_RECTANGULAR_SIDE.0;
    }
}

impl Default for ConnectionFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a connection between two geometries
#[derive(Debug, Clone)]
pub struct Connection {
    /// The two connected geometries
    pub geometries: [GeometryId; 2],
    /// The offset/face index for each geometry
    pub offsets: [usize; 2],
    /// Connection point position
    pub position: Vec3,
    /// Connection direction
    pub direction: Vec3,
    /// Connection flags
    pub flags: ConnectionFlags,
    /// Temporary marker for alignment traversal
    pub temp: u32,
}

impl Connection {
    pub fn new(
        geo1: GeometryId,
        offset1: usize,
        geo2: GeometryId,
        offset2: usize,
        position: Vec3,
        direction: Vec3,
    ) -> Self {
        Self {
            geometries: [geo1, geo2],
            offsets: [offset1, offset2],
            position,
            direction,
            flags: ConnectionFlags::new(),
            temp: 0,
        }
    }
}

/// Interface type for geometry connections
#[derive(Debug, Clone)]
pub enum Interface {
    /// Undefined interface (no connection possible)
    Undefined,
    /// Square/rectangular interface with 4 corner points
    Square { corners: [Vec3; 4] },
    /// Circular interface with a radius
    Circular { radius: f32 },
}

impl Interface {
    pub fn kind(&self) -> InterfaceKind {
        match self {
            Interface::Undefined => InterfaceKind::Undefined,
            Interface::Square { .. } => InterfaceKind::Square,
            Interface::Circular { .. } => InterfaceKind::Circular,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceKind {
    Undefined,
    Square,
    Circular,
}

/// Check if two interfaces match (can be connected)
pub fn interfaces_match(iface1: &Interface, iface2: &Interface) -> bool {
    match (iface1, iface2) {
        (Interface::Circular { radius: r1 }, Interface::Circular { radius: r2 }) => {
            // Allow 5% tolerance for circular interfaces
            // Check both directions
            let ratio = r1 / r2;
            ratio >= 0.95 && ratio <= 1.05
        }
        (Interface::Square { corners: c1 }, Interface::Square { corners: c2 }) => {
            // Check if all 4 corners match (with small tolerance)
            const TOLERANCE: f32 = 0.001;
            const TOLERANCE_SQ: f32 = TOLERANCE * TOLERANCE;

            for corner1 in c1.iter() {
                let mut found = false;
                for corner2 in c2.iter() {
                    if corner1.distance_squared(*corner2) < TOLERANCE_SQ {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return false;
                }
            }
            true
        }
        _ => false, // Different types don't match
    }
}

/// Extract the interface for a geometry at a given offset (face index)
///
/// # Arguments
/// * `geometry` - The geometry to extract interface from
/// * `offset` - The face/cap index (0 = bottom/start, 1 = top/end, 2-5 = sides for some shapes)
///
/// # Returns
/// The interface at the specified offset
pub fn get_interface(geometry: &Geometry, offset: usize) -> Interface {
    let transform = &geometry.transform;
    let scale = get_scale(transform);

    match &geometry.kind {
        GeometryKind::Pyramid(pyramid) => get_pyramid_interface(pyramid, transform, offset),
        GeometryKind::Box(b) => get_box_interface(b, transform, offset),
        GeometryKind::RectangularTorus(torus) => {
            get_rectangular_torus_interface(torus, transform, offset)
        }
        GeometryKind::CircularTorus(torus) => Interface::Circular {
            radius: scale * torus.radius,
        },
        GeometryKind::Cylinder(cylinder) => Interface::Circular {
            radius: scale * cylinder.radius,
        },
        GeometryKind::Snout(snout) => Interface::Circular {
            radius: scale
                * if offset == 0 {
                    snout.radius_bottom
                } else {
                    snout.radius_top
                },
        },
        GeometryKind::EllipticalDish(dish) => Interface::Circular {
            radius: scale * dish.base_radius,
        },
        GeometryKind::SphericalDish(dish) => {
            let r_circ = dish.base_radius;
            let h = dish.height;
            let r_sphere = (r_circ * r_circ + h * h) / (2.0 * h);
            Interface::Circular {
                radius: scale * r_sphere,
            }
        }
        GeometryKind::Sphere(_) | GeometryKind::Line(_) | GeometryKind::FacetGroup(_) => {
            Interface::Undefined
        }
    }
}

/// Extract scale from transform matrix
fn get_scale(transform: &Affine3A) -> f32 {
    // Get the scale from the first column (X axis)
    let x_axis = transform.matrix3.x_axis;
    x_axis.length()
}

/// Transform a local point to world space
fn transform_point(transform: &Affine3A, point: Vec3) -> Vec3 {
    transform.transform_point3(point)
}

/// Get interface for Pyramid geometry
fn get_pyramid_interface(pyramid: &Pyramid, transform: &Affine3A, offset: usize) -> Interface {
    let bx = 0.5 * pyramid.bottom[0];
    let by = 0.5 * pyramid.bottom[1];
    let tx = 0.5 * pyramid.top[0];
    let ty = 0.5 * pyramid.top[1];
    let ox = 0.5 * pyramid.offset[0];
    let oy = 0.5 * pyramid.offset[1];
    let h2 = 0.5 * pyramid.height;

    // Define bottom and top quads
    let bottom_quad = [
        Vec3::new(-bx - ox, -by - oy, -h2),
        Vec3::new(bx - ox, -by - oy, -h2),
        Vec3::new(bx - ox, by - oy, -h2),
        Vec3::new(-bx - ox, by - oy, -h2),
    ];

    let top_quad = [
        Vec3::new(-tx + ox, -ty + oy, h2),
        Vec3::new(tx + ox, -ty + oy, h2),
        Vec3::new(tx + ox, ty + oy, h2),
        Vec3::new(-tx + ox, ty + oy, h2),
    ];

    let corners = if offset < 4 {
        // Side faces (0-3)
        let oo = (offset + 1) & 3;
        [
            transform_point(transform, bottom_quad[offset]),
            transform_point(transform, bottom_quad[oo]),
            transform_point(transform, top_quad[oo]),
            transform_point(transform, top_quad[offset]),
        ]
    } else {
        // Bottom (4) or top (5) face
        let quad = if offset == 4 { bottom_quad } else { top_quad };
        [
            transform_point(transform, quad[0]),
            transform_point(transform, quad[1]),
            transform_point(transform, quad[2]),
            transform_point(transform, quad[3]),
        ]
    };

    Interface::Square { corners }
}

/// Get interface for Box geometry
fn get_box_interface(b: &Box, transform: &Affine3A, offset: usize) -> Interface {
    let xp = 0.5 * b.lengths[0];
    let xm = -xp;
    let yp = 0.5 * b.lengths[1];
    let ym = -yp;
    let zp = 0.5 * b.lengths[2];
    let zm = -zp;

    // Define all 6 faces
    let faces = [
        // Face 0: -X
        [
            Vec3::new(xm, ym, zp),
            Vec3::new(xm, yp, zp),
            Vec3::new(xm, yp, zm),
            Vec3::new(xm, ym, zm),
        ],
        // Face 1: +X
        [
            Vec3::new(xp, ym, zm),
            Vec3::new(xp, yp, zm),
            Vec3::new(xp, yp, zp),
            Vec3::new(xp, ym, zp),
        ],
        // Face 2: -Y
        [
            Vec3::new(xp, ym, zm),
            Vec3::new(xp, ym, zp),
            Vec3::new(xm, ym, zp),
            Vec3::new(xm, ym, zm),
        ],
        // Face 3: +Y
        [
            Vec3::new(xm, yp, zm),
            Vec3::new(xm, yp, zp),
            Vec3::new(xp, yp, zp),
            Vec3::new(xp, yp, zm),
        ],
        // Face 4: -Z
        [
            Vec3::new(xm, yp, zm),
            Vec3::new(xp, yp, zm),
            Vec3::new(xp, ym, zm),
            Vec3::new(xm, ym, zm),
        ],
        // Face 5: +Z
        [
            Vec3::new(xm, ym, zp),
            Vec3::new(xp, ym, zp),
            Vec3::new(xp, yp, zp),
            Vec3::new(xm, yp, zp),
        ],
    ];

    let corners = [
        transform_point(transform, faces[offset][0]),
        transform_point(transform, faces[offset][1]),
        transform_point(transform, faces[offset][2]),
        transform_point(transform, faces[offset][3]),
    ];

    Interface::Square { corners }
}

/// Get interface for RectangularTorus geometry
fn get_rectangular_torus_interface(
    torus: &RectangularTorus,
    transform: &Affine3A,
    offset: usize,
) -> Interface {
    let h2 = 0.5 * torus.height;

    // Define the square cross-section corners
    let square = [
        [torus.outer_radius, -h2],
        [torus.inner_radius, -h2],
        [torus.inner_radius, h2],
        [torus.outer_radius, h2],
    ];

    let corners = if offset == 0 {
        // Start face (at angle 0)
        [
            transform_point(transform, Vec3::new(square[0][0], 0.0, square[0][1])),
            transform_point(transform, Vec3::new(square[1][0], 0.0, square[1][1])),
            transform_point(transform, Vec3::new(square[2][0], 0.0, square[2][1])),
            transform_point(transform, Vec3::new(square[3][0], 0.0, square[3][1])),
        ]
    } else {
        // End face (at angle)
        let cos_a = torus.angle.cos();
        let sin_a = torus.angle.sin();
        [
            transform_point(
                transform,
                Vec3::new(square[0][0] * cos_a, square[0][0] * sin_a, square[0][1]),
            ),
            transform_point(
                transform,
                Vec3::new(square[1][0] * cos_a, square[1][0] * sin_a, square[1][1]),
            ),
            transform_point(
                transform,
                Vec3::new(square[2][0] * cos_a, square[2][0] * sin_a, square[2][1]),
            ),
            transform_point(
                transform,
                Vec3::new(square[3][0] * cos_a, square[3][0] * sin_a, square[3][1]),
            ),
        ]
    };

    Interface::Square { corners }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::BBox3;
    use crate::store::geometry::{
        Box, CircularTorus, Cylinder, EllipticalDish, Geometry, GeometryKind, GeometryType,
        Pyramid, RectangularTorus, Snout, SphericalDish,
    };
    use glam::Affine3A;

    fn create_test_geometry(kind: GeometryKind) -> Geometry {
        Geometry {
            kind,
            geo_type: GeometryType::Primitive,
            transform: Affine3A::IDENTITY,
            bbox_local: BBox3::new(),
            bbox_world: BBox3::new(),
            color: 0,
            transparency: 0,
            sample_start_angle: 0.0,
            color_name: None,
            color_rgb: 0,
            next: None,
        }
    }

    #[test]
    fn test_connection_flags() {
        let mut flags = ConnectionFlags::new();
        assert!(!flags.has_circular_side());
        assert!(!flags.has_rectangular_side());

        flags.set_circular_side();
        assert!(flags.has_circular_side());
        assert!(!flags.has_rectangular_side());

        flags.set_rectangular_side();
        assert!(flags.has_circular_side());
        assert!(flags.has_rectangular_side());
    }

    #[test]
    fn test_circular_interface_match() {
        let iface1 = Interface::Circular { radius: 1.0 };
        let iface2 = Interface::Circular { radius: 1.02 };
        let iface3 = Interface::Circular { radius: 1.2 };

        assert!(interfaces_match(&iface1, &iface2));
        assert!(!interfaces_match(&iface1, &iface3));
    }

    #[test]
    fn test_square_interface_match() {
        let corners1 = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        let corners2 = [
            Vec3::new(0.0001, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];

        let corners3 = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
        ];

        let iface1 = Interface::Square { corners: corners1 };
        let iface2 = Interface::Square { corners: corners2 };
        let iface3 = Interface::Square { corners: corners3 };

        assert!(interfaces_match(&iface1, &iface2));
        assert!(!interfaces_match(&iface1, &iface3));
    }

    #[test]
    fn test_different_interface_types_dont_match() {
        let circular = Interface::Circular { radius: 1.0 };
        let square = Interface::Square {
            corners: [Vec3::ZERO; 4],
        };

        assert!(!interfaces_match(&circular, &square));
    }

    #[test]
    fn test_cylinder_interface() {
        let cylinder = Cylinder {
            radius: 2.0,
            height: 5.0,
        };
        let geo = create_test_geometry(GeometryKind::Cylinder(cylinder));

        let iface = get_interface(&geo, 0);
        match iface {
            Interface::Circular { radius } => {
                assert!((radius - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected circular interface"),
        }
    }

    #[test]
    fn test_snout_interface() {
        let snout = Snout {
            radius_bottom: 2.0,
            radius_top: 1.0,
            height: 5.0,
            offset_x: 0.0,
            offset_y: 0.0,
            bottom_shear_x: 0.0,
            bottom_shear_y: 0.0,
            top_shear_x: 0.0,
            top_shear_y: 0.0,
        };
        let geo = create_test_geometry(GeometryKind::Snout(snout));

        // Bottom interface
        let iface_bottom = get_interface(&geo, 0);
        match iface_bottom {
            Interface::Circular { radius } => {
                assert!((radius - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected circular interface"),
        }

        // Top interface
        let iface_top = get_interface(&geo, 1);
        match iface_top {
            Interface::Circular { radius } => {
                assert!((radius - 1.0).abs() < 0.001);
            }
            _ => panic!("Expected circular interface"),
        }
    }

    #[test]
    fn test_box_interface() {
        let b = Box {
            lengths: [2.0, 3.0, 4.0],
        };
        let geo = create_test_geometry(GeometryKind::Box(b));

        // Test all 6 faces
        for offset in 0..6 {
            let iface = get_interface(&geo, offset);
            match iface {
                Interface::Square { corners } => {
                    // Verify we got 4 corners
                    assert_eq!(corners.len(), 4);
                    // Verify corners form a valid quad (not all the same point)
                    let mut unique = true;
                    for i in 0..3 {
                        if corners[i].distance_squared(corners[i + 1]) < 0.0001 {
                            unique = false;
                        }
                    }
                    assert!(unique, "Box corners should be distinct");
                }
                _ => panic!("Expected square interface for box"),
            }
        }
    }

    #[test]
    fn test_pyramid_interface() {
        let pyramid = Pyramid {
            bottom: [2.0, 2.0],
            top: [1.0, 1.0],
            offset: [0.0, 0.0],
            height: 3.0,
        };
        let geo = create_test_geometry(GeometryKind::Pyramid(pyramid));

        // Test bottom face (offset 4)
        let iface_bottom = get_interface(&geo, 4);
        match iface_bottom {
            Interface::Square { corners } => {
                // Bottom should be 2x2
                let width = corners[0].distance(corners[1]);
                assert!((width - 2.0).abs() < 0.01);
            }
            _ => panic!("Expected square interface"),
        }

        // Test top face (offset 5)
        let iface_top = get_interface(&geo, 5);
        match iface_top {
            Interface::Square { corners } => {
                // Top should be 1x1
                let width = corners[0].distance(corners[1]);
                assert!((width - 1.0).abs() < 0.01);
            }
            _ => panic!("Expected square interface"),
        }
    }

    #[test]
    fn test_rectangular_torus_interface() {
        let torus = RectangularTorus {
            inner_radius: 1.0,
            outer_radius: 2.0,
            height: 3.0,
            angle: std::f32::consts::PI / 2.0, // 90 degrees
        };
        let geo = create_test_geometry(GeometryKind::RectangularTorus(torus));

        // Test start face (offset 0)
        let iface_start = get_interface(&geo, 0);
        match iface_start {
            Interface::Square { corners } => {
                assert_eq!(corners.len(), 4);
            }
            _ => panic!("Expected square interface"),
        }

        // Test end face (offset 1)
        let iface_end = get_interface(&geo, 1);
        match iface_end {
            Interface::Square { corners } => {
                assert_eq!(corners.len(), 4);
            }
            _ => panic!("Expected square interface"),
        }
    }

    #[test]
    fn test_circular_torus_interface() {
        let torus = CircularTorus {
            offset: 3.0,
            radius: 0.5,
            angle: std::f32::consts::PI,
        };
        let geo = create_test_geometry(GeometryKind::CircularTorus(torus));

        let iface = get_interface(&geo, 0);
        match iface {
            Interface::Circular { radius } => {
                assert!((radius - 0.5).abs() < 0.001);
            }
            _ => panic!("Expected circular interface"),
        }
    }

    #[test]
    fn test_elliptical_dish_interface() {
        let dish = EllipticalDish {
            base_radius: 2.5,
            height: 1.0,
        };
        let geo = create_test_geometry(GeometryKind::EllipticalDish(dish));

        let iface = get_interface(&geo, 0);
        match iface {
            Interface::Circular { radius } => {
                assert!((radius - 2.5).abs() < 0.001);
            }
            _ => panic!("Expected circular interface"),
        }
    }

    #[test]
    fn test_spherical_dish_interface() {
        let dish = SphericalDish {
            base_radius: 2.0,
            height: 1.0,
        };
        let geo = create_test_geometry(GeometryKind::SphericalDish(dish));

        let iface = get_interface(&geo, 0);
        match iface {
            Interface::Circular { radius } => {
                // Sphere radius = (r^2 + h^2) / (2h) = (4 + 1) / 2 = 2.5
                assert!((radius - 2.5).abs() < 0.001);
            }
            _ => panic!("Expected circular interface"),
        }
    }

    #[test]
    fn test_interface_with_scaled_transform() {
        let cylinder = Cylinder {
            radius: 1.0,
            height: 2.0,
        };

        // Create geometry with 2x scale
        let transform = Affine3A::from_scale(Vec3::splat(2.0));
        let geo = Geometry {
            kind: GeometryKind::Cylinder(cylinder),
            geo_type: GeometryType::Primitive,
            transform,
            bbox_local: BBox3::new(),
            bbox_world: BBox3::new(),
            color: 0,
            transparency: 0,
            sample_start_angle: 0.0,
            color_name: None,
            color_rgb: 0,
            next: None,
        };

        let iface = get_interface(&geo, 0);
        match iface {
            Interface::Circular { radius } => {
                // Should be scaled: 1.0 * 2.0 = 2.0
                assert!((radius - 2.0).abs() < 0.001);
            }
            _ => panic!("Expected circular interface"),
        }
    }
}
