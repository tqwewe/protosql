use super::*;
use nom;
use std::ops::RangeInclusive;
use std::str;

fn is_word(c: char) -> bool {
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.')
}

named!(word(Span) -> Word,  do_parse!(
        word: take_while!(is_word) >>
        (Word {
            word,
        })
));

named!(hex_integer(Span) -> Integer, do_parse!(
    position: position!()
        >> tag!("0x")
        >> num: map_res!(nom::hex_digit, |s: Span| {
            i32::from_str_radix(s.fragment.as_ref(), 16)
        })
        >> (Integer {
            position,
            value: num
        })
));

named!(integer(Span) -> Integer, do_parse!(
    position: position!()
        >> num: map_res!(nom::digit, |s: Span| {
            str::FromStr::from_str(s.fragment.as_ref())
        })
        >> (Integer {
            position,
            value: num
        })
));

named!(comment(Span) -> (), do_parse!(
    tag!("//")
        >> take_until_and_consume!("\n")
        >> ()
));

named!(block_comment(Span) -> (), do_parse!(
    tag!("/*")
        >> take_until_and_consume!("*/")
        >> ()
));

// word break: multispace or comment
named!(br(Span) -> (), do_parse!(
    alt!(map!(nom::multispace, |_| ())
         | comment
         | block_comment )
        >> ()
));

named!(syntax(Span) -> Syntax, do_parse!(
    tag!("syntax")
        >> many0!(br)
        >> tag!("=")
        >> many0!(br)
        >> proto: alt!(tag!("\"proto2\"") => { |_| Syntax::Proto2 } |
                       tag!("\"proto3\"") => { |_| Syntax::Proto3 })
        >> many0!(br)
        >> tag!(";")
        >> (proto)
    )
);

named!(import(Span) -> Word, do_parse!(
    tag!("import")
        >> many1!(br)
        >> tag!("\"")
        >> path: take_until!("\"")
        >> tag!("\"")
        >> many0!(br)
        >> tag!(";")
        >> (Word { word: path })
));

named!(package(Span) -> Word, do_parse!(
    tag!("package")
        >> many1!(br)
        >> package: word
        >> many0!(br)
        >> tag!(";")
        >> (package)
));

named!(num_range(Span) -> RangeInclusive<i32>, do_parse!(
    from_: integer
        >> many1!(br)
        >> tag!("to")
        >> many1!(br)
        >> to_: integer
        >> (from_.value..=to_.value)
));

named!(reserved_nums(Span) -> Vec<RangeInclusive<i32>>, do_parse!(
    tag!("reserved")
        >> many1!(br)
        >> nums: separated_list!(
            do_parse!(many0!(br)
                      >> tag!(",")
                      >> many0!(br)
                      >> (())
            ),
            alt!(num_range
                 | integer => { |i: Integer| i.value..=i.value })
        )
        >> many0!(br)
        >> tag!(";")
        >> (nums)
));

named!(reserved_names(Span) -> Vec<Word>, do_parse!(
    tag!("reserved")
        >> many1!(br)
        >> names: many1!(do_parse!(
            tag!("\"")
                >> name: word
                >> tag!("\"")
                >> many0!(alt!(br | tag!(",") => { |_| () }))
                >> (name)
        ))
        >> many0!(br)
        >> tag!(";")
        >> (names)
));

// formerly key_val
named!(bracket_option(Span) -> BracketOption, do_parse!(
    tag!("[")
        >> many0!(br)
        >> key: word
        >> many0!(br)
        >> tag!("=")
        >> many0!(br)
        >> value: is_not!("]")
        >> tag!("]")
        >> many0!(br)
        >> (BracketOption {
            key,
            value
        })
));

named!(rule(Span) -> Rule, do_parse!(
    position: position!()
        >> variant: alt!(tag!("optional") => { |_| RuleVariant::Optional } |
                         tag!("repeated") => { |_| RuleVariant::Repeated } |
                         tag!("required") => { |_| RuleVariant::Required } )
        >> (Rule {
            position: Some(position),
            variant
        })
));

named!(map_field(Span) -> MapKVPair, do_parse!(
    tag!("map")
        >> position: position!()
        >> many0!(br)
        >> tag!("<")
        >> many0!(br)
        >> key: field_type
        >> many0!(br)
        >> tag!(",")
        >> many0!(br)
        >> value: field_type
        >> tag!(">")
        >> (MapKVPair {
            position,
            key,
            value
        })
));

