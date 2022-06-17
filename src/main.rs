#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, rc::Rc, vec, vec::Vec};
use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use rosy::{
    allocator,
    memory::active_level4_page_table,
    print, println,
    utils::halt_loop,
    x86_64::{
        address::{PhysicalAddress, VirtualAddress},
        instructions::read_control_register_3,
        paging::{
            FrameAllocator, OffsetMemoryMapper, Page, PageFrame, PageFrameInner, PageInner,
            PageTable, PageTableEntryFlags,
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
    let offset_memory_mapper: &mut OffsetMemoryMapper = unsafe {
        &mut OffsetMemoryMapper::new(
            physical_memory_offset,
            FrameAllocator::new(&boot_info.memory_map),
        )
    };
    allocator::init_heap(offset_memory_mapper).expect("heap initialization failed");

    print_level4_page_table_address();
    verify_level4_page_table_iteration(physical_memory_offset);
    translate_a_bunch_of_virtual_addresses(physical_memory_offset, &offset_memory_mapper);
    verify_page_mapping_works(offset_memory_mapper);
    map_page_which_requires_frame_allocation(offset_memory_mapper);
    perform_heap_allocated_operations();

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
fn translate_a_bunch_of_virtual_addresses(
    physical_memory_offset: VirtualAddress,
    offset_memory_mapper: &OffsetMemoryMapper,
) {
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

fn verify_page_mapping_works(offset_memory_mapper: &mut OffsetMemoryMapper) {
    let page = Page::Normal(PageInner::containing_address(VirtualAddress::new(0x00)));
    let frame = PageFrame::Normal(PageFrameInner::containing_address(PhysicalAddress::new(
        0xb8000,
    )));
    let flags = PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE;
    unsafe { offset_memory_mapper.map_to(page, frame, flags).unwrap() };

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };
}

fn map_page_which_requires_frame_allocation(offset_memory_mapper: &mut OffsetMemoryMapper) {
    let page = Page::Normal(PageInner::containing_address(VirtualAddress::new(
        0xdeadbeaf000,
    )));
    let frame = PageFrame::Normal(PageFrameInner::containing_address(PhysicalAddress::new(
        0xb8000,
    )));
    let flags = PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE;
    unsafe { offset_memory_mapper.map_to(page, frame, flags).unwrap() };

    let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    unsafe { page_ptr.offset(416).write_volatile(0x_f021_f077_f065_f04e) };
}

fn perform_heap_allocated_operations() {
    // allocate a number on the heap
    let heap_value = Box::new(41);
    println!("heap_value at {:p}", heap_value);

    // create a dynamically sized vector
    let mut vec = Vec::new();
    for i in 0..500 {
        vec.push(i);
    }
    println!("vec at {:p}", vec.as_slice());

    // create a reference counted vector -> will be freed when count reaches 0
    let reference_counted = Rc::new(vec![1, 2, 3]);
    let cloned_reference = reference_counted.clone();
    println!(
        "current reference count is {}",
        Rc::strong_count(&cloned_reference)
    );
    core::mem::drop(reference_counted);
    println!(
        "reference count is {} now",
        Rc::strong_count(&cloned_reference)
    );
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
