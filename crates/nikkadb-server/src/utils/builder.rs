// pub struct NikkaServer {
//     database: NikkaDb,
//     clients: HashMap<usize, Client>,
//     tcp_listener: TcpListener,
//     backup_notifier: Sender<bool>,
//     backup_receiver: Receiver<bool>,
//     _log: File,
//     backup: File,
//     wal: File,
//     backup_counter: u8,
// }

use crate::database::NikkaDb;
// pub fn with_port(port: &str) -> Self {
//     let log = OpenOptions::new()
//         .read(true)
//         .write(true)
//         .create(true)
//         .open("log.nikka")
//         .expect("failed to open or create log file");
//
//     let mut backup = OpenOptions::new()
//         .read(true)
//         .write(true)
//         .create(true)
//         .open("backup.nikka")
//         .expect("failed to open or create backup file");
//     let (backup_notifier, backup_receiver) = channel::<bool>();
//     let mut storage_backup_raw = Vec::new();
//
//     backup
//         .seek(SeekFrom::Start(0))
//         .expect("cannot reach backup file");
//     backup
//         .read_to_end(&mut storage_backup_raw)
//         .expect("cannot reach backup file");
//
//     let storage = HashMap::from_bytes(&storage_backup_raw);
//     let mut trie = TrieNode::new();
//
//     for k in storage.keys() {
//         trie.insert(k);
//     }
//
//     let mut database = NikkaDb { storage, trie };
//
//     let wal = OpenOptions::new()
//         .read(true)
//         .write(true)
//         .create(true)
//         .open("wal.nikka")
//         .expect("failed to open or create wal file");
//
//     crate::server::update_from_wal(&mut database, &wal);
//
//     let localhost_v4 = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
//     let addr = SocketAddr::new(localhost_v4, port.parse::<u16>().expect("invalid port"));
//
//     NikkaServer {
//         database,
//         tcp_listener: TcpListener::bind(addr).expect("cannot bind"),
//         clients: HashMap::new(),
//         backup_notifier,
//         backup_receiver,
//         wal,
//         _log: log,
//         backup,
//         backup_counter: 0,
//     }
// }
use crate::server::NikkaServer;
use crate::utils::trie::TrieNode;
use mio::net::TcpListener;
use shared::Deserializable;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::mpsc::channel;

pub struct NikkaBuilder<'names> {
    pub(crate) host: Option<(u8, u8, u8, u8)>,
    pub(crate) port: Option<u16>,
    pub(crate) backup_operations_count: Option<u32>,
    pub(crate) backup: Option<&'names str>,
    pub(crate) wal: Option<&'names str>,
}

impl<'file_names> NikkaBuilder<'file_names> {
    pub fn new() -> NikkaBuilder<'file_names> {
        NikkaBuilder {
            host: None,
            port: None,
            backup_operations_count: None,
            backup: None,
            wal: None,
        }
    }

    pub fn host(mut self, a: u8, b: u8, c: u8, d: u8) -> Self {
        self.host = Some((a, b, c, d));
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn wal_name(mut self, wal_name: &'file_names str) -> Self {
        self.wal = Some(wal_name);
        self
    }

    pub fn backup_name(mut self, backup_name: &'file_names str) -> Self {
        self.backup = Some(backup_name);
        self
    }

    pub fn backup_operations_count(mut self, count: u32) -> Self {
        self.backup_operations_count = Some(count);
        self
    }

    pub fn build(&self) -> NikkaServer {
        let mut backup = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(self.backup.unwrap_or("backup.nikka"))
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
            .open(self.wal.unwrap_or("wal.nikka"))
            .expect("failed to open or create wal file");

        crate::server::update_from_wal(&mut database, &wal);

        let (a, b, c, d) = self.host.unwrap_or((127, 0, 0, 1));

        let localhost_v4 = IpAddr::V4(Ipv4Addr::new(a, b, c, d));
        let addr = SocketAddr::new(localhost_v4, self.port.unwrap_or(0));

        NikkaServer {
            database,
            tcp_listener: TcpListener::bind(addr).expect("cannot bind"),
            clients: HashMap::new(),
            backup_notifier,
            backup_receiver,
            wal,
            backup,
            backup_counter: self.backup_operations_count.unwrap_or(10000),
        }
    }
}
