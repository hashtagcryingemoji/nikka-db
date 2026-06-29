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
        let args = vec![key.to_string(), value.to_string()].as_bytes();

        let request = Request {
            size: 1 + args.len() as u8,
            action: Action::CREATE,
            content_type: NString,
            args,
        };

        let content = request.as_bytes();
        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        let key = key.to_string();
        let mut args = Vec::new();
        args.push(key.len() as u8);
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            size: 1 + args.len() as u8,
            action: Action::GET,
            content_type: NString,
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

        let response: Response = Response::from_bytes(&buffer);

        match response.content_type {
            NNone => None,
            NString => Some(Vec::from_bytes(&response.content)[0].clone()),
        }
    }

    pub fn remove(&mut self, key: &str) {
        let key = key.to_string();
        let mut args = Vec::new();
        args.push(key.len() as u8);
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            size: 1 + args.len() as u8,
            action: Action::DELETE,
            content_type: NString,
            args,
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }

    pub fn get_regex(&mut self, regex: &str) -> Vec<String> {
        let regex = regex.to_string();
        let mut args = Vec::new();
        args.push(regex.len() as u8);
        args.extend_from_slice(regex.as_bytes());

        let request = Request {
            size: (1 + args.len()) as u8,
            action: Action::REGEX,
            content_type: NString,
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

        let response: Response = Response::from_bytes(&buffer);

        Vec::from_bytes(&response.content)
    }

    pub fn begin_transaction(&mut self) {
        let request: Request = Request {
            size: 1,
            action: Action::TSTART,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }

    pub fn send_transaction(&mut self) {
        let request: Request = Request {
            size: 1,
            action: Action::TEND,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }

    pub fn erase_transaction(&mut self) {
        let request: Request = Request {
            size: 1,
            action: Action::TERASE,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }

    pub fn abort_transaction(&mut self) {
        let request: Request = Request {
            size: 1,
            action: Action::TDISCARD,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = request.as_bytes();

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");
    }
}
