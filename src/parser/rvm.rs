use crate::parser::common::{parse_f32, parse_u32, parse_vec3};
use crate::parser::ParseError;
use crate::store::geometry::*;
use crate::store::node::*;
use crate::store::Store;
use glam::{Affine3A, Mat3A, Vec3};
use nom::IResult;
use std::convert::TryInto;

pub fn parse_rvm(input: &[u8], store: &mut Store) -> Result<(), ParseError> {
    let mut offset = 0usize;
    let mut current_color: u32 = 0;

    while offset + 20 <= input.len() {
        if input[offset..offset + 16].iter().all(|b| *b == 0) {
            break;
        }

        let (chunk_type, next_offset, chunk_data) = read_chunk(input, offset)?;

        match chunk_type.as_str() {
            "HEAD" => parse_head(chunk_data, store)?,
            "MODL" => parse_modl(chunk_data, store)?,
            "CNTB" => parse_cntb(chunk_data, store)?,
            "CNTE" => parse_cnte(store),
            "COLR" => {
                current_color = parse_colr(chunk_data)?;
            }
            "PRIM" | "OBST" | "INSU" => {
                let geo_type = match chunk_type.as_str() {
                    "OBST" => GeometryType::Obstruction,
                    "INSU" => GeometryType::Insulation,
                    _ => GeometryType::Primitive,
                };
                parse_geometry_chunk(chunk_data, store, geo_type, current_color)?;
            }
            "END" | "END\0" => break,
            _ => { /* 未知块直接跳过 */ }
        }

        if next_offset <= offset {
            break;
        }
        offset = next_offset;
    }

    Ok(())
}

fn read_chunk(input: &[u8], offset: usize) -> Result<(String, usize, &[u8]), ParseError> {
    if offset + 20 > input.len() {
        return Err(ParseError::UnexpectedEof { offset });
    }

    let tag_bytes = &input[offset..offset + 16];
    let chunk_type = decode_chunk_type(tag_bytes);
    let next_offset =
        u32::from_be_bytes(input[offset + 16..offset + 20].try_into().unwrap()) as usize;

    if next_offset > input.len() || next_offset < offset + 20 {
        return Err(ParseError::InvalidChunkHeader {
            offset,
            expected: "valid next offset".to_string(),
            found: format!("{} -> {}", chunk_type, next_offset),
        });
    }

    let data = &input[offset + 20..next_offset];
    Ok((chunk_type, next_offset, data))
}

