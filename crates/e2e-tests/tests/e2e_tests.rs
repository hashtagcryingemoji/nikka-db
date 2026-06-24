#[test]
fn element_insertion_test() {
    std::thread::spawn(|| {
        let _ = nikkadb_server::server::NikkaServer::new_with_port("5433");
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut db = nikkadb_client::client::NikkaClient::with_port("5433");

    db.add("value", "key");
    assert_eq!(db.get("value"), Some(String::from("key")))
}

#[test]
fn element_delete_test() {
    std::thread::spawn(|| {
        let _ = nikkadb_server::server::NikkaServer::new_with_port("5434");
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let mut db = nikkadb_client::client::NikkaClient::with_port("5434");

    db.add("value", "key");
    db.remove("value");
    assert_eq!(db.get("value"), None)
}
