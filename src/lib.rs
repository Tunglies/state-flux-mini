use arc_swap::ArcSwap;
use std::sync::Arc;
use tokio::sync::Notify;

pub struct State<T> {
    inner: Arc<ArcSwap<T>>,
    notify: Arc<Notify>,
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            notify: self.notify.clone(),
        }
    }
}

impl<T> State<T> {
    pub fn new(initial: T) -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(initial)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn get(&self) -> Arc<T> {
        self.inner.load_full()
    }

    pub fn set(&self, new: T) {
        self.inner.store(Arc::new(new));
        self.notify.notify_waiters();
    }

    pub fn subscribe(&self) -> Subscriber<T> {
        let last = self.inner.load_full();
        Subscriber {
            inner: self.inner.clone(),
            notify: self.notify.clone(),
            last,
        }
    }
}

pub struct Subscriber<T> {
    inner: Arc<ArcSwap<T>>,
    notify: Arc<Notify>,
    last: Arc<T>,
}

impl<T> Clone for Subscriber<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            notify: self.notify.clone(),
            last: self.last.clone(),
        }
    }
}

impl<T> Subscriber<T>
where
    T: PartialEq,
{
    pub async fn changed(&mut self) -> Arc<T> {
        loop {
            let cur = self.inner.load_full();
            let changed = if Arc::ptr_eq(&cur, &self.last) {
                false
            } else {
                *cur != *self.last
            };

            if changed {
                self.last = cur.clone();
                return cur;
            }

            self.notify.notified().await;
        }
    }

    pub fn try_changed(&mut self) -> Option<Arc<T>> {
        let cur = self.inner.load_full();
        let changed = if Arc::ptr_eq(&cur, &self.last) {
            false
        } else {
            *cur != *self.last
        };

        if changed {
            self.last = cur.clone();
            Some(cur)
        } else {
            None
        }
    }

    pub fn peek(&self) -> Arc<T> {
        self.inner.load_full()
    }
}