fn decode_chunk_type(bytes: &[u8]) -> String {
    if bytes.len() == 16 {
        bytes
            .chunks_exact(4)
            .filter_map(|chunk| {
                let v = u32::from_be_bytes(chunk.try_into().ok()?);
                char::from_u32(v)
            })
            .collect()
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

fn decode_padded_string(bytes: &[u8]) -> String {
    let trimmed = bytes.split(|b| *b == 0).next().unwrap_or(&[]);
    String::from_utf8_lossy(trimmed).to_string()
}

fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32, ParseError> {
    if *pos + 4 > data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let v = u32::from_be_bytes(data[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(v)
}

fn read_f32(data: &[u8], pos: &mut usize) -> Result<f32, ParseError> {
    if *pos + 4 > data.len() {
        return Err(ParseError::UnexpectedEof { offset: *pos });
    }
    let v = f32::from_bits(u32::from_be_bytes(data[*pos..*pos + 4].try_into().unwrap()));
    *pos += 4;
    Ok(v)
}

fn parse_head(data: &[u8], store: &mut Store) -> Result<(), ParseError> {
    if data.len() < 12 {
        return Err(ParseError::UnexpectedEof { offset: data.len() });
    }

    let text_bytes = &data[12..];
    let mut parts = text_bytes
        .split(|b| *b == 0)
        .filter_map(|s| std::str::from_utf8(s).ok())
        .filter(|s| !s.is_empty());

    let info = parts.next().unwrap_or_default();
    let note = parts.next().unwrap_or_default();
    let date = parts.next().unwrap_or_default();
    let user = parts.next().unwrap_or_default();
    let encoding = parts.next().unwrap_or("UTF-8");

    let file_node = FileNode {
        info: store.intern_string(info),
        note: store.intern_string(note),
        date: store.intern_string(date),
        user: store.intern_string(user),
        encoding: store.intern_string(encoding),
        path: store.intern_string(""),
    };

    let file_id = store.new_node(NodeKind::File(file_node));
    store.push_node_context(file_id);
    Ok(())
}

fn parse_modl(data: &[u8], store: &mut Store) -> Result<(), ParseError> {
    if data.len() < 12 {
        return Err(ParseError::UnexpectedEof { offset: data.len() });
    }

    let name_words = u32::from_be_bytes(data[8..12].try_into().unwrap()) as usize;
    let name_end = 12 + name_words * 4;
    if name_end > data.len() {
        return Err(ParseError::UnexpectedEof { offset: name_end });
    }

    let model_name = decode_padded_string(&data[12..name_end]);
    let mut project = model_name.clone();

    if data.len() >= name_end + 4 {
        let rest = &data[name_end + 4..];
        let candidate = decode_padded_string(rest);
        if !candidate.is_empty() {
            project = candidate;
        }
    }

    let model_node = ModelNode {
        project: store.intern_string(&project),
        name: store.intern_string(&model_name),
        first_color: None,
    };

    let node_id = store.new_node(NodeKind::Model(model_node));
    store.push_node_context(node_id);
    Ok(())
}

fn parse_cntb(data: &[u8], store: &mut Store) -> Result<(), ParseError> {
    if data.len() < 12 {
        return Err(ParseError::UnexpectedEof { offset: data.len() });
    }

    let name_words = u32::from_be_bytes(data[8..12].try_into().unwrap()) as usize;
    let name_end = 12 + name_words * 4;
    if name_end > data.len() {
        return Err(ParseError::UnexpectedEof { offset: name_end });
    }

    let name = decode_padded_string(&data[12..name_end]);
    let mut translation = glam::Vec3::ZERO;
    let mut material = 0u32;

    if data.len() >= name_end + 16 {
        let tail = &data[name_end..name_end + 16];
        translation = glam::Vec3::new(
            f32::from_bits(u32::from_be_bytes(tail[0..4].try_into().unwrap())),
            f32::from_bits(u32::from_be_bytes(tail[4..8].try_into().unwrap())),
            f32::from_bits(u32::from_be_bytes(tail[8..12].try_into().unwrap())),
        );
        material = u32::from_be_bytes(tail[12..16].try_into().unwrap());
    }

    let group_node = GroupNode {
        name: store.intern_string(&name),
        translation,
        material,
        transparency: 0,
        bbox_world: crate::math::BBox3::new(),
        first_geometry: None,
        attributes: Vec::new(),
    };

    let node_id = store.new_node(NodeKind::Group(group_node));
    store.push_node_context(node_id);
    Ok(())
}

fn parse_cnte(store: &mut Store) {
    store.pop_node_context();
}

fn parse_colr(data: &[u8]) -> Result<u32, ParseError> {
    if data.len() < 16 {
        return Err(ParseError::UnexpectedEof { offset: data.len() });
    }

    let color = u32::from_be_bytes(data[12..16].try_into().unwrap());
    Ok(color & 0x00FF_FFFF)
}

fn parse_geometry_chunk(
    data: &[u8],
    store: &mut Store,
    geo_type: GeometryType,
    current_color: u32,
) -> Result<(), ParseError> {
    // Minimum size: version(4) + kind(4) + matrix(48) + bbox(24) = 80 bytes
    if data.len() < 80 {
        return Err(ParseError::UnexpectedEof { offset: data.len() });
    }

    let mut pos = 0usize;
    let _version = read_u32(data, &mut pos)?;
    let kind_id = read_u32(data, &mut pos)?;

    // Read 3x4 transformation matrix (row-major order in file)
    let m00 = read_f32(data, &mut pos)?;
    let m01 = read_f32(data, &mut pos)?;
    let m02 = read_f32(data, &mut pos)?;
    let m03 = read_f32(data, &mut pos)?;

    let m10 = read_f32(data, &mut pos)?;
    let m11 = read_f32(data, &mut pos)?;
    let m12 = read_f32(data, &mut pos)?;
    let m13 = read_f32(data, &mut pos)?;

    let m20 = read_f32(data, &mut pos)?;
    let m21 = read_f32(data, &mut pos)?;
    let m22 = read_f32(data, &mut pos)?;
    let m23 = read_f32(data, &mut pos)?;

    let mat = Mat3A::from_cols(
        glam::Vec3A::new(m00, m10, m20),
        glam::Vec3A::new(m01, m11, m21),
        glam::Vec3A::new(m02, m12, m22),
    );
    let translation = Vec3::new(m03, m13, m23);
    let transform = Affine3A::from_mat3_translation(mat.into(), translation);

    let bbox_min = Vec3::new(
        read_f32(data, &mut pos)?,
        read_f32(data, &mut pos)?,
        read_f32(data, &mut pos)?,
    );
    let bbox_max = Vec3::new(
        read_f32(data, &mut pos)?,
        read_f32(data, &mut pos)?,
        read_f32(data, &mut pos)?,
    );
    let bbox = crate::math::BBox3::from_min_max(bbox_min, bbox_max);

    // Handle transparency for OBST and INSU types
    let mut transparency = 0u32;
    let has_transparency = matches!(geo_type, GeometryType::Obstruction | GeometryType::Insulation);
    
    if has_transparency && pos + 4 <= data.len() {
        transparency = data[pos] as u32;
        pos += 4; // Skip 4 bytes (transparency + 3 padding bytes)
    }

    let remaining = &data[pos..];
    let kind = match kind_id {
        1 if remaining.len() >= 28 => {
            // Pyramid
            let mut p = 0;
            GeometryKind::Pyramid(Pyramid {
                bottom: [
                    read_f32(remaining, &mut p)?,
                    read_f32(remaining, &mut p)?,
                ],
                top: [
                    read_f32(remaining, &mut p)?,
                    read_f32(remaining, &mut p)?,
                ],
                offset: [
                    read_f32(remaining, &mut p)?,
                    read_f32(remaining, &mut p)?,
                ],
                height: read_f32(remaining, &mut p)?,
            })
        }
        2 if remaining.len() >= 12 => {
            // Box
            let mut p = 0;
            GeometryKind::Box(Box {
                lengths: [
                    read_f32(remaining, &mut p)?,
                    read_f32(remaining, &mut p)?,
                    read_f32(remaining, &mut p)?,
                ],
            })
        }
        3 if remaining.len() >= 16 => {
            // RectangularTorus
            let mut p = 0;
            GeometryKind::RectangularTorus(RectangularTorus {
                inner_radius: read_f32(remaining, &mut p)?,
                outer_radius: read_f32(remaining, &mut p)?,
                height: read_f32(remaining, &mut p)?,
                angle: read_f32(remaining, &mut p)?,
            })
        }
        4 if remaining.len() >= 12 => {
            // CircularTorus
            let mut p = 0;
            GeometryKind::CircularTorus(CircularTorus {
                offset: read_f32(remaining, &mut p)?,
                radius: read_f32(remaining, &mut p)?,
                angle: read_f32(remaining, &mut p)?,
            })
        }
        5 if remaining.len() >= 8 => {
            // EllipticalDish
            let mut p = 0;
            GeometryKind::EllipticalDish(EllipticalDish {
                base_radius: read_f32(remaining, &mut p)?,
                height: read_f32(remaining, &mut p)?,
            })
        }
        6 if remaining.len() >= 8 => {
            // SphericalDish
            let mut p = 0;
            GeometryKind::SphericalDish(SphericalDish {
                base_radius: read_f32(remaining, &mut p)?,
                height: read_f32(remaining, &mut p)?,
            })
        }
        7 if remaining.len() >= 36 => {
            // Snout
            let mut p = 0;
            GeometryKind::Snout(Snout {
                radius_bottom: read_f32(remaining, &mut p)?,
                radius_top: read_f32(remaining, &mut p)?,
                height: read_f32(remaining, &mut p)?,
                offset_x: read_f32(remaining, &mut p)?,
                offset_y: read_f32(remaining, &mut p)?,
                unknown1: read_f32(remaining, &mut p)?,
                unknown2: read_f32(remaining, &mut p)?,
                unknown3: read_f32(remaining, &mut p)?,
                unknown4: read_f32(remaining, &mut p)?,
            })
        }
        8 if remaining.len() >= 8 => {
            // Cylinder
            let mut p = 0;
            GeometryKind::Cylinder(Cylinder {
                radius: read_f32(remaining, &mut p)?,
                height: read_f32(remaining, &mut p)?,
            })
        }
        9 if remaining.len() >= 4 => {
            // Sphere
            let mut p = 0;
            GeometryKind::Sphere(Sphere {
                radius: read_f32(remaining, &mut p)?,
            })
        }
        10 if remaining.len() >= 8 => {
            // Line
            let mut p = 0;
            GeometryKind::Line(Line {
                start_radius: read_f32(remaining, &mut p)?,
                end_radius: read_f32(remaining, &mut p)?,
            })
        }
        11 => {
            // FacetGroup
            let facet = parse_facet_group(remaining)?;
            GeometryKind::FacetGroup(facet)
        }
        _ => {
            // Unknown geometry kind or insufficient data - create a box from bbox as fallback
            let lengths = (bbox_max - bbox_min).to_array();
            GeometryKind::Box(Box { lengths })
        }
    };

    if let Some(parent_id) = store
        .current_node()
        .or_else(|| store.roots().last().copied())
    {
        // Inherit transparency from parent if not explicitly set
        let final_transparency = if !has_transparency {
            if let Some(parent) = store.get_node(parent_id) {
                if let NodeKind::Group(ref group) = parent.kind {
                    group.transparency
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            transparency
        };

        let geo_id = store.new_geometry(parent_id, kind);
        if let Some(geo) = store.get_geometry_mut(geo_id) {
            geo.transform = transform;
            geo.geo_type = geo_type;
            geo.color = current_color;
            geo.bbox_local = bbox;
            geo.bbox_world = bbox.transform(&transform);
            geo.transparency = final_transparency;
        }
    }

    Ok(())
}

fn parse_facet_group(data: &[u8]) -> Result<FacetGroup, ParseError> {
    let mut pos = 0usize;
    if data.len() < 4 {
        return Err(ParseError::UnexpectedEof { offset: pos });
    }

    let num_polygons = read_u32(data, &mut pos)? as usize;
    let mut polygons = Vec::new();

    for _ in 0..num_polygons {
        if pos + 4 > data.len() {
            break;
        }
        // Number of contours in this polygon
        let num_contours = read_u32(data, &mut pos)? as usize;

        // For now, we flatten all contours into a single polygon
        // C++ uses Polygon->Contour->Vertices structure
        let mut all_vertices = Vec::new();
        let mut all_normals = Vec::new();

        for _ in 0..num_contours {
            if pos + 4 > data.len() {
                return Err(ParseError::UnexpectedEof { offset: pos });
            }
            let vertex_count = read_u32(data, &mut pos)? as usize;

            for _ in 0..vertex_count {
                if pos + 24 > data.len() {
                    return Err(ParseError::UnexpectedEof { offset: pos });
                }
                let x = read_f32(data, &mut pos)?;
                let y = read_f32(data, &mut pos)?;
                let z = read_f32(data, &mut pos)?;
                let nx = read_f32(data, &mut pos)?;
                let ny = read_f32(data, &mut pos)?;
                let nz = read_f32(data, &mut pos)?;
                all_vertices.push(Vec3::new(x, y, z));
                all_normals.push(Vec3::new(nx, ny, nz));
            }
        }

        polygons.push(Polygon {
            vertices: all_vertices,
            normals: all_normals,
        });
    }

    Ok(FacetGroup { polygons })
}

#[allow(dead_code)]
fn parse_geometry_kind(input: &[u8], kind_id: u32) -> IResult<&[u8], GeometryKind> {
    match kind_id {
        1 => {
            // Pyramid
            let (input, bottom_x) = parse_f32(input)?;
            let (input, bottom_y) = parse_f32(input)?;
            let (input, top_x) = parse_f32(input)?;
            let (input, top_y) = parse_f32(input)?;
            let (input, offset_x) = parse_f32(input)?;
            let (input, offset_y) = parse_f32(input)?;
            let (input, height) = parse_f32(input)?;

            Ok((
                input,
                GeometryKind::Pyramid(Pyramid {
                    bottom: [bottom_x, bottom_y],
                    top: [top_x, top_y],
                    offset: [offset_x, offset_y],
                    height,
                }),
            ))
        }
        2 => {
            // Box
            let (input, x) = parse_f32(input)?;
            let (input, y) = parse_f32(input)?;
            let (input, z) = parse_f32(input)?;

            Ok((input, GeometryKind::Box(Box { lengths: [x, y, z] })))
        }
        3 => {
            // RectangularTorus
            let (input, inner) = parse_f32(input)?;
            let (input, outer) = parse_f32(input)?;
            let (input, height) = parse_f32(input)?;
            let (input, angle) = parse_f32(input)?;

            Ok((
                input,
                GeometryKind::RectangularTorus(RectangularTorus {
                    inner_radius: inner,
                    outer_radius: outer,
                    height,
                    angle,
                }),
            ))
        }
        4 => {
            // CircularTorus
            let (input, offset) = parse_f32(input)?;
            let (input, radius) = parse_f32(input)?;
            let (input, angle) = parse_f32(input)?;

            Ok((
                input,
                GeometryKind::CircularTorus(CircularTorus {
                    offset,
                    radius,
                    angle,
                }),
            ))
        }
        5 => {
            // EllipticalDish
            let (input, base_radius) = parse_f32(input)?;
            let (input, height) = parse_f32(input)?;

            Ok((
                input,
                GeometryKind::EllipticalDish(EllipticalDish {
                    base_radius,
                    height,
                }),
            ))
        }
        6 => {
            // SphericalDish
            let (input, base_radius) = parse_f32(input)?;
            let (input, height) = parse_f32(input)?;

            Ok((
                input,
                GeometryKind::SphericalDish(SphericalDish {
                    base_radius,
                    height,
                }),
            ))
        }
        7 => {
            // Snout
            let (input, rb) = parse_f32(input)?;
            let (input, rt) = parse_f32(input)?;
            let (input, height) = parse_f32(input)?;
            let (input, ox) = parse_f32(input)?;
            let (input, oy) = parse_f32(input)?;
            let (input, u1) = parse_f32(input)?;
            let (input, u2) = parse_f32(input)?;
            let (input, u3) = parse_f32(input)?;
            let (input, u4) = parse_f32(input)?;

            Ok((
                input,
                GeometryKind::Snout(Snout {
                    radius_bottom: rb,
                    radius_top: rt,
                    height,
                    offset_x: ox,
                    offset_y: oy,
                    unknown1: u1,
                    unknown2: u2,
                    unknown3: u3,
                    unknown4: u4,
                }),
            ))
        }
        8 => {
            // Cylinder
            let (input, radius) = parse_f32(input)?;
            let (input, height) = parse_f32(input)?;

            Ok((input, GeometryKind::Cylinder(Cylinder { radius, height })))
        }
        9 => {
            // Sphere
            let (input, radius) = parse_f32(input)?;

            Ok((input, GeometryKind::Sphere(Sphere { radius })))
        }
        10 => {
            // Line
            let (input, start_radius) = parse_f32(input)?;
            let (input, end_radius) = parse_f32(input)?;

            Ok((
                input,
                GeometryKind::Line(Line {
                    start_radius,
                    end_radius,
                }),
            ))
        }
        11 => {
            // FacetGroup
            let (input, num_polygons) = parse_u32(input)?;
            let mut polygons = Vec::new();

            let mut remaining = input;
            for _ in 0..num_polygons {
                let (rest, num_vertices) = parse_u32(remaining)?;
                let mut vertices = Vec::new();

                let mut vert_remaining = rest;
                for _ in 0..num_vertices {
                    let (rest, vertex) = parse_vec3(vert_remaining)?;
                    vertices.push(vertex);
                    vert_remaining = rest;
                }

                let (rest, normal) = parse_vec3(vert_remaining)?;
                let normals = vec![normal; num_vertices as usize];
                polygons.push(Polygon { vertices, normals });
                remaining = rest;
            }

            Ok((remaining, GeometryKind::FacetGroup(FacetGroup { polygons })))
        }
        _ => Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        ))),
    }
}
