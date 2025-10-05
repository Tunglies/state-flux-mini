use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use state_flux_mini::State;

#[tokio::test]
async fn test_basic_get_set() {
    let state = State::new(42);
    assert_eq!(*state.get(), 42);

    state.set(100);
    assert_eq!(*state.get(), 100);
}

#[tokio::test]
async fn test_single_subscriber_receives_update() {
    let state = State::new(0);
    let mut sub = state.subscribe();

    state.set(1);
    let new = timeout(Duration::from_millis(100), sub.changed())
        .await
        .expect("should be notified");
    assert_eq!(*new, 1);
}

#[tokio::test]
async fn test_try_changed_and_peek() {
    let state = State::new("init".to_string());
    let mut sub = state.subscribe();

    // No change yet
    assert!(sub.try_changed().is_none());

    state.set("new".to_string());
    let changed = sub.try_changed().expect("should detect change");
    assert_eq!(&*changed, "new");
    assert_eq!(&*sub.peek(), "new");
}

#[tokio::test]
async fn test_multiple_subscribers_notified() {
    let state = State::new(5);
    let mut a = state.subscribe();
    let mut b = state.subscribe();

    state.set(10);

    let a_val = timeout(Duration::from_millis(100), a.changed())
        .await
        .expect("a should receive");
    let b_val = timeout(Duration::from_millis(100), b.changed())
        .await
        .expect("b should receive");

    assert_eq!(*a_val, 10);
    assert_eq!(*b_val, 10);
}

#[tokio::test]
async fn test_no_false_trigger_on_same_value() {
    let state = State::new(1);
    let mut sub = state.subscribe();

    // Same value shouldn't trigger changed()
    state.set(1);
    let res = timeout(Duration::from_millis(50), sub.changed()).await;
    assert!(res.is_err(), "no notification should occur");
}

#[tokio::test]
async fn test_pointer_eq_shortcut() {
    // If pointer eq (same Arc) → should skip notify
    let state = State::new(123);
    let mut sub = state.subscribe();

    // re-store same Arc using set (assumes you add this method to State)
    let old = state.get();
    state.set(*old); // same value, new Arc
    let res = timeout(Duration::from_millis(50), sub.changed()).await;
    assert!(res.is_err(), "no change because pointer eq");
}

#[tokio::test]
async fn test_concurrent_writers_and_subscribers() {
    let state = Arc::new(State::new(0));
    let mut subs: Vec<_> = (0..10).map(|_| state.subscribe()).collect();

    // 启动订阅任务（在 set() 前）
    let mut handles = Vec::new();
    for mut sub in subs.drain(..) {
        handles.push(tokio::spawn(async move {
            let mut last = 0;
            for _ in 0..5 {
                let val = timeout(Duration::from_millis(500), sub.changed())
                    .await
                    .expect("subscriber notified");
                assert!(*val > last);
                last = *val;
            }
            last
        }));
    }

    // 确保订阅者已就绪
    tokio::time::sleep(Duration::from_millis(20)).await;

    // 再启动 writer
    let writer = {
        let state = state.clone();
        tokio::spawn(async move {
            for i in 1..=5 {
                state.set(i);
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
    };

    writer.await.unwrap();
    for h in handles {
        h.await.unwrap();
    }
}

#[tokio::test]
async fn test_notify_reuse_after_clone() {
    let state = State::new("A".to_string());
    let mut sub1 = state.subscribe();
    let mut sub2 = state.clone().subscribe();

    state.set("B".into());
    let v1 = timeout(Duration::from_millis(100), sub1.changed())
        .await
        .unwrap();
    let v2 = timeout(Duration::from_millis(100), sub2.changed())
        .await
        .unwrap();

    assert_eq!(&*v1, "B");
    assert_eq!(&*v2, "B");
}
