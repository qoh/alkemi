// This module hierarchy is backwards

pub mod types;

use std::marker::PhantomData;

use lzxd::{Lzxd, WindowSize};
use winnow::{
    Bytes, LocatingSlice, Parser, Result, Stateful,
    binary::{be_u16, le_u32, length_repeat, length_take, u8},
    combinator::{repeat, seq, todo},
    error::{
        AddContext, ContextError, ErrMode, FromExternalError, ParseError, ParserError, StrContext,
        StrContextValue,
    },
    stream::Stream as _,
    token::{any, rest, take},
};

use crate::xnb::types::{i32, string};

#[allow(unused)]
pub(crate) fn parse<'i, O, P: for<'d> Parser<Stream<'d>, O, ContextError>>(
    bytes: &'i [u8],
    primary: P,
) -> Result<XnbAsset<O>, ParseError<Stream<'i>, ContextError>> {
    let stream = Stream {
        // input: Bytes::new(bytes),
        input: LocatingSlice::new(Bytes::new(bytes)),
        state: State {
            type_readers: Vec::new(),
        },
    };
    parse_xnb(primary).parse(stream)
}

// #[allow(unused)]
// pub(crate) fn parse(
//     bytes: &'_ [u8],
// ) -> Result<Option<crate::xnb_readers::magicka_content::Level>, ParseError<Stream<'_>, ContextError>>
// {
//     let stream = Stream {
//         // input: Bytes::new(bytes),
//         input: LocatingSlice::new(Bytes::new(bytes)),
//         state: State {
//             type_readers: Vec::new(),
//         },
//     };
//     parse_xnb(object_any).parse(stream)
// }

#[derive(Debug)]
struct Header {
    // https://github.com/MonoGame/MonoGame/blob/b5ead4c88dd114354f0d433fcb0ce635e7a05212/MonoGame.Framework/Content/ContentManager.cs#L37
    platform: u8,

    // version: u8,
    flags: u8,
    /// This should match the size of the whole file, including the header
    file_size: u32,
}

const HEADER_FLAG_HIDEF_PROFILE: u8 = 1 << 0;
const HEADER_FLAG_COMPRESSED_LZ4: u8 = 1 << 7;
const HEADER_FLAG_COMPRESSED_LZX: u8 = 1 << 7;

enum HeaderFlags {
    Compressed = 0x80,
}

// pub type Stream<'i> = &'i Bytes;
// pub type Stream<'i> = Stateful<&'i Bytes, State<'i>>;
pub type Stream<'i> = Stateful<LocatingSlice<&'i Bytes>, State<'i>>;

#[derive(Debug)]
pub struct State<'i> {
    pub(super) type_readers: Vec<TypeReaderEntry<'i>>,
}

pub(super) struct TypeReaderEntry<'i> {
    pub(super) info: TypeReaderInfo<'i>,
    pub(super) reader: Option<AnyReader>,
}

impl<'i> std::fmt::Debug for TypeReaderEntry<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeReaderEntry")
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

// It's okay to parse the input directly without access to the state, as we don't need it changed.
// impl<'i> std::ops::DerefMut for ContentStream<'i> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.input
//     }
// }

#[derive(Debug)]
pub enum SharedResourceAccessError {
    WrongType,
    OutOfRangeOrUnparsed,
}

pub trait SharedResources {
    fn shared_resource<R: 'static + TypeReaderMeta>(
        &self,
        reference: &SharedResourceReference<R>,
    ) -> Result<Option<&R>, SharedResourceAccessError>;

    fn shared_resource_any<R: 'static>(
        &self,
        reference: &SharedResourceReference<R>,
    ) -> Result<Option<&Box<dyn std::any::Any>>, SharedResourceAccessError>;
}

pub struct XnbAsset<T> {
    primary: T,
    shared_resources: Vec<Option<AnyObject>>,
}

impl<T> XnbAsset<T> {
    pub fn inner(&self) -> &T {
        &self.primary
    }

    pub fn into_inner(self) -> T
    where
        Self: Sized,
    {
        self.primary
    }
}

