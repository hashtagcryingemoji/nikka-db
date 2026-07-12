use crate::database::NikkaDb;
use crate::ClientState::{DEFAULT, TRANSACTION};
use mio::net::TcpStream;
use shared::protocol::Response::{ContentResponse, Error, Success};
use shared::protocol::{extract_key_value, Request, Response};
use shared::Action::{
    CLEAR, CREATE, DELETE, GET, POPF, POPL, PUSHF, PUSHL, REGEX, TDISCARD, TEND, TERASE, TSTART,
};
use shared::ContentType::{KeyValue, NDeque, NInt, NNone, NString, NVector};
use shared::{ContentType, Deserializable, Serializable};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex, RwLock};

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
    fn process_action(
        &mut self,
        request: &Request,
        database: &mut Arc<RwLock<NikkaDb>>,
        wal: &Arc<Mutex<File>>,
    ) -> Response {
        let action = &request.action;
        let args = &request.args;

        let response = match action {
            GET => {
                let content_type = &request.content_type;
                process_get_request(database, args, content_type)
            }
            CREATE => {
                let content_type = request.content_type.clone();
                let mut database = database.write().unwrap();
                process_create_request(&mut *database, args, content_type);

                Success
            }
            DELETE => {
                let mut database = database.write().unwrap();
                process_delete_request(&mut *database, args);

                Success
            }
            REGEX => {
                let size = args[0] as usize;
                let regex_bytes = &args[1..size + 1];
                let regex = str::from_utf8(regex_bytes).expect("");
                let database = database.read().expect("cannot lock mutex");
                let content = database.find_regex(regex).to_bytes();
                ContentResponse(NVector(Box::new(NString)), content)
            }
            TSTART => {
                self.state = TRANSACTION;
                Success
            }
            TEND => {
                self.state = DEFAULT;
                self.process_transaction(database, wal)
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
                let mut database = database.write().expect("cannot lock mutex");
                database.clear();
                Success
            }
            POPF => {
                let mut database = database.write().unwrap();
                process_pop_first_request(&mut *database, args)
            }

            POPL => {
                let mut database = database.write().unwrap();
                process_pop_last_request(&mut *database, args)
            }

            PUSHF => {
                let content_type = request.content_type.clone();
                let mut database = database.write().unwrap();
                process_push_first_request(&mut *database, args, content_type)
            }
            PUSHL => {
                let content_type = request.content_type.clone();
                let mut database = database.write().unwrap();
                process_push_last_request(&mut *database, args, content_type)
            }
        };

        if response == Success && self.should_be_in_wal(request) {
            let serialized_request = request.to_bytes();
            let mut wal = wal.lock().expect("cannot lock mutex in main thread");
            wal.write_all(&serialized_request)
                .expect("cannot write to wal");
            wal.flush().expect("cannot write to wal");
        }

        response
    }

    fn process_transaction(
        &mut self,
        database: &mut Arc<RwLock<NikkaDb>>,
        wal: &Arc<Mutex<File>>,
    ) -> Response {
        let unlocked_database = database.read().expect("cannot lock mutex");
        let mut snapshot = unlocked_database.clone();

        for request in &self.queue {
            let request = request.clone();
            match process_in_transaction(request, &mut snapshot) {
                Success => {}
                Error(mes) => return Error(mes),
                _ => unreachable!(),
            };
        }

        drop(unlocked_database);
        let mut database = database.write().expect("cannot unlock mutex");

        for request in &self.queue {
            let request = request.clone();
            let serialized_request = request.to_bytes();
            let mut wal = wal.lock().expect("cannot lock mutex in main thread");
            wal.write_all(&serialized_request)
                .expect("cannot write to wal");
            wal.flush().expect("cannot write to wal");
            drop(wal);
            process_in_transaction(request, &mut *database);
        }

        Success
    }

    #[inline]
    fn should_be_transaction(&self, request: &Request) -> bool {
        self.state == TRANSACTION
            && (request.action == CREATE
                || request.action == DELETE
                || request.action == PUSHF
                || request.action == PUSHL
                || request.action == POPL
                || request.action == POPF)
    }

    #[inline]
    fn should_be_in_wal(&self, request: &Request) -> bool {
        (request.action == CREATE
            || request.action == DELETE
            || request.action == PUSHL
            || request.action == PUSHF
            || request.action == POPL
            || request.action == POPF)
            && self.state != TRANSACTION
    }
}

fn process_in_transaction(request: Request, snapshot: &mut NikkaDb) -> Response {
    let action = request.action;
    let args = request.args;

    match action {
        CREATE => {
            let mut args_iter = Vec::from_bytes(&args).into_iter();
            if let (Some(key), Some(value)) = (args_iter.next(), args_iter.next()) {
                snapshot.add(key, (NString, value.as_bytes().to_vec()));
            } else {
                return Error("incorrect request".to_string());
            }

            Success
        }
        DELETE => {
            let size = args[0] as usize;
            let key_bytes = &args[1..=size];
            let key = str::from_utf8(key_bytes).expect("");
            snapshot.delete(key);

            Success
        }
        POPF => process_pop_first_request(snapshot, &args),

        POPL => process_pop_last_request(snapshot, &args),

        PUSHF => {
            let content_type = request.content_type.clone();
            process_push_first_request(snapshot, &args, content_type)
        }
        PUSHL => {
            let content_type = request.content_type.clone();
            process_push_last_request(snapshot, &args, content_type)
        }
        _ => unreachable!(),
    }
}

