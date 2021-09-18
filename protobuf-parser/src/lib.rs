//! A nom-based protobuf file parser
//!
//! This crate can be seen as a rust transcription of the
//! [descriptor.proto](https://github.com/google/protobuf/blob/master/src/google/protobuf/descriptor.proto) file

#[macro_use]
extern crate nom;
#[macro_use]
extern crate nom_locate;

mod parser;

use nom::types::CompleteStr;
use nom_locate::LocatedSpan;
use std::convert::AsRef;
use std::ops::RangeInclusive;

pub type Span<'a> = LocatedSpan<CompleteStr<'a>>;

#[derive(Debug, PartialEq, Clone)]
pub struct Word<'a> {
    word: Span<'a>,
}

impl<'a> AsRef<str> for Word<'a> {
    fn as_ref(&self) -> &str {
        self.word.fragment.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Integer<'a> {
    position: Span<'a>,
    value: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Syntax {
    /// Protobuf syntax [2](https://developers.google.com/protocol-buffers/docs/proto) (default)
    Proto2,
    /// Protobuf syntax [3](https://developers.google.com/protocol-buffers/docs/proto3)
    Proto3,
}

impl Default for Syntax {
    fn default() -> Syntax {
        Syntax::Proto2
    }
}

#[derive(Debug, Clone)]
pub struct BracketOption<'a> {
    key: Word<'a>,
    // TODO(blt) This being a Span stinks. We should, instead, have a parser for
    // ProtoValue or some such, which can be an integer, string or bool.
    value: Span<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeclOptionName<'a> {
    BuiltIn(Word<'a>),
    Custom(Word<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclOption<'a> {
    name: DeclOptionName<'a>,
    // TODO(blt) This being a Span stinks. We should, instead, have a parser for
    // ProtoValue or some such, which can be an integer, string or bool.
    value: Span<'a>,
}

/// A field rule
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rule<'a> {
    pub position: Option<Span<'a>>,
    pub variant: RuleVariant,
}

impl<'a> Default for Rule<'a> {
    fn default() -> Rule<'a> {
        Rule {
            position: None,
            variant: RuleVariant::Optional,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleVariant {
    /// A well-formed message can have zero or one of this field (but not more than one).
    Optional,
    /// This field can be repeated any number of times (including zero) in a well-formed message.
    /// The order of the repeated values will be preserved.
    Repeated,
    /// A well-formed message must have exactly one of this field.
    Required,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapKVPair<'a> {
    position: Span<'a>,
    key: FieldType<'a>,
    value: FieldType<'a>,
}

/// Protobuf supported field types
///
/// TODO: Groups (even if deprecated)
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType<'a> {
    /// Protobuf int32
    ///
    /// # Remarks
    ///
    /// Uses variable-length encoding. Inefficient for encoding negative numbers – if
    /// your field is likely to have negative values, use sint32 instead.
    Int32,
    /// Protobuf int64
    ///
    /// # Remarks
    ///
    /// Uses variable-length encoding. Inefficient for encoding negative numbers – if
    /// your field is likely to have negative values, use sint64 instead.
    Int64,
    /// Protobuf uint32
    ///
    /// # Remarks
    ///
    /// Uses variable-length encoding.
    Uint32,
    /// Protobuf uint64
    ///
    /// # Remarks
    ///
    /// Uses variable-length encoding.
    Uint64,
    /// Protobuf sint32
    ///
    /// # Remarks
    ///
    /// Uses ZigZag variable-length encoding. Signed int value. These more efficiently
    /// encode negative numbers than regular int32s.
    Sint32,
    /// Protobuf sint64
    ///
    /// # Remarks
    ///
    /// Uses ZigZag variable-length encoding. Signed int value. These more efficiently
    /// encode negative numbers than regular int32s.
    Sint64,
    /// Protobuf bool
    Bool,
    /// Protobuf fixed64
    ///
    /// # Remarks
    ///
    /// Always eight bytes. More efficient than uint64 if values are often greater than 2^56.
    Fixed64,
    /// Protobuf sfixed64
    ///
    /// # Remarks
    ///
    /// Always eight bytes.
    Sfixed64,
    /// Protobuf double
    Double,
    /// Protobuf string
    ///
    /// # Remarks
    ///
    /// A string must always contain UTF-8 encoded or 7-bit ASCII text.
    String,
    /// Protobuf bytes
    ///
    /// # Remarks
    ///
    /// May contain any arbitrary sequence of bytes.
    Bytes,
    /// Protobut fixed32
    ///
    /// # Remarks
    ///
    /// Always four bytes. More efficient than uint32 if values are often greater than 2^28.
    Fixed32,
    /// Protobut sfixed32
    ///
    /// # Remarks
    ///
    /// Always four bytes.
    Sfixed32,
    /// Protobut float
    Float,
    /// Protobuf message or enum (holds the name)
    MessageOrEnum(Word<'a>),
    /// Protobut map
    Map(Box<MapKVPair<'a>>),
    /// Protobuf group (deprecated)
    Group(Vec<Field<'a>>),
}

/// A Protobuf Field
#[derive(Debug, Clone, PartialEq)]
pub struct Field<'a> {
    /// Field name
    pub name: Word<'a>,
    /// Field `Rule`
    pub rule: Rule<'a>,
    /// Field type
    pub typ: FieldType<'a>,
    /// Tag number
    pub number: Integer<'a>,
    /// Default value for the field
    pub default: Option<Word<'a>>,
    /// Packed property for repeated fields
    pub packed: Option<bool>,
    /// Is the field deprecated
    pub deprecated: bool,
}

/// A protobuf message
#[derive(Debug, Clone, Default)]
pub struct Message<'a> {
    /// Message name
    pub name: Option<Word<'a>>,
    /// Message `Field`s
    pub fields: Vec<Field<'a>>,
    /// Message `OneOf`s
    pub oneofs: Vec<OneOf<'a>>,
    /// Message reserved numbers
    pub reserved_nums: Vec<RangeInclusive<i32>>,
    /// Message reserved names
    pub reserved_names: Vec<Word<'a>>,
    /// Nested messages
    pub messages: Vec<Message<'a>>,
    /// Nested enums
    pub enums: Vec<Enumeration<'a>>,
}

/// A protobuf enumeration field
#[derive(Debug, Clone)]
pub struct EnumValue<'a> {
    /// enum value name
    pub name: Word<'a>,
    /// enum value number
    pub number: Integer<'a>,
}

/// A protobuf enumerator
#[derive(Debug, Clone)]
pub struct Enumeration<'a> {
    /// enum name
    pub name: Word<'a>,
    /// enum values
    pub values: Vec<EnumValue<'a>>,
}

/// A OneOf
#[derive(Debug, Clone)]
pub struct OneOf<'a> {
    position: Span<'a>,
    /// OneOf name
    pub name: Word<'a>,
    /// OneOf fields
    pub fields: Vec<Field<'a>>,
}

#[derive(Debug, Clone)]
pub struct Extension<'a> {
    /// Extend this type with field
    pub extendee: Word<'a>,
    /// Extension field
    pub field: Field<'a>,
}

// NOTE(blt): It's possible that an invalid proto file will still parse into an
// AbstractProto. The careful user will perform validation.
#[derive(Debug, Default, Clone)]
pub struct AbstractProto<'a> {
    /// Imports
    pub import_paths: Vec<Word<'a>>,
    /// Package
    pub package: Option<Word<'a>>,
    /// Protobuf Syntax
    pub syntax: Syntax,
    /// Top level messages
    pub messages: Vec<Message<'a>>,
    /// Top level options
    pub options: Vec<DeclOption<'a>>,
    /// Enums
    pub enums: Vec<Enumeration<'a>>,
    /// Extensions
    pub extensions: Vec<Extension<'a>>,
}

pub fn parse(proto_txt: &'_ str) -> Result<(Span<'_>, AbstractProto<'_>), ::nom::Err<Span<'_>>> {
    parser::parse(LocatedSpan::new(CompleteStr(proto_txt)))
}
