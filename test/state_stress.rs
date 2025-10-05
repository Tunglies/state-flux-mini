use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;
use tokio::time::{Instant, timeout};

use state_flux_mini::State;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn stress_test_many_subscribers_and_writers() {
    const N_SUBSCRIBERS: usize = 1000;
    const N_WRITERS: usize = 10;
    const N_UPDATES: usize = 50;

    let state = Arc::new(State::new(0usize));
    let barrier = Arc::new(Barrier::new(N_WRITERS + 1));

    let mut handles = Vec::new();
    for id in 0..N_WRITERS {
        let state = state.clone();
        let barrier = barrier.clone();
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            for i in 1..=N_UPDATES {
                state.set(i);
                if id % 2 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        }));
    }

    let mut subs: Vec<_> = (0..N_SUBSCRIBERS).map(|_| state.subscribe()).collect();

    barrier.wait().await;
    let start = Instant::now();

    let mut tasks = Vec::new();
    for mut sub in subs.drain(..) {
        let task = tokio::spawn(async move {
            let mut seen_final = false;
            let mut last_seen = 0usize;

            for _ in 0..N_UPDATES * 2 {
                match timeout(Duration::from_secs(2), sub.changed()).await {
                    Ok(v) => {
                        let val = *v;
                        last_seen = val;
                        if val == N_UPDATES {
                            seen_final = true;
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            assert!(
                seen_final,
                "subscriber never saw final update, last_seen={}",
                last_seen
            );
            true
        });

        tasks.push(task);
    }

    for t in tasks {
        assert!(t.await.unwrap());
    }
    for t in handles {
        t.await.unwrap();
    }

    let elapsed = start.elapsed();
    println!(
        "✅ stress test passed: {} subscribers, {} writers, {} updates in {:?}",
        N_SUBSCRIBERS, N_WRITERS, N_UPDATES, elapsed
    );

    assert_eq!(*state.get(), N_UPDATES);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_test_fast_set_get() {
    let state = Arc::new(State::new(0u64));

    let writer = {
        let state = state.clone();
        tokio::spawn(async move {
            for i in 1..=10_000 {
                state.set(i);
                if i % 100 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        })
    };

    let readers: Vec<_> = (0..8)
        .map(|_| {
            let state = state.clone();
            tokio::spawn(async move {
                let mut last = 0u64;
                for _ in 0..10_000 {
                    let cur = *state.get();
                    assert!(cur >= last, "monotonic read check failed");
                    last = cur;
                    tokio::task::yield_now().await;
                }
                true
            })
        })
        .collect();

    writer.await.unwrap();
    for r in readers {
        assert!(r.await.unwrap());
    }

    assert_eq!(*state.get(), 10_000);
}
