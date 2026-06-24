use crate::database::NikkaDb;
use shared::ContentType::{NNone, NString};
use shared::{
    Action::{CREATE, DELETE, GET, REGEX},
    Request, Response, Serializable,
};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

pub struct NikkaServer {
    database: NikkaDb,
    clients: Vec<TcpStream>,
    tcp_listener: TcpListener,
    //log: File,
}

impl NikkaServer {
    pub fn new_with_port(port: &str) {
        let mut serv = NikkaServer {
            database: NikkaDb::new(),
            clients: Vec::new(),
            tcp_listener: TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap(),
            //log: File::create_new("log").unwrap(),
        };
        let (tx, rx) = mpsc::channel::<TcpStream>();
        let listener = serv.tcp_listener;

        thread::spawn(move || {
            println!("server is up");
            loop {
                match listener.accept() {
                    Ok((socket, _)) => {
                        println!("client connected");
                        if tx.send(socket).is_err() {
                            break;
                        }
                    }

                    Err(_) => {},
                }
            }
        });

        'outer_loop: loop {
            while let Ok(new_socket) = rx.try_recv() {
                println!("client recieved");
                serv.clients.push(new_socket);
            }

            for i in (0..serv.clients.len()).rev() {
                let client = &mut serv.clients[i];

                //client.set_read_timeout(Some(Duration::from_secs(1)));

                let mut buffer = [0u8; 1];

                match client.read_exact(&mut buffer) {
                    Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => {
                        if buffer[0] == 0 {
                            serv.clients.remove(i);
                            println!("client disconnected");
                            continue;
                        }
                    }
                    Err(_) => continue 'outer_loop,
                    Ok(()) => {
                        if buffer[0] == 0 {
                            serv.clients.remove(i);
                            println!("client disconnected");
                            continue;
                        }
                    }
                }

                if buffer[0] == 0 {
                    continue;
                }

                println!("content size recieved: {buffer:?}");

                let mut buffer = vec![0u8; buffer[0] as usize];

                client
                    .read_exact(&mut buffer)
                    .expect("error occurred while reading a packet");

                println!("content read: {buffer:?}");

                let request: Request<String> = Request::from_bytes(&buffer);

                //serv.log.seek(SeekFrom::End(0)).expect("log is broken");
                //serv.log.write_all(&buffer).expect("cannot write to buffer");

                let action = request.action;

                let args = request.args;

                match action {
                    GET => {
                        let key = &args[0];
                        let value = serv.database.get(key);

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

                        client
                            .write_all(&response_byte)
                            .expect("error occurred while writing a message");
                    }
                    CREATE => {
                        let mut args_iter = args.into_iter();
                        if let (Some(key), Some(value)) = (args_iter.next(), args_iter.next()) {
                            serv.database.add(key, value);
                        } else {
                            panic!("incorrect request");
                        }
                    }
                    DELETE => {
                        let key = &args[0];
                        serv.database.delete(key);
                    }
                    REGEX => {
                        let regex = &args[0];

                        let content = serv.database.find_regex(regex);

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

                        client
                            .write_all(&response_byte)
                            .expect("error occurred while writing a message");
                    }
                }
            }
        }
    }
}
