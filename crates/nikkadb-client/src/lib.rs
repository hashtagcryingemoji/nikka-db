use std::net::TcpStream;

pub mod client;

pub struct NikkaClient {
    connection: TcpStream,
    buffer: Vec<u8>,
}

pub enum NikkaType {
    TypeInt,
    TypeString,
}

pub enum NikkaTypeWrapper<'a> {
    NikkaInt(u8),
    NikkaString(&'a str),
}
