use crate::database::NikkaDb;
use crate::ClientState::{DEFAULT, TRANSACTION};
use shared::protocol::Response::{ContentResponse, Error, Success};
use shared::protocol::{extract_key_value, Request, Response};
use shared::Action::{
    CLEAR, CREATE, DELETE, GET, POPF, POPL, PUSHF, PUSHL, REGEX, TDISCARD, TEND, TERASE, TSTART,
};
use shared::ContentType::{KeyValue, NDeque, NInt, NNone, NString, NVector};
use shared::{ContentType, Serializable};
use std::collections::{HashMap, VecDeque};
use std::net::TcpStream;
use std::sync::{Arc, Mutex, MutexGuard};

mod database;
pub mod server;
#[cfg(not(feature = "utils_for_test"))]
pub(crate) mod utils;
#[cfg(feature = "utils_for_test")]
pub mod utils;

type Value = (ContentType, Vec<u8>);

struct Client {
    socket: TcpStream,
    state: ClientState,
    queue: VecDeque<Request>,
}

#[derive(PartialEq)]
enum ClientState {
    DEFAULT,
    TRANSACTION,
}

impl Client {
    fn process_action(&mut self, request: Request, mutex: &Arc<Mutex<NikkaDb>>) -> Response {
        let action = request.action;
        let args = request.args;

        match action {
            GET => {
                let content_type = request.content_type;
                let response = match Self::process_get_request(mutex, &args, content_type) {
                    Ok(value) => value,
                    Err(value) => return value,
                };

                response
            }
            CREATE => {
                let content_type = request.content_type;

                let (k, v) = extract_serialized_key_value(&args, content_type);
                let mut database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");
                database.add(k, v);
                drop(database);

                Success
            }
            DELETE => {
                let key = &Vec::from_bytes(&args)[0];
                let mut database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");
                database.delete(key);
                drop(database);

                Success
            }
            REGEX => {
                let regex = &Vec::from_bytes(&args)[0];

                let database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");
                let content = database.find_regex(regex).to_bytes();
                drop(database);

                ContentResponse(NVector(Box::new(NString)), content)
            }
            TSTART => {
                self.state = TRANSACTION;
                Success
            }
            TEND => {
                self.state = DEFAULT;
                self.process_transaction(mutex)
            }
            TERASE => {
                self.queue.clear();
                Success
            }
            TDISCARD => {
                self.state = DEFAULT;
                self.queue.clear();
                Success
            }
            CLEAR => {
                let mut database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");
                database.clear();
                drop(database);

                Success
            }
            POPF => {
                let mut database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");

                let key = String::from_bytes(&args);

                let value = database.pop_first(&key);
                drop(database);

                match value {
                    Some(value) => ContentResponse(value.0, value.1),
                    None => ContentResponse(NNone, vec![]),
                }
            }

            POPL => {
                let mut database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");

                let key = String::from_bytes(&args);

                let value = database.pop_last(&key);
                drop(database);

                match value {
                    Some(value) => ContentResponse(value.0, value.1),
                    None => ContentResponse(NNone, vec![]),
                }
            }

            PUSHF => {
                let mut database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");

                let (mut value_bytes, key, deque) = get_deque_and_push_value(&args, &mut database);

                match deque {
                    None => Error("invalid key for deque".to_string()),

                    Some(mut value) => {
                        let KeyValue(deque_type) = request.content_type else {
                            panic!("logic error")
                        };

                        if value.0 != NDeque(deque_type.clone()) {
                            return Error("invalid key for deque".to_string());
                        }

                        match *deque_type {
                            NInt => {
                                value.1.splice(0..0, value_bytes);
                                database.add(key, value);
                                Success
                            }

                            NString => {
                                let sep = u8::try_from(value_bytes.len())
                                    .expect("value is too big to store");
                                value_bytes.push(sep);
                                value_bytes.insert(0, sep);
                                value.1.splice(0..0, value_bytes);
                                database.add(key, value);
                                Success
                            }

                            _ => panic!("logic error"),
                        }
                    }
                }
            }
            PUSHL => {
                let mut database = mutex
                    .lock()
                    .expect("error when trying to access a db mutex");

                let (mut value_bytes, key, deque) = get_deque_and_push_value(&args, &mut database);

                match deque {
                    None => Error("invalid key for deque".to_string()),

                    Some(mut value) => {
                        let KeyValue(deque_type) = request.content_type else {
                            panic!("logic error")
                        };

                        if value.0 != NDeque(deque_type.clone()) {
                            return Error("invalid key for deque".to_string());
                        }

                        match *deque_type {
                            NInt => {
                                value.1.extend_from_slice(&value_bytes);
                                database.add(key, value);
                                Success
                            }

                            NString => {
                                let sep = u8::try_from(value_bytes.len())
                                    .expect("value is too big to store");
                                value_bytes.push(sep);
                                value_bytes.insert(0, sep);
                                value.1.extend_from_slice(&value_bytes);
                                database.add(key, value);
                                Success
                            }

                            _ => panic!("logic error"),
                        }
                    }
                }
            }
        }
    }

