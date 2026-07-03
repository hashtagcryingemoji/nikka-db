use nikkadb_client::client::NikkaClient;
use nikkadb_server::server::NikkaServer;
use std::thread::spawn;

fn main() {
    basic();
    transaction();
}

fn basic() {
    spawn(|| {
        let db = NikkaServer::with_port("5434");
        db.run();
    });

    let mut client = NikkaClient::with_port("5434");

    client.set_string("language:mascot:go", "gopher");
    client.set_string("language:mascot:java", "duke");
    client.set_string("language:framework:java", "spring");
    client.set_string("language:framework:rust", "axum");

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

    client.set_string("language:framework:typescript", "next.js");
    client.set_string("language:framework:javascript", "react");

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
    spawn(|| {
        let db = NikkaServer::with_port("6767");
        db.run();
    });

    let mut client = NikkaClient::with_port("6767");

    client.begin_transaction();
    client.set_string("golang", "good");
    client.erase_transaction();
    client.set_string("java", "good");
    client.send_transaction();

    println!(
        "{}",
        client
            .get_string("golang")
            .unwrap_or("undefined".to_string())
    );
}