impl<T> SharedResources for XnbAsset<T> {
    fn shared_resource<R: 'static + TypeReaderMeta>(
        &self,
        reference: &SharedResourceReference<R>,
    ) -> Result<Option<&R>, SharedResourceAccessError> {
        self.shared_resources
            .get(reference.index)
            .map(|o| o.as_ref())
            .ok_or(SharedResourceAccessError::OutOfRangeOrUnparsed)?
            .map(|b| {
                b.downcast_ref::<R>()
                    // TODO: Error details could include what type it is
                    .ok_or(SharedResourceAccessError::WrongType)
            })
            .transpose()
    }

    fn shared_resource_any<R: 'static>(
        &self,
        reference: &SharedResourceReference<R>,
    ) -> Result<Option<&Box<dyn std::any::Any>>, SharedResourceAccessError> {
        self.shared_resources
            .get(reference.index)
            .map(|o| o.as_ref())
            .ok_or(SharedResourceAccessError::OutOfRangeOrUnparsed)
    }
}

impl<T> AsRef<T> for XnbAsset<T> {
    fn as_ref(&self) -> &T {
        self.inner()
    }
}

#[derive(Default)]
pub struct EmptySharedResources;

impl SharedResources for EmptySharedResources {
    fn shared_resource<R: 'static + TypeReaderMeta>(
        &self,
        _reference: &SharedResourceReference<R>,
    ) -> Result<Option<&R>, SharedResourceAccessError> {
        // TODO: this could use a more specific error, like 'not included'
        Err(SharedResourceAccessError::OutOfRangeOrUnparsed)
    }

    fn shared_resource_any<R: 'static>(
        &self,
        _reference: &SharedResourceReference<R>,
    ) -> Result<Option<&Box<dyn std::any::Any>>, SharedResourceAccessError> {
        // TODO: this could use a more specific error, like 'not included'
        Err(SharedResourceAccessError::OutOfRangeOrUnparsed)
    }
}

fn parse_xnb<'i, 'p, O, Primary>(
    mut primary: Primary,
) -> impl Parser<Stream<'i>, XnbAsset<O>, ContextError> + 'p
where
    Primary: for<'d> Parser<Stream<'d>, O, ContextError> + 'p,
{
    move |input: &mut Stream<'i>| -> winnow::Result<XnbAsset<O>> {
        let header = seq!(Header {
            _: b"XNB",
            platform: u8,
            _: 4, // Version
            flags: u8,
            file_size: le_u32,
        })
        .parse_next(input)?;
        if header.platform != 119 {
            eprintln!("unhandled .xnb platform: {}", header.platform);
        }
        if (header.flags & !HEADER_FLAG_COMPRESSED_LZX) != 0 {
            eprintln!("unhandled .xnb flags: {}", header.flags);
        }

        let data = if header.flags & HEADER_FLAG_COMPRESSED_LZX != 0 {
            compressed_lxz.parse_next(input)?
        } else if header.flags & HEADER_FLAG_COMPRESSED_LZ4 != 0 {
            todo!("not lzx")
        } else {
            todo!("hmm")
        };

        let mut input = Stream {
            input: LocatingSlice::new(Bytes::new(&data)),
            state: State {
                type_readers: vec![],
            },
        };
        let input = &mut input;
        {
            let type_reader_infos: Vec<_> =
                length_repeat(int_7bitenc.try_map(usize::try_from), type_reader_info)
                    .parse_next(input)?;
            let shared_resources_len = int_7bitenc.try_map(usize::try_from).parse_next(input)?;
            let type_readers: Vec<_> = type_reader_infos
                .into_iter()
                .map(|info| TypeReaderEntry {
                    reader: resolve_type_reader(&info),
                    info,
                })
                .collect();
            // let mut content_stream = Stream {
            //     input,
            //     state: State { type_readers },
            // };
            input.state.type_readers = type_readers; // XXX: it would be nicer if this was a separate stream type

            let primary_value = primary.parse_next(input)?;

            // Read shared resources
            let mut shared_resources = Vec::with_capacity(shared_resources_len);
            for i in 0..shared_resources_len {
                let shared_resource = match object_any.parse_next(input) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!(
                            "error: reading shared resource index {i} failed, unable to read last {skip} of {shared_resources_len} shared resources: {e}",
                            skip = shared_resources_len - i,
                        );
                        break;
                    }
                };
                shared_resources.push(shared_resource);
            }

            // Consume the rest of the input
            let unused = input.finish();
            if !unused.is_empty() {
                eprintln!("warn: {} data bytes unused", unused.len());
            }

            // Ok(primary_value)
            Ok(XnbAsset {
                primary: primary_value,
                shared_resources,
            })
        }
    }
}

