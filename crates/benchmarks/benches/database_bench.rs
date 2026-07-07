use criterion::{criterion_group, criterion_main, Criterion};
use nikkadb_client::NikkaClient;
use nikkadb_server::server::NikkaServer;
use std::hint::black_box;
use std::thread::{sleep, spawn};
use std::time::Duration;

fn crud(client: &mut NikkaClient, key: &str, value: u8) {
    client.set_int(key, value);
    let int = client.get_int(key).unwrap();
    client.set_int(key, value + 1);
    client.remove(key);
    assert_eq!(value, int);
}

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench");

    let db = NikkaServer::with_port("0");
    let port = db.get_port().to_string();

    spawn(|| db.run());

    sleep(Duration::from_millis(100));

    let mut db = NikkaClient::with_port(&port);

    let values: Vec<u8> = (0u8..254u8).collect();
    let keys: Vec<String> = (0..254000).map(|x| x.to_string()).collect();
    let mut keys_iter = keys.into_iter();
    let mut values_iter = values.repeat(1000).into_iter();

    group.bench_function("crud_bench", |x| {
        x.iter(|| {
            crud(
                black_box(&mut db),
                black_box(&keys_iter.next().unwrap()),
                black_box(values_iter.next().unwrap()),
            );
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
