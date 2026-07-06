use nikkadb_client::client::NikkaClient;
use nikkadb_client::NikkaType::{TypeInt, TypeString};
use nikkadb_client::NikkaTypeWrapper;
use nikkadb_server::server::NikkaServer;
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

#[test]
fn element_insertion_test() {
    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

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
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    for _ in 0..200 {
        db.set_string("key", "value");
    }

    db.create_deque("numbers", TypeInt);
    db.push_first("numbers", NikkaTypeWrapper::NikkaInt(1));
    db.push_last("numbers", NikkaTypeWrapper::NikkaInt(2));

    sleep(Duration::from_secs(1));

    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    assert_eq!(db.get_string("key"), Some("value".to_string()));
    assert_eq!(db.pop_first("numbers"), Some(1));
    db.set_int("should be in wal", 12);

    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    assert_eq!(db.get_int("should be in wal"), Some(12));
}

#[test]
fn element_delete_test() {
    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    db.set_string("value", "key");
    db.remove("value");
    assert_eq!(db.get_string("value"), None);
}

#[test]
fn transaction_test() {
    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    client.begin_transaction();
    client.set_string("key1", "value");
    client.erase_transaction();
    client.set_string("key2", "value");
    client.send_transaction();

    assert_eq!(client.get_string("key1"), None);
    assert_eq!(client.get_string("key2").unwrap(), "value".to_string());
}

#[test]
fn regex_test() {
    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    client.set_string("alice:bob", "bob");
    client.set_string("bob:alice", "alice");
    let mut query = client.get_regex("*:*");
    let mut real = vec!["alice:bob".to_string(), "bob:alice".to_string()];
    query.sort();
    real.sort();

    assert_eq!(query, real);
}

#[test]
fn clear_test() {
    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    client.set_string("one", "two");
    client.set_int("three", 3);
    client.clear_database();

    assert_eq!(client.get_string("one"), None);
    assert_eq!(client.get_int("three"), None);
}

#[test]
fn deque_test() {
    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    client.create_deque("numbers", TypeInt);
    client.push_first("numbers", NikkaTypeWrapper::NikkaInt(1));
    client.push_last("numbers", NikkaTypeWrapper::NikkaInt(2));
    assert_eq!(client.pop_first("numbers").unwrap_or(0), 1);
    assert_eq!(client.pop_last("numbers").unwrap_or(0), 2);
    assert_eq!(client.pop_last("numbers").unwrap_or(0), 0);

    client.create_deque("strings", TypeString);
    client.push_first("strings", NikkaTypeWrapper::NikkaString("one"));
    client.push_last("strings", NikkaTypeWrapper::NikkaString("two"));
    assert_eq!(
        client.pop_first("strings").unwrap_or("0".to_string()),
        "one".to_string()
    );
    assert_eq!(
        client.pop_last("strings").unwrap_or("0".to_string()),
        "two".to_string()
    );
    assert_eq!(
        client.pop_last("strings").unwrap_or("0".to_string()),
        "0".to_string()
    );
}
