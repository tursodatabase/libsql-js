use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    sync::{Arc, Condvar, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};

/// A process-wide timer wheel: a single background thread that interrupts
/// connections when their currently registered operation exceeds its deadline.
///
/// All connections share one manager — and therefore one thread. Each
/// registered timeout carries a weak reference to the connection it should
/// interrupt, so a single wheel serves any number of connections.
pub struct QueryTimeoutManager {
    inner: Arc<Inner>,
}

struct Inner {
    state: Mutex<State>,
    cv: Condvar,
}

struct State {
    /// Pending deadlines, soonest first.
    heap: BinaryHeap<Reverse<Entry>>,
    /// Operations currently in flight, keyed by id. An expired entry is only
    /// interrupted if it is still present here; a completed operation removes
    /// itself via its guard's `Drop`, leaving a stale heap entry that is
    /// discarded lazily when its deadline is reached.
    active: HashMap<u64, Weak<libsql::Connection>>,
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

static GLOBAL: OnceLock<QueryTimeoutManager> = OnceLock::new();

impl QueryTimeoutManager {
    /// Returns the process-wide timeout manager, spawning its single
    /// background thread on first use.
    pub fn global() -> &'static QueryTimeoutManager {
        GLOBAL.get_or_init(QueryTimeoutManager::new)
    }

    pub fn new() -> Self {
        let inner = Arc::new(Inner {
            state: Mutex::new(State {
                heap: BinaryHeap::new(),
                active: HashMap::new(),
                next_id: 0,
                shutdown: false,
            }),
            cv: Condvar::new(),
        });

        let bg = Arc::downgrade(&inner);
        std::thread::spawn(move || {
            Self::background_thread(bg);
        });

        Self { inner }
    }

    /// Signal the background thread to exit and clear all entries.
    ///
    /// Only used to tear down standalone managers (e.g. in tests); the global
    /// manager lives for the lifetime of the process.
    pub fn shutdown(&self) {
        {
            let mut state = self.inner.state.lock().unwrap();
            state.shutdown = true;
            state.heap.clear();
            state.active.clear();
        }
        self.inner.cv.notify_one();
    }

    /// Register a timeout for an operation about to run on `conn`. The returned
    /// guard cancels the timeout when dropped (i.e. when the operation
    /// completes).
    pub fn register(
        &self,
        conn: &Arc<libsql::Connection>,
        timeout: Duration,
    ) -> QueryTimeoutGuard {
        let mut state = self.inner.state.lock().unwrap();

        let id = state.next_id;
        state.next_id += 1;

        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400));
        let entry = Entry { id, deadline };

        let is_new_earliest = state
            .heap
            .peek()
            .map_or(true, |Reverse(existing)| entry.deadline < existing.deadline);

        state.active.insert(id, Arc::downgrade(conn));
        state.heap.push(Reverse(entry));
        drop(state);

        // Only the soonest deadline dictates when the thread next wakes, so we
        // only need to nudge it when this registration moves that deadline
        // earlier. Guard drops never need to wake the thread: a cancelled entry
        // is simply skipped when its (unchanged) deadline is reached.
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
        state.active.remove(&op_id);
    }

    fn process_expired_deadlines(state: &mut State) {
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

            // Interrupt only if the operation is still in flight; a completed
            // operation will have removed itself from `active` already.
            if let Some(conn) = state.active.remove(&entry.id) {
                if let Some(conn) = conn.upgrade() {
                    let _ = conn.interrupt();
                }
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

                        Self::process_expired_deadlines(&mut state);
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
        let mgr = QueryTimeoutManager::new();

        let _guard = mgr.register(&conn, Duration::from_millis(200));

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
        let mgr = QueryTimeoutManager::new();

        let guard = mgr.register(&conn, Duration::from_millis(200));
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
        let mgr = QueryTimeoutManager::new();

        let guard = mgr.register(&conn, Duration::from_millis(200));
        drop(guard);

        // Register a second operation after the first one has been deregistered.
        let _guard2 = mgr.register(&conn, Duration::from_millis(5000));

        std::thread::sleep(Duration::from_millis(300));

        let result = conn.execute_batch("SELECT 1").await;
        assert!(
            result.is_ok(),
            "stale timeout entry should not interrupt a different operation"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn one_wheel_serves_many_connections() {
        let mgr = QueryTimeoutManager::new();

        // A short-deadline operation on one connection must be interrupted...
        let slow_conn = test_conn().await;
        let _slow_guard = mgr.register(&slow_conn, Duration::from_millis(200));

        // ...while a long-deadline operation on a different connection,
        // registered through the same manager, is left untouched.
        let fast_conn = test_conn().await;
        let _fast_guard = mgr.register(&fast_conn, Duration::from_millis(60_000));

        let slow = {
            let slow_conn = slow_conn.clone();
            tokio::spawn(async move {
                slow_conn
                    .execute_batch(
                        "WITH RECURSIVE r(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM r) SELECT * FROM r",
                    )
                    .await
            })
        };

        let slow_result = slow.await.unwrap();
        assert!(
            slow_result.is_err(),
            "the short-deadline connection should have been interrupted"
        );

        let fast_result = fast_conn.execute_batch("SELECT 1").await;
        assert!(
            fast_result.is_ok(),
            "a different connection sharing the wheel should not be interrupted"
        );
    }
}
