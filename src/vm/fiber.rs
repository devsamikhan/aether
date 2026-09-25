use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};

pub type FiberTask = Box<dyn FnOnce() + Send + 'static>;

struct WorkerQueue {
    tasks: Mutex<VecDeque<FiberTask>>,
    cv: Condvar,
}

impl WorkerQueue {
    fn new() -> Self {
        Self {
            tasks: Mutex::new(VecDeque::new()),
            cv: Condvar::new(),
        }
    }

    fn push(&self, task: FiberTask) {
        let mut q = self.tasks.lock().unwrap();
        q.push_back(task);
        self.cv.notify_one();
    }

    fn pop_local(&self) -> Option<FiberTask> {
        let mut q = self.tasks.lock().unwrap();
        q.pop_back()
    }

    fn steal(&self) -> Option<FiberTask> {
        let mut q = self.tasks.try_lock().ok()?;
        q.pop_front()
    }
}

pub struct WorkStealingScheduler {
    workers: Vec<Arc<WorkerQueue>>,
    #[allow(dead_code)]
    threads: Mutex<Vec<JoinHandle<()>>>,
    shutdown: Arc<AtomicBool>,
    next_worker: AtomicUsize,
    num_threads: usize,
}

impl WorkStealingScheduler {
    pub fn new(num_threads: usize) -> Self {
        let num_threads = num_threads.max(2);
        let mut workers = Vec::with_capacity(num_threads);
        for _ in 0..num_threads {
            workers.push(Arc::new(WorkerQueue::new()));
        }

        let shutdown = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::with_capacity(num_threads);

        for i in 0..num_threads {
            let my_queue = Arc::clone(&workers[i]);
            let all_queues = workers.clone();
            let is_shutdown = Arc::clone(&shutdown);

            let handle = thread::Builder::new()
                .name(format!("aether-fiber-worker-{}", i))
                .spawn(move || {
                    while !is_shutdown.load(Ordering::Relaxed) {
                        // 1. Local LIFO execution
                        if let Some(task) = my_queue.pop_local() {
                            task();
                            continue;
                        }

                        // 2. Work stealing (FIFO from other workers)
                        let mut stolen = None;
                        for (j, other_q) in all_queues.iter().enumerate() {
                            if j != i {
                                if let Some(task) = other_q.steal() {
                                    stolen = Some(task);
                                    break;
                                }
                            }
                        }

                        if let Some(task) = stolen {
                            task();
                            continue;
                        }

                        // 3. Idle wait
                        let q = my_queue.tasks.lock().unwrap();
                        if q.is_empty() && !is_shutdown.load(Ordering::Relaxed) {
                            let _ = my_queue.cv.wait_timeout(q, std::time::Duration::from_millis(20));
                        }
                    }
                })
                .expect("Failed to spawn fiber worker thread");

            threads.push(handle);
        }

        Self {
            workers,
            threads: Mutex::new(threads),
            shutdown,
            next_worker: AtomicUsize::new(0),
            num_threads,
        }
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let idx = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.num_threads;
        self.workers[idx].push(Box::new(f));
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        for w in &self.workers {
            w.cv.notify_all();
        }
    }
}

static GLOBAL_SCHEDULER: OnceLock<WorkStealingScheduler> = OnceLock::new();

pub fn global_scheduler() -> &'static WorkStealingScheduler {
    GLOBAL_SCHEDULER.get_or_init(|| {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        WorkStealingScheduler::new(threads)
    })
}
