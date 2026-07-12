use crate::errors::NikkaError::IoError;
use std::io::Error;

#[derive(Debug)]
pub enum NikkaError {
    IoError(Error),
    DatabaseError(&'static str),
}

impl From<Error> for NikkaError {
    fn from(io_error: Error) -> Self {
        IoError(io_error)
    }
}
