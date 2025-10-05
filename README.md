# state-flux-mini

A minimal, lightweight state-management crate for Rust providing lock-free reads, cheap atomic updates and async notifications to subscribers.

- Uses arc-swap for fast read-mostly access.
- Uses tokio::sync::Notify for async change notification.
- Small API: State and Subscriber.

## Features

- Lock-free read access via Arc<T>.
- Atomic replace for writes.
- Async subscribers can await changes with low overhead.
- Subscriber change detection compares values (requires `T: PartialEq` for `changed` / `try_changed`).

## Quick examples

Add the crate to your Cargo.toml (example uses the local crate name `state-flux-mini`):

```toml
[dependencies]
state-flux-mini = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Create and use a State:

```rust
use state_flux_mini::State;
use std::sync::Arc;

let state = Arc::new(State::new(0usize));

// read (returns Arc<T>)
let val = state.get();
println!("value = {}", *val);

// write (replaces stored value and notifies subscribers)
state.set(1usize);
```

Subscribe and await changes (async):

```rust
use state_flux_mini::State;
use tokio::task;

#[tokio::main]
async fn main() {
    let state = State::new(0usize);
    let mut sub = state.subscribe();

    // spawn a watcher
    let watcher = task::spawn({
        let mut sub = sub.clone();
        async move {
            let new = sub.changed().await; // requires T: PartialEq
            println!("got change -> {}", *new);
        }
    });

    // trigger change
    state.set(2usize);

    watcher.await.unwrap();
}
```

Try non-blocking check:

```rust
let mut sub = state.subscribe();
if let Some(new) = sub.try_changed() {
    println!("changed -> {}", *new);
}
```

Peek current value without affecting the subscriber:

```rust
let cur = sub.peek();
println!("current -> {}", *cur);
```

## API (summary)

- State<T>
  - new(initial: T) -> Self
  - get(&self) -> Arc<T>
  - set(&self, new: T)
  - subscribe(&self) -> Subscriber<T>

- Subscriber<T>
  - changed(&mut self) -> impl Future<Output = Arc<T>> where T: PartialEq
  - try_changed(&mut self) -> Option<Arc<T>> where T: PartialEq
  - peek(&self) -> Arc<T>
  - Clone is implemented for both State and Subscriber.

Notes:
- get returns Arc<T> so cloning is cheap.
- changed() uses pointer equality first, then value equality (`PartialEq`) to detect meaningful changes.

## Benchmarks

The repository contains Criterion benchmarks (see `benches/state.rs`) that measure:
- get
- set
- set + get
- notification latency for one subscriber

Run with:
```
cargo bench
```
(benchmarks require the `criterion` dev-dependency and a tokio runtime for the async bench.)

## License & Contributions

GNU GENERAL PUBLIC LICENSE V3 (pick whichever fits your repo). Contributions and issues welcome — keep changes small and focused.
