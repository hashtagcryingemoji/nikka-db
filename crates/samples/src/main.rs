use nikkadb_client::client::NikkaClient;
use nikkadb_client::NikkaType::TypeString;
use nikkadb_client::NikkaTypeWrapper;
use nikkadb_server::server::NikkaServer;
use std::thread::{sleep, spawn};
use std::time::Duration;


fn main() {}

fn basic() {
    let db = NikkaServer::with_port("0");
    let port = db.tcp_listener.local_addr().unwrap().port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    let _ = client.set_string("language:mascot:go", "gopher");
    let _ = client.set_string("language:mascot:java", "duke");
    let _ = client.set_string("language:framework:java", "spring");
    let _ = client.set_string("language:framework:rust", "axum");

    println!("all about java");
    for query in client.get_regex("language:*:java") {
        println!(
            "{} - {}",
            query,
            client.get_string(&query).unwrap_or("undefined".to_string())
        );
    }

    println!("take a look on some of the frameworks");
    for query in client.get_regex("language:framework:*") {
        println!(
            "{} - {}",
            query,
            client.get_string(&query).unwrap_or("undefined".to_string())
        );
    }

    println!("everything about everyone");
    for query in client.get_regex("*:*:*") {
        println!(
            "{} - {}",
            query,
            client.get_string(&query).unwrap_or("undefined".to_string())
        );
    }

    let _ = client.set_string("language:framework:typescript", "next.js");
    let _ = client.set_string("language:framework:javascript", "react");

    println!("know the difference!");
    for query in client.get_regex("*:*:%%%%script") {
        println!(
            "{} - {}",
            query,
            client.get_string(&query).unwrap_or("undefined".to_string())
        );
    }

    println!("so similar but so different");
    for query in client.get_regex("*:framework:j*") {
        println!(
            "{} - {}",
            query,
            client.get_string(&query).unwrap_or("undefined".to_string())
        );
    }
}

fn transaction() {
    let db = NikkaServer::with_port("0");
    let port = db.tcp_listener.local_addr().unwrap().port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    client.begin_transaction();
    let _ = client.set_string("one", "1");
    client.erase_transaction();
    let _ = client.set_string("two", "2");
    client.send_transaction();

    println!(
        "{}",
        client.get_string("one").unwrap_or("undefined".to_string())
    );
}

fn deque() {
    let db = NikkaServer::with_port("0");
    let port = db.tcp_listener.local_addr().unwrap().port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    let _ = client.create_deque("tasks", TypeString);
    let _ = client.push_first("tasks", NikkaTypeWrapper::NikkaString("eat"));
    let _ = client.push_last("tasks", NikkaTypeWrapper::NikkaString("dota2"));
    let _ = client.push_last("tasks", NikkaTypeWrapper::NikkaString("repeat")); // tasks: [eat, dota2, sleep]
    println!("{}", client.pop_first::<String>("tasks").unwrap()); // eat
}
