use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{Arc, Condvar, Mutex, Weak},
    time::{Duration, Instant},
};

/// A single background thread with a timer wheel that interrupts a connection
/// when the currently registered operation exceeds its deadline.
pub struct QueryTimeoutManager {
    inner: Arc<Inner>,
}

struct Inner {
    state: Mutex<State>,
    cv: Condvar,
    /// Connection to interrupt on timeout.
    conn: Weak<libsql::Connection>,
}

struct State {
    heap: BinaryHeap<Reverse<Entry>>,
    current_operation_id: Option<u64>,
    next_id: u64,
    shutdown: bool,
}

#[derive(Clone)]
struct Entry {
    id: u64,
    deadline: Instant,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline && self.id == other.id
    }
}

impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deadline
            .cmp(&other.deadline)
            .then(self.id.cmp(&other.id))
    }
}

impl QueryTimeoutManager {
    pub fn new(conn: &Arc<libsql::Connection>) -> Self {
        let inner = Arc::new(Inner {
            state: Mutex::new(State {
                heap: BinaryHeap::new(),
                current_operation_id: None,
                next_id: 0,
                shutdown: false,
            }),
            cv: Condvar::new(),
            conn: Arc::downgrade(conn),
        });

        let bg = Arc::downgrade(&inner);
        std::thread::spawn(move || {
            Self::background_thread(bg);
        });

        Self { inner }
    }

    /// Signal the background thread to exit and clear all entries.
    pub fn shutdown(&self) {
        {
            let mut state = self.inner.state.lock().unwrap();
            state.shutdown = true;
            state.heap.clear();
            state.current_operation_id = None;
        }
        self.inner.cv.notify_one();
    }

    /// Register a timeout for the currently executing operation.
    pub fn register(&self, timeout: Duration) -> QueryTimeoutGuard {
        let mut state = self.inner.state.lock().unwrap();

        let id = state.next_id;
        state.next_id += 1;

        debug_assert!(
            state.current_operation_id.is_none(),
            "only one operation may be active per connection"
        );

        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400));
        let entry = Entry { id, deadline };

        let is_new_earliest = state
            .heap
            .peek()
            .map_or(true, |Reverse(existing)| entry.deadline < existing.deadline);

        state.current_operation_id = Some(id);
        state.heap.push(Reverse(entry));
        drop(state);

        if is_new_earliest {
            self.inner.cv.notify_one();
        }

        QueryTimeoutGuard {
            op_id: id,
            inner: self.inner.clone(),
        }
    }

    fn deregister(inner: &Arc<Inner>, op_id: u64) {
        let mut state = inner.state.lock().unwrap();
        if state.current_operation_id == Some(op_id) {
            state.current_operation_id = None;
        }
    }

    fn process_expired_deadlines(inner: &Arc<Inner>, state: &mut State) {
        let mut should_interrupt = false;
        let now = Instant::now();

        while let Some(Reverse(entry)) = state.heap.peek() {
            if entry.deadline > now {
                break;
            }

            let entry = state
                .heap
                .pop()
                .expect("heap peek succeeded but pop failed")
                .0;

            if state.current_operation_id == Some(entry.id) {
                should_interrupt = true;
            }
        }

        if should_interrupt {
            if let Some(conn) = inner.conn.upgrade() {
                let _ = conn.interrupt();
            }
        }
    }

    fn background_thread(weak: Weak<Inner>) {
        loop {
            let inner = match weak.upgrade() {
                Some(inner) => inner,
                None => return,
            };

            let mut state = inner.state.lock().unwrap();

            loop {
                if state.shutdown {
                    return;
                }

                match state.heap.peek().map(|Reverse(entry)| entry.deadline) {
                    Some(deadline) => {
                        let now = Instant::now();
                        if deadline > now {
                            let wait_for = deadline.saturating_duration_since(now);
                            let (new_state, timeout_result) =
                                inner.cv.wait_timeout(state, wait_for).unwrap();
                            state = new_state;

                            if !timeout_result.timed_out() {
                                continue;
                            }
                        }

                        Self::process_expired_deadlines(&inner, &mut state);
                    }
                    None => {
                        state = inner.cv.wait(state).unwrap();
                    }
                }
            }
        }
    }
}

impl Drop for QueryTimeoutManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// RAII handle for a registered timeout.
pub struct QueryTimeoutGuard {
    op_id: u64,
    inner: Arc<Inner>,
}

impl Drop for QueryTimeoutGuard {
    fn drop(&mut self) {
        QueryTimeoutManager::deregister(&self.inner, self.op_id);
        self.inner.cv.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_conn() -> Arc<libsql::Connection> {
        let db = libsql::Builder::new_local(":memory:")
            .build()
            .await
            .unwrap();
        Arc::new(db.connect().unwrap())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn deadline_expires_interrupts_connection() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new(&conn);

        let _guard = mgr.register(Duration::from_millis(200));

        let fut = {
            let conn = conn.clone();
            tokio::spawn(async move {
                conn.execute_batch(
                    "WITH RECURSIVE r(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM r) SELECT * FROM r",
                )
                .await
            })
        };

        let result = fut.await.unwrap();
        assert!(result.is_err(), "query should have been interrupted");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn guard_dropped_before_deadline_cancels_timeout() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new(&conn);

        let guard = mgr.register(Duration::from_millis(200));
        drop(guard);

        std::thread::sleep(Duration::from_millis(300));

        let result = conn.execute_batch("SELECT 1").await;
        assert!(
            result.is_ok(),
            "connection should not have been interrupted"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn stale_deadline_does_not_interrupt_next_operation() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new(&conn);

        let guard = mgr.register(Duration::from_millis(200));
        drop(guard);

        // Register a second operation after the first one has been deregistered.
        let _guard2 = mgr.register(Duration::from_millis(5000));

        std::thread::sleep(Duration::from_millis(300));

        let result = conn.execute_batch("SELECT 1").await;
        assert!(
            result.is_ok(),
            "stale timeout entry should not interrupt a different operation"
        );
    }
}