fn extract_serialized_key_value(
    args: &[u8],
    content_type: ContentType,
) -> (String, (ContentType, Vec<u8>)) {
    let (k, v) = match content_type {
        KeyValue(value_type) => match *value_type {
            NString => {
                let (k, v) = extract_key_value::<String>(args);
                let value_bytes = v.as_bytes();
                (k, (NString, value_bytes.to_vec()))
            }
            NInt => {
                let (k, v) = extract_key_value::<u8>(args);
                let value_bytes = v.to_bytes();
                (k, (NInt, value_bytes.clone()))
            }
            NDeque(deque_type) => {
                let size = args[0] as usize;
                let key = String::from_bytes(&args[1..=size]);
                (key, (NDeque(deque_type), Vec::new()))
            }
            _ => unreachable!(),
        },

        _ => unreachable!(),
    };
    (k, v)
}

fn get_deque_and_push_value(
    args: &[u8],
    database: &mut NikkaDb,
) -> (Vec<u8>, String, Option<Value>) {
    let key_size = args[0] as usize;
    let key_bytes = &args[1..=key_size];

    let value_size = args[key_size + 1] as usize;
    let value_bytes = args[key_size + 2..key_size + 2 + value_size].to_vec();

    let key = String::from_bytes(key_bytes);

    let deque = database.get(&key);
    (value_bytes, key, deque)
}

fn process_get_request(
    database: &mut Arc<RwLock<NikkaDb>>,
    args: &[u8],
    content_type: &ContentType,
) -> Response {
    let size = args[0] as usize;
    let key_bytes = &args[1..=size];
    let key = str::from_utf8(key_bytes).expect("");
    let database = database.read().expect("cannot block database");
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
                    return Error("invalid key for string".to_string());
                }

                let v = vec![u8::from_bytes(&value.1)];
                ContentResponse(NInt, v)
            }

            None => ContentResponse(NNone, vec![]),
        },
        NNone => match database.get(key) {
            Some(value) => {
                let content_type = value.0;
                match content_type {
                    NString => {
                        let v = vec![String::from_bytes(&value.1)];
                        ContentResponse(NString, v.to_bytes())
                    }
                    NInt => {
                        let v = vec![u8::from_bytes(&value.1)];
                        ContentResponse(NInt, v)
                    }
                    _ => return Error(format!("invalid type to take from bd: {content_type:?}")),
                }
            }
            None => ContentResponse(NNone, vec![]),
        },
        _ => unreachable!(),
    };

    response
}

fn process_create_request(database: &mut NikkaDb, args: &[u8], content_type: ContentType) {
    let (k, v) = extract_serialized_key_value(args, content_type);
    database.add(k, v);
}

fn process_delete_request(database: &mut NikkaDb, args: &[u8]) {
    let size = args[0] as usize;
    let key_bytes = &args[1..=size];
    let key = str::from_utf8(key_bytes).expect("");
    database.delete(key);
}

fn process_pop_first_request(database: &mut NikkaDb, args: &[u8]) -> Response {
    let key = String::from_bytes(args);

    let value = database.pop_first(&key);

    match value {
        Some(value) => ContentResponse(value.0, value.1),
        None => ContentResponse(NNone, vec![]),
    }
}

fn process_pop_last_request(database: &mut NikkaDb, args: &[u8]) -> Response {
    let key = String::from_bytes(args);

    let value = database.pop_last(&key);

    match value {
        Some(value) => ContentResponse(value.0, value.1),
        None => ContentResponse(NNone, vec![]),
    }
}

fn process_push_last_request(
    database: &mut NikkaDb,
    args: &[u8],
    content_type: ContentType,
) -> Response {
    let (mut value_bytes, key, deque) = get_deque_and_push_value(args, database);

    match deque {
        None => Error("invalid key for deque".to_string()),

        Some(mut value) => {
            let KeyValue(deque_type) = content_type else {
                unreachable!()
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
                    let Ok(sep) = u8::try_from(value_bytes.len()) else {
                        return Error("key is too long".to_string());
                    };
                    value_bytes.push(sep);
                    value_bytes.insert(0, sep);
                    value.1.extend_from_slice(&value_bytes);
                    database.add(key, value);
                    Success
                }

                _ => unreachable!(),
            }
        }
    }
}

fn process_push_first_request(
    database: &mut NikkaDb,
    args: &[u8],
    content_type: ContentType,
) -> Response {
    let (mut value_bytes, key, deque) = get_deque_and_push_value(args, database);

    match deque {
        None => Error("invalid key for deque".to_string()),

        Some(mut value) => {
            let KeyValue(deque_type) = content_type else {
                unreachable!()
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
                    let Ok(sep) = u8::try_from(value_bytes.len()) else {
                        return Error("key is too long".to_string());
                    };
                    value_bytes.push(sep);
                    value_bytes.insert(0, sep);
                    value.1.splice(0..0, value_bytes);
                    database.add(key, value);
                    Success
                }

                _ => unreachable!(),
            }
        }
    }
}
