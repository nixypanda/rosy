#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rosy::test_runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use lazy_static::lazy_static;
use rosy::{
    screen_printing::WRITER,
    vga::{Color, ColorCode, ScreenChar, ScreenLocation},
    x86_64::{
        address::{PhysicalAddress, VirtualAddress},
        interrupts,
        paging::{
            OffsetMemoryMapper, Page, PageFrame, PageFrameInner, PageInner, PageTableEntryFlags,
        },
    },
};

const BLACK_ON_WHITLE_STRING_SIZE: usize = 4;
const VGA_BUFFER_START_LOCATION: u64 = 0xb8000;
const BLACK_ON_WHITLE_STRING: u64 = 0x_f021_f077_f065_f04e;

fn black_on_white_string() -> [ScreenChar; BLACK_ON_WHITLE_STRING_SIZE] {
    let color = ColorCode::new(Color::Black, Color::White);
    [
        ScreenChar::new('N', color),
        ScreenChar::new('e', color),
        ScreenChar::new('w', color),
        ScreenChar::new('!', color),
    ]
}

struct TestFixture {
    physical_memory_offset: VirtualAddress,
    memory_mapper: OffsetMemoryMapper,
}

struct FixtureWrapper {
    fixture: Option<TestFixture>,
}

impl Default for FixtureWrapper {
    fn default() -> Self {
        FixtureWrapper {
            fixture: Option::default(),
        }
    }
}

lazy_static! {
    static ref TEST_FIXTURE: spin::Mutex<FixtureWrapper> =
        spin::Mutex::new(FixtureWrapper::default());
}

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    use rosy::x86_64::paging::FrameAllocator;
    rosy::init();

    let physical_memory_offset = VirtualAddress::new(boot_info.physical_memory_offset);
    let frame_allocator = unsafe { FrameAllocator::new(&boot_info.memory_map) };
    let memory_mapper = unsafe { OffsetMemoryMapper::new(physical_memory_offset, frame_allocator) };

    {
        let mut test_fixture = TEST_FIXTURE.lock();
        test_fixture.fixture = Some(TestFixture {
            physical_memory_offset,
            memory_mapper,
        });
    }

    test_main();
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rosy::test_panic_handler(info);
}

#[test_case]
fn test_virtual_address_translation_works() {
    let test_fixture = TEST_FIXTURE.lock();
    let test_fixture = test_fixture.fixture.as_ref().unwrap();

    let addresses = [
        // the identity-mapped vga buffer page
        (VGA_BUFFER_START_LOCATION, VGA_BUFFER_START_LOCATION),
        // some code page
        (0x210281, 0x40f281),
        // some stack page
        // (0x0100_0020_1958, 0x27e958),
        // virtual address mapped to physical address 0
        (test_fixture.physical_memory_offset.as_u64(), 0x00),
    ];

    for (virtual_address, physical_address) in addresses {
        let virt = VirtualAddress::new(virtual_address);
        let expected_physical_address = PhysicalAddress::new(physical_address);
        let actual_physical_address = test_fixture.memory_mapper.translate_address(virt).unwrap();
        assert_eq!(expected_physical_address, actual_physical_address);
    }
}

/// This test works by writing to a specific memory location that is mapped to the screen. Then we
/// validate by checking if it was written proprely.
#[test_case]
fn test_verify_page_mapping_works() {
    let mut test_fixture = TEST_FIXTURE.lock();
    let memory_mapper = &mut test_fixture.fixture.as_mut().unwrap().memory_mapper;

    let starting_memory_location: isize = 400;
    let string_start: ScreenLocation = ScreenLocation::new(20, 0);
    let string_end: ScreenLocation = ScreenLocation::new(20, 3);

    let page = Page::Normal(PageInner::containing_address(VirtualAddress::new(0x00)));
    let frame = PageFrame::Normal(PageFrameInner::containing_address(PhysicalAddress::new(
        VGA_BUFFER_START_LOCATION,
    )));
    let flags = PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE;
    unsafe { memory_mapper.map_to(page, frame, flags).unwrap() };

    interrupts::execute_without_interrupts(|| {
        // Write "New!" to the screen
        let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
        unsafe {
            page_ptr
                .offset(starting_memory_location)
                .write_volatile(BLACK_ON_WHITLE_STRING)
        };

        let writer = WRITER.lock();
        let screen_chars = writer.string_at(string_start, string_end);
        let expected_string = black_on_white_string();

        for (output, expected) in screen_chars.zip(expected_string.iter()) {
            assert_eq!(&output, expected);
        }
    })
}

/// This test works by writing to a specific memory location that is mapped to the screen. Then we
/// validate by checking if it was written proprely.
#[test_case]
fn test_verify_page_mapping_works_when_mapper_needs_to_create_new_pages() {
    let mut test_fixture = TEST_FIXTURE.lock();
    let memory_mapper = &mut test_fixture.fixture.as_mut().unwrap().memory_mapper;

    let starting_memory_location: isize = 404;
    let string_start: ScreenLocation = ScreenLocation::new(20, 16);
    let string_end: ScreenLocation = ScreenLocation::new(20, 20);


    let page = Page::Normal(PageInner::containing_address(VirtualAddress::new(
        0xdeadbeaf000,
    )));
    let frame = PageFrame::Normal(PageFrameInner::containing_address(PhysicalAddress::new(
        VGA_BUFFER_START_LOCATION,
    )));
    let flags = PageTableEntryFlags::PRESENT | PageTableEntryFlags::WRITABLE;
    unsafe { memory_mapper.map_to(page, frame, flags).unwrap() };

    interrupts::execute_without_interrupts(|| {
        // Write "New!" to the screen
        let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
        unsafe {
            page_ptr
                .offset(starting_memory_location)
                .write_volatile(BLACK_ON_WHITLE_STRING)
        };

        let writer = WRITER.lock();
        let screen_chars = writer.string_at(string_start, string_end);
        let expected_string = black_on_white_string();

        for (output, expected) in screen_chars.zip(expected_string.iter()) {
            assert_eq!(&output, expected);
        }
    })
}