fn resolve_type_reader(info: &TypeReaderInfo) -> Option<AnyReader> {
    match *info {
        TypeReaderInfo {
            name: crate::xnb_readers::magicka_content::Level::NAME,
            version: crate::xnb_readers::magicka_content::Level::VERSION,
        } => Some(Box::new(
            || -> Box<dyn for<'g> TypeReaderParser<'g, AnyObject>> {
                Box::new(BoxingParser::new(
                    crate::xnb_readers::magicka_content::level_model,
                ))
            },
        )),
        TypeReaderInfo {
            name: crate::xnb_readers::magicka_effect::DeferredEffect::NAME,
            version: crate::xnb_readers::magicka_effect::DeferredEffect::VERSION,
        } => Some(Box::new(
            || -> Box<dyn for<'g> TypeReaderParser<'g, AnyObject>> {
                Box::new(BoxingParser::new(
                    crate::xnb_readers::magicka_effect::deferred_effect,
                ))
            },
        )),
        TypeReaderInfo {
            name: crate::xnb_readers::skinning::SkinnedModelBasicEffect::NAME,
            version: crate::xnb_readers::skinning::SkinnedModelBasicEffect::VERSION,
        } => Some(Box::new(
            || -> Box<dyn for<'g> TypeReaderParser<'g, AnyObject>> {
                Box::new(BoxingParser::new(
                    crate::xnb_readers::skinning::SkinnedModelBasicEffect::parse,
                ))
            },
        )),
        TypeReaderInfo {
            name: crate::xnb_readers::skinning::SkinnedModelBone::NAME,
            version: crate::xnb_readers::skinning::SkinnedModelBone::VERSION,
        } => Some(Box::new(
            || -> Box<dyn for<'g> TypeReaderParser<'g, AnyObject>> {
                Box::new(BoxingParser::new(
                    crate::xnb_readers::skinning::SkinnedModelBone::parse,
                ))
            },
        )),
        TypeReaderInfo {
            name: crate::xnb_readers::skinning::AnimationClip::NAME,
            version: crate::xnb_readers::skinning::AnimationClip::VERSION,
        } => Some(Box::new(
            || -> Box<dyn for<'g> TypeReaderParser<'g, AnyObject>> {
                Box::new(BoxingParser::new(
                    crate::xnb_readers::skinning::AnimationClip::parse,
                ))
            },
        )),
        /* TypeReaderInfo {
            name: types::AnyExternalReference::NAME,
            version: types::AnyExternalReference::VERSION,
        } => Some(Box::new(
            || -> Box<dyn for<'g> TypeReaderParser<'g, AnyObject>> {
                Box::new(BoxingParser::new(types::AnyExternalReference::parse))
            },
        )), */
        _ => None,
    }
}

// Because P could implement Parser to multiple output types O: Any,
// this type has to select which one to prevent conflicting impls of Parser to Box<Any> for Self
struct BoxingParser<P, O> {
    parser: P,
    _marker: PhantomData<O>,
}

impl<P, O> BoxingParser<P, O> {
    pub fn new(parser: P) -> Self {
        Self {
            parser,
            _marker: PhantomData,
        }
    }
}

impl<'i, O: std::any::Any, P: Parser<Stream<'i>, O, ContextError>>
    Parser<Stream<'i>, AnyObject, ContextError> for BoxingParser<P, O>
where
// P: ,
// O: std::any::Any,
{
    fn parse_next(&mut self, input: &mut Stream<'i>) -> Result<AnyObject, ContextError> {
        let inner = self.parser.parse_next(input)?;
        Ok(Box::new(inner))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeReaderInfo<'a> {
    pub name: &'a str,
    pub version: i32,
}
fn type_reader_info<'a>(input: &mut Stream<'a>) -> winnow::Result<TypeReaderInfo<'a>> {
    seq!(TypeReaderInfo {
        name: string,
        version: i32
    })
    .parse_next(input)
}

pub trait TypeReaderMeta {
    const NAME: &'static str;
    const VERSION: i32;
}

pub trait TypeReaderParser<'i, O>: Parser<Stream<'i>, O, ContextError> {}

