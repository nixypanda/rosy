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

// Take from https://github.com/tinaun/gen-iter/blob/master/src/lib.rs

use core::iter::Iterator;
use core::marker::Unpin;
use core::ops::{Generator, GeneratorState};
use core::pin::Pin;

/// a iterator that holds an internal generator representing
/// the iteration state
#[derive(Copy, Clone, Debug)]
pub struct GenIter<T>(pub T)
where
    T: Generator<Return = ()> + Unpin;

impl<T> Iterator for GenIter<T>
where
    T: Generator<Return = ()> + Unpin,
{
    type Item = T::Yield;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match Pin::new(&mut self.0).resume(()) {
            GeneratorState::Yielded(n) => Some(n),
            GeneratorState::Complete(()) => None,
        }
    }
}

impl<G> From<G> for GenIter<G>
where
    G: Generator<Return = ()> + Unpin,
{
    #[inline]
    fn from(gen: G) -> Self {
        GenIter(gen)
    }
}

/// macro to simplify iterator - via - generator construction
///
/// ```
/// #![feature(generators)]
///
/// use gen_iter::gen_iter;
///
/// let mut g = gen_iter!({
///     yield 1;
///     yield 2;
/// });
///
/// assert_eq!(g.next(), Some(1));
/// assert_eq!(g.next(), Some(2));
/// assert_eq!(g.next(), None);
///
/// ```
#[macro_export]
macro_rules! gen_iter {
    ($block: block) => {
        $crate::utils::GenIter(|| $block)
    };
    (move $block: block) => {
        $crate::utils::GenIter(move || $block)
    };
}
