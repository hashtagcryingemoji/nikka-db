use shared::{
    Action,
    ContentType::{NNone, NString},
    Deserializable, Serializable,
};

pub use crate::NikkaClient;
use crate::{
    Conversion, NikkaType, NikkaTypeWrapper,
    NikkaTypeWrapper::{NikkaInt, NikkaString},
};
use shared::errors::NikkaError;
use shared::errors::NikkaError::DatabaseError;
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

const U8_SIZE: u8 = 1;

impl NikkaClient {
    #[must_use]
    pub fn with_port(port: &str) -> Self {
        let connection =
            TcpStream::connect(format!("127.0.0.1:{port}")).expect("error occurred while binding");
        connection
            .set_nodelay(true)
            .expect("cannot set client to no delay mode");
        NikkaClient {
            buffer: vec![0u8; 1],
            connection,
        }
    }

    pub fn set<T>(&mut self, key: &str, value: T) -> Result<(), NikkaError>
    where
        T: Conversion + Serializable,
    {
        let db_type = T::convert();
        let bytes = value.to_bytes();
        let mut args = vec![key.to_string()].to_bytes();
        match db_type {
            NString => {
                args.push(
                    u8::try_from(bytes.len()).map_err(|_| DatabaseError("key name is too long"))?,
                );
                args.extend_from_slice(&bytes);
            }
            NInt => {
                args.push(U8_SIZE);
                args.extend_from_slice(&bytes);
            }
            _ => unreachable!(),
        }

        let request = Request {
            action: Action::CREATE,
            content_type: KeyValue(Box::new(db_type)),
            args,
        };

        let content = form_packet(&request);
        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            Success => Ok(()),
            Error(_) => Err(DatabaseError("undefined error occurred")),
            _ => Err(DatabaseError("broken request packet")),
        }
    }

    pub fn get<T>(&mut self, key: &str) -> Result<Option<T>, NikkaError>
    where
        T: Serializable + Deserializable + Conversion,
    {
        let key = key.to_string();
        let mut args = Vec::new();
        args.push(u8::try_from(key.len()).map_err(|_| DatabaseError("key name is too long"))?);
        args.extend_from_slice(key.as_bytes());

        let content_type = T::convert();

        let request = Request {
            action: Action::GET,
            content_type,
            args,
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            ContentResponse(content_type, content) => match content_type {
                NNone => Ok(None),
                NInt => Ok(Some(T::from_bytes(&content))),
                _ => Ok(Some(T::from_bytes(&content[1..]))),
            },

            _ => Err(DatabaseError("Invalid key")),
        }
    }

    pub fn remove(&mut self, key: &str) -> Result<(), NikkaError> {
        let key = key.to_string();
        let mut args = Vec::new();
        args.push(u8::try_from(key.len()).map_err(|_| DatabaseError("key name is too long"))?);
        args.extend_from_slice(key.as_bytes());

        let request = Request {
            action: Action::DELETE,
            content_type: NString,
            args,
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            Success => Ok(()),
            Error(_) => Err(DatabaseError("undefined error occurred")),
            _ => Err(DatabaseError("broken request packet")),
        }
    }

    pub fn get_regex(&mut self, regex: &str) -> Result<Option<Vec<String>>, NikkaError> {
        let regex = regex.to_string();
        let mut args = Vec::new();
        args.push(
            u8::try_from(regex.len()).map_err(|_| DatabaseError("regex statement is too long"))?,
        );
        args.extend_from_slice(regex.as_bytes());

        let request = Request {
            action: Action::REGEX,
            content_type: NString,
            args,
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            ContentResponse(content_type, content) => match content_type {
                NVector(_) => Ok(Some(Vec::from_bytes(&content))),

                _ => Ok(Some(Vec::new())),
            },

            _ => Err(DatabaseError("Invalid key")),
        }
    }

    pub fn begin_transaction(&mut self) -> Result<(), NikkaError> {
        let request: Request = Request {
            action: Action::TSTART,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let _response = form_response(&mut self.connection, &mut self.buffer);

        Ok(())
    }

    pub fn send_transaction(&mut self) -> Result<(), NikkaError> {
        let request: Request = Request {
            action: Action::TEND,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection
            .write_all(&content)
            .map_err(|e| NikkaError::from(e))?;

        let _response = form_response(&mut self.connection, &mut self.buffer);
        Ok(())
    }

    pub fn erase_transaction(&mut self) -> Result<(), NikkaError> {
        let request: Request = Request {
            action: Action::TERASE,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let _response = form_response(&mut self.connection, &mut self.buffer);

        Ok(())
    }

    pub fn abort_transaction(&mut self) -> Result<(), NikkaError> {
        let request: Request = Request {
            action: Action::TDISCARD,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let _response = form_response(&mut self.connection, &mut self.buffer);

        Ok(())
    }

    pub fn clear_database(&mut self) -> Result<(), NikkaError> {
        let request: Request = Request {
            action: Action::CLEAR,
            content_type: NNone,
            args: Vec::new(),
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let _response = form_response(&mut self.connection, &mut self.buffer);

        Ok(())
    }

    pub fn create_deque(&mut self, key: &str, deque_type: NikkaType) -> Result<(), NikkaError> {
        let mut args = Vec::with_capacity(1 + key.len());
        args.push(u8::try_from(key.len()).map_err(|_| DatabaseError("deque name is too long"))?);
        args.extend_from_slice(key.as_bytes());

        let true_deque_type = match deque_type {
            NikkaType::TypeU8 => NInt,
            NikkaType::TypeString => NString,
        };

        let request: Request = Request {
            action: Action::CREATE,
            content_type: KeyValue(Box::new(NDeque(Box::new(true_deque_type)))),
            args,
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            Success => Ok(()),

            _ => Err(DatabaseError("cannot create deque")),
        }
    }

    pub fn push_first(&mut self, key: &str, value: NikkaTypeWrapper) -> Result<(), NikkaError> {
        let request = form_push_request(key, &value, PUSHF)?;

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            Success => Ok(()),
            Error(_) => Err(DatabaseError("undefined error occurred")),
            _ => unreachable!(),
        }
    }

    pub fn pop_first<T>(&mut self, key: &str) -> Result<Option<T>, NikkaError>
    where
        T: Deserializable,
    {
        let request = Request {
            action: POPF,
            content_type: NString,
            args: key.as_bytes().to_vec(),
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            ContentResponse(NString | NInt, vec) => Ok(Some(T::from_bytes(&vec))),
            _ => Ok(None),
        }
    }

    pub fn push_last(&mut self, key: &str, value: NikkaTypeWrapper) -> Result<(), NikkaError> {
        let request = form_push_request(key, &value, PUSHL)?;

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            Success => Ok(()),
            Error(_) => Err(DatabaseError("undefined error occurred")),
            _ => unreachable!(),
        }
    }

    pub fn pop_last<T>(&mut self, key: &str) -> Result<Option<T>, NikkaError>
    where
        T: Deserializable,
    {
        let request = Request {
            action: POPL,
            content_type: NString,
            args: key.as_bytes().to_vec(),
        };

        let content = form_packet(&request);

        self.connection.write_all(&content)?;

        let response = form_response(&mut self.connection, &mut self.buffer);

        match response {
            ContentResponse(NString | NInt, vec) => Ok(Some(T::from_bytes(&vec))),
            _ => Ok(None),
        }
    }
}

fn form_push_request(
    key: &str,
    value: &NikkaTypeWrapper,
    action: Action,
) -> Result<Request, NikkaError> {
    let request = match value {
        NikkaInt(int) => {
            let value_bytes = Serializable::to_bytes(int);

            let mut args = Vec::with_capacity(key.len() + value_bytes.len() + 2);
            args.push(u8::try_from(key.len()).map_err(|_| DatabaseError("key is too long"))?);
            args.extend_from_slice(key.as_bytes());

            args.push(
                u8::try_from(value_bytes.len()).map_err(|_| DatabaseError("key is too long"))?,
            );
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
            args.push(u8::try_from(key.len()).map_err(|_| DatabaseError("key name is too long"))?);
            args.extend_from_slice(key.as_bytes());

            args.push(
                u8::try_from(value_bytes.len()).map_err(|_| DatabaseError("value is too big"))?,
            );
            args.extend_from_slice(&value_bytes);
            Request {
                action,
                content_type: KeyValue(Box::new(NString)),
                args,
            }
        }
    };
    Ok(request)
}
