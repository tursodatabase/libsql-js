use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, Weak,
    },
    time::Duration,
};
use tokio::{sync::Notify, time::Instant};

/// A single-background-task timer wheel that interrupts connections when their
/// query deadline expires. Registering a query returns a [`TimeoutGuard`] —
/// dropping the guard cancels the timeout.
pub struct QueryTimeoutManager {
    inner: Arc<Inner>,
}

struct Inner {
    entries: Mutex<Entries>,
    /// Wakes the background task when the earliest deadline changes.
    notify: Arc<Notify>,
    /// Set to `true` to make the background task exit.
    shutdown: AtomicBool,
}

struct Entries {
    heap: BinaryHeap<Reverse<Entry>>,
    next_id: u64,
}

#[derive(Clone)]
struct Entry {
    id: u64,
    deadline: Instant,
    conn: Weak<libsql::Connection>,
    /// Cleared when the guard is dropped (query finished in time).
    active: Arc<AtomicBool>,
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
    pub fn new() -> Self {
        let inner = Arc::new(Inner {
            entries: Mutex::new(Entries {
                heap: BinaryHeap::new(),
                next_id: 0,
            }),
            notify: Arc::new(Notify::new()),
            shutdown: AtomicBool::new(false),
        });
        let bg = Arc::downgrade(&inner);
        tokio::spawn(async move {
            Self::background_task(bg).await;
        });
        Self { inner }
    }

    /// Synchronously remove all entries from the heap, releasing any
    /// connection references the background task is holding.
    pub fn clear(&self) {
        let mut entries = self.inner.entries.lock().unwrap();
        entries.heap.clear();
    }

    /// Signal the background task to exit and clear all entries.
    pub fn shutdown(&self) {
        self.inner.shutdown.store(true, Ordering::Relaxed);
        self.clear();
        self.inner.notify.notify_one();
    }

    /// Register a query. The returned guard must be held for the duration of
    /// the query — dropping it cancels the timeout.
    pub fn register(&self, conn: &Arc<libsql::Connection>, timeout: Duration) -> TimeoutGuard {
        let active = Arc::new(AtomicBool::new(true));
        let mut entries = self.inner.entries.lock().unwrap();
        let id = entries.next_id;
        entries.next_id += 1;
        let deadline = Instant::now()
            .checked_add(timeout)
            .unwrap_or_else(|| Instant::now() + Duration::from_secs(86400));
        let entry = Entry {
            id,
            deadline,
            conn: Arc::downgrade(conn),
            active: active.clone(),
        };
        let is_new_earliest = entries
            .heap
            .peek()
            .map_or(true, |Reverse(e)| entry.deadline < e.deadline);
        entries.heap.push(Reverse(entry));
        drop(entries);
        if is_new_earliest {
            self.inner.notify.notify_one();
        }
        TimeoutGuard {
            active,
            notify: self.inner.notify.clone(),
        }
    }

    async fn background_task(weak: Weak<Inner>) {
        loop {
            let inner = match weak.upgrade() {
                Some(inner) => inner,
                None => return, // Manager dropped — exit.
            };

            if inner.shutdown.load(Ordering::Relaxed) {
                return;
            }

            // Find the next deadline, skipping cancelled entries.
            let next = {
                let mut entries = inner.entries.lock().unwrap();
                loop {
                    match entries.heap.peek() {
                        Some(Reverse(e)) if !e.active.load(Ordering::Relaxed) => {
                            entries.heap.pop();
                        }
                        Some(Reverse(e)) => break Some(e.clone()),
                        None => break None,
                    }
                }
            };

            match next {
                Some(entry) => {
                    tokio::select! {
                        _ = tokio::time::sleep_until(entry.deadline) => {
                            // Deadline reached — interrupt if still active.
                            if entry.active.load(Ordering::Relaxed) {
                                if let Some(conn) = entry.conn.upgrade() {
                                    let _ = conn.interrupt();
                                }
                            }
                            // Remove this entry.
                            let mut entries = inner.entries.lock().unwrap();
                            // Pop entries that are done (expired or cancelled).
                            while let Some(Reverse(e)) = entries.heap.peek() {
                                if !e.active.load(Ordering::Relaxed) || e.id == entry.id {
                                    entries.heap.pop();
                                } else {
                                    break;
                                }
                            }
                        }
                        _ = inner.notify.notified() => {
                            // A new earlier deadline was added; re-check.
                        }
                    }
                }
                None => {
                    // Nothing to do — wait until a new entry is registered.
                    // Must hold the Arc while waiting so we can detect drop next iteration.
                    inner.notify.notified().await;
                }
            }
        }
    }
}

impl Drop for QueryTimeoutManager {
    fn drop(&mut self) {
        // Signal the background task to exit.
        self.inner.shutdown.store(true, Ordering::Relaxed);
        self.inner.notify.notify_one();
    }
}

/// Dropping this guard cancels the associated query timeout.
pub struct TimeoutGuard {
    active: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Drop for TimeoutGuard {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
        // Wake the background task so it can clean up the cancelled entry.
        self.notify.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

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

        // Register a 200ms timeout, then start an infinite query.
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

        // The query should have been interrupted by the timeout.
        let result = fut.await.unwrap();
        assert!(result.is_err(), "query should have been interrupted");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn guard_dropped_before_deadline_cancels_timeout() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new();

        let guard = mgr.register(&conn, Duration::from_millis(200));

        // Query "finishes" before the deadline.
        drop(guard);

        // Wait past where the deadline would have been.
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Connection should still work — no spurious interrupt.
        let result = conn.execute_batch("SELECT 1").await;
        assert!(
            result.is_ok(),
            "connection should not have been interrupted"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn guard_drop_cleans_entry_from_heap() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new();

        let guard = mgr.register(&conn, Duration::from_millis(500));

        drop(guard);
        // Let the background task wake up and clean the cancelled entry.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let entries = mgr.inner.entries.lock().unwrap();
        assert_eq!(
            entries.heap.len(),
            0,
            "dropping guard should clean up the entry from the heap"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn multiple_deadlines_fire_in_order() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new();

        // Register three timeouts at 100ms, 200ms, 300ms.
        let _g1 = mgr.register(&conn, Duration::from_millis(100));
        let _g2 = mgr.register(&conn, Duration::from_millis(200));
        let _g3 = mgr.register(&conn, Duration::from_millis(300));

        // After 150ms, only the first should have fired.
        tokio::time::sleep(Duration::from_millis(150)).await;
        {
            let entries = mgr.inner.entries.lock().unwrap();
            assert_eq!(entries.heap.len(), 2, "only first entry should have fired");
        }

        // After 350ms total, all three should have fired.
        tokio::time::sleep(Duration::from_millis(200)).await;
        {
            let entries = mgr.inner.entries.lock().unwrap();
            assert_eq!(entries.heap.len(), 0, "all entries should be cleaned up");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn new_earlier_deadline_preempts_existing() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new();

        // Register a long timeout first, then a shorter one that should preempt.
        let _g1 = mgr.register(&conn, Duration::from_millis(5000));
        let _g2 = mgr.register(&conn, Duration::from_millis(200));

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
        assert!(
            result.is_err(),
            "shorter deadline should have interrupted the query"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(10000)]
    async fn background_task_exits_when_manager_dropped() {
        let conn = test_conn().await;
        let mgr = QueryTimeoutManager::new();
        let guard = mgr.register(&conn, Duration::from_millis(5000));
        drop(guard);
        drop(mgr);
        // If the background task didn't exit, it would leak — verify no panic.
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
