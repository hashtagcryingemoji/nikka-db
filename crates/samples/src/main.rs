use nikkadb_client::client::NikkaClient;
use nikkadb_client::NikkaType::TypeString;
use nikkadb_client::NikkaTypeWrapper;
use nikkadb_server::server::NikkaServer;
use nikkadb_server::utils::builder::NikkaBuilder;
use std::thread::{sleep, spawn};
use std::time::Duration;

fn main() {}

fn basic() {
    let db = NikkaBuilder::new().build();
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    let _ = client.set("language:mascot:go", "gopher");
    let _ = client.set("language:mascot:java", "duke");
    let _ = client.set::<&str>("language:framework:java", "spring"); // type safety
    let _ = client.set::<&str>("language:framework:rust", "axum");

    println!("all about java");
    for query in client.get_regex("language:*:java").unwrap().unwrap() {
        println!(
            "{} - {}",
            query,
            client
                .get::<String>(&query)
                .unwrap()
                .unwrap_or("undefined".to_string())
        );
    }

    println!("take a look on some of the frameworks");
    for query in client.get_regex("language:framework:*").unwrap().unwrap() {
        println!(
            "{} - {}",
            query,
            client
                .get::<String>(&query)
                .unwrap()
                .unwrap_or("undefined".to_string())
        );
    }

    println!("everything about everyone");
    for query in client.get_regex("*:*:*").unwrap().unwrap() {
        println!(
            "{} - {}",
            query,
            client
                .get::<String>(&query)
                .unwrap()
                .unwrap_or("undefined".to_string())
        );
    }

    let _ = client.set("language:framework:typescript", "next.js");
    let _ = client.set("language:framework:javascript", "react");

    println!("know the difference!");
    for query in client.get_regex("*:*:%%%%script").unwrap().unwrap() {
        println!(
            "{} - {}",
            query,
            client
                .get::<String>(&query)
                .unwrap()
                .unwrap_or("undefined".to_string())
        );
    }

    println!("so similar but so different");
    for query in client.get_regex("*:framework:j*").unwrap().unwrap() {
        println!(
            "{} - {}",
            query,
            client
                .get::<String>(&query)
                .unwrap()
                .unwrap_or("undefined".to_string())
        );
    }
}

fn transaction() {
    let db = NikkaBuilder::new().build();
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    let _ = client.begin_transaction();
    let _ = client.set("one", 1);
    let _ = client.erase_transaction();
    let _ = client.set("two", 2);
    let _ = client.send_transaction();

    println!(
        "{}",
        client
            .get("one")
            .unwrap()
            .unwrap_or("undefined".to_string())
    );
}

fn deque() {
    let db = NikkaBuilder::new().build();
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    let _ = client.create_deque("tasks", TypeString);
    let _ = client.push_first("tasks", NikkaTypeWrapper::NikkaString("eat"));
    let _ = client.push_last("tasks", NikkaTypeWrapper::NikkaString("dota2"));
    let _ = client.push_last("tasks", NikkaTypeWrapper::NikkaString("repeat")); // tasks: [eat, dota2, repeat]
    println!("{}", client.pop_first::<String>("tasks").unwrap().unwrap()); // eat
}

fn deploy() {
    let db = NikkaServer::from_config_or_default("example.config.nikka");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut client = NikkaClient::with_port(&port);

    let _ = client.set("foo", "bar");
}
