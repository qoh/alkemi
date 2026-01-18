use std::marker::PhantomData;

use winnow::{
    Parser as _, Result,
    binary::{le_f32, le_i32, length_take},
    combinator::{alt, seq},
};

use crate::xnb::TypeReaderMeta;

use super::Stream;

pub fn bool(input: &mut Stream) -> Result<bool> {
    u8.map(|n| n != 0).parse_next(input)
}

pub use winnow::binary::{le_i16 as i16, le_u16 as u16, le_u32 as u32, u8};

pub fn i32(input: &mut Stream) -> Result<i32> {
    le_i32.parse_next(input)
}

pub fn f32(input: &mut Stream) -> Result<f32> {
    le_f32.parse_next(input)
}

#[derive(Clone, Copy)]
pub struct Vector2(pub f32, pub f32);
pub fn vec2<Input, Error>(input: &mut Input) -> Result<Vector2, Error>
where
    Input: winnow::stream::StreamIsPartial + winnow::stream::Stream<Token = u8>,
    Error: winnow::error::ParserError<Input>,
{
    seq!(Vector2(le_f32, le_f32)).parse_next(input)
}
impl std::fmt::Debug for Vector2 {
    // More compact
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector2({:?}, {:?})", self.0, self.1)
    }
}

#[derive(Clone, Copy)]
pub struct Vector3(pub f32, pub f32, pub f32);
// pub fn vec3(input: &mut Stream) -> Result<Vector3> {
//     seq!(Vector3(f32, f32, f32)).parse_next(input)
// }
pub fn vec3<Input, Error>(input: &mut Input) -> Result<Vector3, Error>
where
    Input: winnow::stream::StreamIsPartial + winnow::stream::Stream<Token = u8>,
    Error: winnow::error::ParserError<Input>,
{
    seq!(Vector3(le_f32, le_f32, le_f32)).parse_next(input)
}
impl TypeReaderMeta for Vector3 {
    const NAME: &'static str = "UNKNOWN";
    const VERSION: i32 = -1;
}
impl std::fmt::Debug for Vector3 {
    // More compact
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector3({:?}, {:?}, {:?})", self.0, self.1, self.2)
    }
}

#[derive(Clone, Copy)]
pub struct Quaternion(pub f32, pub f32, pub f32, pub f32);
pub fn quat<Input, Error>(input: &mut Input) -> Result<Quaternion, Error>
where
    Input: winnow::stream::StreamIsPartial + winnow::stream::Stream<Token = u8>,
    Error: winnow::error::ParserError<Input>,
{
    seq!(Quaternion(le_f32, le_f32, le_f32, le_f32)).parse_next(input)
}
impl std::fmt::Debug for Quaternion {
    // More compact
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Quaternion({:?}, {:?}, {:?}, {:?})",
            self.0, self.1, self.2, self.3
        )
    }
}

#[derive(Clone, Copy)]
pub struct Matrix(
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
    pub f32,
);
// pub fn vec3(input: &mut Stream) -> Result<Vector3> {
//     seq!(Vector3(f32, f32, f32)).parse_next(input)
// }
pub fn matrix<Input, Error>(input: &mut Input) -> Result<Matrix, Error>
where
    Input: winnow::stream::StreamIsPartial + winnow::stream::Stream<Token = u8>,
    Error: winnow::error::ParserError<Input>,
{
    seq!(Matrix(
        // M1*
        le_f32, le_f32, le_f32, le_f32, // M2*
        le_f32, le_f32, le_f32, le_f32, // M3*
        le_f32, le_f32, le_f32, le_f32, // M4*
        le_f32, le_f32, le_f32, le_f32,
    ))
    .parse_next(input)
}
impl std::fmt::Debug for Matrix {
    // More compact
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Matrix({:?}, {:?}, {:?}, {:?}; {:?}, {:?}, {:?}, {:?}; {:?}, {:?}, {:?}, {:?}; {:?}, {:?}, {:?}, {:?})",
            self.0,
            self.1,
            self.2,
            self.3,
            self.4,
            self.5,
            self.6,
            self.7,
            self.8,
            self.9,
            self.10,
            self.11,
            self.12,
            self.13,
            self.14,
            self.15,
        )
    }
}

#[derive(Debug)]
pub struct AnyExternalReference {
    pub path: String,
}
impl TypeReaderMeta for AnyExternalReference {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.ExternalReferenceReader";
    const VERSION: i32 = 0;
}
impl AnyExternalReference {
    pub fn parse(input: &mut Stream) -> Result<Self> {
        super::string
            .map(|path| AnyExternalReference {
                path: path.to_owned(),
            })
            .parse_next(input)
    }
}

#[derive(Debug)]
pub struct ExternalReference<T> {
    // TODO: Reference input
    pub path: String,
    _marker: PhantomData<T>,
}
impl<T> TypeReaderMeta for ExternalReference<T> {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.ExternalReferenceReader";
    const VERSION: i32 = 0;
}
pub fn external_ref<T>(input: &mut Stream) -> Result<ExternalReference<T>> {
    super::string
        .map(|path| ExternalReference {
            path: path.to_owned(),
            _marker: PhantomData,
        })
        .parse_next(input)
}

// type name: System.String
// type reader name: Microsoft.Xna.Framework.Content.StringReader
pub fn string<'a>(input: &mut Stream<'a>) -> winnow::Result<&'a str> {
    length_take(crate::xnb::int_7bitenc.try_map(usize::try_from))
        .try_map(str::from_utf8)
        .parse_next(input)
}

#[derive(Debug)]
pub struct NetString(pub String); // System.String
impl TypeReaderMeta for NetString {
    const NAME: &'static str = "Microsoft.Xna.Framework.Content.StringReader";
    const VERSION: i32 = 0;
}
pub fn string_object<'a>(input: &mut Stream<'a>) -> winnow::Result<NetString> {
    string.map(|s| NetString(s.to_owned())).parse_next(input)
}
