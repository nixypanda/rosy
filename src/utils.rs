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
