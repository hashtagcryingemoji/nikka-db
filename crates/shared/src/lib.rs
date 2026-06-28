use crate::Action::{CREATE, DELETE, GET, REGEX, TDISCARD, TEND, TERASE, TSTART};
use crate::ContentType::{NNone, NString};
use std::collections::HashMap;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum ContentType {
    NNone = 0,
    NString = 1,
}

pub trait Serializable {
    fn as_bytes(&self) -> Vec<u8>;
    fn from_bytes(content: &[u8]) -> Self;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request<T> {
    pub size: u8,
    pub action: Action,
    pub args: Vec<T>,
}

#[derive(Debug)]
pub struct Response<T> {
    pub size: u8,
    pub content_type: ContentType,
    pub content: Vec<T>,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Action {
    CREATE = 1,
    DELETE = 2,
    GET = 3,
    REGEX = 4,
    TSTART = 5,
    TEND = 6,
    TERASE = 7,
    TDISCARD = 8,
}

pub enum DatabaseManipulation {
    CREATE(String, String),
    DELETE(String),
    GET(String),
    DISCONNECT,
}

impl TryFrom<u8> for Action {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(CREATE),
            2 => Ok(DELETE),
            3 => Ok(GET),
            4 => Ok(REGEX),
            5 => Ok(TSTART),
            6 => Ok(TEND),
            7 => Ok(TERASE),
            8 => Ok(TDISCARD),
            _ => Err("conversion error"),
        }
    }
}

impl TryFrom<u8> for ContentType {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NNone),
            1 => Ok(NString),
            _ => Err("conversion error"),
        }
    }
}

impl TryFrom<Action> for u8 {
    type Error = &'static str;

    fn try_from(value: Action) -> Result<Self, Self::Error> {
        match value {
            CREATE => Ok(1),
            DELETE => Ok(2),
            GET => Ok(3),
            REGEX => Ok(4),
            TSTART => Ok(5),
            TEND => Ok(6),
            TERASE => Ok(7),
            TDISCARD => Ok(8),
        }
    }
}

impl TryFrom<ContentType> for u8 {
    type Error = &'static str;

    fn try_from(value: ContentType) -> Result<Self, Self::Error> {
        match value {
            NNone => Ok(0),
            NString => Ok(1),
        }
    }
}

impl<T> Serializable for Request<T>
where
    T: Serializable,
{
    fn as_bytes(&self) -> Vec<u8> {
        let mut size: u8 = 1;
        let mut packet = Vec::new();
        let op_code: u8 = self.action.try_into().expect("incorrect opcode");
        packet.push(op_code);

        for arg in &self.args {
            let byte_arg = arg.as_bytes();
            let len = u8::try_from(byte_arg.len()).expect("cannot support that arg len yet");
            size += u8::try_from(byte_arg.len()).expect("cannot support that arg len yet") + 1;

            packet.push(len);
            packet.extend_from_slice(&byte_arg);
        }

        packet.insert(0, size);

        packet
    }

    fn from_bytes(packet: &[u8]) -> Request<T> {
        let mut index = 0;

        let action = match Action::try_from(packet[index]) {
            Ok(action) => action,
            Err(e) => panic!("{}", e),
        };

        index += 1;

        let mut args: Vec<T> = Vec::new();

        while index < packet.len() {
            let len = packet[index] as usize;

            index += 1;

            let arg = match packet.get(index..(index + len)) {
                Some(arg) => T::from_bytes(arg),
                None => panic!("broken packet"),
            };

            index += len;

            args.push(arg);
        }

        Request {
            size: u8::try_from(index).expect("cannot support messages that big yet") - 1,
            action,
            args,
        }
    }
}

impl<T> Serializable for Response<T>
where
    T: Serializable,
{
    fn as_bytes(&self) -> Vec<u8> {
        let mut size: u8 = 0;
        let mut packet = Vec::new();
        let op_code: u8 = self.content_type.try_into().expect("incorrect opcode");
        packet.push(op_code);

        size += 1;

        for arg in &self.content {
            let byte_arg = arg.as_bytes();
            let len = u8::try_from(byte_arg.len()).expect("cannot support that arg len yet");
            size += u8::try_from(byte_arg.len()).expect("cannot support that arg len yet") + 1;

            packet.push(len);
            packet.extend_from_slice(&byte_arg);
        }

        packet.insert(0, size);

        packet
    }

    fn from_bytes(packet: &[u8]) -> Response<T> {
        let mut index = 0;

        let content_type = match ContentType::try_from(packet[index]) {
            Ok(content_type) => match content_type {
                NNone => {
                    return Response {
                        size: 1,
                        content_type,
                        content: vec![],
                    }
                }
                NString => NString,
            },
            Err(e) => panic!("{}", e),
        };

        index += 1;

        let mut content: Vec<T> = Vec::new();

        while index < packet.len() {
            let len = packet[index] as usize;

            index += 1;

            let arg = match packet.get(index..(index + len)) {
                Some(arg) => T::from_bytes(arg),
                None => panic!("broken packet"),
            };

            index += len;

            content.push(arg);
        }

        Response {
            size: u8::try_from(index).expect("response size is too big") - 1,
            content_type,
            content,
        }
    }
}

impl Serializable for String {
    fn as_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    fn from_bytes(content: &[u8]) -> Self {
        String::from_utf8(content.to_vec()).unwrap()
    }
}

impl Serializable for HashMap<String, String> {
    fn as_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::new();

        for (k, v) in self {
            let k_bytes = k.as_bytes();
            let k_len = u8::try_from(k.len()).expect("cannot support size that big yet");
            vec.push(k_len);
            vec.extend_from_slice(k_bytes);

            let v_bytes = v.as_bytes();
            let v_len = u8::try_from(v.len()).expect("cannot support size that big yet");
            vec.push(v_len);
            vec.extend_from_slice(v_bytes);
        }

        vec
    }

    fn from_bytes(content: &[u8]) -> Self {
        let mut index = 0;
        let mut hm = HashMap::new();

        while index < content.len() {
            let size = content[index];
            index += 1;
            let k = &content[index..index + size as usize];
            index += size as usize;

            let size = content[index];
            index += 1;
            let v = &content[index..index + size as usize];
            index += size as usize;

            let (key, value) = (String::from_bytes(k), String::from_bytes(v));

            hm.insert(key, value);
        }

        hm
    }
}