named!(field_type(Span) -> FieldType, do_parse!(
    ftype: alt!(
        tag!("int32") => { |_| FieldType::Int32 }
        | tag!("int64") => { |_| FieldType::Int64 }
        | tag!("uint32") => { |_| FieldType::Uint32 }
        | tag!("uint64") => { |_| FieldType::Uint64 }
        | tag!("sint32") => { |_| FieldType::Sint32 }
        | tag!("sint64") => { |_| FieldType::Sint64 }
        | tag!("fixed32") => { |_| FieldType::Fixed32 }
        | tag!("sfixed32") => { |_| FieldType::Sfixed32 }
        | tag!("fixed64") => { |_| FieldType::Fixed64 }
        | tag!("sfixed64") => { |_| FieldType::Sfixed64 }
        | tag!("bool") => { |_| FieldType::Bool }
        | tag!("string") => { |_| FieldType::String }
        | tag!("bytes") => { |_| FieldType::Bytes }
        | tag!("float") => { |_| FieldType::Float }
        | tag!("double") => { |_| FieldType::Double }
        | tag!("group") => { |_| FieldType::Group(Vec::new()) }
        | map_field => { |kv| FieldType::Map(Box::new(kv)) }
        | word => { |w| FieldType::MessageOrEnum(w) }
    )
        >> (ftype)
));

named!(fields_in_braces(Span) -> Vec<Field>, do_parse!(
    tag!("{")
        >> many0!(br)
        >> fields: separated_list!(br, message_field)
        >> many0!(br)
        >> tag!("}")
        >> (fields)
));

named!(one_of(Span) -> OneOf, do_parse!(
    tag!("oneof")
        >> position: position!()
        >> many1!(br)
        >> name: word
        >> many0!(br)
        >> fields: fields_in_braces
        >> many0!(br)
        >> (OneOf {
            position,
            name,
            fields,
        })
));

named!(group_fields_or_semicolon(Span) -> Option<Vec<Field>>, do_parse!(
    res: alt!(
        tag!(";") => { |_| None }
        | fields_in_braces => { Some }
    )
        >> (res)
));

// TODO(blt) This must be extended to support custom options. These are normal
// fields but with a slightly different syntax, like:
//
//    option (my_option) = "Hello world!";

named!(message_field(Span) -> Field, do_parse!(
    rule: opt!(rule)
        >> many0!(br)
        >> typ: field_type
        >> many1!(br)
        >> name: word
        >> many0!(br)
        >> tag!("=")
        >> many0!(br)
        >> number: integer
        >> many0!(br)
        >> bracket_options: many0!(bracket_option)
        >> many0!(br)
        >> group_fields: group_fields_or_semicolon
        >> ({
            let typ = match (typ, group_fields) {
                (FieldType::Group(..), Some(group_fields)) => FieldType::Group(group_fields),
                // TODO: produce error if semicolon is after group or group is without fields
                (typ, _) => typ,
            };

            Field {
                name,
                rule: rule.unwrap_or_default(),
                typ,
                number,
                default: bracket_options
                    .iter()
                    .find(|opt| opt.key.as_ref() == "default")
                    .map(|opt| Word { word: opt.value }),
                packed: bracket_options
                    .iter()
                    .find(|opt| opt.key.as_ref() == "packed")
                    .map(|opt| {
                        // TODO(blt): we should actually extend the parser to be
                        // able to parse a boolean at parse time, rather than
                        // crash deep here
                        str::FromStr::from_str(opt.value.fragment.as_ref()).expect("Cannot parse Packed value")
                    }),
                deprecated: bracket_options
                    .iter()
                    .find(|opt| opt.key.as_ref() == "deprecated")
                    .map_or(false, |opt| {
                        str::FromStr::from_str(opt.value.fragment.as_ref()).expect("Cannot parse Deprecated value")
                    }),
            }
        })
));

