use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_pubsub_client::{ClientConfig, PubSubClient};
use serde_json::json;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn bench_message_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_processing");

    for size in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let config = ClientConfig::new("http://localhost:5000", "bench_consumer", vec![]);
                let client = Arc::new(PubSubClient::new(config));

                for i in 0..size {
                    let msg = json!({"data": format!("message_{}", i)});
                    black_box(msg);
                }
            });
        });
    }

    group.finish();
}

fn bench_idempotence_filter(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("idempotence_filter");

    for size in [100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async move {
                let config = ClientConfig::new("http://localhost:5000", "bench_consumer", vec![])
                    .with_idempotence(true, 1000);
                let client = Arc::new(PubSubClient::new(config));

                for i in 0..size {
                    black_box(format!("msg_id_{}", i));
                }
            });
        });
    }

    group.finish();
}

fn bench_handler_registration(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("handler_registration", |b| {
        b.to_async(&rt).iter(|| async {
            let config = ClientConfig::new("http://localhost:5000", "bench_consumer", vec![]);
            let client = PubSubClient::new(config);

            for i in 0..100 {
                let topic = format!("topic_{}", i);
                client.register_handler(topic, |_msg| async {});
            }

            black_box(client);
        });
    });
}

fn bench_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut data = std::collections::HashMap::new();
                for i in 0..size {
                    data.insert(format!("key_{}", i), format!("value_{}", i));
                }
                let json = json!(data);
                black_box(serde_json::to_string(&json).unwrap());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_message_processing,
    bench_idempotence_filter,
    bench_handler_registration,
    bench_json_serialization
);
criterion_main!(benches);
