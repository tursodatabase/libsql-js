use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    sync::{Arc, Condvar, Mutex, OnceLock, Weak},
    time::{Duration, Instant},
};

/// Something a timeout can interrupt when its deadline expires. Implemented for
/// both libSQL connections and statements so a single wheel can interrupt at
/// whichever granularity the operation registered with: statement-level for the
/// `Statement` operations (so concurrent operations on the same connection are
/// left untouched) and connection-level for connection-wide operations such as
/// `prepare` and `execute_batch`.
pub trait Interruptible: Send + Sync {
    fn interrupt(&self);
}

impl Interruptible for libsql::Connection {
    fn interrupt(&self) {
        let _ = libsql::Connection::interrupt(self);
    }
}

impl Interruptible for libsql::Statement {
    fn interrupt(&self) {
        let _ = libsql::Statement::interrupt(self);
    }
}

/// A process-wide timer wheel: a single background thread that interrupts
/// operations when they exceed their deadline.
///
/// All connections share one manager — and therefore one thread. Each
/// registered timeout carries a weak reference to the target it should
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
    active: HashMap<u64, Weak<dyn Interruptible>>,
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

    /// Register a timeout for an operation about to run against `target`. The
    /// returned guard cancels the timeout when dropped (i.e. when the operation
    /// completes).
    pub fn register<T: Interruptible + 'static>(
        &self,
        target: &Arc<T>,
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

        let weak = Arc::downgrade(target);
        let target: Weak<dyn Interruptible> = weak;
        state.active.insert(id, target);
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
            if let Some(target) = state.active.remove(&entry.id) {
                if let Some(target) = target.upgrade() {
                    target.interrupt();
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
    async fn deadline_expires_interrupts_statement() {
        let conn = test_conn().await;
        let stmt = Arc::new(
            conn.prepare(
                "WITH RECURSIVE r(n) AS (SELECT 1 UNION ALL SELECT n+1 FROM r) SELECT * FROM r",
            )
            .await
            .unwrap(),
        );
        let mgr = QueryTimeoutManager::new();

        let _guard = mgr.register(&stmt, Duration::from_millis(200));

        // Drain the (effectively infinite) result set; the statement-level
        // interrupt must abort it once the deadline expires.
        let result = async {
            let mut rows = stmt.query(()).await?;
            while rows.next().await?.is_some() {}
            Ok::<_, libsql::Error>(())
        }
        .await;

        assert!(result.is_err(), "statement should have been interrupted");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn deadline_expires_interrupts_single_step_aggregate() {
        // A query whose entire work happens inside the first `step()` (an
        // aggregate over a large recursive CTE). libSQL runs that first step
        // synchronously inside `query()`, so this exercises whether a
        // statement-level interrupt aborts an in-progress step.
        let conn = test_conn().await;
        let stmt = Arc::new(
            conn.prepare(
                "WITH RECURSIVE numbers(value) AS (
                   SELECT 1 UNION ALL SELECT value + 1 FROM numbers WHERE value < 1000000000
                 ) SELECT sum(value) FROM numbers",
            )
            .await
            .unwrap(),
        );
        let mgr = QueryTimeoutManager::new();

        let _guard = mgr.register(&stmt, Duration::from_millis(200));

        let result = async {
            let mut rows = stmt.query(()).await?;
            while rows.next().await?.is_some() {}
            Ok::<_, libsql::Error>(())
        }
        .await;

        assert!(result.is_err(), "statement should have been interrupted");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn statement_interrupt_flag_is_sticky_until_reset() {
        // `libsql_stmt_interrupt` sets a per-statement flag that `sqlite3_step`
        // checks at entry and does NOT clear; only `reset()` clears it. So a
        // statement interrupted by a timeout stays poisoned until reset — every
        // operation must reset before stepping it again.
        let conn = test_conn().await;
        let stmt = Arc::new(conn.prepare("SELECT 1").await.unwrap());

        stmt.interrupt().unwrap();

        // The flag survives into the next execution.
        let mut rows = stmt.query(()).await.unwrap();
        assert!(
            rows.next().await.is_err(),
            "sticky interrupt flag should fail the next step"
        );
        drop(rows);

        // ...and reset clears it, making the statement reusable.
        stmt.reset();
        let mut rows = stmt.query(()).await.unwrap();
        assert!(
            rows.next().await.unwrap().is_some(),
            "statement should be reusable after reset"
        );
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
