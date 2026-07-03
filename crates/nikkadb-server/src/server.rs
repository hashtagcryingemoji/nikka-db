use crate::database::NikkaDb;
use crate::utils::trie::TrieNode;
use crate::Client;
use crate::ClientState::{DEFAULT, TRANSACTION};
use shared::protocol::Response::Success;
use shared::protocol::{form_packet, Request};
use shared::Action::TERASE;
use shared::{
    Action::{TDISCARD, TEND},
    Serializable,
};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::ErrorKind::WouldBlock;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;

pub struct NikkaServer {
    database: NikkaDb,
    clients: Vec<Client>,
    tcp_listener: TcpListener,
    backup_notifier: Sender<bool>,
    backup_receiver: Receiver<bool>,
    log: File,
    backup: File,
    backup_counter: u8,
}

impl Default for NikkaServer {
    fn default() -> Self {
        Self::with_port("1402")
    }
}

impl NikkaServer {
    pub fn new() -> Self {
        Self::with_port("1402")
    }

    pub fn with_port(port: &str) -> Self {
        let log = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("log")
            .expect("failed to open or create log file");

        let mut backup = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("backup")
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

        for (k, _) in &storage {
            trie.insert(k);
        }

        let database = NikkaDb { storage, trie };

        NikkaServer {
            database,
            clients: Vec::new(),
            tcp_listener: TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap(),
            backup_notifier,
            backup_receiver,
            log,
            backup,
            backup_counter: 0,
        }
    }

    pub fn run(mut self) {
        let (tx, rx) = channel::<TcpStream>();

        let mutex = Arc::new(Mutex::new(self.database));
        let mutex_clone = Arc::clone(&mutex);

        thread::spawn(move || {
            backup_control(&mutex_clone, &self.backup_receiver, self.backup);
        });

        thread::spawn(move || loop {
            match self.tcp_listener.accept() {
                Ok((socket, _)) => {
                    socket
                        .set_nonblocking(true)
                        .expect("cannot set socket to non blocking mode");
                    if tx.send(socket).is_err() {
                        break;
                    }
                }

                Err(e) => {
                    panic!("unmatched error occurred: {e}")
                }
            }
        });

        'outer_loop: loop {
            while let Ok(new_socket) = rx.try_recv() {
                let client = Client {
                    socket: new_socket,
                    state: DEFAULT,
                    queue: Default::default(),
                };

                self.clients.push(client);
            }

            for i in (0..self.clients.len()).rev() {
                let rclient = &self.clients[i].socket;

                let mut reader = BufReader::new(rclient);

                let mut data_vec = Vec::new();

                'harvesting: loop {
                    #[allow(unused_assignments)]
                    let mut consumed_data_size = 0;
                    match reader.fill_buf() {
                        Ok(buffer) => {
                            if buffer.len() < 1 {
                                self.clients.remove(i);
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
                                sleep(Duration::from_millis(100));
                                continue 'outer_loop;
                            } else {
                                break;
                            }
                        }

                        Err(_) => panic!(),
                    };

                    if consumed_data_size > 0 {
                        reader.consume(consumed_data_size as usize);
                    }
                }

                for bytes in &data_vec {
                    let request = Request::from_bytes(&bytes);

                    if self.clients[i].state == TRANSACTION
                        && (request.action != TDISCARD
                            && request.action != TEND
                            && request.action != TERASE)
                    {
                        self.clients[i].queue.push_back(request);
                        let mut wclient = &self.clients[i].socket;
                        let response_bytes = form_packet(Success);
                        wclient
                            .write_all(&response_bytes)
                            .expect("error occurred while writing a message");
                        continue;
                    }

                    let response_bytes =
                        form_packet(self.clients[i].process_action(request, &mutex));

                    // add response bytes len to form a readable packet

                    let mut wclient = &self.clients[i].socket;

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

fn backup_control(
    database: &Arc<Mutex<NikkaDb>>,
    receiver: &Receiver<bool>,
    mut backup_file: File,
) {
    loop {
        match receiver.try_recv() {
            Ok(_) => {
                let db = database.lock().unwrap();
                let hm = db.storage.as_bytes();
                drop(db);

                backup_file.set_len(0).expect("cannot access file");
                backup_file
                    .seek(SeekFrom::Start(0))
                    .expect("cannot access file");
                backup_file.write_all(&hm).expect("cannot access file");
                backup_file.flush().expect("cannot access file");
            }
            Err(TryRecvError::Disconnected) => {
                break;
            }
            Err(TryRecvError::Empty) => sleep(Duration::from_micros(100)),
        }
    }
}
