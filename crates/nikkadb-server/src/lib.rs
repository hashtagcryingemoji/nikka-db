use crate::database::NikkaDb;
use crate::ClientState::{DEFAULT, TRANSACTION};
use shared::protocol::Response::{ContentResponse, Success};
use shared::protocol::{extract_key_value, Request, Response};
use shared::Action::{CREATE, DELETE, GET, REGEX, TDISCARD, TEND, TERASE, TSTART};
use shared::ContentType::{KeyValue, NInt, NNone, NString};
use shared::{ContentType, Serializable};
use std::collections::{HashMap, VecDeque};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

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
                let key = &Vec::from_bytes(&args)[0];
                let database = mutex.lock().unwrap();
                let response = match request.content_type {
                    NString => match database.get::<String>(key) {
                        Some(value) => {
                            let v = vec![value];
                            ContentResponse(NString, v.as_bytes())
                        }
                        None => ContentResponse(NNone, vec![]),
                    },
                    NInt => match database.get::<u8>(key) {
                        Some(int) => {
                            let v = vec![int];
                            ContentResponse(NInt, v)
                        }

                        None => ContentResponse(NNone, vec![]),
                    },
                    _ => panic!("logic error"),
                };
                drop(database);

                response
            }
            CREATE => {
                let content_type = request.content_type;

                let (k, v) = match content_type {
                    KeyValue(value_type) => match *value_type {
                        NString => {
                            let (k, v) = extract_key_value::<String>(&args);
                            let value_bytes = v.as_bytes();
                            (k, (NString, value_bytes.to_vec()))
                        }
                        NInt => {
                            let (k, v) = extract_key_value::<u8>(&args);
                            let value_bytes = v.as_bytes();
                            (k, (NInt, value_bytes.to_vec()))
                        }
                        _ => panic!("logic error"),
                    },

                    _ => panic!("logic error"),
                };
                let mut database = mutex.lock().unwrap();
                database.add(k, v);
                drop(database);

                Success
            }
            DELETE => {
                let key = &Vec::from_bytes(&args)[0];
                let mut database = mutex.lock().unwrap();
                database.delete(key);
                drop(database);

                Success
            }
            REGEX => {
                let regex = &Vec::from_bytes(&args)[0];

                let database = mutex.lock().unwrap();
                let _content = database.find_regex(regex).as_bytes();
                drop(database);

                ContentResponse(NNone, vec![])
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
        }
    }

    fn process_transaction(&mut self, mutex: &Arc<Mutex<NikkaDb>>) -> Response {
        let mut database = mutex.lock().unwrap();
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
