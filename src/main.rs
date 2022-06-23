#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rosy::{
    allocation,
    async_runtime::{Executor, Task},
    keyboard,
    memory::active_level4_page_table,
    print, println,
    utils::halt_loop,
    x86_64::{
        address::VirtualAddress,
        instructions::read_control_register_3,
        paging::{FrameAllocator, OffsetMemoryMapper, PageTable},
    },
};

entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    rosy::init();

    println!("Hello World!");

    let physical_memory_offset = VirtualAddress::new(boot_info.physical_memory_offset);
    let offset_memory_mapper: &mut OffsetMemoryMapper = unsafe {
        &mut OffsetMemoryMapper::new(
            physical_memory_offset,
            FrameAllocator::new(&boot_info.memory_map),
        )
    };
    allocation::init_heap(offset_memory_mapper).expect("heap initialization failed");

    print_level4_page_table_address();
    verify_level4_page_table_iteration(physical_memory_offset);
    execute_async_tasks();

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    halt_loop();
}

fn verify_level4_page_table_iteration(physical_memory_offset: VirtualAddress) {
    let l4_page_table: &PageTable = unsafe { active_level4_page_table(physical_memory_offset) };

    for (index, entry) in l4_page_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("L4 entry {}: {:?}", index, entry);
        }
    }
}

/// Reads the CR3 register and prints the virtual address of the Level 4 [`PageTable`]
///
/// Note: This is only for testing that the underlying reading from the CR3 register is happening
/// properly
fn print_level4_page_table_address() {
    let (base_page_table_address, _) = read_control_register_3();
    println!(
        "Base Address of the Page Table: {:?}",
        base_page_table_address
    );
}

async fn async_number() -> usize {
    42
}

async fn perform_async_printing() {
    let number = async_number().await;
    println!("async number {}", number);
}

fn execute_async_tasks() {
    let mut executor = Executor::new();
    executor.spawn(Task::new(perform_async_printing()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);

    loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rosy::test_panic_handler(info);
}
