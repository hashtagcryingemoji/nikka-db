use crate::database::NikkaDb;
use crate::utils::trie::TrieNode;
use crate::ClientState::{DEFAULT, TRANSACTION};
use crate::{
    extract_serialized_key_value, process_pop_first_request, process_pop_last_request,
    process_push_first_request, process_push_last_request, Client,
};
use mio::net::TcpListener;
use mio::{Events, Interest, Poll, Token};
use shared::protocol::Response::Success;
use shared::protocol::{form_packet, Request};
use shared::Action::{CREATE, DELETE, POPF, POPL, PUSHF, PUSHL, TERASE};
use shared::{
    Action::{TDISCARD, TEND},
    Serializable,
};
use std::collections::{HashMap, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::ErrorKind::WouldBlock;
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

pub struct NikkaServer {
    database: NikkaDb,
    clients: HashMap<usize, Client>,
    tcp_listener: TcpListener,
    backup_notifier: Sender<bool>,
    backup_receiver: Receiver<bool>,
    _log: File,
    backup: File,
    wal: File,
    backup_counter: u8,
}

impl Default for NikkaServer {
    #[cold]
    fn default() -> Self {
        Self::with_port("1402")
    }
}

const SERVER: Token = Token(0);
impl NikkaServer {
    #[cold]
    pub fn new() -> Self {
        Self::with_port("1402")
    }

    #[cold]
    pub fn with_port(port: &str) -> Self {
        let log = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("log.nikka")
            .expect("failed to open or create log file");

        let mut backup = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("backup.nikka")
            .expect("failed to open or create backup file");
        let (backup_notifier, backup_receiver) = channel::<bool>();
        let mut storage_backup_raw = Vec::new();

        backup
            .seek(SeekFrom::Start(0))
            .expect("cannot reach backup file");
        backup
            .read_to_end(&mut storage_backup_raw)
            .expect("cannot reach backup file");

        let storage = HashMap::from_bytes(&storage_backup_raw);
        let mut trie = TrieNode::new();

        for k in storage.keys() {
            trie.insert(k);
        }

        let mut database = NikkaDb { storage, trie };

        let wal = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("wal.nikka")
            .expect("failed to open or create wal file");

        update_from_wal(&mut database, &wal);

        let localhost_v4 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let addr = SocketAddr::new(localhost_v4, port.parse::<u16>().expect("invalid port"));

        NikkaServer {
            database,
            tcp_listener: TcpListener::bind(addr).expect("cannot bind"),
            clients: HashMap::new(),
            backup_notifier,
            backup_receiver,
            wal,
            _log: log,
            backup,
            backup_counter: 0,
        }
    }

    pub fn get_port(&self) -> u16 {
        self.tcp_listener
            .local_addr()
            .expect("cannot reach to tcp listener")
            .port()
    }

    pub fn run(mut self) {
        let mut unique = 1;
        let mut id_client_map = self.clients;
        let mut poll = Poll::new().expect("cannot poll");
        let mut events = Events::with_capacity(128);
        let mut server = self.tcp_listener;

        poll.registry()
            .register(&mut server, SERVER, Interest::READABLE)
            .expect("cannot register server");

        let mutex = Arc::new(Mutex::new(self.database));
        let mutex_clone = Arc::clone(&mutex);

        let wal_mutex = Arc::new(Mutex::new(self.wal));
        let wal_mutex_clone = Arc::clone(&wal_mutex);

        thread::spawn(move || {
            backup_control(
                &mutex_clone,
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
                            let mut database = mutex.lock().expect("cannot access db mutex");
                            let request = Request::from_bytes(bytes);

                            if client.state == TRANSACTION
                                && (request.action != TDISCARD
                                    && request.action != TEND
                                    && request.action != TERASE)
                            {
                                client.queue.push_back(request);
                                let mut wclient = &client.socket;
                                let response_bytes = form_packet(&Success);
                                wclient
                                    .write_all(&response_bytes)
                                    .expect("error occurred while writing a message");
                                continue;
                            }

                            if request.action == CREATE
                                || request.action == DELETE
                                || request.action == PUSHL
                                || request.action == PUSHF
                                || request.action == POPL
                                || request.action == POPF
                            {
                                let serialized_request = request.to_bytes();
                                let mut wal =
                                    wal_mutex.lock().expect("cannot lock mutex in main thread");
                                wal.write_all(&serialized_request)
                                    .expect("cannot write to wal");
                                wal.flush().expect("cannot write to wal");
                            }

                            let response_bytes =
                                form_packet(&client.process_action(request, &mut database));

                            // add response bytes len to form a readable packet

                            let mut wclient = &client.socket;

                            wclient
                                .write_all(&response_bytes)
                                .expect("error occurred while writing a message");

                            self.backup_counter += 1;

                            if self.backup_counter >= 100 {
                                self.backup_notifier.send(true).expect(
                                    "cannot send backup notification, probably side thread is dead",
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn backup_control(
    database: &Arc<Mutex<NikkaDb>>,
    receiver: &Receiver<bool>,
    mut backup_file: File,
    wal: &Arc<Mutex<File>>,
) {
    loop {
        match receiver.try_recv() {
            Ok(_) => {
                let db = database
                    .lock()
                    .expect("error when trying to access a db mutex");
                let hm = db.storage.to_bytes();
                drop(db);

                let wal = wal.lock().expect("error while locking wal");
                wal.set_len(0).expect("cannot truncate wal");
                drop(wal);

                backup_file.set_len(0).expect("cannot access file");
                backup_file
                    .seek(SeekFrom::Start(0))
                    .expect("cannot access file");
                let mut buffer = BufWriter::new(&backup_file);
                buffer.write_all(&hm).expect("cannot access file");
                buffer.flush().expect("cannot access file");
            }
            Err(TryRecvError::Disconnected) => {
                break;
            }
            Err(TryRecvError::Empty) => sleep(Duration::from_micros(100)),
        }
    }
}

fn update_from_wal(database: &mut NikkaDb, mut wal: &File) {
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