    fn process_get_request(
        mutex: &Arc<Mutex<NikkaDb>>,
        args: &Vec<u8>,
        content_type: ContentType,
    ) -> Result<Response, Response> {
        let key = &Vec::from_bytes(&args)[0];
        let database = mutex
            .lock()
            .expect("error when trying to access a db mutex");
        let response = match content_type {
            NString => match database.get(key) {
                Some(value) => {
                    let v = vec![String::from_bytes(&value.1)];
                    ContentResponse(NString, v.to_bytes())
                }
                None => ContentResponse(NNone, vec![]),
            },
            NInt => match database.get(key) {
                Some(value) => {
                    if value.0 != NInt {
                        return Err(Error("invalid key for string".to_string()));
                    }

                    let v = vec![u8::from_bytes(&value.1)];
                    ContentResponse(NInt, v)
                }

                None => ContentResponse(NNone, vec![]),
            },
            _ => panic!("logic error"),
        };
        drop(database);
        Ok(response)
    }

    fn process_transaction(&mut self, mutex: &Arc<Mutex<NikkaDb>>) -> Response {
        let mut database = mutex
            .lock()
            .expect("error when trying to access a db mutex");
        let mut snapshot = database.storage.clone();

        for request in &self.queue {
            let request = request.clone();
            process_in_transaction(request, &mut snapshot);
        }

        database.storage = snapshot;

        Success
    }
}

fn process_in_transaction(request: Request, snapshot: &mut HashMap<String, Value>) -> Response {
    let action = request.action;
    let args = request.args;

    match action {
        CREATE => {
            let mut args_iter = Vec::from_bytes(&args).into_iter();
            if let (Some(key), Some(value)) = (args_iter.next(), args_iter.next()) {
                snapshot.insert(key, (NString, value.as_bytes().to_vec()));
            } else {
                panic!("incorrect request");
            }

            Success
        }
        DELETE => {
            let key = &Vec::from_bytes(&args)[0];
            snapshot.remove(key);

            Success
        }
        _ => panic!("logic error"),
    }
}

fn extract_serialized_key_value(
    args: &Vec<u8>,
    content_type: ContentType,
) -> (String, (ContentType, Vec<u8>)) {
    let (k, v) = match content_type {
        KeyValue(value_type) => match *value_type {
            NString => {
                let (k, v) = extract_key_value::<String>(&args);
                let value_bytes = v.as_bytes();
                (k, (NString, value_bytes.to_vec()))
            }
            NInt => {
                let (k, v) = extract_key_value::<u8>(&args);
                let value_bytes = v.to_bytes();
                (k, (NInt, value_bytes.clone()))
            }
            NDeque(deque_type) => {
                let size = args[0] as usize;
                let key = String::from_bytes(&args[1..=size]);
                (key, (NDeque(deque_type), Vec::new()))
            }
            _ => panic!("logic error"),
        },

        _ => panic!("logic error"),
    };
    (k, v)
}

fn get_deque_and_push_value(
    args: &[u8],
    database: &mut MutexGuard<NikkaDb>,
) -> (Vec<u8>, String, Option<Value>) {
    let key_size = args[0] as usize;
    let key_bytes = &args[1..=key_size];

    let value_size = args[key_size + 1] as usize;
    let value_bytes = args[key_size + 2..key_size + 2 + value_size].to_vec();

    let key = String::from_bytes(key_bytes);

    let deque = database.get(&key);
    (value_bytes, key, deque)
}
