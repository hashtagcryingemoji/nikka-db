use nikkadb_client::client::NikkaClient;
use nikkadb_server::server::NikkaServer;
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

#[test]
fn element_insertion_test() {
    spawn(|| {
        let db = NikkaServer::with_port("5433");
        db.run()
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port("5433");

    db.set_string("value", "key");
    assert_eq!(db.get("value"), Some(String::from("key")));
    db.set_string("key", "value");
}

#[test]
fn backup_test() {
    spawn(|| {
        let db = NikkaServer::with_port("2221");
        db.run();
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port("2221");

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

    assert_eq!(db.get("key"), Some("value".to_string()));
}

#[test]
fn element_delete_test() {
    spawn(|| {
        let db = NikkaServer::new();
        db.run();
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::default();

    db.set_string("value", "key");
    db.remove("value");
    assert_eq!(db.get("value"), None);
}

#[test]
fn transaction_test() {
    spawn(|| {
        let db = NikkaServer::with_port("67676");
        db.run();
    });

    let mut client = NikkaClient::with_port("67676");

    client.begin_transaction();
    client.set_string("key1", "value");
    client.erase_transaction();
    client.set_string("key2", "value");
    client.send_transaction();

    assert_eq!(client.get("key1"), None);
    assert_eq!(client.get("key2").unwrap(), "value".to_string());
}

#[test]
fn regex_test() {
    spawn(|| {
        let db = NikkaServer::with_port("5433");
        db.run()
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port("5433");

    db.set_string("alice:bob", "bob");
    db.set_string("bob:alice", "alice");
    let mut query = db.get_regex("*:*");
    let mut real = vec!["alice:bob".to_string(), "bob:alice".to_string()];
    query.sort();
    real.sort();

    assert_eq!(query, real);
}