impl<'i, P, O> TypeReaderParser<'i, O> for P where P: Parser<Stream<'i>, O, ContextError> {}

pub trait TypeReaderParserMaker<O> {
    fn make(&self) -> Box<dyn for<'i> TypeReaderParser<'i, O>>;
}

impl<F, O> TypeReaderParserMaker<O> for F
where
    F: Fn() -> Box<dyn for<'i> TypeReaderParser<'i, O>>,
{
    fn make(&self) -> Box<dyn for<'i> TypeReaderParser<'i, O>> {
        (self)()
    }
}

pub trait TypeReader<'i, O>: TypeReaderParser<'i, O>
where
    O: TypeReaderMeta,
{
}

impl<'i, O: TypeReaderMeta, T: TypeReaderParser<'i, O>> TypeReader<'i, O> for T {}

type AnyReader = Box<dyn TypeReaderParserMaker<AnyObject>>;

// type AnyObject = crate::xnb_readers::magicka_content::Level;
type AnyObject = Box<dyn std::any::Any>;

#[derive(Debug)]
struct TypeIdOutOfRange {
    type_id: usize,
    type_count: usize,
}
impl std::error::Error for TypeIdOutOfRange {}
impl std::fmt::Display for TypeIdOutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "type reader id {id} out of range, only {num} readers declared. almost certainly means read misalignment",
            id = self.type_id,
            num = self.type_count,
        )
    }
}

pub fn object_any(input: &mut Stream) -> Result<Option<AnyObject>> {
    let type_id = int_7bitenc.try_map(usize::try_from).parse_next(input)?;
    match type_id {
        0 => Ok(None),
        _ => {
            let Some(type_entry) = input.state.type_readers.get(type_id - 1) else {
                return Err(ContextError::from_external_error(
                    input,
                    TypeIdOutOfRange {
                        type_id: type_id - 1,
                        type_count: input.state.type_readers.len(),
                    },
                ));
            };
            let Some(make_reader) = type_entry.reader.as_ref() else {
                // todo!("type reader impl for dynamic dispatch {:#?}", b.info);
                #[derive(Debug)]
                struct NoTypeReaderError {
                    info: (String, i32),
                }
                impl std::error::Error for NoTypeReaderError {}
                impl std::fmt::Display for NoTypeReaderError {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(
                            f,
                            "no type reader implementation for {:?} version {}",
                            &self.info.0, self.info.1,
                        )
                    }
                }
                return Err(ContextError::from_external_error(
                    input,
                    NoTypeReaderError {
                        info: (type_entry.info.name.to_owned(), type_entry.info.version),
                    },
                ));
            };

            let mut reader = make_reader.make();
            Ok(Some(reader.parse_next(input)?))
        }
    }
}

pub fn object<O, P>(
    // HACK: this shouldn't have to be owned
    reader: P,
) -> ObjectParser<O, P> {
    ObjectParser {
        reader,
        _marker: PhantomData,
    }
}
pub struct ObjectParser<Type, TypeReader> {
    reader: TypeReader,
    _marker: PhantomData<Type>,
}
impl<'i, Type, TypeReader> Parser<Stream<'i>, Option<Type>, ContextError>
    for ObjectParser<Type, TypeReader>
where
    Type: TypeReaderMeta,
    TypeReader: Parser<Stream<'i>, Type, ContextError>,
{
    fn parse_next(&mut self, input: &mut Stream<'i>) -> Result<Option<Type>, ContextError> {
        let start = input.checkpoint();
        let type_id = int_7bitenc.try_map(usize::try_from).parse_next(input)?;
        if type_id == 0 {
            return Ok(None);
        }
        let Some(found_reader) = input.state.type_readers.get(type_id - 1).map(|b| &b.info) else {
            return Err(ContextError::from_external_error(
                input,
                TypeIdOutOfRange {
                    type_id,
                    type_count: input.state.type_readers.len(),
                },
            ));
        };
        let expected_reader = TypeReaderInfo {
            name: Type::NAME,
            version: Type::VERSION,
        };
        if *found_reader != expected_reader {
            #[derive(Debug)]
            struct WrongTypeError {
                expected: (String, i32),
                found: (String, i32),
            }
            impl std::error::Error for WrongTypeError {}
            impl std::fmt::Display for WrongTypeError {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(
                        f,
                        "wrong type found when reading specific polymorphic object.\nfound {:?} (version {})\nexpected {:?} (version {})",
                        &self.found.0, self.found.1, &self.expected.0, self.expected.1
                    )
                }
            }
            return Err(ContextError::from_external_error(
                input,
                WrongTypeError {
                    expected: (expected_reader.name.to_owned(), expected_reader.version),
                    found: (found_reader.name.to_owned(), found_reader.version),
                },
            ));
        }
        Ok(Some(self.reader.parse_next(input)?))
    }
}

