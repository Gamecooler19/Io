use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::error::IoError;
use crate::runtime::GLOBAL_SCHEDULER;

pub struct Task {
    future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>>,
    waker: Arc<TaskWaker>,
    state: TaskState,
    id: usize,
    scheduled: AtomicBool,
}

#[derive(Debug)]
struct TaskState {
    run_count: AtomicUsize,
    last_poll: std::time::Instant,
    last_error: Mutex<Option<IoError>>,
}

impl Task {
    pub fn new<F>(future: F) -> Arc<Self> 
    where 
        F: Future<Output = ()> + Send + 'static,
    {
        static TASK_ID: AtomicUsize = AtomicUsize::new(0);
        
        Arc::new(Self {
            future: Mutex::new(Some(Box::pin(future))),
            waker: Arc::new(TaskWaker::new()),
            state: TaskState {
                run_count: AtomicUsize::new(0),
                last_poll: std::time::Instant::now(),
                last_error: Mutex::new(None),
            },
            id: TASK_ID.fetch_add(1, Ordering::SeqCst),
            scheduled: AtomicBool::new(false),
        })
    }

    pub fn poll(&self, cx: &mut Context) -> Poll<()> {
        let mut future_slot = self.future.lock().unwrap();
        if let Some(mut future) = future_slot.take() {
            match future.as_mut().poll(cx) {
                Poll::Ready(()) => Poll::Ready(()),
                Poll::Pending => {
                    *future_slot = Some(future);
                    Poll::Pending
                }
            }
        } else {
            Poll::Ready(())
        }
    }

    fn mark_scheduled(&self, scheduled: bool) {
        self.scheduled.store(scheduled, Ordering::Release);
    }

    fn is_scheduled(&self) -> bool {
        self.scheduled.load(Ordering::Acquire)
    }

    fn record_poll(&self) {
        self.state.run_count.fetch_add(1, Ordering::SeqCst);
        self.state.last_poll = std::time::Instant::now();
    }

    fn record_error(&self, error: IoError) {
        *self.state.last_error.lock().unwrap() = Some(error);
    }

    pub fn status(&self) -> TaskStatus {
        TaskStatus {
            id: self.id,
            run_count: self.state.run_count.load(Ordering::Relaxed),
            last_poll: self.state.last_poll,
            is_scheduled: self.is_scheduled(),
            last_error: self.state.last_error.lock().unwrap().clone(),
        }
    }
}

impl TaskWaker {
    fn new() -> Self {
        Self {
            task: Arc::new_cyclic(|weak| Task {
                future: Mutex::new(None),
                waker: Arc::new(TaskWaker { task: weak.clone() }),
                state: TaskState {
                    run_count: AtomicUsize::new(0),
                    last_poll: std::time::Instant::now(),
                    last_error: Mutex::new(None),
                },
                id: 0,
                scheduled: AtomicBool::new(false),
            }),
        }
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        // Prevent duplicate scheduling
        if self.task.is_scheduled() {
            return;
        }

        GLOBAL_SCHEDULER.with(|scheduler| {
            if let Some(scheduler) = &*scheduler.borrow() {
                // Mark task as scheduled
                self.task.mark_scheduled(true);

                // Create waker for the task
                let waker = Waker::from(self.clone());
                let mut context = Context::from_waker(&waker);

                // Schedule the task
                scheduler.schedule(Arc::new(Task {
                    future: self.task.future.clone(),
                    waker: self.clone(),
                    state: TaskState {
                        run_count: AtomicUsize::new(0),
                        last_poll: std::time::Instant::now(),
                        last_error: Mutex::new(None),
                    },
                    id: self.task.id,
                    scheduled: AtomicBool::new(false),
                }));

                // Record task polling
                self.task.record_poll();

                // Handle any errors that occurred during polling
                if let Poll::Ready(()) = self.task.poll(&mut context) {
                    // Task completed, clean up
                    self.task.mark_scheduled(false);
                }
            } else {
                // Record error if scheduler is not initialized
                self.task.record_error(IoError::runtime_error(
                    "Attempted to wake task with uninitialized scheduler"
                ));
            }
        });
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.clone().wake()
    }
}

#[derive(Debug)]
pub struct TaskStatus {
    pub id: usize,
    pub run_count: usize,
    pub last_poll: std::time::Instant,
    pub is_scheduled: bool,
    pub last_error: Option<IoError>,
}
