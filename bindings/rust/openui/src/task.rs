//! Async tasks: on the UI thread with [`spawn_local`], on background threads
//! with [`spawn`] and [`spawn_blocking`].
//!
//! UI state never leaves the UI thread. A `spawn_local` future may hold
//! signals, documents and elements; it runs on the UI thread between input
//! events and frames, woken through the event loop rather than by polling.
//! A `spawn` future must be `Send`, so it cannot capture a signal:
//!
//! ```compile_fail
//! let count = openui::create_signal(0);
//! openui::task::spawn(async move { count.set(1) }).detach();
//! ```
//!
//! A task spawned while a reactive scope is current, for example inside a
//! component, is cancelled when that scope is disposed: when the component
//! unmounts or its window closes. A late result never writes to a dead view.
//!
//! ```no_run
//! use openui::task::{spawn, spawn_local};
//!
//! let total = openui::create_signal(0_u64);
//! spawn_local(async move {
//!     let sum = spawn(async { (1..=1_000_000_u64).sum::<u64>() }).await;
//!     total.set(sum);
//! })
//! .detach();
//! ```

use async_task::Runnable;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// A spawned task. Awaiting it yields the future's output; dropping it
/// cancels the task; [`Task::detach`] lets it run to completion unobserved.
#[must_use = "dropping a Task cancels it; call detach() to let it finish"]
pub struct Task<T>(Inner<T>);

enum Inner<T> {
    Plain(async_task::Task<T>),
    /// A UI task bound to a reactive scope; `None` means the scope was
    /// disposed first.
    Scoped(async_task::Task<Option<T>>),
}

impl<T> Task<T> {
    /// Let the task run to completion without awaiting it.
    pub fn detach(self) {
        match self.0 {
            Inner::Plain(task) => task.detach(),
            Inner::Scoped(task) => task.detach(),
        }
    }

    pub fn is_finished(&self) -> bool {
        match &self.0 {
            Inner::Plain(task) => task.is_finished(),
            Inner::Scoped(task) => task.is_finished(),
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    /// A UI task whose scope was disposed never completes; whoever awaits it
    /// normally belongs to that scope too and is cancelled with it.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        match &mut self.0 {
            Inner::Plain(task) => Pin::new(task).poll(cx),
            Inner::Scoped(task) => match Pin::new(task).poll(cx) {
                Poll::Ready(Some(value)) => Poll::Ready(value),
                Poll::Ready(None) | Poll::Pending => Poll::Pending,
            },
        }
    }
}

impl<T> std::fmt::Debug for Task<T> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Task")
            .field("finished", &self.is_finished())
            .finish()
    }
}

/// Run `future` on the UI thread. It may hold signals, documents and
/// elements. If a reactive scope is current, disposing it cancels the task.
pub fn spawn_local<F>(future: F) -> Task<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    if crate::runtime::current_scope().is_none() {
        return spawn_local_unscoped(future);
    }
    let queue = ui_queue();
    let cancel = Rc::new(CancelState::default());
    let guard = cancel.clone();
    crate::on_cleanup(move || guard.cancel());
    let future = Cancellable {
        future: Some(Box::pin(future)),
        cancel,
        _live: Live::new(&queue),
    };
    let (runnable, task) = async_task::spawn_local(future, move |runnable| queue.push(runnable));
    runnable.schedule();
    Task(Inner::Scoped(task))
}

/// [`spawn_local`] without binding the task to the current scope.
pub(crate) fn spawn_local_unscoped<F>(future: F) -> Task<F::Output>
where
    F: Future + 'static,
    F::Output: 'static,
{
    let queue = ui_queue();
    let live = Live::new(&queue);
    let future = async move {
        let _live = live;
        future.await
    };
    let (runnable, task) = async_task::spawn_local(future, move |runnable| queue.push(runnable));
    runnable.schedule();
    Task(Inner::Plain(task))
}

/// Run `future` on the background thread pool. It must be `Send`, so it
/// cannot capture UI state; await the returned task from the UI thread to
/// use its result there.
pub fn spawn<F>(future: F) -> Task<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    Task(Inner::Plain(background().spawn(future)))
}

/// Run a blocking function, such as file IO or a synchronous library call,
/// on a thread pool for blocking work.
pub fn spawn_blocking<T, F>(function: F) -> Task<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Task(Inner::Plain(blocking::unblock(function)))
}

/// Timers for UI and background tasks, driven by `async-io`'s reactor
/// thread.
pub mod time {
    use std::time::{Duration, Instant};

    /// Complete after `duration`.
    pub async fn sleep(duration: Duration) {
        async_io::Timer::after(duration).await;
    }

    /// Ticks every `period`, starting one period from now.
    pub struct Interval(async_io::Timer);

    pub fn interval(period: Duration) -> Interval {
        Interval(async_io::Timer::interval(period))
    }

    impl Interval {
        /// Wait for the next tick and return when it was due.
        pub async fn tick(&mut self) -> Instant {
            use futures_lite::StreamExt;
            self.0.next().await.expect("an interval timer never ends")
        }
    }
}

/// The background executor: a few threads, started on first use.
fn background() -> &'static async_executor::Executor<'static> {
    static EXECUTOR: OnceLock<&'static async_executor::Executor<'static>> = OnceLock::new();
    EXECUTOR.get_or_init(|| {
        let executor: &'static async_executor::Executor<'static> =
            Box::leak(Box::new(async_executor::Executor::new()));
        let threads = std::thread::available_parallelism()
            .map_or(2, usize::from)
            .clamp(1, 4);
        for index in 0..threads {
            std::thread::Builder::new()
                .name(format!("openui-worker-{index}"))
                .spawn(move || {
                    futures_lite::future::block_on(
                        executor.run(futures_lite::future::pending::<()>()),
                    )
                })
                .expect("the background executor can start a thread");
        }
        executor
    })
}

