use alloc::{boxed::Box, collections::VecDeque};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
}

pub struct Executor {
    tasks: VecDeque<Task>,
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Self {
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}

impl Executor {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    pub fn spawn(&mut self, task: Task) {
        self.tasks.push_back(task);
    }

    pub fn run(&mut self) {
        while let Some(mut task) = self.tasks.pop_front() {
            let waker = waker();
            let mut context = Context::from_waker(&waker);
            match task.poll(&mut context) {
                Poll::Ready(_) => {}
                Poll::Pending => {
                    self.tasks.push_back(task);
                }
            }
        }
    }
}

fn raw_waker() -> RawWaker {
    fn clone_no_op(_: *const ()) -> RawWaker {
        raw_waker()
    }
    fn wake_no_op(_: *const ()) {}
    fn wake_by_ref_no_op(_: *const ()) {}
    fn drop_no_op(_: *const ()) {}

    let vtable = &RawWakerVTable::new(clone_no_op, wake_no_op, wake_by_ref_no_op, drop_no_op);
    RawWaker::new(0 as *const (), vtable)
}

fn waker() -> Waker {
    unsafe { Waker::from_raw(raw_waker()) }
}
