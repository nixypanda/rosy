#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rosy::{
    memory::active_level4_page_table,
    print, println,
    utils::halt_loop,
    x86_64::{
        addr::VirtualAddress,
        instructions::read_control_register_3,
        paging::{PageTable, PageTableFrame},
    },
};

entry_point!(kernel_main);

fn get_table_from_frame(
    frame: PageTableFrame,
    physical_memory_offset: u64,
) -> &'static mut PageTable {
    let physical_address = frame.start_address().as_u64() + physical_memory_offset;
    let virtual_address = VirtualAddress::new(physical_address);
    let table = unsafe { &mut *(virtual_address.as_mut_ptr() as *mut PageTable) };
    table
}

pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World!");

    rosy::init();

    let (base_page_table_address, _) = read_control_register_3();
    println!(
        "Base Address of the Page Table: {:?}",
        base_page_table_address
    );

    println!("{:?}", boot_info.physical_memory_offset);

    let physical_memory_offset = VirtualAddress::new(boot_info.physical_memory_offset);
    let l4_page_table: &PageTable = unsafe { active_level4_page_table(physical_memory_offset) };

    for (index, entry) in l4_page_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("L4 entry {}: {:?}", index, entry);

            let frame = entry.frame().unwrap();
            let table = get_table_from_frame(frame, boot_info.physical_memory_offset);

            for (index, entry) in table.iter().enumerate() {
                if !entry.is_unused() {
                    println!("|---L3 entry {}: {:?}", index, entry);
                }
            }
        }
    }

    #[cfg(test)]
    test_main();

    println!("It did not crash!");
    halt_loop();
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
