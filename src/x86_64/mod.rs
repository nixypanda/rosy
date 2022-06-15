//! x86_64 specific functions and data structures, and access to various system registers.

pub mod address;
pub mod descriptor;
pub mod gdt;
pub mod idt;
pub mod instructions;
pub mod interrupts;
pub mod paging;
pub mod port;
pub mod privilege_level;
pub mod rflags;
pub mod segmentation;
pub mod tss;
