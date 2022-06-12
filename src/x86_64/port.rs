use core::{arch::asm, marker::PhantomData};

/// On x86, I/O ports operate on
/// - `u8` (via `inb`/`outb`)
/// - `u16` (via `inw`/`outw`),
/// - `u32` (via `inl`/`outl`).
///
/// This trait is implemented for exactly these types.
pub trait PortRead {
    unsafe fn read_from_port(port: u16) -> Self;
}

/// On x86, I/O ports operate on
/// - `u8` (via `inb`/`outb`)
/// - `u16` (via `inw`/`outw`),
/// - `u32` (via `inl`/`outl`).
///
/// This trait is implemented for exactly these types.
pub trait PortWrite {
    unsafe fn write_to_port(port: u16, value: Self);
}

impl PortRead for u8 {
    unsafe fn read_from_port(port: u16) -> Self {
        let value: u8;
        asm!("in al, dx", out("al") value, in("dx") port, options(nomem, nostack, preserves_flags));
        value
    }
}

impl PortWrite for u8 {
    unsafe fn write_to_port(port: u16, value: Self) {
        asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack, preserves_flags));
    }
}

pub struct Port<T> {
    port: u16,
    phantom: PhantomData<T>,
}

impl<T> Port<T> {
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            phantom: PhantomData,
        }
    }
}

impl<T> Port<T>
where
    T: PortRead,
{
    pub unsafe fn read(&self) -> T {
        T::read_from_port(self.port)
    }
}

impl<T> Port<T>
where
    T: PortWrite,
{
    pub unsafe fn write(&self, value: T) {
        T::write_to_port(self.port, value)
    }
}