enum MessageEvent<'a> {
    Message(Message<'a>),
    Enumeration(Enumeration<'a>),
    Field(Field<'a>),
    ReservedNums(Vec<RangeInclusive<i32>>),
    ReservedNames(Vec<Word<'a>>),
    OneOf(OneOf<'a>),
    Ignore,
}

named!(message_event(Span) -> MessageEvent, do_parse!(
    res: alt!(reserved_nums => { |r| MessageEvent::ReservedNums(r) }
              | reserved_names => { |r| MessageEvent::ReservedNames(r) }
              | message_field => { |f| MessageEvent::Field(f) }
              | message => { |m| MessageEvent::Message(m) }
              | enumerator => { |e| MessageEvent::Enumeration(e) }
              | one_of => { |o| MessageEvent::OneOf(o) }
              | br => { |_| MessageEvent::Ignore })
        >> (res)
));

named!(message_events(Span) -> (Word, Vec<MessageEvent>), do_parse!(
    tag!("message")
        >> many1!(br)
        >> name: word
        >> many0!(br)
        >> tag!("{")
        >> many0!(br)
        >> events: many0!(message_event)
        >> many0!(br)
        >> tag!("}")
        >> many0!(br)
        >> many0!(tag!(";"))
        >> ((name, events))
));

named!(message(Span) -> Message, do_parse!(
    res: map!(
        message_events,
        |(name, events): (Word, Vec<MessageEvent>)| {
            let mut msg = Message {
                name: Some(name),
                ..Message::default()
            };
            for e in events {
                match e {
                    MessageEvent::Field(f) => msg.fields.push(f),
                    MessageEvent::ReservedNums(r) => msg.reserved_nums = r,
                    MessageEvent::ReservedNames(r) => msg.reserved_names = r,
                    MessageEvent::Message(m) => msg.messages.push(m),
                    MessageEvent::Enumeration(e) => msg.enums.push(e),
                    MessageEvent::OneOf(o) => msg.oneofs.push(o),
                    MessageEvent::Ignore => (),
                }
            }
            msg
        }
    )
        >> (res)
));

named!(extensions(Span) -> Vec<Extension>, do_parse!(
    tag!("extend")
        >> many1!(br)
        >> extendee: word
        >> many0!(br)
        >> fields: fields_in_braces
        >> (fields
            .into_iter()
            .map(|field| Extension {
                extendee: extendee.clone(),
                field
            }).collect())
));

named!(enum_value(Span) -> EnumValue, do_parse!(
    name: word
        >> many0!(br)
        >> tag!("=")
        >> many0!(br)
        >> number: alt!(hex_integer | integer)
        >> many0!(br)
        >> tag!(";")
        >> many0!(br)
        >> (EnumValue {
            name,
            number,
        })
));

named!(enumerator(Span) -> Enumeration, do_parse!(
    tag!("enum")
        >> many1!(br)
        >> name: word
        >> many0!(br)
        >> tag!("{")
        >> many0!(br)
        >> values: many0!(enum_value)
        >> many0!(br)
        >> tag!("}")
        >> many0!(br)
        >> many0!(tag!(";"))
        >> (Enumeration {
            name,
            values,
        })
));

named!(decl_option_custom_name(Span) -> DeclOptionName, do_parse!(
    tag!("(")
        >> many0!(br)
        >> name: word
        >> many0!(br)
        >> tag!(")")
        >> (DeclOptionName::Custom(name))
));

named!(decl_option_builtin_name(Span) -> DeclOptionName, do_parse!(
    many0!(br)
        >> name: word
        >> many0!(br)
        >> (DeclOptionName::BuiltIn(name))
));

named!(option(Span) -> DeclOption, do_parse!(
    tag!("option")
        >> many1!(br)
        >> name: alt!(decl_option_custom_name | decl_option_builtin_name)
        >> many0!(br)
        >> tag!("=")
        >> many0!(br)
        >> value: take_until!(";")
        >> many0!(br)
        >> many0!(tag!(";"))
        >> (DeclOption {
            name,
            value
        })
));

named!(service_ignore(Span) -> (), do_parse!(
    tag!("service")
        >> many1!(br)
        >> word
        >> many0!(br)
        >> tag!("{")
        >> take_until_and_consume!("}")
        >> ()
));

#[derive(Debug, Clone)]
pub enum Event<'a> {
    Syntax(Syntax),
    Import(Word<'a>),
    Package(Word<'a>),
    Message(Message<'a>),
    Enum(Enumeration<'a>),
    DeclOption(DeclOption<'a>),
    Extensions(Vec<Extension<'a>>),
    Ignore,
}

named!(event(Span) -> Event, do_parse!(
    res: alt!(
        syntax => { |s| Event::Syntax(s) }
        | import => { |i| Event::Import(i) }
        | package => { |p| Event::Package(p) }
        | message => { |m| Event::Message(m) }
        | enumerator => { |e| Event::Enum(e) }
        | extensions => { |e| Event::Extensions(e) }
        | option => { |o| Event::DeclOption(o) }
        | service_ignore => { |_| Event::Ignore }
        | br => { |_| Event::Ignore })
        >> (res)
));

named!(pub parse(Span) -> AbstractProto, do_parse!(
    res: map!(many0!(event), |events: Vec<Event>| {
        let mut desc = AbstractProto::default();
        for event in events {
            // TODO(blt) provide some validation here. For instance, we can
            // confirm that the package isn't set multiple times.
            match event {
                Event::Syntax(s) => desc.syntax = s,
                Event::Import(i) => desc.import_paths.push(i),
                Event::Package(p) => desc.package = Some(p),
                Event::Message(m) => desc.messages.push(m),
                Event::Enum(e) => desc.enums.push(e),
                Event::Extensions(e) => desc.extensions.extend(e),
                Event::DeclOption(d) => desc.options.push(d),
                Event::Ignore => (),
            }
        }
        desc
    })
        >> (res)
));

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_word_parse() {
        let input = Span::new(CompleteStr("Scenar.io_Inf12o"));
        let output: Result<(Span, Word), _> = word(input);
        assert!(output.is_ok());
        let (remainder, wrd) = output.unwrap();
        assert_eq!(
            wrd,
            Word {
                word: LocatedSpan {
                    offset: 0,
                    line: 1,
                    fragment: CompleteStr("Scenar.io_Inf12o")
                }
            }
        );
        assert_eq!(
            remainder,
            LocatedSpan {
                offset: 16,
                line: 1,
                fragment: CompleteStr("")
            }
        );
    }

    #[test]
    fn test_hex_integer_parse() {
        let input = Span::new(CompleteStr("0x1AEF"));
        let output: Result<(Span, Integer), _> = hex_integer(input);
        assert!(output.is_ok());
        let (remainder, wrd) = output.unwrap();
        assert_eq!(
            wrd,
            Integer {
                position: LocatedSpan {
                    offset: 0,
                    line: 1,
                    fragment: CompleteStr("")
                },
                value: 6895,
            }
        );
        assert_eq!(
            remainder,
            LocatedSpan {
                offset: 6,
                line: 1,
                fragment: CompleteStr("")
            }
        );
    }

    #[test]
    fn test_integer_parse() {
        let input = Span::new(CompleteStr("123456789"));
        let output: Result<(Span, Integer), _> = integer(input);
        assert!(output.is_ok());
        let (remainder, wrd) = output.unwrap();
        assert_eq!(
            wrd,
            Integer {
                position: LocatedSpan {
                    offset: 0,
                    line: 1,
                    fragment: CompleteStr("")
                },
                value: 123456789,
            }
        );
        assert_eq!(
            remainder,
            LocatedSpan {
                offset: 9,
                line: 1,
                fragment: CompleteStr("")
            }
        );
    }

    #[test]
    fn test_message() {
        let input = Span::new(CompleteStr(
            r#"message Person {
  string name = 1;
    int32 id = 2;  // Unique ID number for this person.
      string email = 3;

  enum PhoneType {
      /* a single line block comment */
      MOBILE = 0;
          HOME = 1;
              // a line comment
              WORK = 2;
                }

  message PhoneNumber {
      /*
       * oh dang, check this out all the comments
       */
      string number = 1;
          PhoneType type = 2;
            }

  repeated PhoneNumber phones = 4;

  google.protobuf.Timestamp last_updated = 5;
    }"#,
        ));
        let output: Result<(Span, Message), _> = message(input);
        assert!(output.is_ok());
        let (remainder, _msg) = output.unwrap();
        assert_eq!(
            remainder,
            LocatedSpan {
                offset: 539,
                line: 25,
                fragment: CompleteStr("")
            }
        );
    }

    // #[test]
    // fn test_enum() {
    //     let msg = r#"enum PairingStatus {
    //             DEALPAIRED        = 0;
    //             INVENTORYORPHAN   = 1;
    //             CALCULATEDORPHAN  = 2;
    //             CANCELED          = 3;
    // }"#;

    //     let enumeration = enumerator(msg.as_bytes());
    //     assert!(enumeration.is_ok());
    //     if let Ok((_, mess)) = enumeration {
    //         assert_eq!(4, mess.values.len());
    //     }
    // }

    #[test]
    fn test_builtin_option() {
        let input = Span::new(CompleteStr(r#"option optimize_for = SPEED;"#));
        let output: Result<(Span, DeclOption), _> = option(input);

        println!("OUTPUT: {:?}", output);
        assert!(output.is_ok());
        let (remainder, msg) = output.unwrap();
        println!("MSG: {:?}", msg);
        assert_eq!(
            msg,
            DeclOption {
                name: DeclOptionName::BuiltIn(Word {
                    word: LocatedSpan {
                        offset: 7,
                        line: 1,
                        fragment: CompleteStr("optimize_for")
                    }
                }),
                value: LocatedSpan {
                    offset: 22,
                    line: 1,
                    fragment: CompleteStr("SPEED")
                }
            }
        );
        assert_eq!(
            remainder,
            LocatedSpan {
                offset: 28,
                line: 1,
                fragment: CompleteStr("")
            }
        );
    }

    #[test]
    fn test_custom_option() {
        let input = Span::new(CompleteStr(r#"option (unity.optimize_for) = lolSPEED;"#));
        let output: Result<(Span, DeclOption), _> = option(input);

        println!("OUTPUT: {:?}", output);
        assert!(output.is_ok());
        let (remainder, msg) = output.unwrap();
        assert_eq!(
            msg,
            DeclOption {
                name: DeclOptionName::Custom(Word {
                    word: LocatedSpan {
                        offset: 8,
                        line: 1,
                        fragment: CompleteStr("unity.optimize_for")
                    }
                }),
                value: LocatedSpan {
                    offset: 30,
                    line: 1,
                    fragment: CompleteStr("lolSPEED")
                }
            }
        );
        assert_eq!(
            remainder,
            LocatedSpan {
                offset: 39,
                line: 1,
                fragment: CompleteStr("")
            }
        );
    }

    // #[test]
    // fn test_import() {
    //     let msg = r#"syntax = "proto3";

    // import "test_import_nested_imported_pb.proto";

    // message ContainsImportedNested {
    //     optional ContainerForNested.NestedMessage m = 1;
    //     optional ContainerForNested.NestedEnum e = 2;
    // }
    // "#;
    //     let desc = file_descriptor(msg.as_bytes()).unwrap();
    //     assert_eq!(
    //         vec!["test_import_nested_imported_pb.proto"],
    //         desc.1.import_paths
    //     );
    // }

    // #[test]
    // fn test_package() {
    //     let msg = r#"
    //     package foo.bar;

    // message ContainsImportedNested {
    //     optional ContainerForNested.NestedMessage m = 1;
    //     optional ContainerForNested.NestedEnum e = 2;
    // }
    // "#;
    //     let desc = file_descriptor(msg.as_bytes()).unwrap();
    //     assert_eq!("foo.bar".to_string(), desc.1.package);
    // }

    // #[test]
    // fn test_nested_message() {
    //     let msg = r#"message A
    // {
    //     message B {
    //         repeated int32 a = 1;
    //         optional string b = 2;
    //     }
    //     optional b = 1;
    // }"#;

    //     let mess = message(msg.as_bytes());
    //     if let ::nom::IResult::Done(_, mess) = mess {
    //         assert!(mess.messages.len() == 1);
    //     }
    // }

    // #[test]
    // fn test_map() {
    //     let msg = r#"message A
    // {
    //     optional map<string, int32> b = 1;
    // }"#;

    //     let mess = message(msg.as_bytes());
    //     if let ::nom::IResult::Done(_, mess) = mess {
    //         assert_eq!(1, mess.fields.len());
    //         match mess.fields[0].typ {
    //             FieldType::Map(ref f) => match &**f {
    //                 &(FieldType::String, FieldType::Int32) => (),
    //                 ref f => panic!("Expecting Map<String, Int32> found {:?}", f),
    //             },
    //             ref f => panic!("Expecting map, got {:?}", f),
    //         }
    //     } else {
    //         panic!("Could not parse map message");
    //     }
    // }

    // #[test]
    // fn test_oneof() {
    //     let msg = r#"message A
    // {
    //     optional int32 a1 = 1;
    //     oneof a_oneof {
    //         string a2 = 2;
    //         int32 a3 = 3;
    //         bytes a4 = 4;
    //     }
    //     repeated bool a5 = 5;
    // }"#;

    //     let mess = message(msg.as_bytes());
    //     if let ::nom::IResult::Done(_, mess) = mess {
    //         assert_eq!(1, mess.oneofs.len());
    //         assert_eq!(3, mess.oneofs[0].fields.len());
    //     }
    // }

    // #[test]
    // fn test_reserved() {
    //     let msg = r#"message Sample {
    //    reserved 4, 15, 17 to 20, 30;
    //    reserved "foo", "bar";
    //    uint64 age =1;
    //    bytes name =2;
    // }"#;

    //     let mess = message(msg.as_bytes());
    //     if let ::nom::IResult::Done(_, mess) = mess {
    //         assert_eq!(vec![4..5, 15..16, 17..21, 30..31], mess.reserved_nums);
    //         assert_eq!(
    //             vec!["foo".to_string(), "bar".to_string()],
    //             mess.reserved_names
    //         );
    //         assert_eq!(2, mess.fields.len());
    //     } else {
    //         panic!("Could not parse reserved fields message");
    //     }
    // }

    // #[test]
    // fn test_default_value_int() {
    //     let msg = r#"message Sample {
    //         optional int32 x = 1 [default = 17];
    //     }"#;

    //     let mess = message(msg.as_bytes()).unwrap().1;
    //     assert_eq!("17", mess.fields[0].default.as_ref().expect("default"));
    // }

    // #[test]
    // fn test_default_value_string() {
    //     let msg = r#"message Sample {
    //         optional string x = 1 [default = "ab\nc d\"g\'h\0\"z"];
    //     }"#;

    //     let mess = message(msg.as_bytes()).unwrap().1;
    //     assert_eq!(r#""ab\nc d\"g\'h\0\"z""#, mess.fields[0].default.as_ref().expect("default"));
    // }

    // #[test]
    // fn test_default_value_bytes() {
    //     let msg = r#"message Sample {
    //         optional bytes x = 1 [default = "ab\nc d\xfeE\"g\'h\0\"z"];
    //     }"#;

    //     let mess = message(msg.as_bytes()).unwrap().1;
    //     assert_eq!(r#""ab\nc d\xfeE\"g\'h\0\"z""#, mess.fields[0].default.as_ref().expect("default"));
    // }

    // #[test]
    // fn test_group() {
    //     let msg = r#"message MessageWithGroup {
    //         optional string aaa = 1;

    //         repeated group Identifier = 18 {
    //             optional int32 iii = 19;
    //             optional string sss = 20;
    //         }

    //         required int bbb = 3;
    //     }"#;
    //     let mess = message(msg.as_bytes()).unwrap().1;

    //     assert_eq!("Identifier", mess.fields[1].name);
    //     if let FieldType::Group(ref group_fields) = mess.fields[1].typ {
    //         assert_eq!(2, group_fields.len());
    //     } else {
    //         panic!("expecting group");
    //     }

    //     assert_eq!("bbb", mess.fields[2].name);
    // }

    // #[test]
    // fn test_incorrect_file_descriptor() {
    //     let msg = r#"
    //         message Foo {}

    //         dfgdg
    //     "#;

    //     assert!(FileDescriptor::parse(msg.as_bytes()).is_err());
    // }

    // #[test]
    // fn test_extend() {
    //     let proto = r#"
    //         syntax = "proto2";

    //         extend google.protobuf.FileOptions {
    //             optional bool foo = 17001;
    //             optional string bar = 17002;
    //         }

    //         extend google.protobuf.MessageOptions {
    //             optional bool baz = 17003;
    //         }
    //     "#;

    //     let fd = FileDescriptor::parse(proto.as_bytes()).expect("fd");
    //     assert_eq!(3, fd.extensions.len());
    //     assert_eq!("google.protobuf.FileOptions", fd.extensions[0].extendee);
    //     assert_eq!("google.protobuf.FileOptions", fd.extensions[1].extendee);
    //     assert_eq!("google.protobuf.MessageOptions", fd.extensions[2].extendee);
    //     assert_eq!(17003, fd.extensions[2].field.number);
    // }
}
