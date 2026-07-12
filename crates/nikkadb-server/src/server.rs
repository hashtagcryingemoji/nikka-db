use crate::database::NikkaDb;
use crate::utils::builder::NikkaBuilder;
use crate::utils::parser::parse;
use crate::ClientState::DEFAULT;
use crate::{
    extract_serialized_key_value, process_pop_first_request, process_pop_last_request,
    process_push_first_request, process_push_last_request, Client,
};
use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use shared::protocol::Response::Success;
use shared::protocol::{form_packet, Request};
use shared::Action::{CREATE, DELETE, POPF, POPL, PUSHF, PUSHL};
use shared::{Deserializable, Serializable};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::ErrorKind::WouldBlock;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

pub struct NikkaServer {
    pub(crate) database: NikkaDb,
    pub(crate) clients: HashMap<usize, Client>,
    pub(crate) tcp_listener: TcpListener,
    pub(crate) backup_notifier: Sender<bool>,
    pub(crate) backup_receiver: Receiver<bool>,
    pub(crate) backup: File,
    pub(crate) wal: File,
    pub(crate) backup_counter: u32,
}

const SERVER: Token = Token(0);
impl NikkaServer {
    pub fn get_port(&self) -> u16 {
        self.tcp_listener
            .local_addr()
            .expect("cannot reach to tcp listener")
            .port()
    }

