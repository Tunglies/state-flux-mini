use criterion::{Criterion, black_box, criterion_group, criterion_main};
use state_flux_mini::State;
use std::sync::Arc;

fn bench_state_get(c: &mut Criterion) {
    let state = Arc::new(State::new(123usize));
    c.bench_function("state_get", |b| {
        b.iter(|| {
            let val = state.get();
            black_box(*val);
        })
    });
}

fn bench_state_set(c: &mut Criterion) {
    let state = Arc::new(State::new(0usize));
    c.bench_function("state_set", |b| {
        let mut i = 0usize;
        b.iter(|| {
            i = i.wrapping_add(1);
            state.set(black_box(i));
        })
    });
}

fn bench_state_set_get(c: &mut Criterion) {
    let state = Arc::new(State::new(0usize));
    c.bench_function("state_set_get", |b| {
        b.iter(|| {
            state.set(42);
            let _ = black_box(state.get());
        })
    });
}

fn bench_state_notify(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    c.bench_function("state_notify_one_subscriber", |b| {
        b.to_async(&rt).iter(|| async {
            let state = Arc::new(State::new(0usize));
            let sub = state.subscribe();

            let reader = tokio::spawn({
                let mut sub = sub.clone();
                async move {
                    let _ = sub.changed().await;
                }
            });

            state.set(1usize);
            let _ = reader.await;
        })
    });
}

criterion_group!(
    benches,
    bench_state_get,
    bench_state_set,
    bench_state_set_get,
    bench_state_notify,
);
criterion_main!(benches);
