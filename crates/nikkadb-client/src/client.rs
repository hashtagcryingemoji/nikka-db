use shared::{
    Action,
    ContentType::{NNone, NString},
    Request, Response, Serializable,
};

use std::io::{Read, Write};
use std::net::TcpStream;

pub struct NikkaClient {
    connection: TcpStream,
}

impl Default for NikkaClient {
    fn default() -> Self {
        Self::with_port("1402")
    }
}

impl NikkaClient {
    #[must_use]
    pub fn with_port(port: &str) -> Self {
        NikkaClient {
            connection: TcpStream::connect(format!("127.0.0.1:{port}"))
                .expect("error occurred while binding"),
        }
    }

    pub fn set_string(&mut self, key: &str, value: &str) {
        let args = vec![key.to_string(), value.to_string()];

        let request = Request {
            size: 1 + (args[0].len() + args[1].len()) as u8,
            action: Action::CREATE,
            args,
        };

        let content = request.as_bytes();
        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        let key = key.to_string();
        let args = vec![key];

        let request = Request {
            size: 1 + args[0].len() as u8,
            action: Action::GET,
            args,
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
        let mut buffer = vec![0u8; 1];

        self.connection
            .read_exact(&mut buffer)
            .expect("error occurred while reading a packet");

        let mut buffer = vec![0u8; buffer[0] as usize];

        self.connection
            .read_exact(&mut buffer)
            .expect("error occurred while reading a packet");

        let response: Response<String> = Response::from_bytes(&buffer);

        match response.content_type {
            NNone => None,
            NString => Some(response.content[0].clone()),
        }
    }

    pub fn remove(&mut self, key: &str) {
        let key = key.to_string();
        let args = vec![key];

        let request = Request {
            size: 1 + args[0].len() as u8,
            action: Action::DELETE,
            args,
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }

    pub fn get_regex(&mut self, regex: &str) -> Vec<String> {
        let regex = regex.to_string();
        let args = vec![regex];

        let request = Request {
            size: (1 + args[0].len()) as u8,
            action: Action::REGEX,
            args,
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let mut buffer = vec![0u8; 1];

        self.connection
            .read_exact(&mut buffer)
            .expect("error occurred while reading a packet");

        let mut buffer = vec![0u8; buffer[0] as usize];

        self.connection
            .read_exact(&mut buffer)
            .expect("error occurred while writing a message");

        let response: Response<String> = Response::from_bytes(&buffer);

        response.content
    }
}
