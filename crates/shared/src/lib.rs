pub mod protocol;

use crate::Action::{CREATE, DELETE, GET, REGEX, TDISCARD, TEND, TERASE, TSTART};
use crate::ContentType::{KeyValue, NInt, NNone, NString};

type Value = (ContentType, Vec<u8>);

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum ContentType {
    NNone = 0,
    NString = 1,
    NInt = 2,
    KeyValue(Box<ContentType>) = 3,
}

pub trait Serializable {
    fn as_bytes(&self) -> Vec<u8>;
    fn from_bytes(content: &[u8]) -> Self;
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
            2 => Ok(NInt),
            3 => Ok(KeyValue(Box::new(NNone))),
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
            NInt => Ok(2),
            KeyValue(_) => Ok(3),
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

impl Serializable for Vec<String> {
    fn as_bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();

        for content in self {
            let size = content.len() as u8;
            v.push(size);
            v.extend_from_slice(content.as_bytes());
        }

        v
    }

    fn from_bytes(content: &[u8]) -> Self {
        let mut v = Vec::new();
        let mut index = 0;

        while index < content.len() {
            let size = content[index];
            index += 1;
            let string_bytes = &content[index..index + size as usize];
            index += size as usize;
            let string = String::from_bytes(string_bytes);
            v.push(string)
        }

        v
    }
}

impl Serializable for u8 {
    fn as_bytes(&self) -> Vec<u8> {
        vec![*self]
    }

    fn from_bytes(content: &[u8]) -> Self {
        content[0]
    }
}
