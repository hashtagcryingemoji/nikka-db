use crate::protocol::Response::{ContentResponse, Error, Success};
use crate::ContentType::{KeyValue, NDeque};
use crate::{Action, ContentType, Serializable, Value};
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum Response {
    Success,
    Error(String),
    ContentResponse(ContentType, Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub action: Action,
    pub content_type: ContentType,
    pub args: Vec<u8>,
}

// tlv serialization and deserialization for response
impl Serializable for Response {
    fn to_bytes(&self) -> Vec<u8> {
        let mut packet = Vec::new();

        match self {
            Success => {
                packet.push(9);
            }
            ContentResponse(content_type, content) => {
                packet.push(0);
                packet
                    .push(u8::try_from(content_type.to_owned()).expect("broken response instance"));
                packet.push(u8::try_from(content.len()).expect("content is too big to store"));
                packet.extend_from_slice(content);
            }
            Error(message) => {
                packet.push(1);
                packet.push(u8::try_from(message.len()).expect("message is too long to store"));
                packet.extend_from_slice(message.as_bytes());
            }
        }

        packet
    }

    fn from_bytes(packet: &[u8]) -> Self {
        match packet[0] {
            0 => {
                let content_type = ContentType::try_from(packet[1]).expect("broken packet");
                let content_len = packet[2] as usize;
                let content = &packet[3..content_len + 3];

                ContentResponse(content_type, content.to_vec())
            }
            1 => {
                let message_len = packet[1] as usize;
                let message = String::from_bytes(&packet[2..message_len + 2]);

                Error(message)
            }
            9 => Success,
            _ => panic!("broken packet"),
        }
    }
}

impl<S: std::hash::BuildHasher + Default> Serializable for HashMap<String, Value, S> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut byte_repr: Vec<u8> = Vec::new();

        for (k, v) in self {
            let k_bytes = k.as_bytes();
            let k_len = u8::try_from(k.len()).expect("cannot support size that big yet");
            byte_repr.push(k_len);
            byte_repr.extend_from_slice(k_bytes);

            let content: u8 = v.0.clone().try_into().expect("broken packet");
            byte_repr.push(content);
            if let NDeque(nested_type) = v.0.clone() {
                byte_repr.push(u8::try_from(*nested_type).expect("broken packet"));
            }
            let v_len = u8::try_from(v.1.len()).expect("cannot support size that big yet");
            byte_repr.push(v_len);
            byte_repr.extend_from_slice(&v.1);
        }
        byte_repr
    }

    fn from_bytes(content: &[u8]) -> Self {
        let mut index = 0;
        let mut hm = HashMap::default();

        while index < content.len() {
            let size = content[index];
            index += 1;
            let k = &content[index..index + size as usize];
            index += size as usize;

            let mut content_type = ContentType::try_from(content[index]).expect("broken packet");
            index += 1;
            if let NDeque(_) = content_type {
                let nested_content_type =
                    ContentType::try_from(content[index]).expect("broken packet");
                content_type = NDeque(Box::new(nested_content_type));
                index += 1;
            }
            let size = content[index];
            index += 1;
            let v = &content[index..index + size as usize];
            index += size as usize;

            let (key, value) = (String::from_bytes(k), (content_type, v.to_vec()));

            hm.insert(key, value);
        }

        hm
    }
}

impl Serializable for Request {
    fn to_bytes(&self) -> Vec<u8> {
        let mut packet = Vec::new();
        let op_code: u8 = self.action.try_into().expect("incorrect opcode");
        packet.push(op_code);

        let content_type: u8 = self
            .content_type
            .clone()
            .try_into()
            .expect("incorrect content type");
        packet.push(content_type);

        if let KeyValue(value_type) = self.content_type.clone() {
            packet.push(u8::try_from(*value_type.clone()).expect("broken packet"));

            if let NDeque(content_type) = *value_type {
                packet.push(u8::try_from(*content_type).expect("broken packet"));
            }
        }

        packet.push(u8::try_from(self.args.len()).expect("arg is too big to store"));
        packet.extend_from_slice(&self.args);

        packet
    }

    fn from_bytes(packet: &[u8]) -> Request {
        let mut index = 0;

        let action = match Action::try_from(packet[index]) {
            Ok(action) => action,
            Err(e) => panic!("{}", e),
        };

        index += 1;

        let content_type = match ContentType::try_from(packet[index]) {
            Ok(content_type) => match content_type {
                KeyValue(_) => {
                    index += 1;
                    let value_type = ContentType::try_from(packet[index]).expect("broken packet");
                    let true_value_type = match value_type {
                        NDeque(_) => {
                            index += 1;
                            let nested_type =
                                ContentType::try_from(packet[index]).expect("broken packet");
                            NDeque(Box::new(nested_type))
                        }

                        _ => value_type,
                    };
                    KeyValue(Box::new(true_value_type))
                }
                _ => content_type,
            },
            Err(e) => panic!("{}", e),
        };

        index += 1;
        let args_len = packet[index] as usize;
        index += 1;

        let args = match packet.get(index..index + args_len) {
            Some(arg) => arg.to_vec(),
            None => panic!("broken packet"),
        };

        Request {
            action,
            content_type,
            args,
        }
    }
}

pub fn form_packet<T>(content: &T) -> Vec<u8>
where
    T: Serializable + std::fmt::Debug,
{
    let mut packet = Vec::new();
    packet.extend_from_slice(&content.to_bytes());
    packet.insert(0, u8::try_from(packet.len()).expect("paket is too big"));
    packet
}

pub fn form_response(connection: &mut TcpStream) -> Response {
    let mut buffer = vec![0u8; 1];

    connection
        .read_exact(&mut buffer)
        .expect("error occurred while reading a packet");

    let mut buffer = vec![0u8; buffer[0] as usize];

    connection
        .read_exact(&mut buffer)
        .expect("error occurred while reading a packet");

    Response::from_bytes(&buffer)
}

pub fn extract_key_value<T>(content: &[u8]) -> (String, T)
where
    T: Serializable,
{
    let mut index = 0;
    let k_size = content[index] as usize;
    index += 1;
    let key = String::from_bytes(&content[index..=k_size]);
    index = k_size + 1;
    let v_size = content[index] as usize;
    index += 1;
    (key, T::from_bytes(&content[index..index + v_size]))
}
