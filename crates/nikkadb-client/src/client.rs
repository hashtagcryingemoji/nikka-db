use shared::{
    Action,
    ContentType::{NNone, NString},
    Serializable,
};

use shared::protocol::Response::*;
use shared::protocol::{form_packet, form_response, Request};
use shared::ContentType::{KeyValue, NInt};
use std::io::Write;
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

    pub fn set_string(&mut self, key: &str, value: &str) -> Result<(), String> {
        let args = vec![key.to_string(), value.to_string()].as_bytes();

        let request = Request {
            action: Action::CREATE,
            content_type: KeyValue(Box::new(NString)),
            args,
        };

        let content = form_packet(request);
        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            Success => Ok(()),
            Error(message) => Err(message),
            _ => panic!("broken request packet"),
        }
    }

    pub fn get_string(&mut self, key: &str) -> Option<String> {
        let key = key.to_string();
        let mut args = Vec::new();
        args.push(key.len() as u8);
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            action: Action::GET,
            content_type: NString,
            args,
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            ContentResponse(content_type, content) => match content_type {
                NString => Some(Vec::from_bytes(&content)[0].clone()),
                _ => None,
            },

            _ => panic!("broken response packet"),
        }
    }

    pub fn set_int(&mut self, key: &str, value: u8) -> Result<(), String> {
        let mut args = vec![key.to_string()].as_bytes();
        args.push(1);
        args.push(value);

        let request = Request {
            action: Action::CREATE,
            content_type: KeyValue(Box::new(NInt)),
            args,
        };

        let content = form_packet(request);
        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            Success => Ok(()),
            Error(message) => Err(message),
            _ => panic!("broken request packet"),
        }
    }

    pub fn get_int(&mut self, key: &str) -> Option<u8> {
        let key = key.to_string();
        let mut args = Vec::new();
        args.push(key.len() as u8);
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            action: Action::GET,
            content_type: NInt,
            args,
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            ContentResponse(content_type, content) => match content_type {
                NInt => Some(u8::from_bytes(&content)),
                _ => None,
            },

            _ => panic!("broken response packet"),
        }
    }

    pub fn remove(&mut self, key: &str) -> Result<(), String> {
        let key = key.to_string();
        let mut args = Vec::new();
        args.push(key.len() as u8);
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            action: Action::DELETE,
            content_type: NString,
            args,
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            Success => Ok(()),
            Error(message) => Err(message),
            _ => panic!("broken request packet"),
        }
    }

    pub fn get_regex(&mut self, regex: &str) -> Vec<String> {
        let regex = regex.to_string();
        let mut args = Vec::new();
        args.push(regex.len() as u8);
        args.extend_from_slice(regex.as_bytes());

        let request = Request {
            action: Action::REGEX,
            content_type: NString,
            args,
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            ContentResponse(content_type, _content) => match content_type {
                NString => {
                    todo!("extract regex from raw bytes in content")
                }

                _ => Vec::new(),
            },

            _ => panic!("broken response packet"),
        }
    }

    pub fn begin_transaction(&mut self) {
        let request: Request = Request {
            action: Action::TSTART,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);
    }

    pub fn send_transaction(&mut self) {
        let request: Request = Request {
            action: Action::TEND,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);
    }

    pub fn erase_transaction(&mut self) {
        let request: Request = Request {
            action: Action::TERASE,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);
    }

    pub fn abort_transaction(&mut self) {
        let request: Request = Request {
            action: Action::TDISCARD,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);
    }
}
