use shared::{
    Action,
    ContentType::{NNone, NString},
    Serializable,
};

pub use crate::NikkaClient;
use crate::{
    NikkaType, NikkaTypeWrapper,
    NikkaTypeWrapper::{NikkaInt, NikkaString},
};
use shared::protocol::Response::{ContentResponse, Error, Success};
use shared::protocol::{form_packet, form_response, Request};
use shared::Action::{POPF, POPL, PUSHF, PUSHL};
use shared::ContentType::{KeyValue, NDeque, NInt, NVector};
use std::io::Write;
use std::net::TcpStream;

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
        let args = vec![key.to_string(), value.to_string()].to_bytes();

        let request = Request {
            action: Action::CREATE,
            content_type: KeyValue(Box::new(NString)),
            args,
        };

        let content = form_packet(&request);
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
        args.push(u8::try_from(key.len()).expect("key name is too big"));
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            action: Action::GET,
            content_type: NString,
            args,
        };

        let content = form_packet(&request);

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
        let mut args = vec![key.to_string()].to_bytes();
        args.push(1);
        args.push(value);

        let request = Request {
            action: Action::CREATE,
            content_type: KeyValue(Box::new(NInt)),
            args,
        };

        let content = form_packet(&request);
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
        args.push(u8::try_from(key.len()).expect("key name is too big"));
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            action: Action::GET,
            content_type: NInt,
            args,
        };

        let content = form_packet(&request);

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
        args.push(u8::try_from(key.len()).expect("key is too big to store"));
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            action: Action::DELETE,
            content_type: NString,
            args,
        };

        let content = form_packet(&request);

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
        args.push(u8::try_from(regex.len()).expect("argument is too big"));
        args.extend_from_slice(regex.as_bytes());

        let request = Request {
            action: Action::REGEX,
            content_type: NString,
            args,
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            ContentResponse(content_type, content) => match content_type {
                NVector(_) => Vec::from_bytes(&content),

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

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let _response = form_response(&mut self.connection);
    }

    pub fn send_transaction(&mut self) {
        let request: Request = Request {
            action: Action::TEND,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let _response = form_response(&mut self.connection);
    }

    pub fn erase_transaction(&mut self) {
        let request: Request = Request {
            action: Action::TERASE,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let _response = form_response(&mut self.connection);
    }

    pub fn abort_transaction(&mut self) {
        let request: Request = Request {
            action: Action::TDISCARD,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let _response = form_response(&mut self.connection);
    }

    pub fn clear_database(&mut self) {
        let request: Request = Request {
            action: Action::CLEAR,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let _response = form_response(&mut self.connection);
    }

    pub fn create_deque(&mut self, key: &str, deque_type: NikkaType) -> Result<(), String> {
        let mut args = Vec::with_capacity(1 + key.len());
        args.push(u8::try_from(key.len()).expect("deque name is too long"));
        args.extend_from_slice(key.as_bytes());

        let true_deque_type = match deque_type {
            NikkaType::TypeInt => NInt,
            NikkaType::TypeString => NString,
        };

        let request: Request = Request {
            action: Action::CREATE,
            content_type: KeyValue(Box::new(NDeque(Box::new(true_deque_type)))),
            args,
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            Success => Ok(()),

            _ => Err("cannot create deque".to_string()),
        }
    }

    pub fn push_first(&mut self, key: &str, value: NikkaTypeWrapper) -> Result<(), String> {
        let request = form_push_request(key, value, PUSHF);

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            Success => Ok(()),
            Error(message) => Err(message),
            _ => panic!("logic error"),
        }
    }

    pub fn pop_first<T>(&mut self, key: &str) -> Option<T>
    where
        T: Serializable,
    {
        let request = Request {
            action: POPF,
            content_type: NString,
            args: key.as_bytes().to_vec(),
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            ContentResponse(content_type, vec) => match content_type {
                NString | NInt => Some(T::from_bytes(&vec)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn push_last(&mut self, key: &str, value: NikkaTypeWrapper) -> Result<(), String> {
        let request = form_push_request(key, value, PUSHL);

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            Success => Ok(()),
            Error(message) => Err(message),
            _ => panic!("logic error"),
        }
    }

    pub fn pop_last<T>(&mut self, key: &str) -> Option<T>
    where
        T: Serializable,
    {
        let request = Request {
            action: POPL,
            content_type: NString,
            args: key.as_bytes().to_vec(),
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .expect("error occurred while writing a message");

        let response = form_response(&mut self.connection);

        match response {
            ContentResponse(NString | NInt, vec) => Some(T::from_bytes(&vec)),
            _ => None,
        }
    }
}

fn form_push_request(key: &str, value: NikkaTypeWrapper, action: Action) -> Request {
    let request = match value {
        NikkaInt(int) => {
            let value_bytes = int.to_bytes();

            let mut args = Vec::with_capacity(key.len() + value_bytes.len() + 2);
            args.push(key.len() as u8);
            args.extend_from_slice(key.as_bytes());

            args.push(value_bytes.len() as u8);
            args.extend_from_slice(&value_bytes);

            Request {
                action,
                content_type: KeyValue(Box::new(NInt)),
                args,
            }
        }

        NikkaString(str) => {
            let string = str.to_string();
            let value_bytes = string.to_bytes();

            let mut args = Vec::with_capacity(key.len() + value_bytes.len() + 2);
            args.push(key.len() as u8);
            args.extend_from_slice(key.as_bytes());

            args.push(value_bytes.len() as u8);
            args.extend_from_slice(&value_bytes);
            Request {
                action,
                content_type: KeyValue(Box::new(NString)),
                args,
            }
        }
    };
    request
}
