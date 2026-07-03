use nikkadb_client::client::NikkaClient;
use nikkadb_server::server::NikkaServer;
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

#[test]
fn element_insertion_test() {
    let db = NikkaServer::with_port("0");
    let port = db.tcp_listener.local_addr().unwrap().port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    db.set_string("value", "key");
    assert_eq!(db.get_string("value"), Some(String::from("key")));
    db.set_string("key", "value");
    db.set_int("one", 1);
    assert_eq!(db.get_int("one").unwrap(), 1);
}

#[test]
fn backup_test() {
    let db = NikkaServer::with_port("0");
    let port = db.tcp_listener.local_addr().unwrap().port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    for _ in 0..200 {
        db.set_string("key", "value");
    }

    sleep(Duration::from_secs(1));

    spawn(|| {
        let db = NikkaServer::with_port("2220");
        db.run();
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port("2220");

    assert_eq!(db.get_string("key"), Some("value".to_string()));
}

#[test]
fn element_delete_test() {
    let db = NikkaServer::with_port("0");
    let port = db.tcp_listener.local_addr().unwrap().port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    db.set_string("value", "key");
    db.remove("value");
    assert_eq!(db.get_string("value"), None);
}

#[test]
fn transaction_test() {
    spawn(|| {
        let db = NikkaServer::with_port("6766");
        db.run();
    });

    let mut client = NikkaClient::with_port("6766");

    client.begin_transaction();
    client.set_string("key1", "value");
    client.erase_transaction();
    client.set_string("key2", "value");
    client.send_transaction();

    assert_eq!(client.get_string("key1"), None);
    assert_eq!(client.get_string("key2").unwrap(), "value".to_string());
}

// #[test]
// fn regex_test() {
//     spawn(|| {
//         let db = NikkaServer::with_port("5431");
//         db.run()
//     });
//
//     sleep(Duration::from_millis(100));
//
//     let mut db = NikkaClient::with_port("5431");
//
//     db.set_string("alice:bob", "bob");
//     db.set_string("bob:alice", "alice");
//     let mut query = db.get_regex("*:*");
//     let mut real = vec!["alice:bob".to_string(), "bob:alice".to_string()];
//     query.sort();
//     real.sort();
//
//     assert_eq!(query, real);
// }
