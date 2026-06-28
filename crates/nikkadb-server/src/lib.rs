use crate::database::NikkaDb;
use crate::ClientState::{DEFAULT, TRANSACTION};
use shared::Action::{CREATE, DELETE, GET, REGEX, TDISCARD, TEND, TERASE, TSTART};
use shared::ContentType::{NNone, NString};
use shared::{Request, Response, Serializable};
use std::collections::{HashMap, VecDeque};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

mod database;
pub mod server;
#[cfg(not(feature = "utils_for_test"))]
pub(crate) mod utils;

#[cfg(feature = "utils_for_test")]
pub mod utils;

struct Client {
    socket: TcpStream,
    state: ClientState,
    queue: VecDeque<Request<String>>,
}

#[derive(PartialEq)]
enum ClientState {
    DEFAULT,
    TRANSACTION,
}

impl Client {
    fn process_action(&mut self, request: Request<String>, mutex: &Arc<Mutex<NikkaDb>>) -> Vec<u8> {
        let action = request.action;
        let args = request.args;

        match action {
            GET => {
                let key = &args[0];
                let database = mutex.lock().unwrap();
                let value = database.get(key);
                drop(database);

                let response = match value {
                    Some(value) => {
                        let v = vec![value];
                        Response {
                            size: 1 + v[0].len() as u8,
                            content_type: NString,
                            content: v,
                        }
                    }
                    None => Response {
                        size: 1,
                        content_type: NNone,
                        content: Vec::new(),
                    },
                };

                let response_byte = response.as_bytes();

                response_byte
            }
            CREATE => {
                let mut args_iter = args.into_iter();
                if let (Some(key), Some(value)) = (args_iter.next(), args_iter.next()) {
                    let mut database = mutex.lock().unwrap();
                    database.add(key, value);
                    drop(database);
                } else {
                    panic!("incorrect request");
                }

                Vec::new()
            }
            DELETE => {
                let key = &args[0];
                let mut database = mutex.lock().unwrap();
                database.delete(key);
                drop(database);

                Vec::new()
            }
            REGEX => {
                let regex = &args[0];

                let database = mutex.lock().unwrap();
                let content = database.find_regex(regex);
                drop(database);

                let mut size = 1;

                for piece in &content {
                    size += piece.len() as u8;
                }

                let response = Response {
                    size,
                    content_type: NString,
                    content,
                };

                let response_byte = response.as_bytes();

                response_byte
            }
            TSTART => {
                self.state = TRANSACTION;
                Vec::new()
            }
            TEND => {
                self.state = DEFAULT;
                self.process_transaction(mutex)
            }
            TERASE => {
                self.queue.clear();
                Vec::new()
            }
            TDISCARD => {
                self.state = DEFAULT;
                self.queue.clear();
                Vec::new()
            }
        }
    }

    fn process_transaction(&mut self, mutex: &Arc<Mutex<NikkaDb>>) -> Vec<u8> {
        let mut database = mutex.lock().unwrap();
        let mut snapshot = database.storage.clone();

        for request in &self.queue {
            let request = request.clone();
            process_in_transaction(request, &mut snapshot);
        }

        database.storage = snapshot;

        Vec::new()
    }
}

fn process_in_transaction(
    request: Request<String>,
    snapshot: &mut HashMap<String, String>,
) -> Vec<u8> {
    let action = request.action;
    let args = request.args;

    match action {
        CREATE => {
            let mut args_iter = args.into_iter();
            if let (Some(key), Some(value)) = (args_iter.next(), args_iter.next()) {
                snapshot.insert(key, value);
            } else {
                panic!("incorrect request");
            }

            Vec::new()
        }
        DELETE => {
            let key = &args[0];
            snapshot.remove(key);

            Vec::new()
        }
        _ => panic!("logic error"),
    }
}
