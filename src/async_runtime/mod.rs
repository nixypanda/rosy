use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, task::Wake};
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll, Waker},
};
use crossbeam_queue::ArrayQueue;

use crate::x86_64::interrupts;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
const DEFAULT_TASK_QUEUE_SIZE: usize = 100;

pub struct Task<'a> {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()> + 'a>>,
}

pub struct Executor<'a> {
    tasks: BTreeMap<TaskId, Task<'a>>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

struct TaskWaker {
    id: TaskId,
    queue: Arc<ArrayQueue<TaskId>>,
}

impl<'a> Task<'a> {
    pub fn new(future: impl Future<Output = ()> + 'a) -> Self {
        Self {
            id: TaskId::new(),
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

impl TaskId {
    fn new() -> Self {
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl TaskWaker {
    fn new(id: TaskId, queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker { id, queue }))
    }

    fn wake_task(&self) {
        self.queue.push(self.id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }
}

impl<'a> Executor<'a> {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(DEFAULT_TASK_QUEUE_SIZE)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: Task<'a>) {
        let task_id = task.id;
        if self.tasks.insert(task.id, task).is_some() {
            panic!("Task with same id already present {:?}", task_id);
        }
        self.task_queue.push(task_id).expect("queue full");
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    pub fn run_ready_tasks(&mut self) {
        while let Some(task_id) = self.task_queue.pop() {
            let task = match self.tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };

            let waker = self
                .waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, self.task_queue.clone()));
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(_) => {
                    self.tasks.remove(&task_id);
                    self.waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    fn sleep_if_idle(&self) {
        interrupts::disable();
        if self.task_queue.is_empty() {
            interrupts::enable_and_halt_cpu_till_next_one();
        } else {
            interrupts::enable();
        }
    }
}
