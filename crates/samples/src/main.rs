use nikkadb_server::server::NikkaServer;
use nikkadb_client::client::NikkaClient;

fn main() {

    //create a server side of database
    std::thread::spawn(|| {
        let _ = NikkaServer::new_with_port("5434");
    });

    let mut client = NikkaClient::with_port("5434");

    client.add("language:mascot:go", "gopher");
    client.add("language:mascot:java", "duke");
    client.add("language:framework:java", "spring");
    client.add("language:framework:rust", "axum");

    println!("all about java");
    for query in client.get_regex("language:*:java"){
        println!("{} - {}", query, client.get(&query).unwrap_or("undefined".to_string()));
    }

    println!("all about frameworks");
    for query in client.get_regex("language:framework:*"){
        println!("{} - {}", query, client.get(&query).unwrap_or("undefined".to_string()));
    }

    println!("all about everyone");
    for query in client.get_regex("*:*:*"){
        println!("{} - {}", query, client.get(&query).unwrap_or("undefined".to_string()));
    }

    client.add("language:framework:typescript", "next.js");
    client.add("language:framework:javascript", "react");

    println!("know the difference!");
    for query in client.get_regex("*:*:%%%%script"){
        println!("{} - {}", query, client.get(&query).unwrap_or("undefined".to_string()));
    }

    println!("so similar but so different");
    for query in client.get_regex("*:framework:j*"){
        println!("{} - {}", query, client.get(&query).unwrap_or("undefined".to_string()));
    }


}