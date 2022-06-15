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
        address::VirtualAddress,
        instructions::read_control_register_3,
        paging::{OffsetMemoryMapper, PageTable},
    },
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

    let physical_memory_offset = VirtualAddress::new(boot_info.physical_memory_offset);
    let l4_page_table: &PageTable = unsafe { active_level4_page_table(physical_memory_offset) };

    for (index, entry) in l4_page_table.iter().enumerate() {
        if !entry.is_unused() {
            println!("L4 entry {}: {:?}", index, entry);
        }
    }

    let phys_mem_offset = VirtualAddress::new(boot_info.physical_memory_offset);
    let offset_memory_mapper = OffsetMemoryMapper::new(phys_mem_offset);

    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x210281,
        // some stack page
        0x0100_0020_1958,
        // virtual address mapped to physical address 0
        boot_info.physical_memory_offset,
    ];

    for &address in &addresses {
        let virt = VirtualAddress::new(address);
        let phys = offset_memory_mapper.translate_address(virt);
        println!("{:?} -> {:?}", virt, phys);
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