/// The UI thread's run queue. Wakers may fire on any thread; runnables run
/// only on the UI thread that owns the queue.
pub(crate) struct UiQueue {
    runnables: Mutex<VecDeque<Runnable>>,
    jobs: Mutex<Vec<Box<dyn FnOnce() + Send>>>,
    /// Wakes the event loop, while a native runtime owns this thread.
    waker: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    /// Signalled on every push, for [`run_until_idle`].
    ready: Condvar,
    /// UI tasks spawned and not yet finished or dropped.
    live: AtomicUsize,
}

impl UiQueue {
    fn push(&self, runnable: Runnable) {
        self.runnables
            .lock()
            .expect("the UI run queue lock is not poisoned")
            .push_back(runnable);
        self.notify();
    }

    pub(crate) fn push_job(&self, job: Box<dyn FnOnce() + Send>) {
        self.jobs
            .lock()
            .expect("the UI job queue lock is not poisoned")
            .push(job);
        self.notify();
    }

    fn notify(&self) {
        self.ready.notify_all();
        let waker = self
            .waker
            .lock()
            .expect("the UI waker lock is not poisoned")
            .clone();
        if let Some(wake) = waker {
            wake();
        }
    }

    fn is_empty(&self) -> bool {
        self.runnables
            .lock()
            .expect("the UI run queue lock is not poisoned")
            .is_empty()
            && self
                .jobs
                .lock()
                .expect("the UI job queue lock is not poisoned")
                .is_empty()
    }
}

thread_local! {
    static UI_QUEUE: Arc<UiQueue> = Arc::new(UiQueue {
        runnables: Mutex::new(VecDeque::new()),
        jobs: Mutex::new(Vec::new()),
        waker: Mutex::new(None),
        ready: Condvar::new(),
        live: AtomicUsize::new(0),
    });
}

pub(crate) fn ui_queue() -> Arc<UiQueue> {
    UI_QUEUE.with(Arc::clone)
}

/// Route this thread's UI wakeups to the event loop, or stop doing so.
pub(crate) fn set_loop_waker(waker: Option<Arc<dyn Fn() + Send + Sync>>) {
    let queue = ui_queue();
    let pending = !queue.is_empty();
    *queue
        .waker
        .lock()
        .expect("the UI waker lock is not poisoned") = waker;
    if pending {
        queue.notify();
    }
}

/// How long one turn of UI tasks may run before input and rendering get
/// their turn.
const TURN_BUDGET: Duration = Duration::from_millis(8);

/// Run queued UI work for one bounded turn. Returns whether more is queued.
pub(crate) fn run_turn() -> bool {
    let queue = ui_queue();
    let deadline = Instant::now() + TURN_BUDGET;
    let jobs = std::mem::take(
        &mut *queue
            .jobs
            .lock()
            .expect("the UI job queue lock is not poisoned"),
    );
    for job in jobs {
        job();
    }
    loop {
        let runnable = queue
            .runnables
            .lock()
            .expect("the UI run queue lock is not poisoned")
            .pop_front();
        let Some(runnable) = runnable else {
            break;
        };
        runnable.run();
        if Instant::now() >= deadline {
            break;
        }
    }
    !queue.is_empty()
}

/// Run UI tasks until none is queued or alive, waiting for the timers and
/// background tasks they await. Fails after `timeout` if tasks remain.
pub(crate) fn run_until_idle(timeout: Duration) -> Result<(), usize> {
    let queue = ui_queue();
    let deadline = Instant::now() + timeout;
    loop {
        while run_turn() {}
        let live = queue.live.load(Ordering::Acquire);
        if live == 0 {
            return Ok(());
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(live);
        }
        let guard = queue
            .runnables
            .lock()
            .expect("the UI run queue lock is not poisoned");
        if guard.is_empty() {
            let _ = queue
                .ready
                .wait_timeout(guard, (deadline - now).min(Duration::from_millis(50)))
                .expect("the UI run queue lock is not poisoned");
        }
    }
}

/// Counts a UI task as alive until its future is dropped.
struct Live(Arc<UiQueue>);

impl Live {
    fn new(queue: &Arc<UiQueue>) -> Self {
        queue.live.fetch_add(1, Ordering::AcqRel);
        Self(queue.clone())
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.0.live.fetch_sub(1, Ordering::AcqRel);
    }
}

#[derive(Default)]
struct CancelState {
    cancelled: std::cell::Cell<bool>,
    waker: std::cell::RefCell<Option<Waker>>,
}

impl CancelState {
    fn cancel(&self) {
        self.cancelled.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

/// A UI future that its scope can cancel: it then drops the inner future
/// and finishes with `None`.
struct Cancellable<F> {
    future: Option<Pin<Box<F>>>,
    cancel: Rc<CancelState>,
    _live: Live,
}

impl<F: Future> Future for Cancellable<F> {
    type Output = Option<F::Output>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.cancel.cancelled.get() {
            self.future = None;
            return Poll::Ready(None);
        }
        let Some(future) = self.future.as_mut() else {
            return Poll::Ready(None);
        };
        match future.as_mut().poll(cx) {
            Poll::Ready(value) => {
                self.future = None;
                Poll::Ready(Some(value))
            }
            Poll::Pending => {
                *self.cancel.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