fn int_7bitenc(input: &mut Stream) -> winnow::Result<i32> {
    let mut result: i32 = 0;
    let mut bits = 0;
    loop {
        let value = u8.parse_next(input)? as i32;
        result |= (value & 0x7f) << bits;
        bits += 7;
        if value & 0x80 == 0 {
            break Ok(result);
        }
    }
}

#[derive(Debug)]
pub struct SharedResourceReference<T> {
    index: usize,
    _marker: PhantomData<T>,
}

impl<T> PartialEq for SharedResourceReference<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for SharedResourceReference<T> {}

pub fn shared_resource_ref<T>(
    input: &mut Stream,
) -> winnow::Result<Option<SharedResourceReference<T>>> {
    let num = int_7bitenc.try_map(usize::try_from).parse_next(input)?;
    if num == 0 {
        return Ok(None);
    }
    let index = num - 1;
    // TODO: check that shared resource index {index} is within bounds
    Ok(Some(SharedResourceReference {
        index,
        _marker: PhantomData,
    }))
}

fn compressed_lxz(input: &mut Stream) -> winnow::Result<Box<[u8]>> {
    let decompressed_data_size = le_u32.parse_next(input)? as usize;
    if decompressed_data_size > 1024 * 1024 * 1024 {
        // Don't handle data > 1 GB for now
        todo!();
    }

    let mut decompressed = vec![0; decompressed_data_size].into_boxed_slice();
    let mut remaining_decompressed = decompressed.as_mut();

    let mut lzxd = Lzxd::new(WindowSize::KB64);
    while !input.is_empty() {
        let mut hi = u8.parse_next(input)? as u16;
        let mut lo = u8.parse_next(input)? as u16;
        let mut block_size = (hi << 8) | lo; // We should just parse be_u16
        let mut frame_size = 0x8000; // frame size is 32Kb by default
        // does this block define a frame size?
        if hi == 0xFF {
            hi = lo;
            lo = u8.parse_next(input)? as u16;
            frame_size = (hi << 8) | lo;
            hi = u8.parse_next(input)? as u16;
            lo = u8.parse_next(input)? as u16;
            block_size = (hi << 8) | lo;
        }
        if block_size == 0 || frame_size == 0 {
            let unused: &[u8] = rest.parse_next(input)?;
            if !unused.is_empty() {
                // could debug!() log
                // eprintln!("warn: {} compressed bytes unused", unused.len());
            }
            break;
        }
        // TODO: Use something more winnow native than split_at
        let block = take(block_size as usize).parse_next(input)?;
        // let (block, rest) = input.split_at(block_size as usize);
        // *input = rest;
        let decompressed_block = lzxd
            .decompress_next(block, frame_size as usize)
            .map_err(|e| todo!())?;

        let (mut write_into, rest_remaining) = remaining_decompressed
            .split_at_mut_checked(decompressed_block.len())
            .ok_or_else(|| todo!())?;
        remaining_decompressed = rest_remaining;
        write_into.copy_from_slice(decompressed_block);
    }
    Ok(decompressed)
}

// Used very commonly
pub(crate) fn quicklist<'i, 'p, O, P: Parser<Stream<'i>, O, ContextError> + 'p>(
    parser: P,
) -> impl Parser<Stream<'i>, Vec<O>, ContextError> {
    length_repeat(i32.try_map(usize::try_from), parser)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_parse() {
        let bytes = std::fs::read(
            "/data/SteamLibrary/steamapps/common/Magicka/Content/Levels/WizardCastle/wc_s4.xnb",
        )
        .unwrap();
        match super::parse(&bytes, super::object_any) {
            Ok(_) => {}
            Err(e) => {
                panic!("parsing failed at {}\n{}", e.offset(), e.inner());
            }
        }
    }
}
