use std::net::TcpStream;

pub mod client;

pub struct NikkaClient {
    connection: TcpStream,
}

pub enum NikkaType {
    NikkaInt,
    NikkaString,
}
