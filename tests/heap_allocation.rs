#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rosy::allocator::HEAP_SIZE;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use rosy::allocator;
    use rosy::x86_64::{
        address::VirtualAddress,
        paging::{FrameAllocator, OffsetMemoryMapper},
    };

    rosy::init();
    let physical_memory_offset = VirtualAddress::new(boot_info.physical_memory_offset);
    let frame_allocator = unsafe { FrameAllocator::new(&boot_info.memory_map) };
    let mut mapper = unsafe { OffsetMemoryMapper::new(physical_memory_offset, frame_allocator) };
    allocator::init_heap(&mut mapper).expect("heap initialization failed");

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rosy::test_panic_handler(info)
}

#[test_case]
fn test_basic_heap_allocation_with_box_work() {
    let heap_value = Box::new(41);
    assert_eq!(41, *heap_value);
}

fn sum_of_first_n_numbers(n: u64) -> u64 {
    (n - 1) * n / 2
}

#[test_case]
fn test_large_heap_allocations_work() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), sum_of_first_n_numbers(n));
}

#[test_case]
fn test_allocator_uses_freed_memory_for_subsequent_allocations() {
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
}

#[test_case]
fn test_allocator_frees_up_memory_even_if_there_is_a_long_lived_allocation() {
    let long_lived = Box::new(1);
    for i in 0..HEAP_SIZE {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*long_lived, 1);
}
