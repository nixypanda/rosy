//! Various abstractions currently unorganised

use crate::x86_64::instructions::halt_cpu_till_next_interrupt;

/// Continously halt the cpu.
///
/// This effectively works like a busy loop but the upside is that it keeps the cpu usage to low.
pub fn halt_loop() -> ! {
    loop {
        halt_cpu_till_next_interrupt();
    }
}

/// The type is a generic wrapper around a [`spin::Mutex<A>`].
///
/// It imposes no restrictions on the wrapped type A, so it can be used to wrap all kinds of types,
/// not just allocators. It provides a simple new constructor function that wraps a given value.
/// For convenience, it also provides a lock function that calls lock on the wrapped Mutex
///
/// # Performance
/// The implementation of this type is based on the spin::Mutex type. The spin::Mutex type is a
/// sub-optimal solution for this problem.
pub struct Locked<A> {
    inner: spin::Mutex<A>,
}

impl<A> Locked<A> {
    /// Creates a new Locked<A> wrapper around the given value
    pub const fn new(inner: A) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    /// Locks the wrapped Mutex and returns a reference to the wrapped value
    pub fn lock(&self) -> spin::MutexGuard<A> {
        self.inner.lock()
    }
}
