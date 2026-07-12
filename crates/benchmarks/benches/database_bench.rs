use criterion::{criterion_group, criterion_main, Criterion};
use nikkadb_client::NikkaClient;
use nikkadb_server::utils::builder::NikkaBuilder;
use std::hint::black_box;
use std::thread::{sleep, spawn};
use std::time::Duration;

fn crud(client: &mut NikkaClient, key: &str, value: u8) {
    client.set(key, value);
    let int = client.get::<u8>(key).unwrap().unwrap();
    client.set(key, value + 1);
    client.remove(key);
    assert_eq!(value, int);
}

fn benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("bench");
    group.warm_up_time(Duration::from_secs(5));

    let db = NikkaBuilder::new().backup_operations_count(100000).build();
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
