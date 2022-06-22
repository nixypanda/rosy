//! Keyboard setup

use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{stream::Stream, task::AtomicWaker, StreamExt};

use crate::{
    print, println,
    ps2_keyboard_decoder::{ColemakDHm, DecodedKey, HandleControl, Keyboard, ScancodeSet1},
    warn,
};

/// Pre-allocated fixed size lock-free queue.
///
/// Note: We need a one time heap allocation for this to work. Rust can't do it yet for statics so
/// we use [`OnceCell`] to make sure this is initialized only once.
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Given a scancode it adds it to the global scancode queue for processing.
///
/// It requires the global static `SCANCODE_QUEUE` to be initialized to work properly.
///
/// Called by the keyboard interrupt handler must not block or allocate because we want to keep the
/// work done in an interrupt handler to be as low as possible. We don't want to block any
/// important work that is going on because the interrupt service routine took a lot of time.
pub fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            warn!("Warning: scancode queue full, dropping user input!");
            println!()
        } else {
            WAKER.wake();
        }
    } else {
        warn!("Warning: scancode queue uninitialized");
        println!();
    }
}

/// Wrapper around the static `SCANCODE_QUEUE`
pub struct ScancodeStream {
    // The purpose of the _private field is to prevent construction of the struct from outside of
    // the module. This makes the new function the only way to construct the type.
    _private: (),
}

impl ScancodeStream {
    /// Initialize the `SCANCODE_QUEUE` static
    ///
    /// # panics
    /// panics if it is already initialized to ensure that only a single ScancodeStream instance
    /// can be created.
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let queue = SCANCODE_QUEUE.try_get().expect("Not Initialized");

        WAKER.register(cx.waker());

        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

/// Print out keycods as the user it typing them
pub async fn print_keypresses() {
    let mut scansode_stream = ScancodeStream::new();
    let mut keyboard: Keyboard<ColemakDHm, ScancodeSet1> =
        Keyboard::new(ColemakDHm, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scansode_stream.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