    pub fn run(self) {
        let mut curr_oper_count = 0;
        let mut unique = 1;
        let mut id_client_map = self.clients;
        let mut poll = Poll::new().expect("cannot poll");
        let mut events = Events::with_capacity(128);
        let mut server = self.tcp_listener;

        poll.registry()
            .register(&mut server, SERVER, Interest::READABLE)
            .expect("cannot register server");

        let mut rw_lock = Arc::new(RwLock::new(self.database));
        let rw_lock_clone = Arc::clone(&rw_lock);

        let wal_mutex = Arc::new(Mutex::new(self.wal));
        let wal_mutex_clone = Arc::clone(&wal_mutex);

        thread::spawn(move || {
            backup_control(
                &rw_lock_clone,
                &self.backup_receiver,
                self.backup,
                &wal_mutex_clone,
            );
        });

        'outer_loop: loop {
            poll.poll(&mut events, None).expect("cannot poll");

            for event in &events {
                match event.token() {
                    SERVER => loop {
                        match server.accept() {
                            Ok(mut incoming) => {
                                poll.registry()
                                    .register(&mut incoming.0, Token(unique), Interest::READABLE)
                                    .expect("cannot reg client");
                                id_client_map.insert(
                                    unique,
                                    Client {
                                        socket: incoming.0,
                                        state: DEFAULT,
                                        queue: VecDeque::new(),
                                    },
                                );
                                unique += 1;
                            }
                            Err(e) if e.kind() == WouldBlock => break,
                            _ => unreachable!(),
                        }
                    },
                    client_token => {
                        let client = id_client_map
                            .get_mut(&client_token.0)
                            .expect("cannot find client in the map");
                        let mut data_vec = Vec::new();

                        let mut reader = BufReader::new(&client.socket);

                        'harvesting: loop {
                            #[allow(unused_assignments)]
                            let mut consumed_data_size = 0;
                            match reader.fill_buf() {
                                Ok(buffer) => {
                                    if buffer.is_empty() {
                                        id_client_map.remove(&client_token.0);
                                        continue 'outer_loop;
                                    }

                                    let request_len = buffer[0];

                                    if buffer.len() > request_len as usize {
                                        consumed_data_size = request_len + 1;
                                        let data = &buffer[1..consumed_data_size as usize];

                                        data_vec.push(data.to_vec());
                                    } else {
                                        break 'harvesting;
                                    }
                                }

                                Err(ref e) if e.kind() == WouldBlock => {
                                    if data_vec.is_empty() {
                                        continue 'outer_loop;
                                    }
                                    break;
                                }

                                Err(_) => panic!(),
                            }

                            if consumed_data_size > 0 {
                                reader.consume(consumed_data_size as usize);
                            }
                        }

                        for bytes in &data_vec {
                            let request = Request::from_bytes(bytes);

                            if client.should_be_transaction(&request) {
                                client.queue.push_back(request);
                                let mut wclient = &client.socket;
                                let response_bytes = form_packet(&Success);
                                wclient
                                    .write_all(&response_bytes)
                                    .expect("error occurred while writing a message");
                                continue;
                            }

                            let response =
                                client.process_action(&request, &mut rw_lock, &wal_mutex);

                            let response_bytes = form_packet(&response);

                            // add response bytes len to form a readable packet

                            let mut wclient = &client.socket;

                            wclient
                                .write_all(&response_bytes)
                                .expect("error occurred while writing a message");

                            curr_oper_count += 1;

                            if curr_oper_count >= self.backup_counter {
                                self.backup_notifier.send(true).expect(
                                    "cannot send backup notification, probably side thread is dead",
                                );
                                curr_oper_count = 0;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn from_config_or_default(config_name: &str) -> Self {
        let Some(config_values) = parse(config_name) else {
            return NikkaBuilder::new().build();
        };

        let host = config_values.get("host").map(|raw_host| {
            let host_repr: Vec<u8> = raw_host
                .split(".")
                .map(|x| x.parse::<u8>().expect(&format!("invalid host: {x}")))
                .collect();
            if host_repr.len() != 4 {
                panic!("invalid host: {raw_host}")
            }

            (host_repr[0], host_repr[1], host_repr[2], host_repr[3])
        });

        let builder = NikkaBuilder {
            host,
            port: config_values.get("port").map(|x| x.parse::<u16>().unwrap()),
            backup_operations_count: config_values
                .get("backup_operations_count")
                .map(|x| x.parse::<u32>().unwrap()),
            backup: config_values.get("backup_name").map(|x| x.as_str()),
            wal: config_values.get("wal_name").map(|x| x.as_str()),
        };

        builder.build()
    }
}

fn backup_control(
    database: &Arc<RwLock<NikkaDb>>,
    receiver: &Receiver<bool>,
    mut backup_file: File,
    wal: &Arc<Mutex<File>>,
) {
    loop {
        while receiver.recv().is_ok() {
            let db = database
                .read()
                .expect("error when trying to access a db mutex");
            let hm = db.storage.to_bytes();
            drop(db);

            let mut wal = wal.lock().expect("error while locking wal");
            wal.set_len(0).expect("cannot truncate wal");
            wal.seek(SeekFrom::Start(0)).expect("cannot access wal");
            drop(wal);

            backup_file
                .seek(SeekFrom::Start(0))
                .expect("cannot access file");
            backup_file.set_len(0).expect("cannot access file");

            let mut buffer = BufWriter::new(&backup_file);
            buffer.write_all(&hm).expect("cannot access file");
            buffer.flush().expect("cannot access file");
        }
    }
}

pub(crate) fn update_from_wal(database: &mut NikkaDb, mut wal: &File) {
    let mut raw_requests = vec![];
    let mut requests = Vec::new();
    wal.seek(SeekFrom::Start(0)).expect("cannot reach wal file");

    let mut buffer = BufReader::new(wal);

    buffer
        .read_to_end(&mut raw_requests)
        .expect("cannot reach wal file");

    let mut index = 0;

    while index < raw_requests.len() {
        let request = Request::from_bytes(&raw_requests[index..]);
        index += request.to_bytes().len();
        requests.push(request);
    }

    for request in requests {
        match request.action {
            CREATE => {
                let (k, v) = extract_serialized_key_value(&request.args, request.content_type);
                database.add(k, v);
            }
            DELETE => {
                let key = &Vec::from_bytes(&request.args)[0];
                database.delete(key);
            }
            POPF => {
                process_pop_first_request(database, &request.args);
            }
            POPL => {
                process_pop_last_request(database, &request.args);
            }
            PUSHF => {
                process_push_first_request(database, &request.args, request.content_type);
            }
            PUSHL => {
                process_push_last_request(database, &request.args, request.content_type);
            }
            _ => unreachable!(),
        }
    }
}
