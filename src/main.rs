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
        address::{PhysicalAddress, VirtualAddress},
        instructions::read_control_register_3,
        paging::{
            OffsetMemoryMapper, Page, PageFrame, PageFrameInner, PageInner, PageTable,
            PageTableEntryFlags,
        },
    },
};

entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static BootInfo) -> ! {
    rosy::init();

    println!("Hello World!");

    println!(
        "Physical Memory Offset: {:?}",
        boot_info.physical_memory_offset
    );
    let physical_memory_offset = VirtualAddress::new(boot_info.physical_memory_offset);

    print_level4_page_table_address();
    verify_level4_page_table_iteration(physical_memory_offset);
    translate_a_bunch_of_virtual_addresses(physical_memory_offset);
    verify_page_mapping_works(physical_memory_offset);

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

/// Translaets a few virtual addresses to physical addresses using the [`OffsetMemoryMapper`].
///
/// Note: This is only to test the underlying implementation of mapping [`VirtualAddress`] to
/// [`PhysicalAddress`] is wokring properly.
fn translate_a_bunch_of_virtual_addresses(physical_memory_offset: VirtualAddress) {
    let offset_memory_mapper = unsafe { OffsetMemoryMapper::new(physical_memory_offset) };

    let addresses = [
        // the identity-mapped vga buffer page
        0xb8000,
        // some code page
        0x210281,
        // some stack page
        0x0100_0020_1958,
        // virtual address mapped to physical address 0
        physical_memory_offset.as_u64(),
    ];

    for &address in &addresses {
        let virt = VirtualAddress::new(address);
        let phys = offset_memory_mapper.translate_address(virt);
        println!("{:?} -> {:?}", virt, phys);
    }
}

fn verify_page_mapping_works(physical_memory_offset: VirtualAddress) {
    let offset_memory_mapper = unsafe { OffsetMemoryMapper::new(physical_memory_offset) };

    let page = Page::Normal(PageInner::containing_address(VirtualAddress::new(0x00)));
    let frame = PageFrame::Normal(PageFrameInner::containing_address(PhysicalAddress::new(
        0xb8000,
    )));
    let flags = PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE;
    offset_memory_mapper.map_to(page, frame, flags).unwrap();

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };
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
