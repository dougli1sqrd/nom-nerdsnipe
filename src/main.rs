use std::fmt::Debug;
use std::ops::Deref;
use std::vec;

use nom;
use nom::bytes::streaming::tag;
use nom::error::Error;
use nom::{character, IResult};

fn main() {
    println!("Hello, world!");

    #[derive(Debug)]
    struct I;
    impl IntoExtensionShape for I {
        fn into_shape(&self) -> ExtensionShape {
            ExtensionShape::Tag(String::from("I"))
        }

        fn generate(&self) -> Vec<Extension> {
            vec![Extension(String::from("I"))]
        }
    }
    println!("{:?}", parse_one("I", &I));

    #[derive(Debug)]
    struct X(String);
    impl IntoExtensionShape for X {
        fn into_shape(&self) -> ExtensionShape {
            ExtensionShape::Prefix(String::from("X"))
        }

        fn generate(&self) -> Vec<Extension> {
            vec![Extension(format!("X{}", self.0))]
        }
    }

    println!("{:?}", parse_one("Xabcd", &X(Default::default())));

    #[derive(Debug)]
    struct G;
    impl IntoExtensionShape for G {
        fn into_shape(&self) -> ExtensionShape {
            ExtensionShape::Multi(String::from("G"))
        }
        fn generate(&self) -> Vec<Extension> {
            ["i", "m", "a", "c"].into_iter().map(|x| Extension(x.to_string())).collect()
        }
    }
    
    println!("{:?}", parse_one("G", &G));

    println!("{:?}", parse_one_from_many("IXfoo", &[&I, &X(Default::default())]));
    println!("{:?}", parse_one_from_many("blah", &[&I, &X(Default::default())]));

    struct A;
    impl IntoExtensionShape for A {
        fn into_shape(&self) -> ExtensionShape {
            ExtensionShape::Tag(String::from("A"))
        }

        fn generate(&self) -> Vec<Extension> {
            vec![Extension(String::from("A"))]
        }
    }

    struct C;
    impl IntoExtensionShape for C {
        fn into_shape(&self) -> ExtensionShape {
            ExtensionShape::Tag(String::from("C"))
        }

        fn generate(&self) -> Vec<Extension> {
            vec![Extension(String::from("C"))]
        }
    }
    println!("{:?}", parse("IACXabc", &[&I, &A, &C, &X(Default::default())]))

}

#[derive(Debug)]
pub enum ExtensionShape {
    Tag(String),
    Prefix(String),
    Multi(String)
}

impl ExtensionShape {
    pub fn identifier(&self) -> &str {
        match self {
            ExtensionShape::Tag(i) => i.as_str(),
            ExtensionShape::Prefix(i) => i.as_str(),
            ExtensionShape::Multi(i) => i.as_str(),
        }
    }
}

pub trait IntoExtensionShape {
    fn into_shape(&self) -> ExtensionShape;

    fn generate(&self) -> Vec<Extension>;
}

#[derive(Debug)]
pub struct Extension(String);

pub fn parse_one<'a>(input: &'a str, ext: &dyn IntoExtensionShape) -> IResult<&'a str, Vec<Extension>> {
    let shape = ext.into_shape();
    match shape {
        ExtensionShape::Tag(_) => {
            let id = shape.identifier();
            nom::bytes::complete::tag(id)(input).map(|(rest, x)| (rest, vec![Extension(x.to_owned())]))
        }
        ExtensionShape::Prefix(_) => {
            let id = shape.identifier();
            nom::sequence::pair(tag(id), nom::bytes::complete::take_till1(|c| c == '_'))(input)
                .map(|(rest, (id, tail))| (rest, vec![Extension(format!("{}{}", id, tail))]))
        },
        ExtensionShape::Multi(_) => {
            let id = shape.identifier();
            nom::bytes::complete::tag(id)(input).map(|(rest, _)| (rest, ext.generate()))
        }
    }
}

pub fn parse_one_from_many<'a>(input: &'a str, extensions: &[&dyn IntoExtensionShape]) -> IResult<&'a str, (Vec<Extension>, usize)> {
    let mut inp = input;
    for (pos, ext) in extensions.into_iter().enumerate() {
        match parse_one(inp, *ext) {
            Ok((rest, v)) => {
                return Ok((rest, (v, pos)));
            },
            Err(nom::Err::Error(c)) => {
                inp = c.input;
                continue;
            },
            Err(x) => {
                return Err(x);
            }
        }
    }
    // Could not parse any of the input
    Err(nom::Err::Failure(Error { input: inp, code: nom::error::ErrorKind::Fail }))
}

pub fn parse<'a>(input: &'a str, extensions: &[&dyn IntoExtensionShape]) -> IResult<&'a str, Vec<Extension>> {
    
    fn accum_parse<'a>(input: &'a str, extensions: &[&dyn IntoExtensionShape], mut accum: Vec<Extension>) -> IResult<&'a str, Vec<Extension>> {
        // If we have input left to parse but we ran out of extensions then we failed
        if input.len() > 0 && extensions.is_empty() {
            return Err(nom::Err::Failure(Error { input, code: nom::error::ErrorKind::Complete }));
        }
        if input.is_empty() {
            return Ok(("", accum))
        }
        match parse_one_from_many(input, extensions) {
            Ok((rest, (mut found, pos))) => {
                accum.append(&mut found);
                accum_parse(rest, &extensions[pos + 1..], accum)
            },
            Err(e) => {
                Err(e)
            }
        }
    }

    accum_parse(input, extensions, vec![])
}
