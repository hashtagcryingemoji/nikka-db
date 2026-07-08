use shared::ContentType;
use shared::ContentType::{NInt, NString};
use std::net::TcpStream;

pub mod client;

pub struct NikkaClient {
    connection: TcpStream,
    buffer: Vec<u8>,
}

pub enum NikkaType {
    TypeU8,
    TypeString,
}

pub enum NikkaTypeWrapper<'a> {
    NikkaInt(u8),
    NikkaString(&'a str),
}

pub trait Conversion {
    fn convert() -> ContentType;
}

impl Conversion for u8 {
    fn convert() -> ContentType {
        NInt
    }
}

impl Conversion for &str {
    fn convert() -> ContentType {
        NString
    }
}

impl Conversion for String {
    fn convert() -> ContentType {
        NString
    }
}
