use crate::database::NikkaDb;
use crate::utils::trie::TrieNode;
use shared::ContentType::{NNone, NString};
use shared::{
    Action,
    Action::{CREATE, DELETE, GET, REGEX},
    Request, Response, Serializable,
};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::TryRecvError;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct NikkaServer {
    database: NikkaDb,
    clients: Vec<TcpStream>,
    tcp_listener: TcpListener,
    backup_notifier: Sender<bool>,
    backup_receiver: Receiver<bool>,
    log: File,
    backup: File,
}

impl Default for NikkaServer {
    fn default() -> Self {
        Self::new_with_port("1402")
    }
}

impl NikkaServer {
    pub fn new_with_port(port: &str) -> Self {
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

        //println!("backup has been reached: {:?}", storage_backup_raw);

        let storage = HashMap::from_bytes(&storage_backup_raw);
        let mut trie = TrieNode::new();

        for (k, _) in &storage {
            trie.insert(k);
        }

        //println!("storage from backup {:?}", storage);

        let database = NikkaDb { storage, trie };

        NikkaServer {
            database,
            clients: Vec::new(),
            tcp_listener: TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap(),
            backup_notifier,
            backup_receiver,
            log,
            backup,
        }
    }

    pub fn run(port: &str) {
        let mut counter = 0;

        let mut serv = NikkaServer::new_with_port(port);
        let (tx, rx) = channel::<TcpStream>();
        let listener = serv.tcp_listener;

        let mutex = Arc::new(Mutex::new(serv.database));
        let mutex_clone = Arc::clone(&mutex);

        thread::spawn(move || {
            backup_control(&mutex_clone, serv.backup_receiver, serv.backup);
        });

        thread::spawn(move || {
            //println!("server is up");
            loop {
                match listener.accept() {
                    Ok((socket, _)) => {
                        //println!("client connected");
                        if tx.send(socket).is_err() {
                            break;
                        }
                    }

                    Err(e) => {
                        panic!("unmatched error occurred: {e}")
                    }
                }
            }
        });

        'outer_loop: loop {
            while let Ok(new_socket) = rx.try_recv() {
                //println!("client recieved");
                serv.clients.push(new_socket);
            }

            for i in (0..serv.clients.len()).rev() {
                let mut rclient = &serv.clients[i];

                //client.set_read_timeout(Some(Duration::from_secs(1)));

                let mut buffer = [0u8; 1];

                match rclient.read_exact(&mut buffer) {
                    Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => {
                        if buffer[0] == 0 {
                            serv.clients.remove(i);
                            //println!("client disconnected");
                            continue;
                        }
                    }
                    Err(_) => continue 'outer_loop,
                    Ok(()) => {
                        if buffer[0] == 0 {
                            serv.clients.remove(i);
                            //println!("client disconnected");
                            continue;
                        }
                    }
                }

                if buffer[0] == 0 {
                    continue;
                }

                //println!("content size recieved: {buffer:?}");

                let mut buffer = vec![0u8; buffer[0] as usize];

                rclient
                    .read_exact(&mut buffer)
                    .expect("error occurred while reading a packet");

                //println!("content read: {buffer:?}");

                let request: Request<String> = Request::from_bytes(&buffer);

                serv.log.seek(SeekFrom::End(0)).expect("log is broken");
                serv.log.write_all(&buffer).expect("cannot write to buffer");

                let action = request.action;

                let args = request.args;

                let response_bytes = process_action(action, args, &mutex);

                let mut wclient = &serv.clients[i];

                wclient
                    .write_all(&response_bytes)
                    .expect("error occurred while writing a message");

                counter += 1;

                if counter >= 100 {
                    serv.backup_notifier
                        .send(true)
                        .expect("cannot send backup notification, probably side thread is dead");
                }
            }
        }
    }
}

fn process_action(action: Action, args: Vec<String>, mutex: &Arc<Mutex<NikkaDb>>) -> Vec<u8> {
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
    }
}

fn backup_control(database: &Arc<Mutex<NikkaDb>>, receiver: Receiver<bool>, mut backup_file: File) {
    //println!("backup control");
    loop {
        match receiver.try_recv() {
            Ok(_) => {
                let db = database.lock().unwrap();
                let hm = db.storage.as_bytes();
                drop(db);

                //println!("storage in backup {:?}", hm);

                //println!("backup has been reached");
                backup_file.set_len(0).expect("TODO: panic message");
                backup_file
                    .seek(SeekFrom::Start(0))
                    .expect("TODO: panic message");
                backup_file.write_all(&hm).expect("TODO: panic message");
                backup_file.flush().expect("TODO: panic message");
            }
            Err(TryRecvError::Disconnected) => {
                break;
            }
            Err(TryRecvError::Empty) => thread::sleep(Duration::from_micros(100)),
        }
    }
}
