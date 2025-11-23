use nom::{
    bytes::complete::take,
    number::complete::{be_f32, be_u32},
    IResult,
};

pub fn parse_u32(input: &[u8]) -> IResult<&[u8], u32> {
    be_u32(input)
}

pub fn parse_f32(input: &[u8]) -> IResult<&[u8], f32> {
    be_f32(input)
}

pub fn parse_string(input: &[u8]) -> IResult<&[u8], String> {
    let (input, len) = be_u32(input)?;
    let (input, bytes) = take(len)(input)?;

    // Convert bytes to string, removing null padding
    let s = String::from_utf8_lossy(bytes)
        .trim_end_matches('\0')
        .to_string();

    Ok((input, s))
}

pub fn parse_vec3(input: &[u8]) -> IResult<&[u8], glam::Vec3> {
    let (input, x) = be_f32(input)?;
    let (input, y) = be_f32(input)?;
    let (input, z) = be_f32(input)?;
    Ok((input, glam::Vec3::new(x, y, z)))
}

pub fn parse_mat3x4(input: &[u8]) -> IResult<&[u8], glam::Affine3A> {
    let (input, m00) = be_f32(input)?;
    let (input, m01) = be_f32(input)?;
    let (input, m02) = be_f32(input)?;
    let (input, m03) = be_f32(input)?;

    let (input, m10) = be_f32(input)?;
    let (input, m11) = be_f32(input)?;
    let (input, m12) = be_f32(input)?;
    let (input, m13) = be_f32(input)?;

    let (input, m20) = be_f32(input)?;
    let (input, m21) = be_f32(input)?;
    let (input, m22) = be_f32(input)?;
    let (input, m23) = be_f32(input)?;

    let matrix = glam::Mat3A::from_cols(
        glam::Vec3A::new(m00, m10, m20),
        glam::Vec3A::new(m01, m11, m21),
        glam::Vec3A::new(m02, m12, m22),
    );

    let translation = glam::Vec3A::new(m03, m13, m23);

    Ok((
        input,
        glam::Affine3A::from_mat3_translation(matrix.into(), translation.into()),
    ))
}
