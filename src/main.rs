#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rosy::{
    print, println,
    utils::halt_loop,
    x86_64::{instructions::read_control_register_3, paging::PageTable},
};

entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World!");

    rosy::init();

    let (base_page_table_address, _) = read_control_register_3();
    println!(
        "Base Address of the Page Table: {:?}",
        base_page_table_address
    );

    println!("{:?}", boot_info.physical_memory_offset);
    let base_page_table_virtual_address =
        base_page_table_address + boot_info.physical_memory_offset;

    println!("{:?}", base_page_table_virtual_address);

    let page_table: &PageTable = unsafe {
        let page_table_pointer: *mut PageTable = base_page_table_virtual_address.as_mut_ptr();
        &*page_table_pointer
    };

    for (index, entry) in page_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("{}: {:?}", index, entry);
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
