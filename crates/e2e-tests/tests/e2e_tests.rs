use nikkadb_client::client::NikkaClient;
use nikkadb_server::server::NikkaServer;
use std::thread::sleep;
use std::thread::spawn;
use std::time::Duration;

#[test]
fn element_insertion_test() {
    spawn(|| {
        let _ = NikkaServer::run("5433");
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
        let _ = NikkaServer::run("2221");
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port("2221");

    for _ in 0..200 {
        db.set_string("key", "value");
    }

    sleep(Duration::from_secs(1));

    spawn(|| {
        let _ = NikkaServer::run("2220");
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port("2220");

    assert_eq!(db.get("key"), Some("value".to_string()));
}

#[test]
fn element_delete_test() {
    spawn(|| {
        let _ = NikkaServer::run("1402");
    });

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::default();

    db.set_string("value", "key");
    db.remove("value");
    assert_eq!(db.get("value"), None);
}
